//! Naive Z80 codegen (Stage 1). `HL` is the working accumulator, `DE` secondary;
//! locals (incl. parameters) live in a fixed RAM scratch region (the "virtual
//! register file") and expressions evaluate via the stack. Functions follow the
//! spec-07 calling convention; `*`/`/`/`%` call an appended micro-runtime.
//! Codegen emits the symbolic [`ins::Ins`] stream (the Stage 2 seam); encoding to
//! bytes happens once at the end — see `ins.rs`.

use crate::ir::*;
use crate::lower::consts::DataConst;
use std::collections::HashMap;

mod asm;
mod expr;
mod ins;
mod peephole;
mod runtime;
mod stmt;

use asm::Asm;
use ins::{Imm, R16};
use stmt::{gen_return, gen_stmt};

/// Code-generation target. `Spectrum48` is authentic Z80 — `*`/`/`/`%` use the appended
/// software micro-runtime, so the output runs anywhere (real ROM, `.tap`). `Cell` is the
/// micro-VM (the `cell80` crate): those ops lower to the `ED FE` host-trap (serviced natively
/// by the cell bus — see the Cell80 plan), so no software runtime is appended. `ED FE` is
/// a no-op on real hardware, so it never reaches a real game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Spectrum48,
    Cell,
}

/// Compile a whole program (functions laid out in order, micro-runtime appended).
///
/// If `entry` is set, a tiny `DI; CALL entry; EI; RET` trampoline is emitted **at
/// `org`** so callers can `USR org`. The `DI` matters: the compiler keeps live
/// values in `DE`/`BC` across instructions, but the Spectrum's interrupt routine
/// clobbers `BC`/`DE` (its keyboard scan), so an interrupt mid-computation would
/// corrupt arithmetic. Disabling interrupts for the run avoids that; `EI` restores
/// them before returning to BASIC.
pub fn codegen_program(
    funcs: &[(String, Func)],
    org: u16,
    entry: Option<&str>,
    target: Target,
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    codegen_program_c(funcs, &[], org, entry, target)
}

/// [`codegen_program`] with a **const-data section**: after the functions, every
/// const the program references (`Expr::ConstAddr`) is laid into the image at its
/// own symbol; unreferenced consts are dropped (nothing could address them).
pub(crate) fn codegen_program_c(
    funcs: &[(String, Func)],
    consts: &[DataConst],
    org: u16,
    entry: Option<&str>,
    target: Target,
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut a = Asm::new(org, target);
    if let Some(e) = entry {
        a.fx(&[0xF3]); // DI
        a.call(e); // CALL entry
        a.fx(&[0xFB]); // EI
        a.fx(&[0xC9]); // RET
    }
    let mut base = 0u16;
    for (name, func) in funcs {
        a.define(name);
        a.base = base;
        emit_func(&mut a, func);
        base += func.n_locals as u16;
    }
    emit_const_data(&mut a, funcs, consts);
    let (code, symbols) = a.finish()?;
    // Locals live at the fixed `SCRATCH` base (slot `i` at `SCRATCH + i*2`), *above* the code.
    // If the emitted code (incl. the appended runtime) grows up into that region, the per-call
    // slot writes silently corrupt machine code — the same class of bug `codegen_loop` guards.
    // Fail loudly instead of emitting a wrong image. (The frame loop uses code-relative scratch
    // and its own `state_base` ceiling; this is the fixed-`SCRATCH` whole-program path.)
    let code_end = org as u32 + code.len() as u32;
    if code_end > asm::SCRATCH as u32 {
        return Err(format!(
            "rustz80: program too large — code ends at {code_end:#06x}, overrunning the locals \
             scratch region at {:#06x}",
            asm::SCRATCH
        ));
    }
    let total_slots: u32 = funcs.iter().map(|(_, f)| f.n_locals as u32).sum();
    if asm::SCRATCH as u32 + total_slots * 2 > 0x1_0000 {
        return Err(format!(
            "rustz80: too many locals — {total_slots} slots from {:#06x} overrun the 64 KiB \
             address space",
            asm::SCRATCH
        ));
    }
    Ok((code, symbols))
}

/// A generic **frame-synced entry loop** at `org`: zero a `state_bytes` region at
/// `state_base`, then each interrupt do `EI; HALT; DI; CALL entry(state_base, 0, 0);
/// JP loop` — interrupts on only for the `HALT` frame-sync, off during `entry` (so
/// its arithmetic isn't corrupted by the ROM's keyboard scan). The compiler knows
/// nothing about "games": `entry`, `state_base`, and `state_bytes` are the caller's.
pub fn codegen_loop(
    funcs: &[(String, Func)],
    org: u16,
    entry: &str,
    state_base: u16,
    state_bytes: u16,
) -> Result<Vec<u8>, String> {
    codegen_loop_c(funcs, &[], org, entry, state_base, state_bytes)
}

