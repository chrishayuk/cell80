//! `canon` — the deterministic **source canonicalization pass** (M2.5) and **dialect
//! normalizer** (M2.6). Text in, text out, *before* hashing and lowering: the cell layer
//! canonicalizes the source it is about to hash so that structurally identical programs
//! reach one byte-identical spelling — the precondition for precipitation (same schema ⇒
//! same artifact hash, `docs/math-campaign-amendment.md`).
//!
//! Two strengths, chosen by [`CanonMode`]:
//!
//! - **Light** (default, semantics-preserving): the dialect normalizer only — strip
//!   statement macros, rewrite a trailing `let`/`return` into a tail expression, collapse
//!   redundant parens. If no rule fires the input text is returned **byte-identical**
//!   (no hash churn for already-clean sources).
//! - **Full** (the campaign/compose path): additionally rewrites every *straight-line
//!   arithmetic* `fn` into canonical form — bindings alpha-renamed to slots (`q0…` for
//!   parameters, `v0…` for derived values) in dataflow order, ops topologically sorted
//!   with a deterministic tie-break, constants folded exactly (decimal literals become
//!   exact fractions), `*`/`/` chains flattened so division happens **once at the end**
//!   (defer-division), and the arithmetic lane auto-widened to `u32` when any literal or
//!   folded constant exceeds `u16::MAX`.
//!
//! Full mode is deliberately **not** semantics-preserving around truncating division:
//! reassociating `a / b * c` into `a * c / b` is the defer-division *repair* (precision
//! fix), applied deterministically and recorded as a typed [`Repair`]. Constants are
//! treated as exact rationals; a constant division that cannot be exact is a typed
//! compile error ([`DiagCode::InexactConstDivision`]), not a silently truncated value.
//! Functions outside the straight-line subset (control flow, state, casts, intrinsics)
//! fall back to Light with a [`DiagCode::NonStraightLine`] note — never an error.
//!
//! The **unit base-scale table** ([`canonical_unit`], versioned by
//! [`UNIT_TABLE_VERSION`]) lives here, not in any prompt: money → cents, time → seconds,
//! unknown nouns → count, rates → explicit `numerator_per_denominator`. Scaling is
//! applied only where a [`UnitHint`] names a binding or literal, and every application
//! is recorded ([`DiagCode::UnitScaled`]).
//!
//! This pass is invoked by the cell layer on source text (so the canonical form reaches
//! both the manifest's source hash and codegen); the `compile_*` entry points in this
//! crate stay pure and never canonicalize behind the caller's back.

use crate::diag::{Diag, DiagCode, Repair};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::visit_mut::VisitMut;

/// Version of the unit base-scale table below. Bumped whenever an entry changes —
/// canonical hashes are only comparable within one table version.
pub const UNIT_TABLE_VERSION: u32 = 1;

/// How hard to canonicalize. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CanonMode {
    /// Return the source untouched.
    Off,
    /// Dialect normalizer only; byte-identical output when no rule fires.
    #[default]
    Light,
    /// Normalizer + straight-line canonicalization (slots, topo order, folding,
    /// defer-division, width).
    Full,
}

/// A caller-supplied unit tag for a parameter, `let` binding, or literal (matched by
/// its exact source text, e.g. `"16.50"`). Units are normalized through the base-scale
/// table; scale factors are applied to hinted *literals* and recorded for parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitHint {
    pub ident: String,
    pub unit: String,
}

/// Options for [`canonicalize_source`].
#[derive(Debug, Clone, Default)]
pub struct CanonOptions {
    pub mode: CanonMode,
    pub hints: Vec<UnitHint>,
    /// Widen the arithmetic lane to `u32` regardless of what the constants require —
    /// the compose harness sets this (composed cells default to a `u32` return).
    pub wide_default: bool,
}

/// One alpha-rename: the source name survives only here, never in the rendered Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rename {
    pub source_name: String,
    pub slot: String,
    /// Canonical unit (post-table), when a hint named this binding.
    pub unit: Option<String>,
    /// Multiplicative factor into the canonical base unit (1 = already canonical).
    pub factor: u32,
}

/// What [`canonicalize_source`] returns.
#[derive(Debug, Clone)]
pub struct CanonOutput {
    /// The canonical source. Equal to the input iff `changed` is false.
    pub source: String,
    pub changed: bool,
    pub renames: Vec<Rename>,
    pub repairs: Vec<Repair>,
    /// The arithmetic lane was widened to u32 (by constants or `wide_default`).
    pub widened: bool,
}

// ---------------------------------------------------------------------------
// The unit base-scale table (fixed and versioned here, not in any prompt).
// ---------------------------------------------------------------------------

/// Canonical base + multiplicative factor for one unit word. Unknown nouns are the
/// `count` convention by design (sheep, cups, GB — a count of that noun).
fn base_scale(word: &str) -> (&'static str, u32) {
    match word {
        "cents" | "cent" | "money" => ("cents", 1),
        "dollars" | "dollar" | "usd" | "bucks" | "pounds" | "gbp" | "euros" | "euro" | "eur" => {
            ("cents", 100)
        }
        "seconds" | "second" | "secs" | "sec" | "time" => ("seconds", 1),
        "minutes" | "minute" | "mins" | "min" => ("seconds", 60),
        "hours" | "hour" | "hrs" | "hr" => ("seconds", 3600),
        "days" | "day" => ("seconds", 86400),
        "weeks" | "week" => ("seconds", 604800),
        "meters" | "meter" | "metres" | "metre" | "m" | "distance" => ("meters", 1),
        "km" | "kilometers" | "kilometres" => ("meters", 1000),
        "miles" | "mile" => ("meters", 1609),
        "" | "scalar" | "ratio" => ("scalar", 1),
        "count" | "items" | "item" => ("count", 1),
        _ => ("count", 1),
    }
}

