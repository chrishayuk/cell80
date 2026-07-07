//! Expression codegen — evaluate an `Expr` into `HL` (arithmetic, bitwise, traps, u32).
use super::asm::*;
use super::ins::{Imm, R16};
use super::runtime::*;
use super::Target;
use crate::ir::*;

/// Evaluate `e`, leaving the result in `HL`.
pub(super) fn gen_expr(a: &mut Asm, e: &Expr) {
    match e {
        Expr::Lit(n) => {
            a.ld_imm(R16::Hl, Imm::Abs(*n)); // LD HL, n
        }
        Expr::Var(slot) => {
            let addr = a.slot(*slot);
            a.ld_hl_mem(addr); // LD HL, (slot)
        }
        Expr::Bin(op, l, r, w) => {
            // The 8-bit accumulator lane: a byte-typed *chain* (≥2 ops) of `+`/`-`/`&`/`|`/`^`
            // over literal/var operands computes in `A` (`ADD A,n` / `ADD A,(HL)`) — no per-op
            // `PUSH`/`POP` spill and no intermediate `LD H,0` mask — then zero-extends into
            // `HL`. Restricted to a chain because a *single* byte op breaks even (or loses to
            // the HL path, which often already holds the left operand); each extra op in a
            // chain then saves ~3 bytes. Anything else falls through to the HL path below.
            if is_a_chain(e) {
                gen_expr8(a, e);
                a.fx(&[0x6F]); // LD L, A
                mask_to_width(a, Width::Byte); // LD H, 0  -> HL = zero-extended byte
                return;
            }
            // Const-fold a literal-only op (e.g. `2 * 3 + 4`).
            if let (Expr::Lit(x), Expr::Lit(y)) = (&**l, &**r) {
                if let Some(v) = const_fold(*op, *x, *y) {
                    a.ld_imm(R16::Hl, Imm::Abs(v)); // LD HL, v
                    mask_to_width(a, *w);
                    return;
                }
            }
            match op {
                BinOp::Add => {
                    gen_expr(a, l);
                    a.push(R16::Hl); // PUSH HL
                    gen_expr(a, r);
                    a.pop(R16::De); // POP DE  (DE = l)
                    a.add_hl(R16::De); // ADD HL, DE
                }
                BinOp::Sub => gen_sub(a, l, r),
                // `x * const` → shift-and-add (no `__mul16`); else the runtime/trap.
                BinOp::Mul => match const_operand(l, r) {
                    Some((k, other)) => {
                        gen_expr(a, other);
                        gen_mul_const(a, k);
                    }
                    None => gen_mul(a, l, r),
                },
                // `x / 2^n` → shift right; else the runtime/trap. Signed (`i16`)
                // divide truncates toward zero — always the signed runtime.
                BinOp::Div if *w == Width::SWord => gen_sdivmod(a, l, r, false),
                BinOp::Div => match lit_val(r) {
                    Some(k) if k.is_power_of_two() => {
                        gen_expr(a, l);
                        gen_shr_const(a, k.trailing_zeros());
                    }
                    _ => gen_divmod(a, l, r, false),
                },
                // `x % 2^n` → mask the low bits; else the runtime/trap.
                BinOp::Rem if *w == Width::SWord => gen_sdivmod(a, l, r, true),
                BinOp::Rem => match lit_val(r) {
                    Some(k) if k.is_power_of_two() => {
                        gen_expr(a, l);
                        gen_and_mask(a, k - 1);
                    }
                    _ => gen_divmod(a, l, r, true),
                },
                BinOp::Or => gen_bitwise(a, l, r, 0xB3, 0xB2, *w), // OR E / OR D
                BinOp::And => gen_bitwise(a, l, r, 0xA3, 0xA2, *w), // AND E / AND D
                BinOp::Xor => gen_bitwise(a, l, r, 0xAB, 0xAA, *w), // XOR E / XOR D
                // Shift by a constant amount (RHS is always a literal).
                BinOp::Shl => {
                    gen_expr(a, l);
                    for _ in 0..lit_u8(r) {
                        a.add_hl(R16::Hl); // ADD HL,HL  (logical << 1)
                    }
                }
                // `i16 >> n` is an *arithmetic* shift (sign-propagating SRA).
                BinOp::Shr if *w == Width::SWord => {
                    gen_expr(a, l);
                    for _ in 0..lit_u8(r) {
                        a.fx(&[0xCB, 0x2C]); // SRA H
                        a.fx(&[0xCB, 0x1D]); // RR L
                    }
                }
                BinOp::Shr => {
                    gen_expr(a, l);
                    gen_shr_const(a, lit_u8(r) as u32);
                }
            }
            mask_to_width(a, *w);
        }
        Expr::Index(base, index, w) => {
            gen_elem_addr(a, *base, index); // HL = &base[index]
            match w {
                Width::Word | Width::SWord => {
                    a.fx(&[0x5E]); // LD E,(HL)
                    a.fx(&[0x23]); // INC HL
                    a.fx(&[0x56]); // LD D,(HL)
                    a.ex_de_hl(); // EX DE,HL   -> HL = value
                }
                Width::Byte => {
                    a.fx(&[0x6E]); // LD L,(HL)
                    a.fx(&[0x26, 0x00]); // LD H, 0    -> HL = zero-extended byte
                }
                Width::DWord | Width::F32 => unreachable!("wide array elements are unsupported"),
            }
        }
        Expr::Call(name, args) => gen_call(a, name, args),
        Expr::Trunc(e) => {
            gen_expr(a, e);
            a.fx(&[0x26, 0x00]); // LD H, 0   (mask to u8)
        }
        Expr::Peek(addr) => {
            gen_expr(a, addr); // HL = addr
            a.fx(&[0x6E]); // LD L,(HL)   -- read mem[addr] into L
            a.fx(&[0x26, 0x00]); // LD H, 0     -> HL = zero-extended byte
        }
        Expr::InPort(port) => {
            gen_expr(a, port); // HL = port
            a.fx(&[0x44]); // LD B,H
            a.fx(&[0x4D]); // LD C,L   (BC = port)
            a.fx(&[0xED, 0x78]); // IN A,(C)
            a.fx(&[0x6F]); // LD L,A
            a.fx(&[0x26, 0x00]); // LD H,0   -> HL = port byte
        }
        Expr::AddrOf(slot) => {
            let addr = a.slot(*slot);
            a.ld_imm(R16::Hl, addr); // LD HL, &local
        }
        Expr::ConstAddr(name) => {
            a.ld_sym(R16::Hl, name); // LD HL, <const-data symbol>
        }
        Expr::Deref(ptr, off) => {
            gen_expr(a, ptr); // HL = base pointer
            gen_add_offset(a, *off);
            a.fx(&[0x5E]); // LD E,(HL)
            a.fx(&[0x23]); // INC HL
            a.fx(&[0x56]); // LD D,(HL)
            a.ex_de_hl(); // EX DE,HL   -> HL = u16 at *(ptr + off)
        }
        Expr::PtrIndex { ptr, off, index } => {
            gen_ptr_elem_addr(a, ptr, *off, index); // HL = ptr + off + index*2
            a.fx(&[0x5E]); // LD E,(HL)
            a.fx(&[0x23]); // INC HL
            a.fx(&[0x56]); // LD D,(HL)
            a.ex_de_hl(); // EX DE,HL   -> HL = u16 element
        }
        Expr::MulConst(e, k) => {
            gen_expr(a, e);
            gen_mul_const(a, *k);
        }
        Expr::LoadAt(addr, w) => {
            gen_expr(a, addr); // HL = byte address
            match w {
                Width::Word | Width::SWord => {
                    a.fx(&[0x5E]); // LD E,(HL)
                    a.fx(&[0x23]); // INC HL
                    a.fx(&[0x56]); // LD D,(HL)
                    a.ex_de_hl(); // EX DE,HL
                }
                Width::Byte => {
                    a.fx(&[0x6E]); // LD L,(HL)
                    a.fx(&[0x26, 0x00]); // LD H, 0  (zero-extend)
                }
                Width::DWord | Width::F32 => {
                    unreachable!("wide array/field elements are unsupported")
                }
            }
        }
        // A comparison as a value → `1`/`0` in `HL`.
        Expr::Cmp {
            cmp,
            lhs,
            rhs,
            signed,
        } => gen_cmp(a, *cmp, lhs, rhs, *signed),
        // Short-circuit `&&` / `||` on bool operands.
        Expr::Logic { and, lhs, rhs } => {
            gen_expr(a, lhs); // HL = lhs (0/1)
            a.fx(&[0x7D]); // LD A,L
            a.fx(&[0xB4]); // OR H    -> Z set iff HL == 0
            let end = a.label();
            // `&&`: short-circuit to `end` when lhs is false (HL already 0).
            // `||`: short-circuit to `end` when lhs is true  (HL already 1).
            a.jump(if *and { 0xCA } else { 0xC2 }, end); // JP Z / JP NZ
            gen_expr(a, rhs); // else the result is rhs (0/1)
            a.place(end);
        }
        // Shift by a runtime amount: a counted `ADD HL,HL` / `SRL H;RR L` loop.
        Expr::ShiftVar { left, e, amount, w } => {
            gen_expr(a, e);
            a.push(R16::Hl); // PUSH HL  (value)
            gen_expr(a, amount); // HL = amount
            a.fx(&[0x7D]); // LD A,L   (count = low byte)
            a.pop(R16::Hl); // POP HL   (value)
            a.fx(&[0xB7]); // OR A     -> Z iff count == 0
            let done = a.label();
            let top = a.label();
            a.jump(0xCA, done); // JP Z,done   (no shift)
            a.place(top);
            if *left {
                a.add_hl(R16::Hl); // ADD HL,HL          (<< 1)
            } else if *w == Width::SWord {
                a.fx(&[0xCB, 0x2C]); // SRA H              (>> 1, arithmetic — sign propagates)
                a.fx(&[0xCB, 0x1D]); // RR L
            } else {
                a.fx(&[0xCB, 0x3C]); // SRL H
                a.fx(&[0xCB, 0x1D]); // RR L               (>> 1, logical)
            }
            a.fx(&[0x3D]); // DEC A
            a.jump(0xC2, top); // JP NZ,top
            a.place(done);
            mask_to_width(a, *w);
        }
        // A u32 comparison materialised to `0`/`1` in HL.
        Expr::Cmp32 { cmp, lhs, rhs } => gen_cmp32(a, *cmp, lhs, rhs),
        // `x as u16` — the low word of a `u32` value (the high word is discarded).
        Expr::Trunc32(e) => gen_expr32(a, e),
        // `halt(code)` — code in HL, then the HALT trap (no-op on real hardware).
        Expr::Halt(code) => {
            gen_expr(a, code);
            gen_trap(a, TRAP_HALT);
        }
        Expr::Lit32(_)
        | Expr::Var32(_)
        | Expr::Deref32(..)
        | Expr::Bin32(..)
        | Expr::Shift32 { .. }
        | Expr::Widen(..) => {
            unreachable!("u32 node in a 16-bit context — the lowering guards reject these")
        }
    }
}

