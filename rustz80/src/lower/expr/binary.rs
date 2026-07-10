//! Binary operators, comparisons, shifts, and the f32 kernel routing.

use super::super::Ctx;
use super::*;
use crate::ir::*;

fn bin_op(op: &syn::BinOp) -> Result<BinOp, String> {
    Ok(match op {
        syn::BinOp::Add(_) => BinOp::Add,
        syn::BinOp::Sub(_) => BinOp::Sub,
        syn::BinOp::Mul(_) => BinOp::Mul,
        syn::BinOp::Div(_) => BinOp::Div,
        syn::BinOp::Rem(_) => BinOp::Rem,
        syn::BinOp::BitOr(_) => BinOp::Or,
        syn::BinOp::BitAnd(_) => BinOp::And,
        syn::BinOp::BitXor(_) => BinOp::Xor,
        syn::BinOp::Shl(_) => BinOp::Shl,
        syn::BinOp::Shr(_) => BinOp::Shr,
        _ => {
            return Err(
                "unsupported operator — the dialect has `+ - * / % & | ^ << >>`, \
                 comparisons, and `&&`/`||` (no `..`, no overloading)"
                    .into(),
            )
        }
    })
}

/// The comparison op for a `syn` binop, if it is one (the value form of `<`/`<=`/…).
fn cmp_op(op: &syn::BinOp) -> Option<Cmp> {
    Some(match op {
        syn::BinOp::Lt(_) => Cmp::Lt,
        syn::BinOp::Le(_) => Cmp::Le,
        syn::BinOp::Gt(_) => Cmp::Gt,
        syn::BinOp::Ge(_) => Cmp::Ge,
        syn::BinOp::Eq(_) => Cmp::Eq,
        syn::BinOp::Ne(_) => Cmp::Ne,
        _ => return None,
    })
}

/// `&&` → `Some(true)`, `||` → `Some(false)`, else `None`.
fn logic_op(op: &syn::BinOp) -> Option<bool> {
    match op {
        syn::BinOp::And(_) => Some(true),
        syn::BinOp::Or(_) => Some(false),
        _ => None,
    }
}

/// Whether `e` is an integer literal (a constant shift amount uses the unrolled path).
fn is_int_literal(e: &syn::Expr) -> bool {
    matches!(e, syn::Expr::Lit(l) if matches!(l.lit, syn::Lit::Int(_)))
}

/// A shift amount — must be an integer literal (variable shifts are unsupported).
fn lit_shift_amount(e: &syn::Expr) -> Result<u8, String> {
    if let syn::Expr::Lit(l) = e {
        if let syn::Lit::Int(i) = &l.lit {
            return i.base10_parse::<u8>().map_err(|e| e.to_string());
        }
    }
    Err("shift amount must be an integer literal".into())
}

