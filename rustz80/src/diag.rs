//! Typed, machine-readable diagnostics (M2.6). Every deterministic pass names what it
//! found with a **stable code** instead of prose alone: a [`Repair`] when the pass fixed
//! it, a [`Diag`] when it couldn't. The dialect normalizer keys its rewrites off these
//! codes (a table, not string-matching), and campaign repair rows get their taxonomy for
//! free — *which* error classes recur becomes a query instead of a grep.
//!
//! Code bands: `E01xx` plan wire format · `E02xx` dialect shape (normalizer-fixable) ·
//! `E03xx` constants and width · `E04xx` units · `E05xx` structure/parse.

use std::fmt;

/// Stable diagnostic codes. The numeric code and slug are a contract — downstream
/// harnesses classify repair rows by them, so codes are append-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagCode {
    /// E0101 — a bare numeric literal where an operand id was required.
    BareLiteralOperand,
    /// E0102 — a let-bound literal quantity lifted to a parameter (literal lifting;
    /// registered amendment path 2026-07-07): the schema generalizes over the value,
    /// and the counterfactual battery can perturb it.
    QuantityLifted,
    /// E0201 — a statement macro in the body (stripped; the dialect has no macros).
    StatementMacro,
    /// E0202 — redundant parentheses (collapsed).
    RedundantParens,
    /// E0203 — the body ends in a `let` instead of a tail expression (rewritten).
    TrailingLet,
    /// E0204 — a compound expression as a call argument (bound to a fresh slot).
    CompoundCallArg,
    /// E0205 — a numeric method call rewritten to its kernel (`a.max(b)` → `imax(a, b)`).
    /// Registered amendment 2026-07-06 (`docs/math-campaign-amendment.md`).
    MethodToKernel,
    /// E0207 — `if E == lit { lit } else { 0 }` (verify-not-compute) rewritten to
    /// return the computed side `E`. Registered amendment 2026-07-07.
    VerifyRewrite,
    /// E0208 — an integer literal's width suffix was stripped (suffixes are
    /// advisory in Full mode — the lane rules own width; an impossible suffix like
    /// `88000u16` is named). Registered amendment 2026-07-07.
    SuffixNormalized,
    /// E0209 — a model-written narrowing cast (`as u16`) inside checked-lane
    /// arithmetic was dropped: the compiler owns width. Registered amendment
    /// 2026-07-07 (output-changing; replay precision-checked).
    NarrowingDropped,
    /// E0210 — `if <cond> then <a> else <b>` (a non-Rust conditional some models
    /// emit) desugared to `if <cond> { <a> } else { <b> }` before parsing, so the
    /// verify shape reaches `E0207` instead of dying at `E0501`. A `!`/`panic!()`
    /// else-arm coerces to `0`. Registered amendment 2026-07-07.
    ThenDesugared,
    /// E0301 — a constant exceeds `u16::MAX`; the arithmetic lane auto-widens to u32.
    WidthExceedsU16,
    /// E0302 — a constant division that cannot be exact (division by constant zero,
    /// or a declared-exact division with a nonzero remainder).
    InexactConstDivision,
    /// E0303 — a constant folds negative in an unsigned dialect.
    NegativeConst,
    /// E0304 — a fractional constant survives folding (needs a unit scale).
    RequiresFractionalScale,
    /// E0401 — a literal was scaled to the canonical base unit (e.g. dollars → cents).
    UnitScaled,
    /// E0402 — a unit name was normalized to its canonical spelling.
    UnitNormalized,
    /// E0501 — source does not parse.
    Parse,
    /// E0502 — a construct outside the straight-line subset; the fn was left as-is
    /// (light normalization only).
    NonStraightLine,
    /// E0503 — the widened lane calls named functions; the linker should prefer
    /// `_u32` overloads when resolving them.
    WideCall,
    /// E0504 — a call target unknown to the compiler (the linker's cue to resolve
    /// and inline a library cell).
    UnknownCallTarget,
    /// E0505 — a `let` binding unreachable from the result (dropped).
    DeadLet,
    /// E0206 — a `<chain> % m` tail rewrote its whole additive/multiplicative chain
    /// into `mod_add_u32`/`mod_mul_u32` steps threaded through `m`, so intermediates
    /// never grow past it (the mod-space rewrite).
    ModSpaceRewrite,
}