/// The first literal operand as a `u8` shift amount (the lowering guarantees a literal).
pub(super) fn lit_u8(e: &Expr) -> u8 {
    match e {
        Expr::Lit(k) => *k as u8,
        _ => unreachable!("shift amount must be a constant"),
    }
}

/// The literal value of `e`, if it is one.
pub(super) fn lit_val(e: &Expr) -> Option<u16> {
    match e {
        Expr::Lit(k) => Some(*k),
        _ => None,
    }
}

/// For a commutative op, a literal operand `(k, other)` if exactly one side is a literal.
pub(super) fn const_operand<'a>(l: &'a Expr, r: &'a Expr) -> Option<(u16, &'a Expr)> {
    match (lit_val(l), lit_val(r)) {
        (Some(k), None) => Some((k, r)),
        (None, Some(k)) => Some((k, l)),
        _ => None, // both-literal is const-folded; neither falls through
    }
}

/// Fold a literal-only binary op at compile time (`None` = leave to the runtime: a
/// `Div`/`Rem` by zero, or a shift, which the normal path handles).
pub(super) fn const_fold(op: BinOp, x: u16, y: u16) -> Option<u16> {
    Some(match op {
        BinOp::Add => x.wrapping_add(y),
        BinOp::Sub => x.wrapping_sub(y),
        BinOp::Mul => x.wrapping_mul(y),
        BinOp::And => x & y,
        BinOp::Or => x | y,
        BinOp::Xor => x ^ y,
        BinOp::Div if y != 0 => x / y,
        BinOp::Rem if y != 0 => x % y,
        _ => return None,
    })
}

