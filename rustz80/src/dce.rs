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
        Expr::Cmp { lhs, rhs, .. }
        | Expr::Logic { lhs, rhs, .. }
        | Expr::Cmp32 { lhs, rhs, .. } => {
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
        | Expr::Deref32(e, _)
        | Expr::MulConst(e, _)
        | Expr::LoadAt(e, _)
        | Expr::Trunc32(e)
        | Expr::Widen(e)
        | Expr::SignExtend(e)
        | Expr::Shift32 { e, .. }
        | Expr::Halt(e) => calls_in_expr(e, out),
        Expr::Lit(_)
        | Expr::Var(_)
        | Expr::AddrOf(_)
        | Expr::ConstAddr(_)
        | Expr::Lit32(_)
        | Expr::Var32(_) => {}
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
        Stmt::Store(a, _, b) | Stmt::Store32(a, _, b) => {
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

/// The const-data names referenced (via [`Expr::ConstAddr`]) anywhere in `funcs` —
/// the data section's own DCE: only consts a kept function actually addresses are
/// laid into the image. Walks with the same traversal as `calls_in_*`, collecting
/// `ConstAddr` leaves instead of call names.
pub(crate) fn const_refs(funcs: &[(String, Func)]) -> HashSet<String> {
    fn in_expr(e: &Expr, out: &mut HashSet<String>) {
        if let Expr::ConstAddr(n) = e {
            out.insert(n.clone());
            return;
        }
        // Reuse the call walker's traversal by piggybacking on `calls_in_expr`'s
        // shape — a `ConstAddr` never nests inside itself, so a manual recursion
        // over the same arms keeps the two walkers in sync via the exhaustive match.
        match e {
            Expr::Call(_, args) => args.iter().for_each(|a| in_expr(a, out)),
            Expr::Bin(_, l, r, _) | Expr::Bin32(_, l, r) => {
                in_expr(l, out);
                in_expr(r, out);
            }
            Expr::Cmp { lhs, rhs, .. }
            | Expr::Logic { lhs, rhs, .. }
            | Expr::Cmp32 { lhs, rhs, .. } => {
                in_expr(lhs, out);
                in_expr(rhs, out);
            }
            Expr::ShiftVar { e, amount, .. } => {
                in_expr(e, out);
                in_expr(amount, out);
            }
            Expr::Index(_, idx, _) => in_expr(idx, out),
            Expr::PtrIndex { ptr, index, .. } => {
                in_expr(ptr, out);
                in_expr(index, out);
            }
            Expr::Trunc(e)
            | Expr::Peek(e)
            | Expr::InPort(e)
            | Expr::Deref(e, _)
            | Expr::Deref32(e, _)
            | Expr::MulConst(e, _)
            | Expr::LoadAt(e, _)
            | Expr::Trunc32(e)
            | Expr::Widen(e)
            | Expr::SignExtend(e)
            | Expr::Shift32 { e, .. }
            | Expr::Halt(e) => in_expr(e, out),
            Expr::Lit(_)
            | Expr::Var(_)
            | Expr::AddrOf(_)
            | Expr::ConstAddr(_)
            | Expr::Lit32(_)
            | Expr::Var32(_) => {}
        }
    }
    fn in_stmt(s: &Stmt, out: &mut HashSet<String>) {
        match s {
            Stmt::Assign(_, e) | Stmt::Assign32(_, e) | Stmt::Eval(e) | Stmt::AssignTuple(_, e) => {
                in_expr(e, out)
            }
            Stmt::StoreIndex(_, a, b, _) | Stmt::Poke(a, b) | Stmt::StoreAt(a, b, _) => {
                in_expr(a, out);
                in_expr(b, out);
            }
            Stmt::Store(a, _, b) | Stmt::Store32(a, _, b) => {
                in_expr(a, out);
                in_expr(b, out);
            }
            Stmt::PtrStoreIndex {
                ptr, index, value, ..
            } => {
                in_expr(ptr, out);
                in_expr(index, out);
                in_expr(value, out);
            }
            Stmt::Fill { value, .. } => in_expr(value, out),
            Stmt::If(c, t, e) => {
                in_expr(&c.lhs, out);
                in_expr(&c.rhs, out);
                t.iter().for_each(|s| in_stmt(s, out));
                e.iter().for_each(|s| in_stmt(s, out));
            }
            Stmt::While(c, b) => {
                in_expr(&c.lhs, out);
                in_expr(&c.rhs, out);
                b.iter().for_each(|s| in_stmt(s, out));
            }
            Stmt::Loop(b) => b.iter().for_each(|s| in_stmt(s, out)),
            Stmt::ForRange { end, body, .. } => {
                in_expr(end, out);
                body.iter().for_each(|s| in_stmt(s, out));
            }
            Stmt::Return(Some(e)) => in_expr(e, out),
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
        }
    }
    let mut out = HashSet::new();
    for (_, f) in funcs {
        for s in &f.body {
            in_stmt(s, &mut out);
        }
        for e in &f.ret {
            in_expr(e, &mut out);
        }
    }
    out
}

/// The names a function's body (and tail returns) directly calls — its outgoing edges
/// in the call graph.
pub(crate) fn callees(f: &Func) -> Vec<String> {
    let mut out = Vec::new();
    for s in &f.body {
        calls_in_stmt(s, &mut out);
    }
    for e in &f.ret {
        calls_in_expr(e, &mut out);
    }
    out
}

/// Find a call cycle — direct (`f → f`) or mutual (`f → g → f`) recursion — returning
/// the cycle as a rendered path. Stage 1 gives every function **static** local slots,
/// so a recursive call silently clobbers the caller's locals: any value read from a slot
/// after the recursive call returns is the *innermost* frame's, not this one's
/// (tail-shaped recursion only works by accident, riding the hardware stack). Rejecting
/// the cycle at lowering keeps the "an accepted program matches rustc" contract true.
/// Unknown callees (prelude routes resolved later) are treated as leaves.
pub(crate) fn find_recursion(funcs: &[(String, Func)]) -> Option<String> {
    let graph: HashMap<&str, Vec<String>> = funcs
        .iter()
        .map(|(n, f)| (n.as_str(), callees(f)))
        .collect();
    // Iterative three-colour DFS; `path` carries the grey chain for the error message.
    #[derive(Clone, Copy, PartialEq)]
    enum C {
        White,
        Grey,
        Black,
    }
    let mut colour: HashMap<&str, C> = graph.keys().map(|&n| (n, C::White)).collect();
    for &start in graph.keys() {
        if colour[start] != C::White {
            continue;
        }
        // Stack of (node, next-callee index); `path` mirrors the grey chain.
        let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
        colour.insert(start, C::Grey);
        let mut path: Vec<&str> = vec![start];
        while let Some(&mut (node, ref mut i)) = stack.last_mut() {
            let callees = &graph[node];
            if *i >= callees.len() {
                colour.insert(node, C::Black);
                stack.pop();
                path.pop();
                continue;
            }
            let next = callees[*i].as_str();
            *i += 1;
            match colour.get(next).copied() {
                Some(C::Grey) => {
                    // Found the cycle: render it from `next`'s position in the path.
                    let from = path.iter().position(|&n| n == next).unwrap_or(0);
                    let mut cycle: Vec<&str> = path[from..].to_vec();
                    cycle.push(next);
                    return Some(cycle.join(" → "));
                }
                Some(C::White) => {
                    colour.insert(next, C::Grey);
                    if let Some(&key) = graph.keys().find(|&&k| k == next) {
                        stack.push((key, 0));
                        path.push(key);
                    }
                }
                _ => {} // Black (done) or unknown (a leaf resolved later)
            }
        }
    }
    None
}