/// [`codegen_loop`] with a **const-data section** (see [`codegen_program_c`]) —
/// the data lays after the pruned functions, *below* the placed scratch region, so
/// the size guard covers code + data + locals against `state_base`.
pub(crate) fn codegen_loop_c(
    funcs: &[(String, Func)],
    consts: &[DataConst],
    org: u16,
    entry: &str,
    state_base: u16,
    state_bytes: u16,
) -> Result<Vec<u8>, String> {
    // Inline single-call-site helpers, then DCE.
    let inlined = crate::inline::inline(funcs.to_vec(), &[entry]);
    let pruned = crate::dce::prune(inlined, &[entry]);

    // Place the locals scratch region *just above the emitted code* rather than at a fixed
    // address, so a large program's code can't grow into its own locals (which silently
    // corrupted execution). Slot operands stay symbolic in the instruction stream and the
    // encoded length is independent of the scratch *value* (slot refs are always 2-byte
    // immediates), so one emission suffices: measure, place scratch, encode.
    let mut a = emit_loop(&pruned, consts, org, entry, state_base, state_bytes);
    a.seal();
    let code_end = org.wrapping_add(a.encoded_len());
    let scratch = (code_end + 1) & !1; // round up to a u16 boundary
    let total_slots: u32 = pruned.iter().map(|(_, f)| f.n_locals as u32).sum();
    let scratch_top = scratch as u32 + total_slots * 2;
    if scratch_top > state_base as u32 {
        return Err(format!(
            "rustz80: program too large — code ends at {code_end:#06x} and {total_slots} locals \
             (to {scratch_top:#06x}) would overrun the state region at {state_base:#06x}"
        ));
    }
    a.scratch = scratch;
    Ok(a.finish()?.0)
}

/// Emit the frame-loop preamble + the (already inlined/pruned) functions. Scratch stays
/// symbolic — [`codegen_loop`] measures the stream, then encodes with scratch placed.
fn emit_loop(
    pruned: &[(String, Func)],
    consts: &[DataConst],
    org: u16,
    entry: &str,
    state_base: u16,
    state_bytes: u16,
) -> Asm {
    // Games are authentic Z80 (real ROM); always the Spectrum target.
    let mut a = Asm::new(org, Target::Spectrum48);
    a.fx(&[0xF3]); // DI
                   // Zero the state region (memset via LD (HL),0 + LDIR).
    if state_bytes >= 2 {
        a.ld_imm(R16::Hl, Imm::Abs(state_base)); // LD HL, STATE
        a.fx(&[0x36, 0x00]); // LD (HL), 0
        a.ld_imm(R16::De, Imm::Abs(state_base + 1)); // LD DE, STATE+1
        a.ld_imm(R16::Bc, Imm::Abs(state_bytes - 1)); // LD BC, n-1
        a.fx(&[0xED, 0xB0]); // LDIR
    } else if state_bytes == 1 {
        a.ld_imm(R16::Hl, Imm::Abs(state_base));
        a.fx(&[0x36, 0x00]);
    }
    let loop_l = a.label();
    a.place(loop_l);
    a.fx(&[0xFB]); // EI
    a.fx(&[0x76]); // HALT     (wait for the 50 Hz frame interrupt)
    a.fx(&[0xF3]); // DI
    a.ld_imm(R16::Hl, Imm::Abs(state_base)); // LD HL, &state   (first arg)
    a.ld_imm(R16::De, Imm::Abs(0)); // LD DE, 0   (second arg, unused)
    a.ld_imm(R16::Bc, Imm::Abs(0)); // LD BC, 0   (third arg, unused)
    a.call(entry); // CALL entry
    a.jump(0xC3, loop_l); // JP loop

    let mut base = 0u16;
    for (name, func) in pruned {
        a.define(name);
        a.base = base;
        emit_func(&mut a, func);
        base += func.n_locals as u16;
    }
    emit_const_data(&mut a, pruned, consts);
    a
}

/// Lay the referenced const data into the image: for each const some kept function
/// addresses (via `Expr::ConstAddr`), define its symbol and append its packed bytes.
/// Sits after the last function's `RET` (never fallen into) and inside the measured
/// stream, so scratch placement and the size guards account for it.
fn emit_const_data(a: &mut Asm, funcs: &[(String, Func)], consts: &[DataConst]) {
    if consts.is_empty() {
        return;
    }
    let used = crate::dce::const_refs(funcs);
    for d in consts.iter().filter(|d| used.contains(&d.name)) {
        a.define(&d.name);
        a.data_bytes(d.bytes.clone());
    }
}

fn emit_func(a: &mut Asm, f: &Func) {
    // Prologue: copy parameters from the convention registers into their slots.
    for i in 0..f.params {
        let slot = a.slot(i);
        match i {
            0 => a.st_hl_mem(slot),            // LD (slot), HL
            1 => a.st_wide_mem(R16::De, slot), // LD (slot), DE
            2 => a.st_wide_mem(R16::Bc, slot), // LD (slot), BC
            _ => unreachable!(),
        }
    }
    // The epilogue label — `return` jumps here. The body and tail fall through to
    // it; an early `return` skips the tail (its value is already in `HL`).
    let end = a.label();
    a.func_end = Some(end);
    for s in &f.body {
        gen_stmt(a, s);
    }
    gen_return(a, &f.ret);
    a.place(end);
    a.func_end = None;
    a.fx(&[0xC9]); // RET
}