/// `HL = l * r` (full 16-bit, neither operand constant). Spectrum: the software runtime.
/// Cell: an `ED FE` host trap, serviced natively by the cell bus.
pub(super) fn gen_mul(a: &mut Asm, l: &Expr, r: &Expr) {
    // `x * x` (one variable squared) — load it once and fan it out to the operand
    // registers, instead of evaluating + reloading the operand twice. Restricted to a bare
    // `Var` so it stays side-effect-free (`f() * f()` must still evaluate twice).
    let square = matches!((l, r), (Expr::Var(s1), Expr::Var(s2)) if s1 == s2);
    match a.target {
        Target::Spectrum48 => {
            if square {
                gen_expr(a, l); // HL = x
                a.fx(&[0x54]);
                a.fx(&[0x5D]); // ld d,h ; ld e,l   (DE = x)
            } else {
                gen_pair(a, l, r); // HL = r, DE = l
            }
            a.call("__mul16"); // HL = HL * DE
            a.needs_mul = true;
        }
        Target::Cell => {
            if square {
                gen_expr(a, l); // HL = x
                a.fx(&[0x54]);
                a.fx(&[0x5D]); // ld d,h ; ld e,l   (DE = x)
            } else {
                gen_expr(a, l);
                a.push(R16::Hl); // PUSH HL  (l)
                gen_expr(a, r); // HL = r
                a.pop(R16::De); // POP DE   (DE = l)
            }
            a.fx(&[0x44]);
            a.fx(&[0x4D]); // ld b,h ; ld c,l   (BC = the value left in HL)
            gen_trap(a, TRAP_MUL16); // HL = BC * DE
        }
    }
}

/// Emit a call: arguments evaluated left-to-right onto the stack, popped into the
/// convention registers, then `CALL`. A **wide first argument** (the one-u32
/// convention, docs 10 §Calls) occupies `HL:DE` — two pushes, two pops — leaving
/// `BC` for at most one more 16-bit argument. A **wide second argument** stays on
/// the stack for the callee to pop (the `__mul32` shape) — it is evaluated *first*
/// (lowering guarantees the args are effect-free), then any 16-bit third, then the
/// first wide last so it sits in `HL:DE` at `CALL`. The result lands wherever the
/// callee leaves it (`HL`, or `HL:DE` for a wide return — the caller knows which).
pub(super) fn gen_call(a: &mut Asm, name: &str, args: &[Expr]) {
    let (wide_first, wide_second) = a
        .wide_sigs
        .get(name)
        .map(|(wp, ws, _)| (*wp, *ws))
        .unwrap_or((false, false));
    if wide_second {
        gen_expr32(a, &args[1]);
        a.push(R16::De); // PUSH DE   (arg1.high — popped last by the callee)
        a.push(R16::Hl); // PUSH HL   (arg1.low — on top, popped first)
        if args.len() > 2 {
            gen_expr(a, &args[2]);
            a.push(R16::Hl);
        }
        gen_expr32(a, &args[0]); // HL:DE = arg0 (nested calls balance the stack)
        if args.len() > 2 {
            a.pop(R16::Bc); // the one extra 16-bit argument
        }
    } else if wide_first {
        gen_expr32(a, &args[0]);
        a.push(R16::De); // PUSH DE   (arg0.high)
        a.push(R16::Hl); // PUSH HL   (arg0.low)
        for arg in &args[1..] {
            gen_expr(a, arg);
            a.push(R16::Hl);
        }
        if args.len() > 1 {
            a.pop(R16::Bc); // the one extra 16-bit argument
        }
        a.pop(R16::Hl); // arg0.low
        a.pop(R16::De); // arg0.high
    } else {
        for arg in args {
            gen_expr(a, arg);
            a.push(R16::Hl); // PUSH HL
        }
        const POP: [R16; 3] = [R16::Hl, R16::De, R16::Bc];
        for i in (0..args.len()).rev() {
            a.pop(POP[i]);
        }
    }
    a.call(name);
}