/// Normalize a unit through the base-scale table: `(canonical, num_factor, den_factor)`.
/// A rate `x_per_y` normalizes each side (`dollars_per_egg` → `("cents_per_count", 100, 1)`);
/// a plain unit has `den_factor == 1`.
pub fn canonical_unit(unit: &str) -> (String, u32, u32) {
    match unit.split_once("_per_") {
        Some((num, den)) => {
            let (nb, nf) = base_scale(num);
            let (db, df) = base_scale(den);
            (format!("{nb}_per_{db}"), nf, df)
        }
        None => {
            let (b, f) = base_scale(unit);
            (b.to_string(), f, 1)
        }
    }
}

// ---------------------------------------------------------------------------
// Exact rational constants.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rat {
    n: i128,
    d: i128, // > 0, reduced
}

fn gcd128(a: i128, b: i128) -> i128 {
    let (mut x, mut y) = (a.abs(), b.abs());
    while y != 0 {
        let t = x % y;
        x = y;
        y = t;
    }
    x
}

impl Rat {
    fn int(n: i128) -> Rat {
        Rat { n, d: 1 }
    }
    fn new(n: i128, d: i128) -> Rat {
        debug_assert!(d != 0);
        let s = if d < 0 { -1 } else { 1 };
        let g = gcd128(n, d).max(1);
        Rat {
            n: s * n / g,
            d: s * d / g,
        }
    }
    fn mul(self, o: Rat) -> Rat {
        Rat::new(self.n * o.n, self.d * o.d)
    }
    fn div(self, o: Rat) -> Rat {
        Rat::new(self.n * o.d, self.d * o.n)
    }
    fn add(self, o: Rat) -> Rat {
        Rat::new(self.n * o.d + o.n * self.d, self.d * o.d)
    }
    fn sub(self, o: Rat) -> Rat {
        Rat::new(self.n * o.d - o.n * self.d, self.d * o.d)
    }
    fn is_int(self) -> bool {
        self.d == 1
    }
    fn is_zero(self) -> bool {
        self.n == 0
    }
    fn is_one(self) -> bool {
        self.n == 1 && self.d == 1
    }
}

/// Exact decimal-literal parse: `"16.50"` → 33/2. No float arithmetic anywhere.
fn parse_decimal(digits: &str) -> Option<Rat> {
    if digits.contains(['e', 'E']) {
        return None; // exponent floats stay out of the dialect
    }
    let clean: String = digits.chars().filter(|c| *c != '_').collect();
    match clean.split_once('.') {
        Some((int, frac)) => {
            let scale = 10i128.checked_pow(frac.len() as u32)?;
            let int: i128 = if int.is_empty() { 0 } else { int.parse().ok()? };
            let frac: i128 = if frac.is_empty() {
                0
            } else {
                frac.parse().ok()?
            };
            Some(Rat::new(int * scale + frac, scale))
        }
        None => clean.parse().ok().map(Rat::int),
    }
}

// ---------------------------------------------------------------------------
// Light normalization (the dialect normalizer): semantics-preserving rewrites.
// ---------------------------------------------------------------------------

struct ParenFold {
    fired: bool,
}

impl VisitMut for ParenFold {
    fn visit_expr_mut(&mut self, e: &mut syn::Expr) {
        syn::visit_mut::visit_expr_mut(self, e); // bottom-up
        if let syn::Expr::Paren(p) = e {
            if matches!(
                &*p.expr,
                syn::Expr::Paren(_) | syn::Expr::Lit(_) | syn::Expr::Path(_)
            ) {
                let inner = (*p.expr).clone();
                *e = inner;
                self.fired = true;
            }
        }
    }
}

fn returns_value(sig: &syn::Signature) -> bool {
    !matches!(sig.output, syn::ReturnType::Default)
}

/// Apply the normalizer to one fn body. Returns whether anything fired.
fn normalize_block(block: &mut syn::Block, value_fn: bool, repairs: &mut Vec<Repair>) -> bool {
    let mut fired = false;
    // Strip statement macros — the dialect has none; a bare `println!(…);` line is
    // exactly the model-dialect noise the normalizer exists for.
    let before = block.stmts.len();
    block.stmts.retain(|s| {
        if let syn::Stmt::Macro(m) = s {
            repairs.push(Repair::new(
                DiagCode::StatementMacro,
                format!("stripped `{}!`", m.mac.path.to_token_stream()),
            ));
            false
        } else {
            true
        }
    });
    fired |= block.stmts.len() != before;
    // Trailing `let` / trailing `return` → tail expression (the row93 class).
    if value_fn {
        let rewrite = match block.stmts.last() {
            Some(syn::Stmt::Local(l)) => l.init.as_ref().map(|i| ((*i.expr).clone(), "let")),
            Some(syn::Stmt::Expr(syn::Expr::Return(r), _)) => {
                r.expr.as_ref().map(|e| ((**e).clone(), "return"))
            }
            _ => None,
        };
        if let Some((tail, kind)) = rewrite {
            *block.stmts.last_mut().unwrap() = syn::Stmt::Expr(tail, None);
            repairs.push(Repair::new(
                DiagCode::TrailingLet,
                format!("rewrote trailing `{kind}` to a tail expression"),
            ));
            fired = true;
        }
    }
    // Collapse redundant parens.
    let mut fold = ParenFold { fired: false };
    fold.visit_block_mut(block);
    if fold.fired {
        repairs.push(Repair::new(
            DiagCode::RedundantParens,
            "collapsed redundant parentheses",
        ));
        fired = true;
    }
    fired
}

