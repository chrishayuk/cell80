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
    /// **Literal lifting** (`Full` mode, entry fns only): a let-bound bare literal —
    /// a quantity the model *named* — becomes a `q*` parameter instead of folding
    /// into the constants; its value is returned in [`CanonOutput::lifted`]. The
    /// schema then generalizes over the numbers (same structure, different values ⇒
    /// same artifact — the H-M3 shape), and the counterfactual battery can perturb
    /// composed cells. Inline expression constants (`* 30 / 100`) stay baked —
    /// they're structure, not quantities; values over `u16::MAX` stay baked too
    /// (parameter ABI). Unit scaling applies before lifting (`16.50` dollars lifts
    /// as `1650`).
    pub lift_literals: bool,
    /// **Checked emission** (the campaign/compose path, paired with lifting): the
    /// arithmetic lane is forced wide and every add/sub/mul emits through the
    /// checked prelude kernels (`add_checked_u32`/`sub_checked_u32`/
    /// `mul_checked_u32`) — overflow and negative intermediates **escalate**
    /// instead of wrapping. Without this, lifting opens a silent-wrap hole: a
    /// lifted quantity is no longer a constant, so the fold can't see that
    /// `q0 * 1000` exceeds `u16::MAX` at the source's own values (found by the
    /// cross-language parity check: 88*1000/11 wrapped to 2042 instead of 8000 —
    /// and identical schemas wrap identically, so a gate could even *agree* on
    /// the wrapped value). Matches the plan renderer's checked-line semantics.
    pub checked: bool,
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
    /// Lifted quantities, `(slot, original value)` in slot order — the arguments a
    /// caller passes to run the canonical cell at the source's original numbers.
    pub lifted: Vec<(String, u64)>,
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
    /// `a as u16` — meaningful only in the wide lane (narrow values already wrap at
    /// 16 bits, so it aliases away); `as u32` never builds a node (zero-extension is
    /// the identity — it just forces the wide lane).
    Trunc(usize),
    /// A comparison — only ever a `Select` condition. Normalized: `>`/`>=` flip to
    /// `<`/`<=` with swapped operands; `==`/`!=` sort operands by structural key.
    Cmp {
        op: CmpKind,
        a: usize,
        b: usize,
    },
    /// `if c { t } else { f }` as a value. Emission is **lazy where it must be**:
    /// nodes used only inside one arm render inline in that arm (so a guarded
    /// division — `if b != 0 { a / b } else { 0 } — never evaluates eagerly and
    /// keeps its kill-avoidance semantics); nodes the condition needs, or that both
    /// arms share, hoist as ordinary ops (the taken branch would compute them
    /// anyway, so hoisting cannot introduce a kill the original lacked).
    Select {
        c: usize,
        t: usize,
        f: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmpKind {
    Lt,
    Le,
    Eq,
    Ne,
}

impl CmpKind {
    fn sym(self) -> &'static str {
        match self {
            CmpKind::Lt => "<",
            CmpKind::Le => "<=",
            CmpKind::Eq => "==",
            CmpKind::Ne => "!=",
        }
    }
    fn tag(self) -> &'static str {
        match self {
            CmpKind::Lt => "lt",
            CmpKind::Le => "le",
            CmpKind::Eq => "eq",
            CmpKind::Ne => "ne",
        }
    }
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
    /// An explicit `as u32` in the source forces the wide lane.
    force_wide: bool,
    /// Campaign lane (checked kernels): model narrowing casts drop (`E0209`).
    checked: bool,
}

