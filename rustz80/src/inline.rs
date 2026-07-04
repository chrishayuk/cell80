//! A conservative function **inliner** (IR → IR), run before codegen.
//!
//! It folds each **single-call-site**, early-return-free, scalar/void function into its
//! one caller. Single-call-site is the key: inlining there never *duplicates* code, so
//! it's a pure size win — it removes the call's prologue/epilogue, the param copies into
//! scratch slots, and the `CALL`/`RET`. The now-uncalled callee is then dropped by
//! [`crate::dce::prune`]. The point (for the `chuk-speccy` authoring plane): an author can
//! decompose a big `update` into small `&mut self` helpers and the tape is as compact as
//! if they'd hand-inlined it.
//!
//! Two refinements make "as compact as hand-inlining" literal, not approximate:
//! - **Argument substitution** — when an arg is *pure* (`Var`/`Lit`/`&local`) and the
//!   matching parameter is *read-only* in the body (never assigned, never address-taken),
//!   the arg is substituted directly into the body. No param-bind `Assign`, no param slot.
//!   So `self.step(e)` with `self` a pointer and `e` a local inlines to exactly the
//!   hand-written body — no copies. (Only non-pure / mutated params fall back to a slot.)
//! - **Slot reuse** — callee locals are allocated at a `water` mark that pops after each
//!   inlined body, so sibling inlines reuse the same slots (nested ones stack above). The
//!   scratch region grows by the *deepest* inline, not the *sum* of helpers' locals.
//!
//! Conservative on purpose — only inline a call that appears as a whole statement
//! (`f(a);` or `x = f(a);`), only when the callee has **no early `return`** and returns at
//! most one value, never into itself or a cycle. A call nested in an expression/condition
//! is left as a real call (hoist it to a `let` to inline it). Correctness is backed by the
//! differential tests (`tests/diff.rs`): inlined output is run against `rustc`.

use crate::ir::*;
use std::collections::{HashMap, HashSet};

/// How a callee slot is realised in the caller: substituted with an arg expression
/// (pure, read-only param), or relocated to a caller scratch slot.
enum Slot {
    Subst(Expr),
    Reloc(usize),
}

/// Inline single-call-site, early-return-free helpers into their caller. `roots` are
/// never inlined away (e.g. a game's `entry`). Returns the rewritten function list (the
/// folded-in callees become uncalled — run [`crate::dce::prune`] after to drop them).
pub(crate) fn inline(mut funcs: Vec<(String, Func)>, roots: &[&str]) -> Vec<(String, Func)> {
    let counts = call_counts(&funcs);
    let cand: HashMap<String, Func> = funcs
        .iter()
        .filter(|(n, f)| {
            counts.get(n.as_str()).copied().unwrap_or(0) == 1
                && !roots.contains(&n.as_str())
                && f.ret.len() <= 1
                && !has_return(&f.body)
        })
        .map(|(n, f)| (n.clone(), f.clone()))
        .collect();
    if cand.is_empty() {
        return funcs;
    }
    // One recursive pass: chains fold via the recursion in `expand`.
    for (name, f) in funcs.iter_mut() {
        let mut water = f.n_locals;
        let mut max = f.n_locals;
        let mut stack = vec![name.clone()];
        let body = std::mem::take(&mut f.body);
        f.body = inline_stmts(body, &cand, &mut stack, &mut water, &mut max);
        f.n_locals = max;
    }
    funcs
}

fn inline_stmts(
    stmts: Vec<Stmt>,
    cand: &HashMap<String, Func>,
    stack: &mut Vec<String>,
    water: &mut usize,
    max: &mut usize,
) -> Vec<Stmt> {
    let go = |g: &str, stack: &[String]| cand.contains_key(g) && !stack.iter().any(|n| n == g);
    let mut out = Vec::new();
    for s in stmts {
        match s {
            Stmt::Eval(Expr::Call(g, args)) => {
                if go(&g, stack) {
                    let gf = cand[&g].clone();
                    expand(&g, &gf, args, None, cand, stack, water, max, &mut out);
                } else {
                    out.push(Stmt::Eval(Expr::Call(g, args)));
                }
            }
            Stmt::Assign(slot, Expr::Call(g, args)) => {
                if go(&g, stack) {
                    let gf = cand[&g].clone();
                    expand(&g, &gf, args, Some(slot), cand, stack, water, max, &mut out);
                } else {
                    out.push(Stmt::Assign(slot, Expr::Call(g, args)));
                }
            }
            Stmt::If(c, t, e) => out.push(Stmt::If(
                c,
                inline_stmts(t, cand, stack, water, max),
                inline_stmts(e, cand, stack, water, max),
            )),
            Stmt::While(c, b) => out.push(Stmt::While(c, inline_stmts(b, cand, stack, water, max))),
            Stmt::Loop(b) => out.push(Stmt::Loop(inline_stmts(b, cand, stack, water, max))),
            Stmt::ForRange {
                var,
                end,
                inclusive,
                width,
                body,
            } => out.push(Stmt::ForRange {
                var,
                end,
                inclusive,
                width,
                body: inline_stmts(body, cand, stack, water, max),
            }),
            other => out.push(other),
        }
    }
    out
}

