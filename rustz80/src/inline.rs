//! A conservative function **inliner** (IR → IR), run before codegen.
//!
//! It folds each **single-call-site**, early-return-free function into its one caller —
//! scalar, void, or wide (a `u32`-param/`u32`-return kernel; its two-slot params bind via
//! `Assign32` and pure wide args substitute just like scalars, so a once-called shared
//! kernel is as compact inlined as the loop it replaced). Single-call-site is the key:
//! inlining there never *duplicates* code, so
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
            // A wide-return call (`let w: u32 = kernel(..)`) lands in `Assign32`.
            Stmt::Assign32(slot, Expr::Call(g, args)) => {
                if go(&g, stack) {
                    let gf = cand[&g].clone();
                    expand(&g, &gf, args, Some(slot), cand, stack, water, max, &mut out);
                } else {
                    out.push(Stmt::Assign32(slot, Expr::Call(g, args)));
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

    // When the callee writes no memory, an *effect-free* arg (e.g. a `self.field` read)
    // is stable throughout the body — the body only ever writes its own relocated scratch
    // slots (and the aliased result, which no arg reads), never the caller memory the arg
    // reads. So such an arg substitutes just like a pure one, no bind/copy. This is what
    // lets a kernel called on struct fields (`gcd_u32(self.n, self.d)`) fold for free.
    let body_pure = no_mem_writes(&g.body);

    // A `u32` param spans *two* consecutive slots (low, high) but is one call arg;
    // the first arg rides wide when `wide_param`, the second when `wide_second`
    // (the cross-call convention — irrelevant here, but it fixes the slot layout).
    let arg_wide = |i: usize| (i == 0 && g.wide_param) || (i == 1 && g.wide_second);

    // Result-aliasing: when the call binds a slot (`let g = kernel(..)`) and the kernel
    // returns one of its own *locals* (e.g. `gcd`'s reduced `x`), relocate that local
    // straight onto the result slot instead of computing-then-copying. This is what
    // makes a folded kernel byte-identical to the hand-inlined loop — no trailing copy.
    // Sound only if no argument reads the result slot (else the body could clobber a
    // still-live input); a wide result also reserves its high word.
    let alias: Option<(usize, usize, bool)> = match (result, g.ret.first()) {
        (Some(r), Some(Expr::Var32(s)))
            if *s >= g.params
                && !args
                    .iter()
                    .any(|a| reads_slot(a, r) || reads_slot(a, r + 1)) =>
        {
            Some((*s, r, true))
        }
        (Some(r), Some(Expr::Var(s)))
            if *s >= g.params && !args.iter().any(|a| reads_slot(a, r)) =>
        {
            Some((*s, r, false))
        }
        _ => None,
    };

    // Build the slot plan arg-by-arg (params), then 1:1 for locals. Track the
    // relocated params to bind after — a wide bind is one `Assign32`.
    let mut plan: Vec<Slot> = Vec::with_capacity(g.n_locals);
    let mut next = base;
    let mut binds: Vec<(usize, Expr, bool)> = Vec::new();
    for (i, a) in args.iter().enumerate() {
        let s = plan.len(); // this arg's (low) slot
        let wide = arg_wide(i);
        // A wide value lives across `s`/`s+1` but is only ever named by its low slot
        // (`Var32(s)`/`Assign32(s)`), so the read-only test on `s` covers both words.
        let readonly = !written.contains(&s) && !addrd.contains(&s);
        if readonly && (pure(a) || (body_pure && effect_free(a))) {
            plan.push(Slot::Subst(a.clone()));
            if wide {
                plan.push(Slot::Subst(a.clone())); // high word — never named alone
            }
        } else {
            binds.push((next, a.clone(), wide));
            plan.push(Slot::Reloc(next));
            next += 1;
            if wide {
                plan.push(Slot::Reloc(next));
                next += 1;
            }
        }
    }
    // Locals (`params..n_locals`) relocate one caller slot each — a wide local's two
    // callee slots stay adjacent in the caller, preserving low/high addressing. The
    // aliased return-local (if any) lands on the result slot instead of fresh scratch.
    for s in g.params..g.n_locals {
        if let Some((as_slot, rslot, wide)) = alias {
            if s == as_slot {
                plan.push(Slot::Reloc(rslot));
                continue;
            }
            if wide && s == as_slot + 1 {
                plan.push(Slot::Reloc(rslot + 1));
                continue;
            }
        }
        plan.push(Slot::Reloc(next));
        next += 1;
    }
    *water = next;
    if *water > *max {
        *max = *water;
    }

    // Bind the slot-backed params (substituted ones need no copy).
    for (n, a, wide) in binds {
        if wide {
            out.push(Stmt::Assign32(n, a));
        } else {
            out.push(Stmt::Assign(n, a));
        }
    }

    // Remap the body into the caller, then recursively inline within it (nested helpers
    // allocate above `next`).
    let remapped: Vec<Stmt> = g.body.iter().map(|s| remap_stmt(s, &plan)).collect();
    stack.push(g_name.to_string());
    let inlined = inline_stmts(remapped, cand, stack, water, max);
    stack.pop();
    out.extend(inlined);

    // The tail return copies into the result slot — unless it was aliased onto it, in
    // which case the body already left the value there.
    if alias.is_none() {
        if let (Some(slot), Some(r)) = (result, g.ret.first()) {
            // A wide return feeds the caller's `Assign32` slot; a scalar its `Assign`.
            if g.wide_ret {
                out.push(Stmt::Assign32(slot, remap_expr(r, &plan)));
            } else {
                out.push(Stmt::Assign(slot, remap_expr(r, &plan)));
            }
        }
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

/// Does expression `e` read caller slot `slot` (as a value, address, or index base)? Used
/// to keep result-aliasing sound — an argument that reads the result slot forbids the alias.
fn reads_slot(e: &Expr, slot: usize) -> bool {
    let r = |x: &Expr| reads_slot(x, slot);
    match e {
        Expr::Var(s) | Expr::Var32(s) | Expr::AddrOf(s) => *s == slot,
        Expr::Index(s, i, _) => *s == slot || r(i),
        Expr::Lit(_) | Expr::Lit32(_) | Expr::ConstAddr(_) => false,
        Expr::Bin(_, a, b, _) | Expr::Bin32(_, a, b) => r(a) || r(b),
        Expr::Cmp { lhs, rhs, .. }
        | Expr::Logic { lhs, rhs, .. }
        | Expr::Cmp32 { lhs, rhs, .. } => r(lhs) || r(rhs),
        Expr::Call(_, args) => args.iter().any(r),
        Expr::Trunc(a)
        | Expr::Trunc32(a)
        | Expr::Widen(a)
        | Expr::SignExtend(a)
        | Expr::Peek(a)
        | Expr::InPort(a)
        | Expr::Halt(a)
        | Expr::MulConst(a, _)
        | Expr::LoadAt(a, _)
        | Expr::Deref(a, _)
        | Expr::Deref32(a, _)
        | Expr::Shift32 { e: a, .. } => r(a),
        Expr::PtrIndex { ptr, index, .. } => r(ptr) || r(index),
        Expr::ShiftVar { e, amount, .. } => r(e) || r(amount),
    }
}

/// Does the body write memory anywhere (a raw poke, a pointer/array store, a fill)? If not,
/// any memory an *effect-free* arg reads stays stable across the inlined body — the premise
/// that lets a `self.field` arg substitute rather than copy.
fn no_mem_writes(body: &[Stmt]) -> bool {
    body.iter().all(|s| match s {
        Stmt::Poke(..)
        | Stmt::Store(..)
        | Stmt::Store32(..)
        | Stmt::StoreAt(..)
        | Stmt::StoreIndex(..)
        | Stmt::PtrStoreIndex { .. }
        | Stmt::Fill { .. } => false,
        Stmt::If(_, t, e) => no_mem_writes(t) && no_mem_writes(e),
        Stmt::While(_, b) | Stmt::Loop(b) => no_mem_writes(b),
        Stmt::ForRange { body, .. } => no_mem_writes(body),
        _ => true,
    })
}

/// An expression with no observable side effect (no call, port read, or halt). Memory
/// *reads* (`Deref`, `Peek`, `Index`) qualify — their value is stable only when the context
/// writes no memory, which the caller checks separately via [`no_mem_writes`].
fn effect_free(e: &Expr) -> bool {
    match e {
        Expr::Call(..) | Expr::InPort(_) | Expr::Halt(_) => false,
        Expr::Lit(_) | Expr::Var(_) | Expr::AddrOf(_) | Expr::ConstAddr(_) => true,
        Expr::Lit32(_) | Expr::Var32(_) => true,
        Expr::Bin(_, a, b, _) | Expr::Bin32(_, a, b) => effect_free(a) && effect_free(b),
        Expr::Cmp { lhs, rhs, .. }
        | Expr::Logic { lhs, rhs, .. }
        | Expr::Cmp32 { lhs, rhs, .. } => effect_free(lhs) && effect_free(rhs),
        Expr::Index(_, i, _) => effect_free(i),
        Expr::Trunc(x)
        | Expr::Trunc32(x)
        | Expr::Widen(x)
        | Expr::SignExtend(x)
        | Expr::Peek(x) => effect_free(x),
        Expr::Deref(p, _) | Expr::Deref32(p, _) => effect_free(p),
        Expr::PtrIndex { ptr, index, .. } => effect_free(ptr) && effect_free(index),
        Expr::MulConst(x, _) | Expr::LoadAt(x, _) => effect_free(x),
        Expr::ShiftVar { e, amount, .. } => effect_free(e) && effect_free(amount),
        Expr::Shift32 { e, .. } => effect_free(e),
    }
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
            | Expr::SignExtend(a)
            | Expr::Halt(a)
            | Expr::Shift32 { e: a, .. } => ex(a, out),
            Expr::PtrIndex { ptr, index, .. } => {
                ex(ptr, out);
                ex(index, out);
            }
            Expr::Cmp { lhs, rhs, .. }
            | Expr::Logic { lhs, rhs, .. }
            | Expr::Cmp32 { lhs, rhs, .. } => {
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
        | Expr::SignExtend(a)
        | Expr::Halt(a)
        | Expr::Shift32 { e: a, .. } => count_expr(a, m),
        Expr::PtrIndex { ptr, index, .. } => {
            count_expr(ptr, m);
            count_expr(index, m);
        }
        Expr::Cmp { lhs, rhs, .. }
        | Expr::Logic { lhs, rhs, .. }
        | Expr::Cmp32 { lhs, rhs, .. } => {
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
        // A wide read: substitute the (pure, read-only) wide arg, or read the
        // relocated slot pair. `&local` still only names relocated slots.
        Expr::Var32(s) => match &plan[*s] {
            Slot::Subst(a) => a.clone(),
            Slot::Reloc(n) => Expr::Var32(*n),
        },
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
        Expr::SignExtend(a) => Expr::SignExtend(e(a)),
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
        Expr::Cmp32 { cmp, lhs, rhs } => Expr::Cmp32 {
            cmp: *cmp,
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