/// Materialise a **u32** comparison to `0`/`1` in `HL` (unsigned — the dialect has
/// no `i32`). Ordering: the 32-bit `SBC` chain computes `l - r` and its final borrow
/// *is* `l < r`; `LD HL,0 ; ADC HL,HL` turns the carry into the bool (`CCF` negates
/// for `>=`). `>`/`<=` swap operands. Equality: `OR` over the difference's four
/// bytes, then `CP 1` sets carry iff zero. Branch-free — no labels.
fn gen_cmp32(a: &mut Asm, cmp: Cmp, lhs: &Expr, rhs: &Expr) {
    // Normalise Gt/Le to Lt/Ge with swapped operands.
    let (l, r, cmp) = match cmp {
        Cmp::Gt => (rhs, lhs, Cmp::Lt),
        Cmp::Le => (rhs, lhs, Cmp::Ge),
        c => (lhs, rhs, c),
    };
    // The Sub32 operand sequence: r evaluated first (pushed), then l; SBC = l - r.
    gen_expr32(a, r);
    a.push(R16::De); // PUSH DE   (r.high)
    a.push(R16::Hl); // PUSH HL   (r.low)
    gen_expr32(a, l); // HL = l.low, DE = l.high
    a.pop(R16::Bc); // POP BC    (r.low)
    a.fx(&[0xB7]); // OR A        (clear carry)
    a.fx(&[0xED, 0x42]); // SBC HL,BC   (low diff, borrow out)
    a.ex_de_hl(); // EX DE,HL    (HL = l.high; EX/POP leave flags alone)
    a.pop(R16::Bc); // POP BC    (r.high)
    a.fx(&[0xED, 0x42]); // SBC HL,BC   (high diff − borrow → CF = l < r)
    match cmp {
        Cmp::Lt => {}
        Cmp::Ge => a.fx(&[0x3F]), // CCF
        Cmp::Eq | Cmp::Ne => {
            // diff == 0 across all four bytes (high diff in HL, low diff in DE).
            a.fx(&[0x7C]); // LD A,H
            a.fx(&[0xB5]); // OR L
            a.fx(&[0xB2]); // OR D
            a.fx(&[0xB3]); // OR E
            a.fx(&[0xFE, 0x01]); // CP 1   → CF = (diff == 0)
            if cmp == Cmp::Ne {
                a.fx(&[0x3F]); // CCF
            }
        }
        _ => unreachable!("normalised above"),
    }
    a.ld_imm(R16::Hl, Imm::Abs(0)); // LD HL,0   (flags preserved)
    a.fx(&[0xED, 0x6A]); // ADC HL,HL → HL = the carry (0/1)
}

/// `HL:DE = l * r` (mod 2^32). Both targets share the convention: `l` pushed on the
/// stack (low word on top), `r` in `HL:DE`; the callee/trap leaves the product in
/// `HL:DE` and the two stack words in place, so the cleanup is one shape.
fn gen_mul32(a: &mut Asm, l: &Expr, r: &Expr) {
    gen_expr32(a, l);
    a.push(R16::De); // PUSH DE   (l.high)
    a.push(R16::Hl); // PUSH HL   (l.low)
    gen_expr32(a, r); // HL:DE = r
    match a.target {
        Target::Spectrum48 => {
            a.call("__mul32");
            a.needs_mul32 = true;
        }
        Target::Cell => gen_trap(a, TRAP_MUL32),
    }
    a.pop(R16::Bc); // POP BC   ─┐ drop l
    a.pop(R16::Bc); // POP BC   ─┘
}

/// `HL:DE = l / r` (or `l % r` if `rem`). Same convention as [`gen_mul32`]; the
/// quotient comes back in `HL:DE` and the remainder in the two stack words — popped
/// as the result for `%`, dropped for `/`.
fn gen_divmod32(a: &mut Asm, l: &Expr, r: &Expr, rem: bool) {
    gen_expr32(a, l);
    a.push(R16::De); // PUSH DE   (l.high)
    a.push(R16::Hl); // PUSH HL   (l.low)
    gen_expr32(a, r); // HL:DE = r (the divisor)
    match a.target {
        Target::Spectrum48 => {
            a.call("__divmod32");
            a.needs_div32 = true;
        }
        Target::Cell => gen_trap(a, TRAP_DIVMOD32),
    }
    if rem {
        a.pop(R16::Hl); // POP HL   (rem.low)
        a.pop(R16::De); // POP DE   (rem.high)
    } else {
        a.pop(R16::Bc); // POP BC   ─┐ drop the remainder
        a.pop(R16::Bc); // POP BC   ─┘
    }
}

/// `HL = l / r` (or `l % r`) for **signed** (`i16`) operands: `__sdivmod16` takes the
/// absolute values through the unsigned core (software or trap, per target) and fixes
/// the signs up (quotient truncates toward zero; the remainder takes the dividend's
/// sign — rustc semantics).
fn gen_sdivmod(a: &mut Asm, l: &Expr, r: &Expr, rem: bool) {
    gen_pair(a, r, l); // HL = l (dividend), DE = r (divisor)
    a.call("__sdivmod16");
    a.needs_sdiv = true;
    if rem {
        a.ex_de_hl(); // EX DE,HL  -> HL = remainder
    }
}

/// `HL = l / r` (or `l % r` if `rem`), neither a power of two. Spectrum: the software
/// runtime. Cell: an `ED FE` host trap.
pub(super) fn gen_divmod(a: &mut Asm, l: &Expr, r: &Expr, rem: bool) {
    match a.target {
        Target::Spectrum48 => {
            gen_pair(a, r, l); // HL = l, DE = r
            a.call("__divmod16"); // HL = l/r, DE = l%r
            a.needs_div = true;
            if rem {
                a.ex_de_hl(); // EX DE,HL  -> HL = remainder
            }
        }
        Target::Cell => {
            gen_expr(a, r);
            a.push(R16::Hl); // PUSH HL  (r = divisor)
            gen_expr(a, l);
            a.fx(&[0x44]);
            a.fx(&[0x4D]); // ld b,h ; ld c,l   (BC = l = dividend)
            a.pop(R16::De); // POP DE   (DE = r = divisor)
            gen_trap(a, TRAP_DIVMOD16); // HL = BC/DE, DE = BC%DE
            if rem {
                a.ex_de_hl(); // EX DE,HL  -> HL = remainder
            }
        }
    }
}

