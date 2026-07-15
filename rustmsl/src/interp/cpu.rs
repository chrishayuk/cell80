//! The CPU reference VM — the transliteration target for the MSL kernel
//! ([`super::gpu`]). Bit-identical to `cell80_core::Interp`, values and IR-step
//! counts both (asserted by `assert_parity` in `super::tests`).

use super::bytecode::{CellProgram, Inst};
use cell80_core::ir::{BinOp, Cmp, Width};

/// The VM's outcome, mirroring `Interp`'s: a value return, a `halt(code)`, a
/// fuel-exhaustion trap (with the step count *at* the trap — the parity point),
/// or divide-by-zero.
#[derive(Debug, PartialEq)]
pub enum VmOut {
    Value(Vec<u16>, u64),
    Halt(u16, u64),
    Fuel(u64),
    DivZero,
}

const VM_FUEL: u64 = 100_000_000;

fn mask(v: u16, w: Width) -> u16 {
    if w == Width::Byte {
        v & 0xFF
    } else {
        v
    }
}

fn cmp16(cmp: Cmp, l: u16, r: u16, signed: bool) -> bool {
    if signed && !matches!(cmp, Cmp::Eq | Cmp::Ne) {
        let (l, r) = (l as i16, r as i16);
        match cmp {
            Cmp::Lt => l < r,
            Cmp::Le => l <= r,
            Cmp::Gt => l > r,
            Cmp::Ge => l >= r,
            _ => unreachable!(),
        }
    } else {
        match cmp {
            Cmp::Lt => l < r,
            Cmp::Le => l <= r,
            Cmp::Gt => l > r,
            Cmp::Ge => l >= r,
            Cmp::Eq => l == r,
            Cmp::Ne => l != r,
        }
    }
}

fn cmp32(cmp: Cmp, l: u32, r: u32, signed: bool) -> bool {
    if signed && !matches!(cmp, Cmp::Eq | Cmp::Ne) {
        let (l, r) = (l as i32, r as i32);
        match cmp {
            Cmp::Lt => l < r,
            Cmp::Le => l <= r,
            Cmp::Gt => l > r,
            Cmp::Ge => l >= r,
            _ => unreachable!(),
        }
    } else {
        match cmp {
            Cmp::Lt => l < r,
            Cmp::Le => l <= r,
            Cmp::Gt => l > r,
            Cmp::Ge => l >= r,
            Cmp::Eq => l == r,
            Cmp::Ne => l != r,
        }
    }
}