/// Splice a callee's body in at the call site. Build a per-slot plan — pure read-only
/// params are *substituted*, everything else gets a fresh caller slot at `water` — then
/// bind the slot-params, emit the remapped body (recursively inlining within it), assign
/// the tail return if used, and **pop** `water` so the next sibling inline reuses the
/// slots. `max` tracks the peak (the caller's `n_locals`).
#[allow(clippy::too_many_arguments)]
fn expand(
    g_name: &str,
    g: &Func,
    args: Vec<Expr>,
    result: Option<usize>,
    cand: &HashMap<String, Func>,
    stack: &mut Vec<String>,
    water: &mut usize,
    max: &mut usize,
    out: &mut Vec<Stmt>,
) {
    let base = *water;
    // A param can be substituted iff its arg is pure *and* it's read-only in the body.
    let mut written = HashSet::new();
    let mut addrd = HashSet::new();
    collect_written(&g.body, &mut written);
    collect_addr(&g.body, &mut addrd);

    let mut plan: Vec<Slot> = Vec::with_capacity(g.n_locals);
    let mut next = base;
    // Slots `0..params` are parameters (have an `args[s]`); `params..n_locals` are locals.
    #[allow(clippy::needless_range_loop)]
    for s in 0..g.n_locals {
        let subst = s < g.params && pure(&args[s]) && !written.contains(&s) && !addrd.contains(&s);
        if subst {
            plan.push(Slot::Subst(args[s].clone()));
        } else {
            plan.push(Slot::Reloc(next));
            next += 1;
        }
    }
    *water = next;
    if *water > *max {
        *max = *water;
    }

    // Bind the slot-backed params (substituted ones need no copy).
    for (i, a) in args.iter().enumerate() {
        if i < g.params {
            if let Slot::Reloc(n) = plan[i] {
                out.push(Stmt::Assign(n, a.clone()));
            }
        }
    }

    // Remap the body into the caller, then recursively inline within it (nested helpers
    // allocate above `next`).
    let remapped: Vec<Stmt> = g.body.iter().map(|s| remap_stmt(s, &plan)).collect();
    stack.push(g_name.to_string());
    let inlined = inline_stmts(remapped, cand, stack, water, max);
    stack.pop();
    out.extend(inlined);

    if let (Some(slot), Some(r)) = (result, g.ret.first()) {
        out.push(Stmt::Assign(slot, remap_expr(r, &plan)));
    }
    *water = base; // pop
}

/// Pure, freely-duplicable expressions — safe to substitute for a param (no side effects,
/// re-reading them any number of times is identical).
fn pure(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Var(_)
            | Expr::Lit(_)
            | Expr::AddrOf(_)
            | Expr::ConstAddr(_)
            | Expr::Var32(_)
            | Expr::Lit32(_)
    )
}

/// Any early `return` anywhere in the body (incl. nested blocks)? Such a function isn't a
/// candidate — its value isn't simply its tail `ret`.
fn has_return(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Return(_) => true,
        Stmt::If(_, t, e) => has_return(t) || has_return(e),
        Stmt::While(_, b) | Stmt::Loop(b) => has_return(b),
        Stmt::ForRange { body, .. } => has_return(body),
        _ => false,
    })
}

// --- read-only analysis (which slots are written / address-taken) -----------