/// `HL >>= n` (logical), as `SRL H; RR L` per step.
pub(super) fn gen_shr_const(a: &mut Asm, n: u32) {
    for _ in 0..n {
        a.fx(&[0xCB, 0x3C]); // SRL H
        a.fx(&[0xCB, 0x1D]); // RR L
    }
}

/// `HL &= mask` (a compile-time constant), byte-wise through the accumulator.
pub(super) fn gen_and_mask(a: &mut Asm, mask: u16) {
    a.fx(&[0x7D]); // LD A,L
    a.fx(&[0xE6, mask as u8]); // AND lo
    a.fx(&[0x6F]); // LD L,A
    a.fx(&[0x7C]); // LD A,H
    a.fx(&[0xE6, (mask >> 8) as u8]); // AND hi
    a.fx(&[0x67]); // LD H,A
}

/// Wrap `HL` to a byte (`u8`) by zeroing `H`.
pub(super) fn mask_to_width(a: &mut Asm, w: Width) {
    if w == Width::Byte {
        a.fx(&[0x26, 0x00]); // LD H, 0
    }
}

/// The byte binary ops the 8-bit accumulator lane handles (`ADD`/`SUB`/`AND`/`OR`/`XOR`
/// have direct `A, n` / `A, (HL)` forms; `*`/`/`/`%`/shifts do not).
fn is_a_op(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::And | BinOp::Or | BinOp::Xor
    )
}

/// A right operand loadable straight against `A`: a literal (`<op> A, n`) or a byte var
/// (`LD HL, slot; <op> A, (HL)`). Anything computed would need a spill — not worth it.
fn a_loadable(e: &Expr) -> bool {
    matches!(e, Expr::Lit(_) | Expr::Var(_))
}

/// Is `e` a byte expression the accumulator lane can evaluate spill-free: a chain of
/// [`is_a_op`] ops whose left is itself favourable and whose right is [`a_loadable`]? A
/// bare literal/var is favourable too, so a leaf recursion bottoms out — but the caller
/// only enters this path for a byte `Bin`, so the lane always does real work.
fn a_favorable(e: &Expr) -> bool {
    match e {
        Expr::Lit(_) | Expr::Var(_) => true,
        Expr::Bin(op, l, r, Width::Byte) if is_a_op(*op) => a_favorable(l) && a_loadable(r),
        _ => false,
    }
}

/// The A-path is a net win only for a **chain** (≥2 ops): a single byte op breaks even
/// with — or loses to — the HL path (which frequently already holds the left operand,
/// while the lane must reload it). So gate on the left operand *also* being a byte op,
/// with the whole tree [`a_favorable`]; each op beyond the first then saves ~3 bytes.
fn is_a_chain(e: &Expr) -> bool {
    matches!(e,
        Expr::Bin(op, l, _, Width::Byte)
            if is_a_op(*op)
                && matches!(&**l, Expr::Bin(lop, _, _, Width::Byte) if is_a_op(*lop)))
        && a_favorable(e)
}

/// Evaluate an [`a_favorable`] byte expression into the **A register** (8-bit, wrapping),
/// no `PUSH`/`POP`. The caller zero-extends `A` into `HL` afterwards.
fn gen_expr8(a: &mut Asm, e: &Expr) {
    match e {
        Expr::Lit(n) => a.fx(&[0x3E, *n as u8]), // LD A, n
        Expr::Var(slot) => {
            let addr = a.slot(*slot);
            a.ld_a_mem(addr); // LD A, (slot)
        }
        Expr::Bin(op, l, r, _) => {
            gen_expr8(a, l); // A = l
            gen_op8(a, *op, r); // A = A <op> r
        }
        _ => unreachable!("gen_expr8 on a non-favourable expr"),
    }
}

/// `A = A <op> r` for a loadable `r`: the immediate form for a literal, the `(HL)` form
/// for a byte var.
fn gen_op8(a: &mut Asm, op: BinOp, r: &Expr) {
    let (imm_op, hl_op) = match op {
        BinOp::Add => (0xC6u8, 0x86u8), // ADD A,n / ADD A,(HL)
        BinOp::Sub => (0xD6, 0x96),     // SUB n   / SUB (HL)
        BinOp::And => (0xE6, 0xA6),     // AND n   / AND (HL)
        BinOp::Or => (0xF6, 0xB6),      // OR n    / OR (HL)
        BinOp::Xor => (0xEE, 0xAE),     // XOR n   / XOR (HL)
        _ => unreachable!("gen_op8 on a non-accumulator op"),
    };
    match r {
        Expr::Lit(n) => a.fx(&[imm_op, *n as u8]),
        Expr::Var(slot) => {
            let addr = a.slot(*slot);
            a.ld_imm(R16::Hl, addr); // LD HL, slot
            a.fx(&[hl_op]); // <op> A, (HL)
        }
        _ => unreachable!("gen_op8 on a non-loadable operand"),
    }
}