/// Execute a `CellProgram`. Returns the outcome + IR steps — the things the
/// correctness gate compares against `Interp`.
pub fn cpu_run(prog: &CellProgram, args: &[u16]) -> VmOut {
    let mut slots = vec![0u16; prog.n_locals];
    for (slot, &arg) in slots.iter_mut().zip(args.iter().take(prog.params)) {
        *slot = arg;
    }
    let mut stack: Vec<u16> = Vec::with_capacity(prog.max_depth + 1);
    let mut steps = 0u64;
    let mut pc = 0usize;
    loop {
        match &prog.code[pc] {
            Inst::Step => {
                steps += 1;
                if steps >= VM_FUEL {
                    return VmOut::Fuel(steps);
                }
            }
            Inst::PushLit(n) => stack.push(*n),
            Inst::PushVar(s) => stack.push(slots[*s]),
            Inst::Bin(op, w) => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                let raw = match op {
                    BinOp::Add => a.wrapping_add(b),
                    BinOp::Sub => a.wrapping_sub(b),
                    BinOp::Mul => a.wrapping_mul(b),
                    BinOp::Or => a | b,
                    BinOp::And => a & b,
                    BinOp::Xor => a ^ b,
                    BinOp::Div | BinOp::Rem => {
                        if b == 0 {
                            return VmOut::DivZero;
                        }
                        match (op, *w == Width::SWord) {
                            (BinOp::Div, true) => (a as i16).wrapping_div(b as i16) as u16,
                            (BinOp::Rem, true) => (a as i16).wrapping_rem(b as i16) as u16,
                            (BinOp::Div, false) => a / b,
                            (BinOp::Rem, false) => a % b,
                            _ => unreachable!(),
                        }
                    }
                    BinOp::Shl | BinOp::Shr => unreachable!("shifts are ShiftLit"),
                };
                stack.push(mask(raw, *w));
            }
            Inst::ShiftLit { left, k, w, signed } => {
                let a = stack.pop().unwrap();
                let raw = if *left {
                    if *k >= 16 {
                        0
                    } else {
                        a << *k
                    }
                } else if *signed {
                    ((a as i16) >> (*k).min(15)) as u16
                } else if *k >= 16 {
                    0
                } else {
                    a >> *k
                };
                stack.push(mask(raw, *w));
            }
            Inst::Cmp(cmp, signed) => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(cmp16(*cmp, a, b, *signed) as u16);
            }
            Inst::Trunc => {
                let a = stack.pop().unwrap();
                stack.push(a & 0xFF);
            }
            Inst::Popcnt => {
                let a = stack.pop().unwrap();
                stack.push(a.count_ones() as u16);
            }
            Inst::Clz => {
                let a = stack.pop().unwrap();
                stack.push(a.leading_zeros() as u16);
            }
            Inst::Ctz => {
                let a = stack.pop().unwrap();
                stack.push(a.trailing_zeros() as u16);
            }
            Inst::Bin32(op, signed) => {
                let bh = stack.pop().unwrap() as u32;
                let bl = stack.pop().unwrap() as u32;
                let ah = stack.pop().unwrap() as u32;
                let al = stack.pop().unwrap() as u32;
                let (a, b) = (al | (ah << 16), bl | (bh << 16));
                let res = match op {
                    BinOp::Add => a.wrapping_add(b),
                    BinOp::Sub => a.wrapping_sub(b),
                    BinOp::Mul => a.wrapping_mul(b),
                    BinOp::Or => a | b,
                    BinOp::And => a & b,
                    BinOp::Xor => a ^ b,
                    BinOp::Div | BinOp::Rem => {
                        if b == 0 {
                            return VmOut::DivZero;
                        }
                        match (op, *signed) {
                            (BinOp::Div, true) => (a as i32).wrapping_div(b as i32) as u32,
                            (BinOp::Rem, true) => (a as i32).wrapping_rem(b as i32) as u32,
                            (BinOp::Div, false) => a / b,
                            (BinOp::Rem, false) => a % b,
                            _ => unreachable!(),
                        }
                    }
                    BinOp::Shl | BinOp::Shr => unreachable!("wide shifts are Shift32"),
                };
                stack.push((res & 0xFFFF) as u16);
                stack.push((res >> 16) as u16);
            }
            Inst::Shift32 { left, k, signed } => {
                let ah = stack.pop().unwrap() as u32;
                let al = stack.pop().unwrap() as u32;
                let a = al | (ah << 16);
                let res = if *signed && !*left {
                    ((a as i32) >> (*k).min(31) as u32) as u32
                } else if *k >= 32 {
                    0
                } else if *left {
                    a << *k
                } else {
                    a >> *k
                };
                stack.push((res & 0xFFFF) as u16);
                stack.push((res >> 16) as u16);
            }
            Inst::Cmp32(cmp, signed) => {
                let bh = stack.pop().unwrap() as u32;
                let bl = stack.pop().unwrap() as u32;
                let ah = stack.pop().unwrap() as u32;
                let al = stack.pop().unwrap() as u32;
                let r = cmp32(*cmp, al | (ah << 16), bl | (bh << 16), *signed);
                stack.push(r as u16);
            }
            Inst::SextHi => {
                let lo = stack.pop().unwrap();
                stack.push(lo);
                stack.push(if lo & 0x8000 != 0 { 0xFFFF } else { 0 });
            }
            Inst::Store(s) => slots[*s] = stack.pop().unwrap(),
            Inst::Pop => {
                stack.pop().unwrap();
            }
            Inst::Dup => {
                let v = *stack.last().unwrap();
                stack.push(v);
            }
            Inst::JmpZero(t) => {
                if stack.pop().unwrap() == 0 {
                    pc = *t;
                    continue;
                }
            }
            Inst::Jmp(t) => {
                pc = *t;
                continue;
            }
            Inst::Ret(arity) => {
                return VmOut::Value(stack[..*arity].to_vec(), steps);
            }
            Inst::Halt => {
                return VmOut::Halt(*stack.last().unwrap(), steps);
            }
        }
        pc += 1;
    }
}