fn collect_written(body: &[Stmt], out: &mut HashSet<usize>) {
    for s in body {
        match s {
            Stmt::Assign(slot, _) | Stmt::Assign32(slot, _) => {
                out.insert(*slot);
            }
            Stmt::StoreIndex(slot, _, _, _) => {
                out.insert(*slot);
            }
            Stmt::Fill { base, .. } => {
                out.insert(*base);
            }
            Stmt::AssignTuple(slots, _) => {
                out.extend(slots.iter().copied());
            }
            Stmt::If(_, t, e) => {
                collect_written(t, out);
                collect_written(e, out);
            }
            Stmt::While(_, b) | Stmt::Loop(b) => collect_written(b, out),
            Stmt::ForRange { var, body, .. } => {
                out.insert(*var);
                collect_written(body, out);
            }
            _ => {}
        }
    }
}

fn collect_addr(body: &[Stmt], out: &mut HashSet<usize>) {
    fn ex(e: &Expr, out: &mut HashSet<usize>) {
        match e {
            Expr::AddrOf(s) => {
                out.insert(*s);
            }
            Expr::Bin(_, a, b, _) | Expr::Bin32(_, a, b) => {
                ex(a, out);
                ex(b, out);
            }
            Expr::Call(_, args) => args.iter().for_each(|a| ex(a, out)),
            Expr::Index(_, i, _) => ex(i, out),
            Expr::Trunc(a)
            | Expr::Peek(a)
            | Expr::InPort(a)
            | Expr::Deref(a, _)
            | Expr::Deref32(a, _)
            | Expr::MulConst(a, _)
            | Expr::LoadAt(a, _)
            | Expr::Trunc32(a)
            | Expr::Widen(a)
            | Expr::Halt(a)
            | Expr::Shift32 { e: a, .. } => ex(a, out),
            Expr::PtrIndex { ptr, index, .. } => {
                ex(ptr, out);
                ex(index, out);
            }
            Expr::Cmp { lhs, rhs, .. } | Expr::Logic { lhs, rhs, .. } => {
                ex(lhs, out);
                ex(rhs, out);
            }
            Expr::ShiftVar { e, amount, .. } => {
                ex(e, out);
                ex(amount, out);
            }
            Expr::Lit(_) | Expr::Var(_) | Expr::ConstAddr(_) | Expr::Var32(_) | Expr::Lit32(_) => {}
        }
    }
    fn st(s: &Stmt, out: &mut HashSet<usize>) {
        match s {
            Stmt::Assign(_, e) | Stmt::Assign32(_, e) | Stmt::Eval(e) | Stmt::AssignTuple(_, e) => {
                ex(e, out)
            }
            Stmt::StoreIndex(_, a, b, _) | Stmt::Poke(a, b) | Stmt::StoreAt(a, b, _) => {
                ex(a, out);
                ex(b, out);
            }
            Stmt::Store(p, _, v) | Stmt::Store32(p, _, v) => {
                ex(p, out);
                ex(v, out);
            }
            Stmt::PtrStoreIndex {
                ptr, index, value, ..
            } => {
                ex(ptr, out);
                ex(index, out);
                ex(value, out);
            }
            Stmt::Fill { value, .. } => ex(value, out),
            Stmt::If(c, t, e) => {
                ex(&c.lhs, out);
                ex(&c.rhs, out);
                t.iter().for_each(|s| st(s, out));
                e.iter().for_each(|s| st(s, out));
            }
            Stmt::While(c, b) => {
                ex(&c.lhs, out);
                ex(&c.rhs, out);
                b.iter().for_each(|s| st(s, out));
            }
            Stmt::Loop(b) => b.iter().for_each(|s| st(s, out)),
            Stmt::ForRange { end, body, .. } => {
                ex(end, out);
                body.iter().for_each(|s| st(s, out));
            }
            Stmt::Return(Some(e)) => ex(e, out),
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
        }
    }
    body.iter().for_each(|s| st(s, out));
}

// --- call counting ----------------------------------------------------------

fn call_counts(funcs: &[(String, Func)]) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for (_, f) in funcs {
        for s in &f.body {
            count_stmt(s, &mut m);
        }
        for e in &f.ret {
            count_expr(e, &mut m);
        }
    }
    m
}

fn count_cond(c: &Cond, m: &mut HashMap<String, usize>) {
    count_expr(&c.lhs, m);
    count_expr(&c.rhs, m);
}

