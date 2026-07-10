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
    codegen_program_c(funcs, &[], org, entry, target, &HashMap::new())
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
    externs: &HashMap<String, u16>,
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut a = Asm::new(org, target);
    a.wide_sigs = wide_sig_map(funcs);
    if !externs.is_empty() {
        // Extern (bank) call boundaries: seed the wide-call shapes for names with
        // no local definition, and the absolute addresses for encode.
        for (name, sig) in crate::softfloat::BANK_WIDE_SIGS {
            a.wide_sigs.entry(name.to_string()).or_insert(*sig);
        }
        a.externs = externs.clone();
    }
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
    a.seal();
    // Locals live *above* the code. The classic base is the descriptor's `scratch`
    // (`0x9000`) — kept whenever the code fits below it, so every historical image
    // stays byte-identical — but a larger program (a multi-kernel f32 cell) places
    // scratch just past its own code instead of failing at the historical window.
    // Slot operands stay symbolic in the stream and encode as 2-byte immediates
    // regardless of value, so one emission suffices: measure, place scratch, encode
    // (the same move `codegen_loop_c` has always made against its `state_base`).
    let desc = target.descriptor();
    let code_end = org as u32 + a.encoded_len() as u32;
    let scratch = if code_end <= desc.scratch as u32 {
        desc.scratch as u32
    } else {
        (code_end + 1) & !1 // round up to a u16 slot boundary
    };
    let ceiling: u32 = desc.ceiling as u32;
    let total_slots: u32 = funcs.iter().map(|(_, f)| f.n_locals as u32).sum();
    let scratch_top = scratch + total_slots * 2;
    if scratch_top > ceiling {
        return Err(format!(
            "rustz80: program too large — code ends at {code_end:#06x} and {total_slots} local \
             slots (scratch {scratch:#06x}..{scratch_top:#06x}) overrun the {ceiling:#06x} ceiling"
        ));
    }
    a.scratch = scratch as u16;
    let (code, symbols) = a.finish()?;
    Ok((code, symbols))
}

/// Compile a self-contained routine bank: `funcs` at `org` with locals at a
/// **fixed** private scratch base (the caller guarantees the region is disjoint
/// from any calling program's own scratch). No entry preamble — every fn is a
/// plain `CALL` target; the symbol map is the bank's public interface.
pub(crate) fn codegen_bank(
    funcs: &[(String, Func)],
    org: u16,
    scratch: u16,
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut a = Asm::new(org, Target::Cell);
    a.wide_sigs = wide_sig_map(funcs);
    let mut base = 0u16;
    for (name, func) in funcs {
        a.define(name);
        a.base = base;
        emit_func(&mut a, func);
        base += func.n_locals as u16;
    }
    a.seal();
    let total_slots: u32 = funcs.iter().map(|(_, f)| f.n_locals as u32).sum();
    if scratch as u32 + total_slots * 2 > org as u32 {
        return Err(format!(
            "bank locals ({total_slots} slots from {scratch:#06x}) overrun the bank code at {org:#06x}"
        ));
    }
    a.scratch = scratch;
    a.finish()
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
    a.wide_sigs = wide_sig_map(pruned);
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

/// The call-boundary map codegen lays calls by (see `Asm::wide_sigs`).
fn wide_sig_map(funcs: &[(String, Func)]) -> HashMap<String, (bool, bool, bool)> {
    funcs
        .iter()
        .filter(|(_, f)| f.wide_param || f.wide_ret)
        .map(|(n, f)| (n.clone(), (f.wide_param, f.wide_second, f.wide_ret)))
        .collect()
}

fn emit_func(a: &mut Asm, f: &Func) {
    a.func_ret_wide = f.wide_ret;
    if f.wide_second {
        // Two-wide prologue: the first u32 arrives in HL:DE (slots 0-1); the
        // second sits on the stack under the return address (low on top — the
        // caller pushed high then low). Storing HL/DE first frees DE to hold
        // the return address across the pops; BC (an optional third u16 param)
        // stays untouched. Callee-popped: the caller does no cleanup.
        a.st_hl_mem(a.slot(0)); //             LD (slot0), HL
        a.st_wide_mem(R16::De, a.slot(1)); //  LD (slot1), DE
        a.pop(R16::De); //                     POP DE      (return address)
        a.pop(R16::Hl);
        a.st_hl_mem(a.slot(2)); //             LD (slot2), HL   (arg1.low)
        a.pop(R16::Hl);
        a.st_hl_mem(a.slot(3)); //             LD (slot3), HL   (arg1.high)
        a.push(R16::De); //                    PUSH DE     (return address back)
        if f.params == 5 {
            a.st_wide_mem(R16::Bc, a.slot(4)); // the third, 16-bit, param
        }
    } else {
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