impl DiagCode {
    pub fn code(self) -> &'static str {
        match self {
            DiagCode::BareLiteralOperand => "E0101",
            DiagCode::QuantityLifted => "E0102",
            DiagCode::StatementMacro => "E0201",
            DiagCode::RedundantParens => "E0202",
            DiagCode::TrailingLet => "E0203",
            DiagCode::CompoundCallArg => "E0204",
            DiagCode::MethodToKernel => "E0205",
            DiagCode::VerifyRewrite => "E0207",
            DiagCode::SuffixNormalized => "E0208",
            DiagCode::NarrowingDropped => "E0209",
            DiagCode::ThenDesugared => "E0210",
            DiagCode::WidthExceedsU16 => "E0301",
            DiagCode::InexactConstDivision => "E0302",
            DiagCode::NegativeConst => "E0303",
            DiagCode::RequiresFractionalScale => "E0304",
            DiagCode::UnitScaled => "E0401",
            DiagCode::UnitNormalized => "E0402",
            DiagCode::Parse => "E0501",
            DiagCode::NonStraightLine => "E0502",
            DiagCode::WideCall => "E0503",
            DiagCode::UnknownCallTarget => "E0504",
            DiagCode::DeadLet => "E0505",
            DiagCode::ModSpaceRewrite => "E0206",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            DiagCode::BareLiteralOperand => "bare_literal_operand",
            DiagCode::QuantityLifted => "quantity_lifted",
            DiagCode::StatementMacro => "statement_macro",
            DiagCode::RedundantParens => "redundant_parens",
            DiagCode::TrailingLet => "trailing_let",
            DiagCode::CompoundCallArg => "compound_call_arg",
            DiagCode::MethodToKernel => "method_to_kernel",
            DiagCode::VerifyRewrite => "verify_rewrite",
            DiagCode::SuffixNormalized => "suffix_normalized",
            DiagCode::NarrowingDropped => "narrowing_dropped",
            DiagCode::ThenDesugared => "then_desugared",
            DiagCode::WidthExceedsU16 => "width_exceeds_u16",
            DiagCode::InexactConstDivision => "inexact_const_division",
            DiagCode::NegativeConst => "negative_const",
            DiagCode::RequiresFractionalScale => "requires_fractional_scale",
            DiagCode::UnitScaled => "unit_scaled",
            DiagCode::UnitNormalized => "unit_normalized",
            DiagCode::Parse => "parse",
            DiagCode::NonStraightLine => "non_straight_line",
            DiagCode::WideCall => "wide_call",
            DiagCode::UnknownCallTarget => "unknown_call_target",
            DiagCode::DeadLet => "dead_let",
            DiagCode::ModSpaceRewrite => "mod_space_rewrite",
        }
    }
}

/// A typed rejection: stable code, human message, and — when a deterministic fix
/// exists but wasn't applied (wrong mode, or needs the linker/library) — the fix,
/// named so a repair rung can be keyed off it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub code: DiagCode,
    pub message: String,
    pub suggested_fix: Option<String>,
}

impl Diag {
    pub fn new(code: DiagCode, message: impl Into<String>) -> Self {
        Diag {
            code,
            message: message.into(),
            suggested_fix: None,
        }
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.suggested_fix = Some(fix.into());
        self
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{} {}] {}",
            self.code.code(),
            self.code.slug(),
            self.message
        )?;
        if let Some(fix) = &self.suggested_fix {
            write!(f, " (fix: {fix})")?;
        }
        Ok(())
    }
}

/// One applied deterministic fix: the code names the class, `detail` carries the
/// specifics (`from=dollars factor=100`). Campaign repair rows serialize these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repair {
    pub code: DiagCode,
    pub detail: String,
}

impl Repair {
    pub fn new(code: DiagCode, detail: impl Into<String>) -> Self {
        Repair {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for Repair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{} {}",
            self.code.code(),
            self.code.slug(),
            self.detail
        )
    }
}

/// Classify a legacy string error from the lowering/codegen path into a stable code.
/// A bridge, not the destination: new passes emit [`Diag`] directly; this maps the
/// most common existing rejection strings so downstream taxonomy queries see one code
/// space. Unrecognized strings return `None` — callers keep the prose.
pub fn classify_error(msg: &str) -> Option<DiagCode> {
    if msg.contains("parse error") {
        return Some(DiagCode::Parse);
    }
    if msg.contains("unknown call target") {
        return Some(DiagCode::UnknownCallTarget);
    }
    if msg.contains("a float literal") || msg.contains("unsuffixed decimal") {
        return Some(DiagCode::RequiresFractionalScale);
    }
    if msg.contains("no macros in the dialect") {
        return Some(DiagCode::StatementMacro);
    }
    if msg.contains("declare `-> u32`") || msg.contains("narrow with `as u16`") {
        return Some(DiagCode::WidthExceedsU16);
    }
    None
}
