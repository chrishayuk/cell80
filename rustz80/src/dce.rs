//! Dead-code elimination over the lowered IR: keep only the functions reachable from the
//! entry **roots**, dropping the rest. The cell layer prepends a shared **prelude** of
//! reusable kernels (`gcd`, `imin`, `iabs_diff`, …) to every cell — rooting at the cell's
//! entry then keeps the kernels (and only the kernels) that entry actually reaches, so a
//! prepended library never bloats a cartridge, and ordinary dead code is dropped too.

use crate::ir::*;
use std::collections::{HashMap, HashSet};

/// Names directly called inside an expression (recursively into sub-expressions).
fn calls_in_expr(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Call(name, args) => {
            out.push(name.clone());
            for a in args {
                calls_in_expr(a, out);
            }
        }
        Expr::Bin(_, l, r, _) | Expr::Bin32(_, l, r) => {
            calls_in_expr(l, out);
            calls_in_expr(r, out);
        }
        Expr::Cmp { lhs, rhs, .. } | Expr::Logic { lhs, rhs, .. } => {
            calls_in_expr(lhs, out);
            calls_in_expr(rhs, out);
        }
        Expr::ShiftVar { e, amount, .. } => {
            calls_in_expr(e, out);
            calls_in_expr(amount, out);
        }
        Expr::Index(_, idx, _) => calls_in_expr(idx, out),
        Expr::PtrIndex { ptr, index, .. } => {
            calls_in_expr(ptr, out);
            calls_in_expr(index, out);
        }
        Expr::Trunc(e)
        | Expr::Peek(e)
        | Expr::InPort(e)
        | Expr::Deref(e, _)
        | Expr::MulConst(e, _)
        | Expr::LoadAt(e, _)
        | Expr::Trunc32(e)
        | Expr::Widen(e)
        | Expr::Shift32 { e, .. }
        | Expr::Halt(e) => calls_in_expr(e, out),
        Expr::Lit(_) | Expr::Var(_) | Expr::AddrOf(_) | Expr::Lit32(_) | Expr::Var32(_) => {}
    }
}

fn calls_in_cond(c: &Cond, out: &mut Vec<String>) {
    calls_in_expr(&c.lhs, out);
    calls_in_expr(&c.rhs, out);
}

fn calls_in_stmt(s: &Stmt, out: &mut Vec<String>) {
    match s {
        Stmt::Assign(_, e) | Stmt::Assign32(_, e) | Stmt::Eval(e) | Stmt::AssignTuple(_, e) => {
            calls_in_expr(e, out)
        }
        Stmt::StoreIndex(_, a, b, _) | Stmt::Poke(a, b) | Stmt::StoreAt(a, b, _) => {
            calls_in_expr(a, out);
            calls_in_expr(b, out);
        }
        Stmt::Store(a, _, b) => {
            calls_in_expr(a, out);
            calls_in_expr(b, out);
        }
        Stmt::PtrStoreIndex {
            ptr, index, value, ..
        } => {
            calls_in_expr(ptr, out);
            calls_in_expr(index, out);
            calls_in_expr(value, out);
        }
        Stmt::Fill { value, .. } => calls_in_expr(value, out),
        Stmt::If(c, t, e) => {
            calls_in_cond(c, out);
            for s in t {
                calls_in_stmt(s, out);
            }
            for s in e {
                calls_in_stmt(s, out);
            }
        }
        Stmt::While(c, b) => {
            calls_in_cond(c, out);
            for s in b {
                calls_in_stmt(s, out);
            }
        }
        Stmt::Loop(b) => {
            for s in b {
                calls_in_stmt(s, out);
            }
        }
        Stmt::ForRange { end, body, .. } => {
            calls_in_expr(end, out);
            for s in body {
                calls_in_stmt(s, out);
            }
        }
        Stmt::Return(Some(e)) => calls_in_expr(e, out),
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
    }
}

fn calls_in_func(f: &Func, out: &mut Vec<String>) {
    for s in &f.body {
        calls_in_stmt(s, out);
    }
    for e in &f.ret {
        calls_in_expr(e, out);
    }
}

/// Keep only the functions reachable (transitively) from `roots`, dropping the rest. An
/// **empty `roots` is a no-op** — every function is kept — so a caller that doesn't designate
/// entries (a whole-program/game compile) gets the full image, while the cell layer passes
/// its entry (`run`/`main`/`Type::run`) and prunes everything that entry can't reach.
pub(crate) fn prune(funcs: Vec<(String, Func)>, roots: &[&str]) -> Vec<(String, Func)> {
    if roots.is_empty() {
        return funcs;
    }
    let by_name: HashMap<&str, &Func> = funcs.iter().map(|(n, f)| (n.as_str(), f)).collect();
    let mut keep: HashSet<String> = HashSet::new();
    let mut work: Vec<String> = Vec::new();
    for &r in roots {
        if by_name.contains_key(r) && keep.insert(r.to_string()) {
            work.push(r.to_string());
        }
    }
    while let Some(n) = work.pop() {
        if let Some(f) = by_name.get(n.as_str()) {
            let mut callees = Vec::new();
            calls_in_func(f, &mut callees);
            for c in callees {
                if keep.insert(c.clone()) {
                    work.push(c);
                }
            }
        }
    }
    funcs
        .into_iter()
        .filter(|(n, _)| keep.contains(n))
        .collect()
}