/// Evaluate a `u32` expression into the `HL:DE` pair (`HL` = low word, `DE` = high word).
pub(super) fn gen_expr32(a: &mut Asm, e: &Expr) {
    match e {
        Expr::Lit32(n) => {
            a.ld_imm(R16::Hl, Imm::Abs(*n as u16)); // LD HL, low16
            a.ld_imm(R16::De, Imm::Abs((*n >> 16) as u16)); // LD DE, high16
        }
        Expr::Var32(slot) => {
            let (lo, hi) = (a.slot(*slot), a.slot_hi(*slot));
            a.ld_hl_mem(lo); // LD HL,(addr)      low word
            a.ld_wide_mem(R16::De, hi); // LD DE,(addr+2)    high word
        }
        // Wide field read through a pointer: 4 little-endian bytes at *(ptr + off).
        Expr::Deref32(ptr, off) => {
            gen_expr(a, ptr); // HL = base pointer
            gen_add_offset(a, *off); // HL = &field
            a.fx(&[0x5E]); // LD E,(HL)   low word
            a.fx(&[0x23]); // INC HL
            a.fx(&[0x56]); // LD D,(HL)
            a.fx(&[0x23]); // INC HL
            a.fx(&[0x4E]); // LD C,(HL)   high word
            a.fx(&[0x23]); // INC HL
            a.fx(&[0x46]); // LD B,(HL)
            a.ex_de_hl(); // EX DE,HL    -> HL = low word
            a.fx(&[0x50]); // LD D,B
            a.fx(&[0x59]); // LD E,C      -> DE = high word
        }
        Expr::Trunc32(e) => gen_expr32(a, e),
        // A call to a wide-returning fn: the callee leaves the value in HL:DE.
        Expr::Call(name, args) => gen_call(a, name, args),
        // `x as u32` — evaluate the 16-bit value into HL, then zero-extend: DE (high word) = 0.
        Expr::Widen(inner) => {
            gen_expr(a, inner); // HL = the u16 value
            a.ld_imm(R16::De, Imm::Abs(0)); // LD DE, 0   (high word)
        }
        Expr::Bin32(op, l, r) => match op {
            BinOp::Or | BinOp::And | BinOp::Xor => {
                gen_expr32(a, l);
                a.push(R16::De); // PUSH DE   (l.high)
                a.push(R16::Hl); // PUSH HL   (l.low)
                gen_expr32(a, r); // HL = r.low, DE = r.high
                a.pop(R16::Bc); // POP BC    (l.low)
                gen_bitwise_bc(a, op, false); // HL = r.low OP l.low
                a.ex_de_hl(); // EX DE,HL  -> HL = r.high
                a.pop(R16::Bc); // POP BC    (l.high)
                gen_bitwise_bc(a, op, true); // HL = r.high OP l.high; EX back below
                a.ex_de_hl(); // EX DE,HL  -> HL = low, DE = high
            }
            // 32-bit add: word add, then the carry chains into the high word.
            BinOp::Add => {
                gen_expr32(a, l);
                a.push(R16::De); // PUSH DE   (l.high)
                a.push(R16::Hl); // PUSH HL   (l.low)
                gen_expr32(a, r); // HL = r.low, DE = r.high
                a.pop(R16::Bc); // POP BC    (l.low)
                a.add_hl(R16::Bc); // ADD HL,BC   (low sum, CF out)
                a.ex_de_hl(); // EX DE,HL    (HL = r.high; flags survive)
                a.pop(R16::Bc); // POP BC    (l.high)
                a.fx(&[0xED, 0x4A]); // ADC HL,BC   (high sum + carry)
                a.ex_de_hl(); // EX DE,HL  -> HL = low, DE = high
            }
            // 32-bit sub: `SBC` chains the borrow (r evaluated first, like `gen_sub`).
            BinOp::Sub => {
                gen_expr32(a, r);
                a.push(R16::De); // PUSH DE   (r.high)
                a.push(R16::Hl); // PUSH HL   (r.low)
                gen_expr32(a, l); // HL = l.low, DE = l.high
                a.pop(R16::Bc); // POP BC    (r.low)
                a.fx(&[0xB7]); // OR A        (clear carry)
                a.fx(&[0xED, 0x42]); // SBC HL,BC   (low diff, borrow out)
                a.ex_de_hl(); // EX DE,HL    (HL = l.high)
                a.pop(R16::Bc); // POP BC    (r.high)
                a.fx(&[0xED, 0x42]); // SBC HL,BC   (high diff - borrow)
                a.ex_de_hl(); // EX DE,HL  -> HL = low, DE = high
            }
            BinOp::Mul => gen_mul32(a, l, r),
            BinOp::Div => gen_divmod32(a, l, r, false),
            BinOp::Rem => gen_divmod32(a, l, r, true),
            BinOp::Shl | BinOp::Shr => unreachable!("u32 shifts lower to Shift32"),
        },
        Expr::Shift32 { left, e, k } => {
            gen_expr32(a, e); // HL:DE = lo:hi
                              // Constant shifts decompose word/byte-first: a 16-bit distance is one
                              // register move, an 8-bit distance one byte rotation, and only the
                              // residue pays per-bit shifts — `<< 31` is ~15 bytes, not 248. (The
                              // softfloat kernels live on 16/23/31-bit field shifts.)
            let mut k = *k;
            if k >= 32 {
                // Everything shifts out (what the per-bit loop used to compute).
                a.fx(&[0x21, 0x00, 0x00]); // LD HL,0
                a.fx(&[0x11, 0x00, 0x00]); // LD DE,0
            } else if k >= 16 {
                k -= 16;
                if *left {
                    // hi = lo << residue, lo = 0 — the residue shifts a zero low
                    // word, so only the (new) high word needs moving.
                    a.fx(&[0xEB]); // EX DE,HL        (hi = old lo)
                    a.fx(&[0x21, 0x00, 0x00]); // LD HL,0  (lo = 0)
                    if k >= 8 {
                        k -= 8;
                        a.fx(&[0x53]); // LD D,E
                        a.fx(&[0x1E, 0x00]); // LD E,0
                    }
                    for _ in 0..k {
                        a.fx(&[0xCB, 0x23]); // SLA E
                        a.fx(&[0xCB, 0x12]); // RL D
                    }
                } else {
                    // lo = hi >> residue, hi = 0.
                    a.fx(&[0xEB]); // EX DE,HL        (lo = old hi)
                    a.fx(&[0x11, 0x00, 0x00]); // LD DE,0  (hi = 0)
                    if k >= 8 {
                        k -= 8;
                        a.fx(&[0x6C]); // LD L,H
                        a.fx(&[0x26, 0x00]); // LD H,0
                    }
                    for _ in 0..k {
                        a.fx(&[0xCB, 0x3C]); // SRL H
                        a.fx(&[0xCB, 0x1D]); // RR L
                    }
                }
            } else {
                if k >= 8 {
                    k -= 8;
                    if *left {
                        // bytes up: D←E, E←H, H←L, L←0
                        a.fx(&[0x53]); // LD D,E
                        a.fx(&[0x5C]); // LD E,H
                        a.fx(&[0x65]); // LD H,L
                        a.fx(&[0x2E, 0x00]); // LD L,0
                    } else {
                        // bytes down: L←H, H←E, E←D, D←0
                        a.fx(&[0x6C]); // LD L,H
                        a.fx(&[0x63]); // LD H,E
                        a.fx(&[0x5A]); // LD E,D
                        a.fx(&[0x16, 0x00]); // LD D,0
                    }
                }
                for _ in 0..k {
                    if *left {
                        // DE:HL << 1  (low first, carry up)
                        a.fx(&[0xCB, 0x25]); // SLA L
                        a.fx(&[0xCB, 0x14]); // RL H
                        a.fx(&[0xCB, 0x13]); // RL E
                        a.fx(&[0xCB, 0x12]); // RL D
                    } else {
                        // DE:HL >> 1  (high first, carry down)
                        a.fx(&[0xCB, 0x3A]); // SRL D
                        a.fx(&[0xCB, 0x1B]); // RR E
                        a.fx(&[0xCB, 0x1C]); // RR H
                        a.fx(&[0xCB, 0x1D]); // RR L
                    }
                }
            }
        }
        _ => unreachable!("not a u32 expression"),
    }
}

