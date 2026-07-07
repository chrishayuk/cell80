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
use std::collections::HashMap;
use syn::visit_mut::VisitMut;

mod full;
mod light;
mod rat;
mod units;

use full::{doc_line, print_item, Fail, FnCanon};
use light::{normalize_block, returns_value};
pub use units::canonical_unit;

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