struct FnOut {
    text: String,
    renames: Vec<Rename>,
    repairs: Vec<Repair>,
    widened: bool,
    /// `(slot, original value)` for lifted quantities, in slot order.
    lifted: Vec<(String, u64)>,
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
            syn::Expr::Cast(cast) => {
                let wide = match type_width(&cast.ty) {
                    Some(w) => w,
                    None => return soft("cast to a non-u16/u32 type"),
                };
                let inner = self.build(&cast.expr)?;
                if wide {
                    // `as u32` is zero-extension — the identity, but it commits the
                    // fn to the wide lane (matching the source author's intent).
                    self.force_wide = true;
                    return Ok(inner);
                }
                if self.checked {
                    // Registered amendment `E0209`: in the checked lane the
                    // compiler owns width — a model's mid-chain `as u16` is
                    // bookkeeping noise, and truncating a protected wide value
                    // is never what the arithmetic means.
                    self.repairs.push(Repair::new(
                        DiagCode::NarrowingDropped,
                        "`as u16` dropped in the checked lane",
                    ));
                    return Ok(inner);
                }
                if let Some(r) = self.as_const(inner) {
                    if r.is_int() && r.n >= 0 {
                        return Ok(self.intern_const(Rat::int(r.n & 0xFFFF)));
                    }
                }
                let key = format!("t({})", self.keys[inner]);
                Ok(self.intern(Node::Trunc(inner), key))
            }
            syn::Expr::If(iff) => self.build_select(iff),
            // Registered amendment 2026-07-06 (`E0205 method_to_kernel`): the numeric
            // method spellings models reach for rewrite to the prelude kernels that
            // already exist — deterministic, semantics-preserving at u16, recorded.
            // Anything not in the table stays a soft fallback, never a guess.
            syn::Expr::MethodCall(mc) => {
                let kernel = match mc.method.to_string().as_str() {
                    "max" => "imax",
                    "min" => "imin",
                    "abs_diff" => "iabs_diff",
                    other => return soft(format!("method call `.{other}`")),
                };
                if mc.args.len() != 1 {
                    return soft("method-call arity");
                }
                let recv = self.build(&mc.receiver)?;
                let arg = self.build(&mc.args[0])?;
                self.repairs.push(Repair::new(
                    DiagCode::MethodToKernel,
                    format!(".{}() -> {kernel}()", mc.method),
                ));
                let args = vec![recv, arg];
                let key = format!("f{kernel}({})", self.key_list(&args));
                Ok(self.intern(
                    Node::Call {
                        name: kernel.to_string(),
                        args,
                    },
                    key,
                ))
            }
            other => soft(format!(
                "expression outside the straight-line subset ({})",
                expr_kind(other)
            )),
        }
    }

    /// `if <comparison> { <value> } else { <value> }` as a canonical Select node.
    /// Conditions are single comparisons (the dialect's boolean surface); arms are
    /// single value expressions (an `else if` chain nests as a Select in `f`).
    fn build_select(&mut self, iff: &syn::ExprIf) -> Result<usize, Fail> {
        let c = self.build_cmp(&iff.cond)?;
        let t = match block_value(&iff.then_branch) {
            Some(e) => self.build(e)?,
            None => return soft("if-arm is not a single value expression"),
        };
        let Some((_, else_expr)) = &iff.else_branch else {
            return soft("if-value without an else arm");
        };
        let f = match &**else_expr {
            syn::Expr::Block(b) => match block_value(&b.block) {
                Some(e) => self.build(e)?,
                None => return soft("else-arm is not a single value expression"),
            },
            syn::Expr::If(nested) => self.build_select(nested)?,
            other => self.build(other)?,
        };
        // Constant condition folds the select away entirely.
        if let Node::Cmp { op, a, b } = self.nodes[c] {
            if let (Some(x), Some(y)) = (self.as_const(a), self.as_const(b)) {
                if x.is_int() && y.is_int() {
                    let taken = match op {
                        CmpKind::Lt => x.n < y.n,
                        CmpKind::Le => x.n <= y.n,
                        CmpKind::Eq => x.n == y.n,
                        CmpKind::Ne => x.n != y.n,
                    };
                    return Ok(if taken { t } else { f });
                }
            }
        }
        if t == f {
            return Ok(t); // both arms identical — the condition is decoration
        }
        // Registered amendment 2026-07-07 (`E0207 verify_rewrite`): the
        // verify-not-compute shape `if E == lit { lit } else { 0 }` returns the
        // computed side `E` — the comparison contains a real derivation; the
        // stated literal and the degenerate zero arm are both noise.
        if let Node::Cmp {
            op: CmpKind::Eq,
            a,
            b,
        } = self.nodes[c]
        {
            let zero_else = self.as_const(f).is_some_and(|r| r.is_zero());
            if zero_else {
                let (expr_side, lit_side) = if self.as_const(a).is_some() {
                    (b, a)
                } else {
                    (a, b)
                };
                if self.as_const(lit_side).is_some()
                    && self.as_const(expr_side).is_none()
                    && t == lit_side
                {
                    self.repairs.push(Repair::new(
                        DiagCode::VerifyRewrite,
                        "if E == lit { lit } else { 0 } -> E (computed side wins)",
                    ));
                    return Ok(expr_side);
                }
            }
        }
        let key = format!("s?({}|{}|{})", self.keys[c], self.keys[t], self.keys[f]);
        Ok(self.intern(Node::Select { c, t, f }, key))
    }

    fn build_cmp(&mut self, cond: &syn::Expr) -> Result<usize, Fail> {
        match cond {
            syn::Expr::Paren(p) => self.build_cmp(&p.expr),
            syn::Expr::Binary(bin) => {
                let (op, flip) = match bin.op {
                    syn::BinOp::Lt(_) => (CmpKind::Lt, false),
                    syn::BinOp::Le(_) => (CmpKind::Le, false),
                    syn::BinOp::Gt(_) => (CmpKind::Lt, true),
                    syn::BinOp::Ge(_) => (CmpKind::Le, true),
                    syn::BinOp::Eq(_) => (CmpKind::Eq, false),
                    syn::BinOp::Ne(_) => (CmpKind::Ne, false),
                    _ => return soft("condition is not a comparison"),
                };
                let (mut a, mut b) = (self.build(&bin.left)?, self.build(&bin.right)?);
                if flip {
                    std::mem::swap(&mut a, &mut b);
                }
                if matches!(op, CmpKind::Eq | CmpKind::Ne) && self.keys[a] > self.keys[b] {
                    std::mem::swap(&mut a, &mut b); // symmetric ops sort operands
                }
                let key = format!("b{}({}|{})", op.tag(), self.keys[a], self.keys[b]);
                Ok(self.intern(Node::Cmp { op, a, b }, key))
            }
            _ => soft("condition is not a comparison"),
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
        lift: bool,
        checked: bool,
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
            force_wide: false,
            checked,
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
        // Real (declared) parameters keep their POSITIONAL slots — the caller's
        // ABI is positional, so reordering the signature by dataflow would silently
        // remap arguments (found by the guarded-division test: `b` became `q0` and
        // `--args a,b` divided the wrong way). Only lifted quantities — which have
        // no external caller — get dataflow-ordered slots, appended after.
        let real_params = c.params.len();
        // Statements: `let <ident> = <arith>;` … ending in a value tail.
        let stmts = &f.block.stmts;
        let Some((tail_stmt, bindings)) = stmts.split_last() else {
            return soft("empty body");
        };
        let mut lifted_pos: Vec<(usize, u64)> = Vec::new();
        for s in bindings {
            // SSA reassignment: `x = <arith>;` where `x` is already bound rebinds
            // the name to the new value — models write accumulator style
            // (`total = total + n`) constantly, and a rebind is exactly what a
            // `let` shadow is. Semantics-preserving in a straight-line body.
            if let syn::Stmt::Expr(syn::Expr::Assign(assign), Some(_)) = s {
                if let syn::Expr::Path(path) = &*assign.left {
                    if let Some(ident) = path.path.get_ident() {
                        let name = ident.to_string();
                        if c.env.contains_key(&name) {
                            let id = c.build(&assign.right)?;
                            let id = c.scale_const(id, &name);
                            c.env.insert(name.clone(), id);
                            c.let_names.push((name, id));
                            continue;
                        }
                        return soft(format!("assignment to unbound `{name}`"));
                    }
                }
                return soft("assignment to a non-name");
            }
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
            // Literal lifting: a let-bound bare literal is a *named quantity* —
            // promote it to a parameter so the schema generalizes over the value
            // and the battery can perturb it. Post-scaling, u16-range, ints only.
            if lift && real_params == 0 {
                if let Some(r) = c.as_const(id) {
                    if r.is_int() && (0..=65535).contains(&r.n) {
                        let pos = c.params.len();
                        let pid = c.intern(Node::Param(pos), format!("p{pos}"));
                        c.params.push((name.clone(), false));
                        c.param_nodes.push(pid);
                        lifted_pos.push((pos, r.n as u64));
                        c.repairs.push(Repair::new(
                            DiagCode::QuantityLifted,
                            format!("`{name}` = {} lifted to a parameter", r.n),
                        ));
                        c.env.insert(name.clone(), pid);
                        c.let_names.push((name, pid));
                        continue;
                    }
                }
            }
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
        // Eager vs branch-lazy: nodes reachable without crossing a Select arm are
        // eager (hoisted ops); a Select's condition is always eager; nodes BOTH arms
        // share are eager too (the taken branch would compute them regardless, so
        // hoisting cannot introduce a kill the original lacked). Everything else is
        // arm-exclusive and renders inline inside its arm — the guarded-division
        // idiom keeps its lazy kill-avoidance.
        let mut eager: HashSet<usize> = HashSet::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if !eager.insert(id) {
                continue;
            }
            match &c.nodes[id] {
                Node::Select { c: cond, .. } => stack.push(*cond),
                n => stack.extend(node_deps(n)),
            }
        }
        loop {
            let mut grew = false;
            let selects: Vec<(usize, usize)> = eager
                .iter()
                .filter_map(|&i| match c.nodes[i] {
                    Node::Select { t, f, .. } => Some((t, f)),
                    _ => None,
                })
                .collect();
            for (t, f) in selects {
                let closure = |from: usize| -> HashSet<usize> {
                    let mut seen = HashSet::new();
                    let mut st = vec![from];
                    while let Some(id) = st.pop() {
                        if !seen.insert(id) {
                            continue;
                        }
                        match &c.nodes[id] {
                            Node::Select { c: cond, .. } => st.push(*cond),
                            n => st.extend(node_deps(n)),
                        }
                    }
                    seen
                };
                let (rt, rf) = (closure(t), closure(f));
                for &shared in rt.intersection(&rf) {
                    if eager.insert(shared) {
                        grew = true;
                    }
                }
            }
            if !grew {
                break;
            }
        }
        let is_op = |n: &Node| {
            matches!(
                n,
                Node::Sum { .. }
                    | Node::MulDiv { .. }
                    | Node::Rem(..)
                    | Node::Call { .. }
                    | Node::Select { .. }
                    | Node::Trunc(_)
            )
        };
        // Readiness looks through non-op eager deps (a Cmp condition) to the ops
        // beneath them, and ignores arm-exclusive nodes (they render inline).
        let eager_op_deps = |id: usize, eager: &HashSet<usize>| -> Vec<usize> {
            let mut out = Vec::new();
            let mut st = node_deps(&c.nodes[id]);
            while let Some(d) = st.pop() {
                if !eager.contains(&d) {
                    continue;
                }
                if is_op(&c.nodes[d]) {
                    out.push(d);
                } else {
                    st.extend(node_deps(&c.nodes[d]));
                }
            }
            out
        };
        let mut pending: Vec<usize> = eager
            .iter()
            .copied()
            .filter(|&i| is_op(&c.nodes[i]))
            .collect();
        pending.sort();
        let mut emitted: HashSet<usize> = HashSet::new();
        let mut order: Vec<usize> = Vec::new();
        while !pending.is_empty() {
            let mut ready: Vec<usize> = pending
                .iter()
                .copied()
                .filter(|&i| eager_op_deps(i, &eager).iter().all(|d| emitted.contains(d)))
                .collect();
            debug_assert!(!ready.is_empty(), "op DAG is acyclic by construction");
            ready.sort_by_key(|&i| (op_rank(&c.nodes[i]), c.keys[i].clone()));
            let next = ready[0];
            order.push(next);
            emitted.insert(next);
            pending.retain(|&i| i != next);
        }

        // ---- width: literals / folded constants decide; wide_default forces ----
        let mut widened =
            wide_default || checked || ret_wide || c.force_wide || c.params.iter().any(|(_, w)| *w);
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

        // ---- mod-space rewrite: a `<chain> % m` tail with a leaf modulus and a
        // straight-line (+/-/*) chain rewrites to a step-wise mod-reduced chain
        // threaded through m — reduce, combine via the existing checked kernel,
        // reduce again — so no intermediate ever grows past m, instead of summing
        // the whole wide chain and reducing once at the end. This is the AIME
        // "reduce mod 1000" finishing move, done from the start. Each step is
        // inlined (no new shared kernel): the Z80 calling convention caps a
        // function at two u32 parameters (`a`/`b` ride HL:DE and the stack; a
        // third has nowhere to go), so a 3-arg `mod_add_u32(a, b, m)` free
        // function can't exist — the two existing 2-arg kernels
        // (`add_checked_u32`, `mul_checked_u32`) plus a couple of inline `%`/`if`
        // lines do the same job. Only fires in the wide lane (u16 arithmetic
        // doesn't need it), only when the modulus is a leaf (param/const — so its
        // value is available before any chain op, with no ordering hazard), and
        // only when every op reachable from the chain root is a pure Sum or a
        // division-free MulDiv (no Call, no nested Rem, no fractional constant) —
        // anything else falls back to the existing wide-then-mod emission,
        // unchanged.
        let mod_rewrite: Option<(usize, usize, HashSet<usize>)> = widened
            .then(|| match c.nodes[root] {
                Node::Rem(a_id, m_id)
                    if matches!(c.nodes[m_id], Node::Param(_) | Node::Const(_)) =>
                {
                    let mut ops: HashSet<usize> = HashSet::new();
                    let mut seen: HashSet<usize> = HashSet::new();
                    let mut stack = vec![a_id];
                    let mut clean = true;
                    while let Some(id) = stack.pop() {
                        if !seen.insert(id) {
                            continue;
                        }
                        match &c.nodes[id] {
                            Node::Sum { pos, neg, .. } => {
                                ops.insert(id);
                                stack.extend(pos.iter().chain(neg));
                            }
                            Node::MulDiv { num, den, k } => {
                                if !den.is_empty() || !k.is_int() {
                                    clean = false;
                                    break;
                                }
                                ops.insert(id);
                                stack.extend(num.iter());
                            }
                            Node::Param(_) | Node::Const(_) => {}
                            Node::Rem(..)
                            | Node::Call { .. }
                            | Node::Trunc(_)
                            | Node::Cmp { .. }
                            | Node::Select { .. } => {
                                clean = false;
                                break;
                            }
                        }
                    }
                    (clean && !ops.is_empty()).then_some((a_id, m_id, ops))
                }
                _ => None,
            })
            .flatten();
        if mod_rewrite.is_some() {
            c.repairs.push(Repair::new(
                DiagCode::ModSpaceRewrite,
                "chain % m rewritten to mod_add_u32/mod_mul_u32 steps threaded through m",
            ));
        }

        // ---- slots: q* on first use in emission order, v* per emitted op ----
        let mut slot_of_param: HashMap<usize, usize> = HashMap::new(); // param pos → q index
        for pos in 0..real_params {
            slot_of_param.insert(pos, pos); // positional ABI: q-slot == position
        }
        let assign_param = |pos: usize, slot_of_param: &mut HashMap<usize, usize>| {
            let next = slot_of_param.len();
            slot_of_param.entry(pos).or_insert(next);
        };
        // Walk each ordered op's FULL subtree (deterministic child order) so params
        // that only appear inside a Select arm still get dataflow-ordered slots.
        let assign_subtree = |from: usize, slot_of_param: &mut HashMap<usize, usize>| {
            let mut st = vec![from];
            let mut seen: HashSet<usize> = HashSet::new();
            while let Some(id) = st.pop() {
                if !seen.insert(id) {
                    continue;
                }
                if let Node::Param(pos) = c.nodes[id] {
                    assign_param(pos, slot_of_param);
                }
                // Depth-first, children in declaration order (reverse for the stack).
                let mut ds = node_deps(&c.nodes[id]);
                ds.reverse();
                st.extend(ds);
            }
        };
        for &id in &order {
            assign_subtree(id, &mut slot_of_param);
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
        if let Some((_, m_id, _)) = &mod_rewrite {
            // Matches mod_add_u32/mod_sub_u32/mod_mul_u32's own out_of_domain halt —
            // checked once, since m is a leaf and invariant across the whole chain.
            lines.push(format!(
                "    if {} == 0u32 {{ halt(0xFF06u16); }}",
                atom_of[m_id]
            ));
        }
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
                    let mod_m = mod_rewrite.as_ref().and_then(|(_, m_id, ops)| {
                        ops.contains(&id).then(|| atom_of[m_id].clone())
                    });
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
                    if let Some(m) = mod_m {
                        // Mod-space: thread `m` through every step (reduce, combine via
                        // the existing checked kernel, reduce again) so no intermediate
                        // exceeds it, instead of summing wide and reducing once at the end.
                        let mut acc = adds[0].clone();
                        for a in &adds[1..] {
                            let ra = fresh(&mut lines, format!("{acc} % {m}"));
                            let rb = fresh(&mut lines, format!("{a} % {m}"));
                            let s = fresh(&mut lines, format!("add_checked_u32({ra}, {rb})"));
                            acc = fresh(
                                &mut lines,
                                format!("if {s} >= {m} {{ {s} - {m} }} else {{ {s} }}"),
                            );
                        }
                        for s in &subs {
                            let ra = fresh(&mut lines, format!("{acc} % {m}"));
                            let rb = fresh(&mut lines, format!("{s} % {m}"));
                            acc = fresh(
                                &mut lines,
                                format!("if {ra} >= {rb} {{ {ra} - {rb} }} else {{ {m} - ({rb} - {ra}) }}"),
                            );
                        }
                        if adds.len() == 1 && subs.is_empty() {
                            acc = fresh(&mut lines, format!("{acc} % {m}"));
                        }
                        acc
                    } else if checked {
                        // Campaign lane: adds and subs go through the checked
                        // kernels — overflow/negative escalates, never wraps.
                        let mut acc = adds[0].clone();
                        for a in &adds[1..] {
                            acc = fresh(&mut lines, format!("add_checked_u32({acc}, {a})"));
                        }
                        for s in &subs {
                            acc = fresh(&mut lines, format!("sub_checked_u32({acc}, {s})"));
                        }
                        if adds.len() == 1 && subs.is_empty() {
                            acc = fresh(&mut lines, acc);
                        }
                        acc
                    } else {
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
                }
                Node::MulDiv { num, den, k } => {
                    let mod_m = mod_rewrite.as_ref().and_then(|(_, m_id, ops)| {
                        ops.contains(&id).then(|| atom_of[m_id].clone())
                    });
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
                    if let Some(m) = mod_m {
                        // The mod-rewrite scan only admits division-free MulDiv nodes
                        // (an integral k, no `den`), so `nums` alone carries the chain.
                        // `mul_checked_u32` (existing 2-arg kernel) escalates honestly if
                        // a step's product itself overflows u32 — no hardcoded modulus cap.
                        let mut acc = nums[0].clone();
                        for a in &nums[1..] {
                            let ra = fresh(&mut lines, format!("{acc} % {m}"));
                            let rb = fresh(&mut lines, format!("{a} % {m}"));
                            let p = fresh(&mut lines, format!("mul_checked_u32({ra}, {rb})"));
                            acc = fresh(&mut lines, format!("{p} % {m}"));
                        }
                        if nums.len() == 1 {
                            acc = fresh(&mut lines, format!("{acc} % {m}"));
                        }
                        acc
                    } else if checked {
                        let kchain = |lines: &mut Vec<String>,
                                      atoms: &[String],
                                      fresh: &mut dyn FnMut(&mut Vec<String>, String) -> String|
                         -> String {
                            let mut acc = atoms[0].clone();
                            for a in &atoms[1..] {
                                acc = fresh(lines, format!("mul_checked_u32({acc}, {a})"));
                            }
                            acc
                        };
                        let na = kchain(&mut lines, &nums, &mut fresh);
                        if dens.is_empty() {
                            if nums.len() == 1 {
                                fresh(&mut lines, na)
                            } else {
                                na
                            }
                        } else {
                            let da = kchain(&mut lines, &dens, &mut fresh);
                            fresh(&mut lines, format!("{na} / {da}"))
                        }
                    } else {
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
                }
                Node::Rem(a, b) => match &mod_rewrite {
                    Some((a_id, m_id, _)) if *a_id == a && *m_id == b => atom_of[&a].clone(),
                    _ => {
                        let rhs = format!("{} % {}", atom_of[&a], atom_of[&b]);
                        fresh(&mut lines, rhs)
                    }
                },
                Node::Call { name, args } => {
                    // In the wide lane the comparison kernels have `_u32` siblings in
                    // the prelude — E0205 rewrites target those, with wide arguments
                    // (a u16 kernel can't take checked-lane values; found on the
                    // granite row22 replay).
                    let wide_kernel =
                        widened && matches!(name.as_str(), "imax" | "imin" | "iabs_diff");
                    if wide_kernel {
                        let rhs = format!(
                            "{name}_u32({})",
                            args.iter()
                                .map(|d| atom_of[d].clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        let a = fresh(&mut lines, rhs);
                        atom_of.insert(id, a);
                        continue;
                    }
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
                Node::Trunc(a) => {
                    if !widened {
                        // Narrow-lane values already wrap at 16 bits: the cast is
                        // the identity — alias, no line.
                        atom_of[&a].clone()
                    } else if let Node::Param(pos) = c.nodes[a] {
                        format!("q{}", slot_of_param[&pos]) // a parameter IS u16
                    } else {
                        fresh(&mut lines, format!("{} as u16", atom_of[&a]))
                    }
                }
                Node::Select { c: cond, t, f } => {
                    let cs = render_inline(
                        &c,
                        cond,
                        &atom_of,
                        widened,
                        checked,
                        suffix,
                        &slot_of_param,
                    )?;
                    let ts =
                        render_inline(&c, t, &atom_of, widened, checked, suffix, &slot_of_param)?;
                    let fs =
                        render_inline(&c, f, &atom_of, widened, checked, suffix, &slot_of_param)?;
                    fresh(&mut lines, format!("if {cs} {{ {ts} }} else {{ {fs} }}"))
                }
                Node::Cmp { .. } => unreachable!("conditions render inside their Select"),
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
        let mut lifted: Vec<(usize, u64)> = lifted_pos
            .iter()
            .map(|(pos, v)| (slot_of_param[pos], *v))
            .collect();
        lifted.sort();
        Ok(FnOut {
            text,
            renames,
            repairs: c.repairs,
            widened,
            lifted: lifted
                .into_iter()
                .map(|(slot, v)| (format!("q{slot}"), v))
                .collect(),
        })
    }
}

/// Render a node as an inline expression string — used for a `Select`'s condition
/// and for arm-exclusive subtrees, which must stay lazy inside their `if` arm.
/// Eager nodes resolve to their already-emitted atoms; everything else renders
/// recursively (operands parenthesized whenever compound).
fn render_inline(
    c: &FnCanon,
    id: usize,
    atom_of: &HashMap<usize, String>,
    widened: bool,
    checked: bool,
    suffix: &str,
    slot_of_param: &HashMap<usize, usize>,
) -> Result<String, Fail> {
    if let Some(a) = atom_of.get(&id) {
        return Ok(a.clone());
    }
    let wrap = |s: String| -> String {
        if s.contains(' ') {
            format!("({s})")
        } else {
            s
        }
    };
    let rat_atom = |r: Rat| -> Result<String, Fail> {
        if !r.is_int() || r.n < 0 {
            return Err(Fail::Hard(Diag::new(
                DiagCode::RequiresFractionalScale,
                format!("constant {}/{} in a branch is not a whole number", r.n, r.d),
            )));
        }
        Ok(format!("{}{}", r.n, suffix))
    };
    Ok(match &c.nodes[id] {
        Node::Param(pos) => {
            let q = format!("q{}", slot_of_param[pos]);
            if widened && !c.params[*pos].1 {
                format!("({q} as u32)")
            } else {
                q
            }
        }
        Node::Const(r) => rat_atom(*r)?,
        Node::Sum { pos, neg, k } => {
            let mut adds: Vec<String> = Vec::new();
            for d in pos {
                adds.push(wrap(render_inline(
                    c,
                    *d,
                    atom_of,
                    widened,
                    checked,
                    suffix,
                    slot_of_param,
                )?));
            }
            if k.n > 0 {
                adds.push(rat_atom(*k)?);
            }
            if adds.is_empty() {
                adds.push(format!("0{suffix}"));
            }
            if checked {
                let mut out = adds[0].clone();
                for a in &adds[1..] {
                    out = format!("add_checked_u32({out}, {a})");
                }
                for d in neg {
                    out = format!(
                        "sub_checked_u32({out}, {})",
                        render_inline(c, *d, atom_of, widened, checked, suffix, slot_of_param)?
                    );
                }
                if k.n < 0 {
                    out = format!("sub_checked_u32({out}, {})", rat_atom(Rat::int(-k.n))?);
                }
                out
            } else {
                let mut out = adds.join(" + ");
                for d in neg {
                    out = format!(
                        "{out} - {}",
                        wrap(render_inline(
                            c,
                            *d,
                            atom_of,
                            widened,
                            checked,
                            suffix,
                            slot_of_param
                        )?)
                    );
                }
                if k.n < 0 {
                    out = format!("{out} - {}", rat_atom(Rat::int(-k.n))?);
                }
                out
            }
        }
        Node::MulDiv { num, den, k } => {
            let mut nums: Vec<String> = Vec::new();
            for d in num {
                nums.push(wrap(render_inline(
                    c,
                    *d,
                    atom_of,
                    widened,
                    checked,
                    suffix,
                    slot_of_param,
                )?));
            }
            if k.n != 1 || nums.is_empty() {
                nums.push(rat_atom(Rat::int(k.n))?);
            }
            let mut dens: Vec<String> = Vec::new();
            for d in den {
                dens.push(wrap(render_inline(
                    c,
                    *d,
                    atom_of,
                    widened,
                    checked,
                    suffix,
                    slot_of_param,
                )?));
            }
            if k.d != 1 {
                dens.push(rat_atom(Rat::int(k.d))?);
            }
            let join_mul = |xs: &[String]| -> String {
                if checked {
                    let mut acc = xs[0].clone();
                    for x in &xs[1..] {
                        acc = format!("mul_checked_u32({acc}, {x})");
                    }
                    acc
                } else {
                    xs.join(" * ")
                }
            };
            let na = join_mul(&nums);
            if dens.is_empty() {
                na
            } else if dens.len() == 1 {
                format!("{na} / {}", dens[0])
            } else if checked {
                format!("{na} / {}", join_mul(&dens))
            } else {
                format!("{na} / ({})", dens.join(" * "))
            }
        }
        Node::Rem(a, b) => format!(
            "{} % {}",
            wrap(render_inline(
                c,
                *a,
                atom_of,
                widened,
                checked,
                suffix,
                slot_of_param
            )?),
            wrap(render_inline(
                c,
                *b,
                atom_of,
                widened,
                checked,
                suffix,
                slot_of_param
            )?)
        ),
        Node::Call { name, args } => {
            let mut rendered = Vec::with_capacity(args.len());
            for d in args {
                rendered.push(render_inline(
                    c,
                    *d,
                    atom_of,
                    widened,
                    checked,
                    suffix,
                    slot_of_param,
                )?);
            }
            format!("{name}({})", rendered.join(", "))
        }
        Node::Trunc(a) => {
            if !widened {
                render_inline(c, *a, atom_of, widened, checked, suffix, slot_of_param)?
            } else {
                format!(
                    "{} as u16",
                    wrap(render_inline(
                        c,
                        *a,
                        atom_of,
                        widened,
                        checked,
                        suffix,
                        slot_of_param
                    )?)
                )
            }
        }
        Node::Cmp { op, a, b } => format!(
            "{} {} {}",
            wrap(render_inline(
                c,
                *a,
                atom_of,
                widened,
                checked,
                suffix,
                slot_of_param
            )?),
            op.sym(),
            wrap(render_inline(
                c,
                *b,
                atom_of,
                widened,
                checked,
                suffix,
                slot_of_param
            )?)
        ),
        Node::Select { c: cond, t, f } => format!(
            "if {} {{ {} }} else {{ {} }}",
            render_inline(c, *cond, atom_of, widened, checked, suffix, slot_of_param)?,
            render_inline(c, *t, atom_of, widened, checked, suffix, slot_of_param)?,
            render_inline(c, *f, atom_of, widened, checked, suffix, slot_of_param)?
        ),
    })
}

fn node_deps(n: &Node) -> Vec<usize> {
    match n {
        Node::Param(_) | Node::Const(_) => Vec::new(),
        Node::Sum { pos, neg, .. } => pos.iter().chain(neg).copied().collect(),
        Node::MulDiv { num, den, .. } => num.iter().chain(den).copied().collect(),
        Node::Rem(a, b) => vec![*a, *b],
        Node::Call { args, .. } => args.clone(),
        Node::Trunc(a) => vec![*a],
        Node::Cmp { a, b, .. } => vec![*a, *b],
        Node::Select { c, t, f } => vec![*c, *t, *f],
    }
}

/// The single value expression of a block, if that is all the block holds.
fn block_value(b: &syn::Block) -> Option<&syn::Expr> {
    match b.stmts.as_slice() {
        [syn::Stmt::Expr(e, None)] => Some(e),
        _ => None,
    }
}

fn op_rank(n: &Node) -> u8 {
    match n {
        Node::Sum { .. } => 0,
        Node::MulDiv { .. } => 1,
        Node::Rem(..) => 2,
        Node::Call { .. } => 3,
        Node::Param(_) | Node::Const(_) => 4,
        Node::Select { .. } => 5,
        Node::Trunc(_) => 6,
        Node::Cmp { .. } => 7,
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
            lifted: Vec::new(),
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
    let mut lifted: Vec<(String, u64)> = Vec::new();
    let mut full_texts: HashMap<usize, String> = HashMap::new();
    if matches!(opts.mode, CanonMode::Full) {
        // Width belongs to the compiler (registered amendment `E0208`): integer
        // suffixes are advisory in Full mode. Strip them all — the lane rules
        // re-emit canonical ones — and NAME the impossible ones (`88000u16`),
        // which would otherwise die even in light-fallback fns.
        struct SuffixStrip {
            stripped: usize,
            impossible: Vec<String>,
        }
        impl VisitMut for SuffixStrip {
            fn visit_lit_int_mut(&mut self, lit: &mut syn::LitInt) {
                let suffix = lit.suffix().to_string();
                if suffix.is_empty() {
                    return;
                }
                if let Ok(v) = lit.base10_parse::<u128>() {
                    let max: u128 = match suffix.as_str() {
                        "u8" => u8::MAX as u128,
                        "u16" => u16::MAX as u128,
                        "u32" => u32::MAX as u128,
                        "i16" => i16::MAX as u128,
                        _ => u128::MAX,
                    };
                    if v > max {
                        self.impossible.push(format!("{v}{suffix}"));
                    }
                    self.stripped += 1;
                    *lit = syn::LitInt::new(lit.base10_digits(), lit.span());
                }
            }
        }
        let mut strip = SuffixStrip {
            stripped: 0,
            impossible: Vec::new(),
        };
        strip.visit_file_mut(&mut file);
        if !strip.impossible.is_empty() {
            repairs.push(Repair::new(
                DiagCode::SuffixNormalized,
                format!(
                    "impossible suffixes stripped: {}",
                    strip.impossible.join(", ")
                ),
            ));
            light_fired = true; // the text changed even if no fn full-canonicalizes
        } else if strip.stripped > 0 {
            repairs.push(Repair::new(
                DiagCode::SuffixNormalized,
                format!("{} advisory width suffixes stripped", strip.stripped),
            ));
            light_fired = true;
        }
        let hints: HashMap<String, String> = opts
            .hints
            .iter()
            .map(|h| (h.ident.clone(), h.unit.clone()))
            .collect();
        for (i, item) in file.items.iter().enumerate() {
            if let syn::Item::Fn(f) = item {
                let lift = opts.lift_literals && (f.sig.ident == "run" || f.sig.ident == "main");
                match FnCanon::run(f, &hints, opts.wide_default, lift, opts.checked) {
                    Ok(out) => {
                        full_texts.insert(i, out.text);
                        repairs.extend(out.repairs);
                        renames.extend(out.renames);
                        widened |= out.widened;
                        lifted.extend(out.lifted);
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
            lifted,
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
        lifted,
    })
}