/// `HL = HL <op> BC` for one 16-bit word of a `u32` bitwise op (`| & ^`), word-wise
/// through the accumulator. `_high` is documentation only — the op is the same per word.
pub(super) fn gen_bitwise_bc(a: &mut Asm, op: &BinOp, _high: bool) {
    let (oc, ob) = match op {
        BinOp::Or => (0xB1, 0xB0),  // OR C / OR B
        BinOp::And => (0xA1, 0xA0), // AND C / AND B
        BinOp::Xor => (0xA9, 0xA8), // XOR C / XOR B
        _ => unreachable!("u32 supports only | & ^"),
    };
    a.fx(&[0x7D]); // LD A,L
    a.fx(&[oc]); // <op> C   -> A = L <op> C
    a.fx(&[0x6F]); // LD L,A
    a.fx(&[0x7C]); // LD A,H
    a.fx(&[ob]); // <op> B   -> A = H <op> B
    a.fx(&[0x67]); // LD H,A
}

/// `HL *= k` for a compile-time constant: a power of two shifts (`ADD HL,HL`), else
/// the `__mul16` micro-runtime.
pub(super) fn gen_mul_const(a: &mut Asm, k: u16) {
    if k == 1 {
        return;
    }
    if k == 0 {
        a.ld_imm(R16::Hl, Imm::Abs(0)); // LD HL, 0
        return;
    }
    if k.is_power_of_two() {
        for _ in 0..k.trailing_zeros() {
            a.add_hl(R16::Hl); // ADD HL,HL
        }
        return;
    }
    // General constant: shift-and-add (no `__mul16`). Move the value to DE, build the
    // result in HL by `result = result*2 (+ value)` per bit from the top.
    a.ex_de_hl(); // EX DE,HL   (DE = value)
    a.ld_imm(R16::Hl, Imm::Abs(0)); // LD HL, 0   (result)
    let top = 15 - k.leading_zeros();
    for i in (0..=top).rev() {
        a.add_hl(R16::Hl); // ADD HL,HL   (result <<= 1)
        if k & (1 << i) != 0 {
            a.add_hl(R16::De); // ADD HL,DE   (result += value)
        }
    }
}

/// Leave `HL = ptr + off + index*2` — the address of a `u16` array element reached
/// through a pointer (`self.arr[index]`). `index*2` uses `ADD HL,HL` (no multiply
/// runtime); `index` is evaluated once.
pub(super) fn gen_ptr_elem_addr(a: &mut Asm, ptr: &Expr, off: usize, index: &Expr) {
    gen_expr(a, index); // HL = index
    a.add_hl(R16::Hl); // ADD HL,HL   (index * 2)
    a.push(R16::Hl); // PUSH HL
    gen_expr(a, ptr); // HL = base pointer
    gen_add_offset(a, off); // HL = ptr + off
    a.pop(R16::De); // POP DE      (DE = index*2)
    a.add_hl(R16::De); // ADD HL,DE   -> HL = ptr + off + index*2
}

/// `HL += off` (a small constant byte offset), if non-zero.
pub(super) fn gen_add_offset(a: &mut Asm, off: usize) {
    if off != 0 {
        a.ld_imm(R16::De, Imm::Abs(off as u16)); // LD DE, off
        a.add_hl(R16::De); // ADD HL, DE
    }
}