/// Lower a binary expression. Shifts take a constant RHS for a `u32` LHS (16-bit
/// shifts also take a runtime amount); a `u32` operand makes the op 32-bit (`Bin32`),
/// zero-extending a mixed 16-bit side. `u32` comparisons are not supported yet.
pub(super) fn lower_binary(b: &syn::ExprBinary, ctx: &mut Ctx) -> Result<(Expr, Width), String> {
    // A comparison used as a value (`(a < b) as u16`, `let f = a == b;`) materialises to
    // a `0`/`1` bool. In condition position a comparison stays a tight `Cond` (handled by
    // `lower_cond`), so this only fires when a comparison is a real value.
    if let Some(cmp) = cmp_op(&b.op) {
        let (le, lw) = lower_expr(&b.left, ctx)?;
        let (re, rw) = lower_expr(&b.right, ctx)?;
        // f32 compares through the comparison kernels — Rust semantics exactly
        // (NaN false on every ordered op, -0 == +0), never an integer bit compare.
        if lw == Width::F32 || rw == Width::F32 {
            let call = f32_cmp_call(cmp, le, lw, re, rw, ctx)?;
            return Ok((
                Expr::Cmp {
                    cmp: if cmp == Cmp::Ne { Cmp::Eq } else { Cmp::Ne },
                    lhs: Box::new(Expr::Trunc32(Box::new(call))),
                    rhs: Box::new(Expr::Lit(0)),
                    signed: false,
                },
                Width::Byte,
            ));
        }
        // A 32-bit side makes it a 32-bit compare (a 16-bit side extends); an i32
        // side orders by two's complement. i32 and u32 never mix (rustc rejects the
        // comparison too).
        if lw.is_int_wide() || rw.is_int_wide() {
            let signed = lw == Width::SDWord || rw == Width::SDWord;
            if signed && (lw == Width::DWord || rw == Width::DWord) {
                return Err("i32 and u32 don't mix in a comparison — cast one side \
                            explicitly (`as i32` / `as u32`)"
                    .into());
            }
            return Ok((
                Expr::Cmp32 {
                    cmp,
                    lhs: Box::new(coerce32s(le, lw, signed)),
                    rhs: Box::new(coerce32s(re, rw, signed)),
                    signed,
                },
                Width::Byte,
            ));
        }
        let signed = lw == Width::SWord || rw == Width::SWord;
        let (lhs, rhs) = (Box::new(le), Box::new(re));
        return Ok((
            Expr::Cmp {
                cmp,
                lhs,
                rhs,
                signed,
            },
            Width::Byte,
        ));
    }
    // Short-circuit `&&` / `||` on bool operands → a `0`/`1` value.
    if let Some(and) = logic_op(&b.op) {
        let lhs = Box::new(lower_expr16(&b.left, ctx, "`&&`/`||` operand")?);
        let rhs = Box::new(lower_expr16(&b.right, ctx, "`&&`/`||` operand")?);
        return Ok((Expr::Logic { and, lhs, rhs }, Width::Byte));
    }
    let op = bin_op(&b.op)?;
    if matches!(op, BinOp::Shl | BinOp::Shr) {
        let (le, lw) = lower_expr(&b.left, ctx)?;
        if lw == Width::F32 {
            return Err("shifts are not defined on f32 (Rust rejects them too)".into());
        }
        // A runtime (non-literal) 16-bit shift amount → a counted shift loop. `u32`
        // shifts and literal amounts keep the unrolled constant path below.
        if !lw.is_int_wide() && !is_int_literal(&b.right) {
            let (ae, aw) = lower_expr(&b.right, ctx)?;
            // A `u32` amount is fine in rustc (`x << y32`) — only its low byte counts.
            let amount = Box::new(if aw.is_int_wide() {
                Expr::Trunc32(Box::new(ae))
            } else {
                ae
            });
            return Ok((
                Expr::ShiftVar {
                    left: matches!(op, BinOp::Shl),
                    e: Box::new(le),
                    amount,
                    w: lw,
                },
                lw,
            ));
        }
        let k = lit_shift_amount(&b.right)?;
        if lw.is_int_wide() {
            return Ok((
                Expr::Shift32 {
                    left: matches!(op, BinOp::Shl),
                    e: Box::new(le),
                    k,
                    // `i32 >> k` is arithmetic (sign-propagating); `<<` ignores it.
                    signed: lw == Width::SDWord,
                },
                lw,
            ));
        }
        return Ok((
            Expr::Bin(op, Box::new(le), Box::new(Expr::Lit(k as u16)), lw),
            lw,
        ));
    }
    let (le, lw) = lower_expr(&b.left, ctx)?;
    let (re, rw) = lower_expr(&b.right, ctx)?;
    // f32 arithmetic routes through the owned softfloat kernels — as *calls*, which
    // keeps the canon pass's algebraic rewrites structurally unable to touch float
    // chains (F0.6: the sugar and the canon guard land together, by construction).
    if lw == Width::F32 || rw == Width::F32 {
        require_f32_pair(lw, rw)?;
        let kernel = match op {
            BinOp::Add => "fadd",
            BinOp::Sub => "fsub",
            BinOp::Mul => "fmul",
            BinOp::Div => "fdiv",
            _ => {
                return Err(
                    "`%`, shifts, and bitwise ops are not defined on f32 (Rust rejects \
                     them too; `%` on reals is a host question)"
                        .into(),
                )
            }
        };
        // Two wide args reorder evaluation (the first is computed last — the stack
        // shape); pure operands are unaffected, effectful pairs must hoist.
        if f32_operand_effects(&le) && f32_operand_effects(&re) {
            return Err(format!(
                "both operands of this f32 `{}` have side effects and the wide call \
                 convention reorders them — hoist one to a `let` binding",
                match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    _ => "/",
                }
            ));
        }
        ctx.mark_f32(kernel);
        return Ok((Expr::Call(kernel.to_string(), vec![le, re]), Width::F32));
    }
    if lw.is_int_wide() || rw.is_int_wide() {
        // Full 32-bit arithmetic: `+ - * / %` and `| & ^`. A 16-bit side extends
        // (the unsuffixed-literal mixing rustc allows, `part as u32 * 100`); an i32
        // side makes `/`/`%` signed (add/sub/mul/bitwise share the bit patterns).
        // i32 and u32 never mix (rustc rejects the op too).
        let signed = lw == Width::SDWord || rw == Width::SDWord;
        if signed && (lw == Width::DWord || rw == Width::DWord) {
            return Err("i32 and u32 don't mix in arithmetic — cast one side \
                        explicitly (`as i32` / `as u32`)"
                .into());
        }
        return Ok((
            Expr::Bin32(
                op,
                Box::new(coerce32s(le, lw, signed)),
                Box::new(coerce32s(re, rw, signed)),
                signed,
            ),
            if signed { Width::SDWord } else { Width::DWord },
        ));
    }
    Ok((Expr::Bin(op, Box::new(le), Box::new(re), lw), lw))
}