// ---------------------------------------------------------------------------
// Full canonicalization: the straight-line arithmetic DAG.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Node {
    Param(usize),
    Const(Rat),
    /// `sum(pos) − sum(neg) + k` — n-ary additive, operand lists sorted by structural
    /// key. Reordering is sound: `+`/`-` are associative and commutative mod 2^width
    /// (release-rustc wrapping semantics).
    Sum {
        pos: Vec<usize>,
        neg: Vec<usize>,
        k: Rat,
    },
    /// `(∏ num · k) / ∏ den` — a flattened `*`/`/` chain, divided **once at the end**
    /// (defer-division). Constant factors live in the exact rational `k`.
    MulDiv {
        num: Vec<usize>,
        den: Vec<usize>,
        k: Rat,
    },
    Rem(usize, usize),
    Call {
        name: String,
        args: Vec<usize>,
    },
}

enum Fail {
    /// Outside the straight-line subset — the fn falls back to Light, never an error.
    Soft(String),
    /// A typed defect in the arithmetic itself — a real compile error.
    Hard(Diag),
}

fn soft<T>(reason: impl Into<String>) -> Result<T, Fail> {
    Err(Fail::Soft(reason.into()))
}

struct FnCanon<'a> {
    nodes: Vec<Node>,
    keys: Vec<String>,
    memo: HashMap<String, usize>,
    env: HashMap<String, usize>,
    /// let-bound names per node id, for renames + dead-let reporting.
    let_names: Vec<(String, usize)>,
    params: Vec<(String, bool)>, // (source name, declared u32)
    param_nodes: Vec<usize>,
    hints: &'a HashMap<String, String>,
    repairs: Vec<Repair>,
    max_lit: i128,
}

struct FnOut {
    text: String,
    renames: Vec<Rename>,
    repairs: Vec<Repair>,
    widened: bool,
}

impl<'a> FnCanon<'a> {
    fn intern(&mut self, node: Node, key: String) -> usize {
        if let Some(&id) = self.memo.get(&key) {
            return id;
        }
        let id = self.nodes.len();
        self.nodes.push(node);
        self.keys.push(key.clone());
        self.memo.insert(key, id);
        id
    }

    fn intern_const(&mut self, r: Rat) -> usize {
        self.intern(Node::Const(r), format!("c{}/{}", r.n, r.d))
    }