/// `HL = left <op> right` (16-bit, byte-wise), where `op_e`/`op_d` are the
/// `OP E` / `OP D` opcodes (commutative, so operand order is irrelevant).
pub(super) fn gen_bitwise(a: &mut Asm, l: &Expr, r: &Expr, op_e: u8, op_d: u8, w: Width) {
    gen_expr(a, l);
    a.push(R16::Hl); // PUSH HL
    gen_expr(a, r);
    a.pop(R16::De); // POP DE       (DE = l, HL = r)
    a.fx(&[0x7D]); // LD A,L
    a.fx(&[op_e]); // OP E
    a.fx(&[0x6F]); // LD L,A
                   // A `u8` bitwise result is always < 256, so the caller's trailing `mask_to_width`
                   // (`LD H,0`) supplies the correct high byte — computing `H OP D` here is dead work.
                   // Skip it for `Width::Byte` (the 8-bit lane): three bytes saved per byte bitwise op.
    if w != Width::Byte {
        a.fx(&[0x7C]); // LD A,H
        a.fx(&[op_d]); // OP D
        a.fx(&[0x67]); // LD H,A
    }
}

/// Evaluate so that `HL = second`, `DE = first` (the operand layout the runtime
/// and `SBC` want).
pub(super) fn gen_pair(a: &mut Asm, first: &Expr, second: &Expr) {
    gen_expr(a, first);
    a.push(R16::Hl); // PUSH HL
    gen_expr(a, second);
    a.pop(R16::De); // POP DE  (DE = first)
}

/// Leave `HL = &base[index]` (each element is `u16`, so address = slot base + index*2).
pub(super) fn gen_elem_addr(a: &mut Asm, base: usize, index: &Expr) {
    gen_expr(a, index); // HL = index
    a.add_hl(R16::Hl); // ADD HL,HL  (index * 2)
    let base_addr = a.slot(base);
    a.ld_imm(R16::De, base_addr); // LD DE, base_addr
    a.add_hl(R16::De); // ADD HL, DE  -> element address
}

/// `HL = left - right`, flags from the subtraction (carry = borrow).
pub(super) fn gen_sub(a: &mut Asm, left: &Expr, right: &Expr) {
    gen_pair(a, right, left); // HL = left, DE = right
    a.fx(&[0xB7]); // OR A   (clear carry)
    a.fx(&[0xED, 0x52]); // SBC HL, DE
}

/// The operand order and the conditional-jump opcode that is taken when a comparison is
/// **false**. After `gen_sub(left, right)` (i.e. `SBC HL,DE`): carry = `left < right`,
/// zero = `left == right`. `swap` flips the operands so `>`/`<=` reuse the `<`/`≥` test
/// (`a > b ≡ b < a`). Shared by the branch form (`gen_cond_skip`) and the value form
/// (`gen_cmp`).
pub(super) fn cmp_false_jump(cmp: Cmp) -> (bool, u8) {
    const JP_NC: u8 = 0xD2;
    const JP_C: u8 = 0xDA;
    const JP_NZ: u8 = 0xC2;
    const JP_Z: u8 = 0xCA;
    match cmp {
        Cmp::Lt => (false, JP_NC),
        Cmp::Ge => (false, JP_C),
        Cmp::Eq => (false, JP_NZ),
        Cmp::Ne => (false, JP_Z),
        Cmp::Gt => (true, JP_NC), // a>b ≡ b<a
        Cmp::Le => (true, JP_C),  // a<=b ≡ !(b<a)
    }
}

/// Materialise a comparison as a `1`/`0` value in `HL` (a `bool`). `signed` orders by
/// two's complement (`i16`): `<` is S ⊕ V after the subtraction, not the carry.
pub(super) fn gen_cmp(a: &mut Asm, cmp: Cmp, lhs: &Expr, rhs: &Expr, signed: bool) {
    let false_l = a.label();
    let end_l = a.label();
    if signed && !matches!(cmp, Cmp::Eq | Cmp::Ne) {
        // Normalize to Lt/Ge by swapping (a > b ≡ b < a; a <= b ≡ b >= a).
        let (swap, want_lt) = match cmp {
            Cmp::Lt => (false, true),
            Cmp::Ge => (false, false),
            Cmp::Gt => (true, true),
            Cmp::Le => (true, false),
            _ => unreachable!(),
        };
        let (left, right) = if swap { (rhs, lhs) } else { (lhs, rhs) };
        gen_sub(a, left, right); // S/V/Z from `left - right`
                                 // `left < right` (signed) ⟺ S ⊕ V. Route the four flag cases to false_l.
        let no_ovf = a.label();
        let true_l = a.label();
        a.jump(0xE2, no_ovf); // JP PO (V = 0)
                              // V = 1: lt ⟺ S = 0.
        a.jump(if want_lt { 0xF2 } else { 0xFA }, true_l); // JP P / JP M
        a.jump(0xC3, false_l);
        a.place(no_ovf);
        // V = 0: lt ⟺ S = 1.
        a.jump(if want_lt { 0xFA } else { 0xF2 }, true_l); // JP M / JP P
        a.jump(0xC3, false_l);
        a.place(true_l);
    } else {
        let (swap, jp_false) = cmp_false_jump(cmp);
        let (left, right) = if swap { (rhs, lhs) } else { (lhs, rhs) };
        gen_sub(a, left, right); // flags set; HL clobbered (overwritten below)
        a.jump(jp_false, false_l);
    }
    a.ld_imm(R16::Hl, Imm::Abs(1)); // LD HL, 1   (true)
    a.jump(0xC3, end_l); // JP end
    a.place(false_l);
    a.ld_imm(R16::Hl, Imm::Abs(0)); // LD HL, 0   (false)
    a.place(end_l);
}