fn count_stmt(s: &Stmt, m: &mut HashMap<String, usize>) {
    match s {
        Stmt::Assign(_, e) | Stmt::Assign32(_, e) | Stmt::Eval(e) | Stmt::AssignTuple(_, e) => {
            count_expr(e, m)
        }
        Stmt::StoreIndex(_, a, b, _) | Stmt::Poke(a, b) | Stmt::StoreAt(a, b, _) => {
            count_expr(a, m);
            count_expr(b, m);
        }
        Stmt::Store(p, _, v) | Stmt::Store32(p, _, v) => {
            count_expr(p, m);
            count_expr(v, m);
        }
        Stmt::PtrStoreIndex {
            ptr, index, value, ..
        } => {
            count_expr(ptr, m);
            count_expr(index, m);
            count_expr(value, m);
        }
        Stmt::Fill { value, .. } => count_expr(value, m),
        Stmt::If(c, t, e) => {
            count_cond(c, m);
            t.iter().for_each(|s| count_stmt(s, m));
            e.iter().for_each(|s| count_stmt(s, m));
        }
        Stmt::While(c, b) => {
            count_cond(c, m);
            b.iter().for_each(|s| count_stmt(s, m));
        }
        Stmt::Loop(b) => b.iter().for_each(|s| count_stmt(s, m)),
        Stmt::ForRange { end, body, .. } => {
            count_expr(end, m);
            body.iter().for_each(|s| count_stmt(s, m));
        }
        Stmt::Return(Some(e)) => count_expr(e, m),
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
    }
}

fn count_expr(e: &Expr, m: &mut HashMap<String, usize>) {
    match e {
        Expr::Call(n, args) => {
            *m.entry(n.clone()).or_default() += 1;
            args.iter().for_each(|a| count_expr(a, m));
        }
        Expr::Bin(_, a, b, _) | Expr::Bin32(_, a, b) => {
            count_expr(a, m);
            count_expr(b, m);
        }
        Expr::Index(_, idx, _) => count_expr(idx, m),
        Expr::Trunc(a)
        | Expr::Peek(a)
        | Expr::InPort(a)
        | Expr::Deref(a, _)
        | Expr::Deref32(a, _)
        | Expr::MulConst(a, _)
        | Expr::LoadAt(a, _)
        | Expr::Trunc32(a)
        | Expr::Widen(a)
        | Expr::Halt(a)
        | Expr::Shift32 { e: a, .. } => count_expr(a, m),
        Expr::PtrIndex { ptr, index, .. } => {
            count_expr(ptr, m);
            count_expr(index, m);
        }
        Expr::Cmp { lhs, rhs, .. } | Expr::Logic { lhs, rhs, .. } => {
            count_expr(lhs, m);
            count_expr(rhs, m);
        }
        Expr::ShiftVar { e, amount, .. } => {
            count_expr(e, m);
            count_expr(amount, m);
        }
        Expr::Lit(_)
        | Expr::Var(_)
        | Expr::AddrOf(_)
        | Expr::ConstAddr(_)
        | Expr::Lit32(_)
        | Expr::Var32(_) => {}
    }
}

// --- remap a callee's slots into the caller (substitute or relocate) ---------

/// A slot position (the target of a write, an array base, etc.) must always relocate — a
/// substituted param is a pure value, never a write target.
fn reloc(plan: &[Slot], s: usize) -> usize {
    match &plan[s] {
        Slot::Reloc(n) => *n,
        Slot::Subst(_) => unreachable!("a substituted (read-only) param can't be a slot target"),
    }
}

fn remap_cond(c: &Cond, plan: &[Slot]) -> Cond {
    Cond {
        cmp: c.cmp,
        lhs: remap_expr(&c.lhs, plan),
        rhs: remap_expr(&c.rhs, plan),
        signed: c.signed,
    }
}

