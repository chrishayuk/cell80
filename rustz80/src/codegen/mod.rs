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

// The target enum lives with the descriptors in `cell80-core` (A5); re-exported
// here where it has always been importable from.
pub use cell80_core::Target;

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
    reject_signed32(funcs)?;
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
    reject_signed32(funcs)?;
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
    reject_signed32(&pruned)?;

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

/// Phase 5 A3: `i32` lowers to the IR and runs on the reference interpreter, but no
/// machine backend emits the **signed-32** operations yet (WS-B gives RV32
/// `slt`/signed div natively; Z80 would need flag gymnastics the robo targets don't
/// justify). Only the ops whose signedness changes the bits are gated — signed
/// add/sub/mul/bitwise share the unsigned patterns and pass through untouched.
fn find_signed32(funcs: &[(String, Func)]) -> Option<&'static str> {
    fn in_expr(e: &Expr) -> Option<&'static str> {
        match e {
            Expr::Cmp32 { signed: true, .. } => return Some("a signed 32-bit comparison"),
            Expr::Bin32(BinOp::Div | BinOp::Rem, .., true) => {
                return Some("a signed 32-bit divide")
            }
            Expr::Shift32 {
                left: false,
                signed: true,
                ..
            } => return Some("an arithmetic 32-bit shift"),
            _ => {}
        }
        match e {
            Expr::Lit(_)
            | Expr::Var(_)
            | Expr::AddrOf(_)
            | Expr::ConstAddr(_)
            | Expr::Lit32(_)
            | Expr::Var32(_) => None,
            Expr::Bin(_, l, r, _) | Expr::Bin32(_, l, r, _) => in_expr(l).or_else(|| in_expr(r)),
            Expr::Cmp { lhs, rhs, .. }
            | Expr::Cmp32 { lhs, rhs, .. }
            | Expr::Logic { lhs, rhs, .. } => in_expr(lhs).or_else(|| in_expr(rhs)),
            Expr::ShiftVar { e, amount, .. } => in_expr(e).or_else(|| in_expr(amount)),
            Expr::PtrIndex { ptr, index, .. } => in_expr(ptr).or_else(|| in_expr(index)),
            Expr::Index(_, i, _) => in_expr(i),
            Expr::Call(_, args) => args.iter().find_map(in_expr),
            Expr::Trunc(x)
            | Expr::Trunc32(x)
            | Expr::Widen(x)
            | Expr::SignExtend(x)
            | Expr::Peek(x)
            | Expr::InPort(x)
            | Expr::Halt(x)
            | Expr::MulConst(x, _)
            | Expr::LoadAt(x, _)
            | Expr::Deref(x, _)
            | Expr::Deref32(x, _)
            | Expr::Shift32 { e: x, .. } => in_expr(x),
        }
    }
    fn in_stmt(s: &Stmt) -> Option<&'static str> {
        match s {
            Stmt::Assign(_, e)
            | Stmt::Assign32(_, e)
            | Stmt::Poke(_, e)
            | Stmt::Eval(e)
            | Stmt::AssignTuple(_, e) => in_expr(e),
            Stmt::StoreIndex(_, i, v, _) => in_expr(i).or_else(|| in_expr(v)),
            Stmt::Store(p, _, v) | Stmt::Store32(p, _, v) | Stmt::StoreAt(p, v, _) => {
                in_expr(p).or_else(|| in_expr(v))
            }
            Stmt::PtrStoreIndex {
                ptr, index, value, ..
            } => in_expr(ptr)
                .or_else(|| in_expr(index))
                .or_else(|| in_expr(value)),
            Stmt::Fill { value, .. } => in_expr(value),
            Stmt::If(c, t, e) => in_cond(c)
                .or_else(|| t.iter().find_map(in_stmt))
                .or_else(|| e.iter().find_map(in_stmt)),
            Stmt::While(c, body) => in_cond(c).or_else(|| body.iter().find_map(in_stmt)),
            Stmt::Loop(body) => body.iter().find_map(in_stmt),
            Stmt::ForRange { end, body, .. } => {
                in_expr(end).or_else(|| body.iter().find_map(in_stmt))
            }
            Stmt::Return(v) => v.as_ref().and_then(in_expr),
            Stmt::Break | Stmt::Continue => None,
        }
    }
    fn in_cond(c: &Cond) -> Option<&'static str> {
        in_expr(&c.lhs).or_else(|| in_expr(&c.rhs))
    }
    funcs.iter().find_map(|(_, f)| {
        f.body
            .iter()
            .find_map(in_stmt)
            .or_else(|| f.ret.iter().find_map(in_expr))
    })
}

/// The gate the compile entries run before emitting: a clean, instructive error
/// instead of a miscompile or a panic deep in codegen.
pub(crate) fn reject_signed32(funcs: &[(String, Func)]) -> Result<(), String> {
    match find_signed32(funcs) {
        Some(what) => Err(format!(
            "rustz80: this program uses {what} — i32 compiles to the IR and runs on \
             the reference interpreter (`interp_*`), but no machine backend emits \
             signed-32 ops yet (Phase 5 WS-B lands them on RV32)"
        )),
        None => Ok(()),
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