/// Effect check for f32 operand ordering: the softfloat kernels are pure (no memory
/// writes; the conversion pair's typed halt is a domain error either way round), so
/// a kernel call is effect-free iff its arguments are — otherwise defer to the
/// conservative `has_effects`. Without this, `a.floor() + a.ceil()` would be
/// rejected as "two effectful operands".
pub(crate) fn f32_operand_effects(e: &Expr) -> bool {
    match e {
        Expr::Call(name, args) if crate::softfloat::KERNEL_DEPS.iter().any(|(n, _)| n == name) => {
            args.iter().any(f32_operand_effects)
        }
        _ => has_effects(e),
    }
}

/// Both sides of an f32 op must be f32 — no implicit int↔float conversion, ever
/// (the repr-tag discipline: bit patterns never cross representations silently).
pub(crate) fn require_f32_pair(lw: Width, rw: Width) -> Result<(), String> {
    if lw != rw {
        return Err(
            "f32 and integer values don't mix — there are no implicit conversions; \
             keep the computation in one representation (explicit conversion kernels \
             arrive with the F1 wave)"
                .into(),
        );
    }
    Ok(())
}

/// An f32 comparison as a kernel call returning 0/1 in a u32: `>`/`>=` swap operands
/// onto `flt`/`fle`; `!=` is the caller's negation of `feq`.
pub(crate) fn f32_cmp_call(
    cmp: Cmp,
    le: Expr,
    lw: Width,
    re: Expr,
    rw: Width,
    ctx: &mut Ctx,
) -> Result<Expr, String> {
    require_f32_pair(lw, rw)?;
    let (kernel, swap) = match cmp {
        Cmp::Eq | Cmp::Ne => ("feq", false),
        Cmp::Lt => ("flt", false),
        Cmp::Gt => ("flt", true),
        Cmp::Le => ("fle", false),
        Cmp::Ge => ("fle", true),
    };
    if f32_operand_effects(&le) && f32_operand_effects(&re) {
        return Err(
            "both operands of this f32 comparison have side effects and the wide call \
             convention reorders them — hoist one to a `let` binding"
                .into(),
        );
    }
    ctx.mark_f32(kernel);
    let (l, r) = if swap { (re, le) } else { (le, re) };
    Ok(Expr::Call(kernel.to_string(), vec![l, r]))
}