fn remap_stmt(s: &Stmt, plan: &[Slot]) -> Stmt {
    let e = |x: &Expr| remap_expr(x, plan);
    let b = |xs: &[Stmt]| xs.iter().map(|s| remap_stmt(s, plan)).collect();
    match s {
        Stmt::Assign(slot, x) => Stmt::Assign(reloc(plan, *slot), e(x)),
        Stmt::Assign32(slot, x) => Stmt::Assign32(reloc(plan, *slot), e(x)),
        Stmt::StoreIndex(slot, i, v, w) => Stmt::StoreIndex(reloc(plan, *slot), e(i), e(v), *w),
        Stmt::Poke(a, v) => Stmt::Poke(e(a), e(v)),
        Stmt::Store(p, off, v) => Stmt::Store(e(p), *off, e(v)),
        Stmt::Store32(p, off, v) => Stmt::Store32(e(p), *off, e(v)),
        Stmt::PtrStoreIndex {
            ptr,
            off,
            index,
            value,
        } => Stmt::PtrStoreIndex {
            ptr: Box::new(e(ptr)),
            off: *off,
            index: Box::new(e(index)),
            value: e(value),
        },
        Stmt::StoreAt(a, v, w) => Stmt::StoreAt(e(a), e(v), *w),
        Stmt::Fill { base, count, value } => Stmt::Fill {
            base: reloc(plan, *base),
            count: *count,
            value: e(value),
        },
        Stmt::Eval(x) => Stmt::Eval(e(x)),
        Stmt::AssignTuple(slots, x) => {
            Stmt::AssignTuple(slots.iter().map(|s| reloc(plan, *s)).collect(), e(x))
        }
        Stmt::If(c, t, el) => Stmt::If(remap_cond(c, plan), b(t), b(el)),
        Stmt::While(c, body) => Stmt::While(remap_cond(c, plan), b(body)),
        Stmt::Loop(body) => Stmt::Loop(b(body)),
        Stmt::ForRange {
            var,
            end,
            inclusive,
            width,
            body,
        } => Stmt::ForRange {
            var: reloc(plan, *var),
            end: e(end),
            inclusive: *inclusive,
            width: *width,
            body: b(body),
        },
        Stmt::Break => Stmt::Break,
        Stmt::Continue => Stmt::Continue,
        Stmt::Return(o) => Stmt::Return(o.as_ref().map(&e)),
    }
}

fn remap_expr(x: &Expr, plan: &[Slot]) -> Expr {
    let e = |y: &Expr| Box::new(remap_expr(y, plan));
    match x {
        // A read of a slot: substitute the arg (pure param) or read the relocated slot.
        Expr::Var(s) => match &plan[*s] {
            Slot::Subst(a) => a.clone(),
            Slot::Reloc(n) => Expr::Var(*n),
        },
        // u32 locals and `&local` never name a substituted (scalar, read-only) param.
        Expr::Var32(s) => Expr::Var32(reloc(plan, *s)),
        Expr::AddrOf(s) => Expr::AddrOf(reloc(plan, *s)),
        Expr::Index(s, i, w) => Expr::Index(reloc(plan, *s), e(i), *w),
        Expr::Lit(n) => Expr::Lit(*n),
        Expr::Lit32(n) => Expr::Lit32(*n),
        Expr::ConstAddr(n) => Expr::ConstAddr(n.clone()),
        Expr::Bin(op, a, c, w) => Expr::Bin(*op, e(a), e(c), *w),
        Expr::Bin32(op, a, c) => Expr::Bin32(*op, e(a), e(c)),
        Expr::Call(n, args) => Expr::Call(
            n.clone(),
            args.iter().map(|a| remap_expr(a, plan)).collect(),
        ),
        Expr::Trunc(a) => Expr::Trunc(e(a)),
        Expr::Trunc32(a) => Expr::Trunc32(e(a)),
        Expr::Widen(a) => Expr::Widen(e(a)),
        Expr::Peek(a) => Expr::Peek(e(a)),
        Expr::InPort(a) => Expr::InPort(e(a)),
        Expr::Halt(a) => Expr::Halt(e(a)),
        Expr::Deref(p, off) => Expr::Deref(e(p), *off),
        Expr::Deref32(p, off) => Expr::Deref32(e(p), *off),
        Expr::PtrIndex { ptr, off, index } => Expr::PtrIndex {
            ptr: e(ptr),
            off: *off,
            index: e(index),
        },
        Expr::MulConst(a, k) => Expr::MulConst(e(a), *k),
        Expr::LoadAt(a, w) => Expr::LoadAt(e(a), *w),
        Expr::Cmp {
            cmp,
            lhs,
            rhs,
            signed,
        } => Expr::Cmp {
            cmp: *cmp,
            lhs: e(lhs),
            rhs: e(rhs),
            signed: *signed,
        },
        Expr::Logic { and, lhs, rhs } => Expr::Logic {
            and: *and,
            lhs: e(lhs),
            rhs: e(rhs),
        },
        Expr::ShiftVar {
            left,
            e: x,
            amount,
            w,
        } => Expr::ShiftVar {
            left: *left,
            e: e(x),
            amount: e(amount),
            w: *w,
        },
        Expr::Shift32 { left, e: x, k } => Expr::Shift32 {
            left: *left,
            e: e(x),
            k: *k,
        },
    }
}