    fn key_list(&self, ids: &[usize]) -> String {
        ids.iter()
            .map(|&i| self.keys[i].as_str())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn as_const(&self, id: usize) -> Option<Rat> {
        match self.nodes[id] {
            Node::Const(r) => Some(r),
            _ => None,
        }
    }

    /// The hinted scale factor for a name or literal spelling, with its canonical unit.
    fn hint_scale(&self, ident: &str) -> Option<(String, u32, u32)> {
        self.hints.get(ident).map(|u| canonical_unit(u))
    }

    fn scale_const(&mut self, id: usize, ident: &str) -> usize {
        let Some((unit, nf, df)) = self.hint_scale(ident) else {
            return id;
        };
        let Some(r) = self.as_const(id) else {
            return id;
        };
        if nf == 1 && df == 1 {
            return id;
        }
        let scaled = r.mul(Rat::new(nf as i128, df as i128));
        self.repairs.push(Repair::new(
            DiagCode::UnitScaled,
            format!(
                "`{ident}`: {} -> {unit} factor={nf}{}",
                self.hints[ident],
                if df != 1 {
                    format!("/{df}")
                } else {
                    String::new()
                }
            ),
        ));
        self.intern_const(scaled)
    }

    fn build(&mut self, e: &syn::Expr) -> Result<usize, Fail> {
        match e {
            syn::Expr::Paren(p) => self.build(&p.expr),
            syn::Expr::Group(g) => self.build(&g.expr),
            syn::Expr::Lit(l) => match &l.lit {
                syn::Lit::Int(i) => {
                    let v: i128 = i
                        .base10_parse()
                        .map_err(|_| Fail::Soft("unparseable integer".into()))?;
                    self.max_lit = self.max_lit.max(v);
                    let id = self.intern_const(Rat::int(v));
                    Ok(self.scale_const(id, &i.base10_digits().replace('_', "")))
                }
                syn::Lit::Float(f) => {
                    let digits = f.base10_digits().to_string();
                    let r = parse_decimal(&digits)
                        .ok_or_else(|| Fail::Soft(format!("float literal `{digits}`")))?;
                    let id = self.intern_const(r);
                    Ok(self.scale_const(id, digits.trim_end_matches('.')))
                }
                _ => soft("non-numeric literal"),
            },
            syn::Expr::Path(p) => {
                let id = p
                    .path
                    .get_ident()
                    .ok_or(Fail::Soft("path expression".into()))?
                    .to_string();
                self.env
                    .get(&id)
                    .copied()
                    .ok_or(Fail::Soft(format!("unknown name `{id}`")))
            }
            syn::Expr::Unary(u) => match u.op {
                syn::UnOp::Neg(_) => {
                    let x = self.build(&u.expr)?;
                    match self.as_const(x) {
                        Some(r) => Ok(self.intern_const(Rat::int(0).sub(r))),
                        None => soft("negation of a non-constant"),
                    }
                }
                _ => soft("unary operator"),
            },
            syn::Expr::Binary(b) => match b.op {
                syn::BinOp::Add(_) | syn::BinOp::Sub(_) => self.build_sum(e),
                syn::BinOp::Mul(_) | syn::BinOp::Div(_) => self.build_muldiv(e),
                syn::BinOp::Rem(_) => {
                    let a = self.build(&b.left)?;
                    let bb = self.build(&b.right)?;
                    if let (Some(x), Some(y)) = (self.as_const(a), self.as_const(bb)) {
                        if x.is_int() && y.is_int() {
                            if y.n == 0 {
                                return Err(Fail::Hard(Diag::new(
                                    DiagCode::InexactConstDivision,
                                    "remainder by constant zero",
                                )));
                            }
                            return Ok(self.intern_const(Rat::int(x.n % y.n)));
                        }
                    }
                    let key = format!("r({}|{})", self.keys[a], self.keys[bb]);
                    Ok(self.intern(Node::Rem(a, bb), key))
                }
                _ => soft("non-arithmetic operator"),
            },
            syn::Expr::Call(c) => {
                let name = match &*c.func {
                    syn::Expr::Path(p) => p
                        .path
                        .get_ident()
                        .ok_or(Fail::Soft("qualified call".into()))?
                        .to_string(),
                    _ => return soft("computed callee"),
                };
                let mut args = Vec::new();
                for a in &c.args {
                    if !matches!(
                        a,
                        syn::Expr::Path(_) | syn::Expr::Lit(_) | syn::Expr::Paren(_)
                    ) {
                        self.repairs.push(Repair::new(
                            DiagCode::CompoundCallArg,
                            format!("`{name}`: compound argument bound to a slot"),
                        ));
                    }
                    args.push(self.build(a)?);
                }
                let key = format!("f{name}({})", self.key_list(&args));
                Ok(self.intern(Node::Call { name, args }, key))
            }
            other => soft(format!(
                "expression outside the straight-line subset ({})",
                expr_kind(other)
            )),
        }
    }

    fn build_sum(&mut self, e: &syn::Expr) -> Result<usize, Fail> {
        let mut pos = Vec::new();
        let mut neg = Vec::new();
        let mut k = Rat::int(0);
        self.sum_terms(e, true, &mut pos, &mut neg, &mut k)?;
        pos.sort_by(|a, b| self.keys[*a].cmp(&self.keys[*b]));
        neg.sort_by(|a, b| self.keys[*a].cmp(&self.keys[*b]));
        // Cancel matching +x / −x pairs (sound mod 2^width).
        let mut i = 0;
        while i < pos.len() {
            if let Some(j) = neg.iter().position(|&n| n == pos[i]) {
                neg.remove(j);
                pos.remove(i);
            } else {
                i += 1;
            }
        }
        if pos.is_empty() && neg.is_empty() {
            return Ok(self.intern_const(k));
        }
        if pos.len() == 1 && neg.is_empty() && k.is_zero() {
            return Ok(pos[0]);
        }
        let key = format!(
            "s({}|{}|{}/{})",
            self.key_list(&pos),
            self.key_list(&neg),
            k.n,
            k.d
        );
        Ok(self.intern(Node::Sum { pos, neg, k }, key))
    }

    fn sum_terms(
        &mut self,
        e: &syn::Expr,
        plus: bool,
        pos: &mut Vec<usize>,
        neg: &mut Vec<usize>,
        k: &mut Rat,
    ) -> Result<(), Fail> {
        match e {
            syn::Expr::Paren(p) => self.sum_terms(&p.expr, plus, pos, neg, k),
            syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::Add(_)) => {
                self.sum_terms(&b.left, plus, pos, neg, k)?;
                self.sum_terms(&b.right, plus, pos, neg, k)
            }
            syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::Sub(_)) => {
                self.sum_terms(&b.left, plus, pos, neg, k)?;
                self.sum_terms(&b.right, !plus, pos, neg, k)
            }
            other => {
                let id = self.build(other)?;
                match self.as_const(id) {
                    Some(r) => *k = if plus { k.add(r) } else { k.sub(r) },
                    None => {
                        if plus {
                            pos.push(id)
                        } else {
                            neg.push(id)
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn build_muldiv(&mut self, e: &syn::Expr) -> Result<usize, Fail> {
        let mut num = Vec::new();
        let mut den = Vec::new();
        let mut k = Rat::int(1);
        self.mul_factors(e, false, &mut num, &mut den, &mut k)?;
        if k.is_zero() {
            return Ok(self.intern_const(Rat::int(0)));
        }
        num.sort_by(|a, b| self.keys[*a].cmp(&self.keys[*b]));
        den.sort_by(|a, b| self.keys[*a].cmp(&self.keys[*b]));
        // Cancel matching num/den factors — exact under the single deferred division.
        let mut i = 0;
        while i < num.len() {
            if let Some(j) = den.iter().position(|&n| n == num[i]) {
                den.remove(j);
                num.remove(i);
            } else {
                i += 1;
            }
        }
        if num.is_empty() && den.is_empty() {
            // The whole chain is constant: it must be exact — `100/3` is a wrong
            // plan, not a silently truncated 33.
            if !k.is_int() {
                return Err(Fail::Hard(
                    Diag::new(
                        DiagCode::InexactConstDivision,
                        format!("constant division folds to {}/{}", k.n, k.d),
                    )
                    .with_fix("make the constant division exact or keep a variable factor"),
                ));
            }
            return Ok(self.intern_const(k));
        }
        if num.len() == 1 && den.is_empty() && k.is_one() {
            return Ok(num[0]);
        }
        let key = format!(
            "m({}|{}|{}/{})",
            self.key_list(&num),
            self.key_list(&den),
            k.n,
            k.d
        );
        Ok(self.intern(Node::MulDiv { num, den, k }, key))
    }

    fn mul_factors(
        &mut self,
        e: &syn::Expr,
        invert: bool,
        num: &mut Vec<usize>,
        den: &mut Vec<usize>,
        k: &mut Rat,
    ) -> Result<(), Fail> {
        match e {
            syn::Expr::Paren(p) => self.mul_factors(&p.expr, invert, num, den, k),
            syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::Mul(_)) => {
                self.mul_factors(&b.left, invert, num, den, k)?;
                self.mul_factors(&b.right, invert, num, den, k)
            }
            syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::Div(_)) => {
                self.mul_factors(&b.left, invert, num, den, k)?;
                self.mul_factors(&b.right, !invert, num, den, k)
            }
            other => {
                let id = self.build(other)?;
                match self.as_const(id) {
                    Some(r) => {
                        if invert {
                            if r.is_zero() {
                                return Err(Fail::Hard(Diag::new(
                                    DiagCode::InexactConstDivision,
                                    "division by constant zero",
                                )));
                            }
                            *k = k.div(r);
                        } else {
                            *k = k.mul(r);
                        }
                    }
                    None => {
                        if invert {
                            den.push(id)
                        } else {
                            num.push(id)
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Canonicalize one free fn. `Err(Soft)` = not straight-line (fall back to Light).
    fn run(
        f: &syn::ItemFn,
        hints: &'a HashMap<String, String>,
        wide_default: bool,
    ) -> Result<FnOut, Fail> {
        if f.sig.generics.params.iter().next().is_some()
            || f.sig.unsafety.is_some()
            || f.sig.abi.is_some()
        {
            return soft("generics/unsafe/abi");
        }
        let ret_wide = match &f.sig.output {
            syn::ReturnType::Default => return soft("no return value"),
            syn::ReturnType::Type(_, t) => match type_width(t) {
                Some(w) => w,
                None => return soft("non-u16/u32 return type"),
            },
        };
        let mut c = FnCanon {
            nodes: Vec::new(),
            keys: Vec::new(),
            memo: HashMap::new(),
            env: HashMap::new(),
            let_names: Vec::new(),
            params: Vec::new(),
            param_nodes: Vec::new(),
            hints,
            repairs: Vec::new(),
            max_lit: 0,
        };
        for p in &f.sig.inputs {
            let syn::FnArg::Typed(pt) = p else {
                return soft("self parameter");
            };
            let syn::Pat::Ident(pi) = &*pt.pat else {
                return soft("non-ident parameter pattern");
            };
            let wide = match type_width(&pt.ty) {
                Some(w) => w,
                None => return soft("non-u16/u32 parameter"),
            };
            let pos = c.params.len();
            let name = pi.ident.to_string();
            let id = c.intern(Node::Param(pos), format!("p{pos}"));
            c.env.insert(name.clone(), id);
            c.params.push((name, wide));
            c.param_nodes.push(id);
        }
        // Statements: `let <ident> = <arith>;` … ending in a value tail.
        let stmts = &f.block.stmts;
        let Some((tail_stmt, bindings)) = stmts.split_last() else {
            return soft("empty body");
        };
        for s in bindings {
            let syn::Stmt::Local(l) = s else {
                return soft("non-let statement");
            };
            let name = match &l.pat {
                syn::Pat::Ident(pi) => pi.ident.to_string(),
                syn::Pat::Type(pt) => match &*pt.pat {
                    syn::Pat::Ident(pi) => pi.ident.to_string(),
                    _ => return soft("let pattern"),
                },
                _ => return soft("let pattern"),
            };
            let Some(init) = &l.init else {
                return soft("let without initializer");
            };
            if init.diverge.is_some() {
                return soft("let-else");
            }
            let mut id = c.build(&init.expr)?;
            id = c.scale_const(id, &name);
            c.env.insert(name.clone(), id);
            c.let_names.push((name, id));
        }
        let root = match tail_stmt {
            syn::Stmt::Expr(e, None) => c.build(e)?,
            _ => return soft("no tail expression"),
        };

        // ---- linearize: topo order with deterministic tie-break ----
        let mut reachable: HashSet<usize> = HashSet::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if !reachable.insert(id) {
                continue;
            }
            for d in node_deps(&c.nodes[id]) {
                stack.push(d);
            }
        }
        let is_op = |n: &Node| {
            matches!(
                n,
                Node::Sum { .. } | Node::MulDiv { .. } | Node::Rem(..) | Node::Call { .. }
            )
        };
        let mut pending: Vec<usize> = reachable
            .iter()
            .copied()
            .filter(|&i| is_op(&c.nodes[i]))
            .collect();
        pending.sort();
        let mut emitted: HashSet<usize> = reachable
            .iter()
            .copied()
            .filter(|&i| !is_op(&c.nodes[i]))
            .collect();
        let mut order: Vec<usize> = Vec::new();
        while !pending.is_empty() {
            let mut ready: Vec<usize> = pending
                .iter()
                .copied()
                .filter(|&i| node_deps(&c.nodes[i]).iter().all(|d| emitted.contains(d)))
                .collect();
            debug_assert!(!ready.is_empty(), "op DAG is acyclic by construction");
            ready.sort_by_key(|&i| (op_rank(&c.nodes[i]), c.keys[i].clone()));
            let next = ready[0];
            order.push(next);
            emitted.insert(next);
            pending.retain(|&i| i != next);
        }

        // ---- width: literals / folded constants decide; wide_default forces ----
        let mut widened = wide_default || ret_wide || c.params.iter().any(|(_, w)| *w);
        let mut const_over = c.max_lit > 65535;
        for &id in &reachable {
            let over = |r: Rat| r.n.abs() > 65535 || r.d > 65535;
            match &c.nodes[id] {
                Node::Const(r) => const_over |= over(*r),
                Node::Sum { k, .. } | Node::MulDiv { k, .. } => const_over |= over(*k),
                _ => {}
            }
        }
        if const_over && !widened {
            widened = true;
            c.repairs.push(Repair::new(
                DiagCode::WidthExceedsU16,
                "a constant exceeds u16::MAX — lane widened to u32",
            ));
        }
        if widened {
            let calls: Vec<&str> = order
                .iter()
                .filter_map(|&i| match &c.nodes[i] {
                    Node::Call { name, .. } => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            if !calls.is_empty() {
                c.repairs.push(Repair::new(
                    DiagCode::WideCall,
                    format!(
                        "wide lane calls: {} (prefer _u32 overloads)",
                        calls.join(",")
                    ),
                ));
            }
        }

        // ---- slots: q* on first use in emission order, v* per emitted op ----
        let mut slot_of_param: HashMap<usize, usize> = HashMap::new(); // param pos → q index
        let assign_param = |pos: usize, slot_of_param: &mut HashMap<usize, usize>| {
            let next = slot_of_param.len();
            slot_of_param.entry(pos).or_insert(next);
        };
        for &id in &order {
            for d in node_deps(&c.nodes[id]) {
                if let Node::Param(pos) = c.nodes[d] {
                    assign_param(pos, &mut slot_of_param);
                }
            }
        }
        if let Node::Param(pos) = c.nodes[root] {
            assign_param(pos, &mut slot_of_param);
        }
        for pos in 0..c.params.len() {
            assign_param(pos, &mut slot_of_param); // unused params: after, in position order
        }

        // ---- emission ----
        let suffix = if widened { "u32" } else { "u16" };
        let mut atom_of: HashMap<usize, String> = HashMap::new();
        let const_atom = |r: Rat, _c: &FnCanon| -> Result<String, Fail> {
            if !r.is_int() {
                // Division-context fractions already died in `build_muldiv`; a
                // fractional constant here is an unscaled decimal literal.
                return Err(Fail::Hard(
                    Diag::new(
                        DiagCode::RequiresFractionalScale,
                        format!("constant {}/{} is not an integer", r.n, r.d),
                    )
                    .with_fix("scale to the base unit (e.g. cents)"),
                ));
            }
            if r.n < 0 {
                return Err(Fail::Hard(Diag::new(
                    DiagCode::NegativeConst,
                    format!("constant folds to {}", r.n),
                )));
            }
            if r.n > u32::MAX as i128 || (!widened && r.n > 65535) {
                return Err(Fail::Hard(Diag::new(
                    DiagCode::WidthExceedsU16,
                    format!("constant {} exceeds the lane width", r.n),
                )));
            }
            Ok(format!("{}{}", r.n, suffix))
        };
        let param_atom = |pos: usize| {
            let q = format!("q{}", slot_of_param[&pos]);
            if widened && !c.params[pos].1 {
                format!("({q} as u32)")
            } else {
                q
            }
        };
        // Pre-seed leaf atoms.
        for &id in &reachable {
            match &c.nodes[id] {
                Node::Param(pos) => {
                    atom_of.insert(id, param_atom(*pos));
                }
                Node::Const(r) => {
                    atom_of.insert(id, const_atom(*r, &c)?);
                }
                _ => {}
            }
        }
        let mut lines: Vec<String> = Vec::new();
        let mut vslots = 0usize;
        let mut fresh = |lines: &mut Vec<String>, rhs: String| -> String {
            let v = format!("v{vslots}");
            vslots += 1;
            lines.push(format!("    let {v} = {rhs};"));
            v
        };
        let chain = |lines: &mut Vec<String>,
                     atoms: &[String],
                     op: &str,
                     fresh: &mut dyn FnMut(&mut Vec<String>, String) -> String|
         -> String {
            let mut acc = atoms[0].clone();
            for a in &atoms[1..] {
                acc = fresh(lines, format!("{acc} {op} {a}"));
            }
            acc
        };
        let mut defer_div_applied = false;
        for &id in &order {
            let node = c.nodes[id].clone();
            let atom = match node {
                Node::Sum { pos, neg, k } => {
                    let mut adds: Vec<String> = pos.iter().map(|d| atom_of[d].clone()).collect();
                    let mut subs: Vec<String> = neg.iter().map(|d| atom_of[d].clone()).collect();
                    if !k.is_int() {
                        return Err(Fail::Hard(
                            Diag::new(
                                DiagCode::RequiresFractionalScale,
                                format!("additive constant {}/{} is fractional", k.n, k.d),
                            )
                            .with_fix("scale to the base unit (e.g. cents)"),
                        ));
                    }
                    if k.n > 0 {
                        adds.push(const_atom(k, &c)?);
                    } else if k.n < 0 {
                        subs.push(const_atom(Rat::int(-k.n), &c)?);
                    }
                    if adds.is_empty() {
                        adds.insert(0, format!("0{suffix}"));
                    }
                    let mut acc = chain(&mut lines, &adds, "+", &mut fresh);
                    for s in &subs {
                        acc = fresh(&mut lines, format!("{acc} - {s}"));
                    }
                    // A pure chain of length 1 with no subs emitted nothing: bind it.
                    if adds.len() == 1 && subs.is_empty() {
                        acc = fresh(&mut lines, acc);
                    }
                    acc
                }
                Node::MulDiv { num, den, k } => {
                    let mut nums: Vec<String> = num.iter().map(|d| atom_of[d].clone()).collect();
                    let mut dens: Vec<String> = den.iter().map(|d| atom_of[d].clone()).collect();
                    if k.n != 1 || nums.is_empty() {
                        nums.push(const_atom(Rat::int(k.n), &c)?);
                    }
                    if k.d != 1 {
                        dens.push(const_atom(Rat::int(k.d), &c)?);
                    }
                    if !dens.is_empty() {
                        defer_div_applied = true;
                    }
                    let na = chain(&mut lines, &nums, "*", &mut fresh);
                    if dens.is_empty() {
                        if nums.len() == 1 {
                            fresh(&mut lines, na)
                        } else {
                            na
                        }
                    } else {
                        let da = chain(&mut lines, &dens, "*", &mut fresh);
                        fresh(&mut lines, format!("{na} / {da}"))
                    }
                }
                Node::Rem(a, b) => {
                    let rhs = format!("{} % {}", atom_of[&a], atom_of[&b]);
                    fresh(&mut lines, rhs)
                }
                Node::Call { name, args } => {
                    // Call arguments keep their *natural* width, not the lane width:
                    // a u16 parameter stays `q0` (no `as u32`) and a small constant
                    // stays `u16`-suffixed, so a u16 library callee still links in a
                    // widened lane. A wide v-slot argument stays wide — that callee
                    // must be a `_u32` overload (the `E0503` story), never a silent
                    // truncation.
                    let mut arg_atoms = Vec::with_capacity(args.len());
                    for d in &args {
                        let atom = match &c.nodes[*d] {
                            Node::Param(pos) => format!("q{}", slot_of_param[pos]),
                            Node::Const(r) => {
                                if !widened || r.n <= 65535 {
                                    format!("{}u16", r.n)
                                } else {
                                    atom_of[d].clone()
                                }
                            }
                            _ => atom_of[d].clone(),
                        };
                        arg_atoms.push(atom);
                    }
                    let rhs = format!("{name}({})", arg_atoms.join(", "));
                    fresh(&mut lines, rhs)
                }
                Node::Param(_) | Node::Const(_) => unreachable!("leaves are pre-seeded"),
            };
            atom_of.insert(id, atom);
        }
        if defer_div_applied {
            c.repairs.push(Repair::new(
                DiagCode::RedundantParens,
                "defer_division: mul/div chain divides once at the end",
            ));
        }

        // ---- renames + dead lets ----
        let mut renames: Vec<Rename> = Vec::new();
        for (pos, (name, _)) in c.params.iter().enumerate() {
            let (unit, factor) = match c.hints.get(name) {
                Some(u) => {
                    let (cu, nf, _df) = canonical_unit(u);
                    (Some(cu), nf)
                }
                None => (None, 1),
            };
            renames.push(Rename {
                source_name: name.clone(),
                slot: format!("q{}", slot_of_param[&pos]),
                unit,
                factor,
            });
        }
        let mut dead: Vec<&str> = Vec::new();
        for (name, id) in &c.let_names {
            if reachable.contains(id) {
                if let Some(a) = atom_of.get(id) {
                    if a.starts_with('v') {
                        renames.push(Rename {
                            source_name: name.clone(),
                            slot: a.clone(),
                            unit: c.hints.get(name).map(|u| canonical_unit(u).0),
                            factor: c.hints.get(name).map_or(1, |u| canonical_unit(u).1),
                        });
                    }
                }
            } else {
                dead.push(name);
            }
        }
        if !dead.is_empty() {
            dead.sort();
            dead.dedup();
            c.repairs.push(Repair::new(
                DiagCode::DeadLet,
                format!("dropped unused bindings: {}", dead.join(",")),
            ));
        }

        // ---- assemble text ----
        let ret = if widened { "u32" } else { "u16" };
        let mut sig_params: Vec<(usize, usize)> = (0..c.params.len())
            .map(|pos| (slot_of_param[&pos], pos))
            .collect();
        sig_params.sort();
        let params_txt = sig_params
            .iter()
            .map(|(slot, pos)| format!("q{slot}: {}", if c.params[*pos].1 { "u32" } else { "u16" }))
            .collect::<Vec<_>>()
            .join(", ");
        let tail = atom_of[&root].clone();
        let mut text = String::new();
        for a in &f.attrs {
            if let Some(doc) = doc_line(a) {
                text.push_str(&format!("///{doc}\n"));
            }
        }
        text.push_str(&format!("fn {}({params_txt}) -> {ret} {{\n", f.sig.ident));
        for l in &lines {
            text.push_str(l);
            text.push('\n');
        }
        text.push_str(&format!("    {tail}\n}}\n"));
        Ok(FnOut {
            text,
            renames,
            repairs: c.repairs,
            widened,
        })
    }
}

fn node_deps(n: &Node) -> Vec<usize> {
    match n {
        Node::Param(_) | Node::Const(_) => Vec::new(),
        Node::Sum { pos, neg, .. } => pos.iter().chain(neg).copied().collect(),
        Node::MulDiv { num, den, .. } => num.iter().chain(den).copied().collect(),
        Node::Rem(a, b) => vec![*a, *b],
        Node::Call { args, .. } => args.clone(),
    }
}

fn op_rank(n: &Node) -> u8 {
    match n {
        Node::Sum { .. } => 0,
        Node::MulDiv { .. } => 1,
        Node::Rem(..) => 2,
        Node::Call { .. } => 3,
        Node::Param(_) | Node::Const(_) => 4,
    }
}

/// `Some(false)` = u16, `Some(true)` = u32, `None` = outside the canonical widths.
fn type_width(t: &syn::Type) -> Option<bool> {
    if let syn::Type::Path(p) = t {
        if let Some(id) = p.path.get_ident() {
            if id == "u16" {
                return Some(false);
            }
            if id == "u32" {
                return Some(true);
            }
        }
    }
    None
}

fn expr_kind(e: &syn::Expr) -> &'static str {
    match e {
        syn::Expr::If(_) => "if",
        syn::Expr::Match(_) => "match",
        syn::Expr::While(_) | syn::Expr::ForLoop(_) | syn::Expr::Loop(_) => "loop",
        syn::Expr::Cast(_) => "cast",
        syn::Expr::MethodCall(_) => "method call",
        syn::Expr::Field(_) => "field access",
        syn::Expr::Index(_) => "index",
        syn::Expr::Assign(_) => "assignment",
        syn::Expr::Macro(_) => "macro",
        syn::Expr::Reference(_) => "reference",
        _ => "other",
    }
}

/// The doc text of a `///` / `//!` attribute, if this is one.
fn doc_line(a: &syn::Attribute) -> Option<String> {
    if !a.path().is_ident("doc") {
        return None;
    }
    if let syn::Meta::NameValue(nv) = &a.meta {
        if let syn::Expr::Lit(l) = &nv.value {
            if let syn::Lit::Str(s) = &l.lit {
                return Some(s.value());
            }
        }
    }
    None
}

fn print_item(item: &syn::Item) -> String {
    item.to_token_stream().to_string()
}

/// Canonicalize `src` under `opts`. See the module docs for what each mode does.
/// The output's `source` is what the caller should hash and compile; `changed` is
/// false (and `source == src`) when nothing fired.
pub fn canonicalize_source(src: &str, opts: &CanonOptions) -> Result<CanonOutput, Diag> {
    if matches!(opts.mode, CanonMode::Off) {
        return Ok(CanonOutput {
            source: src.to_string(),
            changed: false,
            renames: Vec::new(),
            repairs: Vec::new(),
            widened: false,
        });
    }
    let mut file: syn::File =
        syn::parse_str(src).map_err(|e| Diag::new(DiagCode::Parse, format!("{e}")))?;
    let mut repairs = Vec::new();
    let mut light_fired = false;
    for item in &mut file.items {
        match item {
            syn::Item::Fn(f) => {
                light_fired |= normalize_block(&mut f.block, returns_value(&f.sig), &mut repairs);
            }
            syn::Item::Impl(imp) => {
                for it in &mut imp.items {
                    if let syn::ImplItem::Fn(m) = it {
                        light_fired |=
                            normalize_block(&mut m.block, returns_value(&m.sig), &mut repairs);
                    }
                }
            }
            _ => {}
        }
    }
    let mut renames = Vec::new();
    let mut widened = false;
    let mut full_texts: HashMap<usize, String> = HashMap::new();
    if matches!(opts.mode, CanonMode::Full) {
        let hints: HashMap<String, String> = opts
            .hints
            .iter()
            .map(|h| (h.ident.clone(), h.unit.clone()))
            .collect();
        for (i, item) in file.items.iter().enumerate() {
            if let syn::Item::Fn(f) = item {
                match FnCanon::run(f, &hints, opts.wide_default) {
                    Ok(out) => {
                        full_texts.insert(i, out.text);
                        repairs.extend(out.repairs);
                        renames.extend(out.renames);
                        widened |= out.widened;
                    }
                    Err(Fail::Soft(reason)) => {
                        repairs.push(Repair::new(
                            DiagCode::NonStraightLine,
                            format!("fn {}: {reason} (light normalization only)", f.sig.ident),
                        ));
                    }
                    Err(Fail::Hard(d)) => return Err(d),
                }
            }
        }
    }
    if full_texts.is_empty() && !light_fired {
        return Ok(CanonOutput {
            source: src.to_string(),
            changed: false,
            renames,
            repairs,
            widened,
        });
    }
    let mut out = String::new();
    for a in &file.attrs {
        match doc_line(a) {
            Some(doc) => out.push_str(&format!("//!{doc}\n")),
            None => out.push_str(&format!("{}\n", a.to_token_stream())),
        }
    }
    for (i, item) in file.items.iter().enumerate() {
        match full_texts.get(&i) {
            Some(t) => out.push_str(t),
            None => {
                out.push_str(&print_item(item));
                out.push('\n');
            }
        }
    }
    let changed = out != src;
    Ok(CanonOutput {
        source: out,
        changed,
        renames,
        repairs,
        widened,
    })
}
