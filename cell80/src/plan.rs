//! The **plan IR + renderer + solve loop** — M2 of the math campaign
//! (`docs/math-campaign-spec.md`). The plan is a *wire format between model and
//! renderer*, never executable: the model extracts quantities (typed, unit-tagged),
//! ops, a target, and constraints; the renderer emits trivial canonical dialect
//! Rust (quantities = state fields); rustz80 compiles it — **the plan IS a cell** —
//! and the runner executes, verifies, and perturbs it, memoized.
//!
//! The renderer is deliberately dumb: one op → one checked line; units are checked
//! *symbolically at render time* (dollars + hours dies before compilation);
//! constraints render as trailing checks that halt in the escalation band. Output
//! is deterministic and canonical — and since M2.5, **canonical means slots**:
//! quantities render as `q0, q1, …` assigned in dataflow order (first use in the
//! topologically sorted op sequence), op outputs as `v0, v1, …`, and the model's
//! noun identifiers survive only as metadata in [`Rendered::renames`]. Identical
//! structure ⇒ byte-identical source ⇒ identical artifact hash, whatever the
//! nouns — the detail the precipitation story (H-M3) hangs on. Identifier safety
//! is structural: natural words never become Rust identifiers, so the
//! `final`-class keyword leak is impossible by construction.

use super::{Cartridge, CartridgeOpts, CellConfig, CellHost, Halt};
use std::collections::HashMap;

/// A candidate plan: the model's extraction, ready to render.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub quantities: Vec<Quantity>,
    pub ops: Vec<PlanOp>,
    pub target: String,
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Quantity {
    pub id: String,
    /// The raw 32-bit payload: the integer itself for `int`/`q8`/`q16`, the IEEE
    /// binary32 **bits** for `f32`, the two's-complement bits for `i32` (the repr
    /// tag interprets, bits never lie about which they are — the renderer refuses
    /// every mixed-repr op).
    pub value: u32,
    pub unit: String,
    /// The representation tag (F-wave amendment §F0): orthogonal to dimension the
    /// way `scale` is orthogonal to `unit` — `dollars` says what it measures,
    /// `repr` says how the bits encode it.
    pub repr: Repr,
}

/// A quantity's representation — the type-flow discipline that keeps model-composed
/// plans from doing integer arithmetic on float bits (the silent-wrong class the
/// gate can't otherwise catch, because both derivations would err fluently).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repr {
    /// Plain integers (the GSM default — exact, checked, escalating).
    #[default]
    Int,
    /// Signed 32-bit integers, carried as **two's-complement bits in the u32
    /// state field** (backend zero has no native signed-32 lane — signed add/
    /// sub/mul are bit-identical to u32 patterns, so the renderer emits its own
    /// sign discipline instead: escalate on signed overflow, magnitudes for
    /// mul/div, truncation toward zero — rustc `i32` division semantics). The
    /// range is symmetric: `i32::MIN` has no negation, so parse refuses it and
    /// any op that would produce it escalates — every live value satisfies
    /// `|v| <= i32::MAX` and `0u32 - bits` is always the true magnitude.
    I32,
    /// Q8.8 fixed point (raw scaled integer; `scale` semantics ride the tag).
    Q8,
    /// Q16.16 fixed point.
    Q16,
    /// IEEE binary32 through the owned softfloat kernels (`correctly_rounded`,
    /// never spelled "exact").
    F32,
}

impl Repr {
    fn parse(s: &str) -> Result<Repr, String> {
        Ok(match s {
            "" | "int" => Repr::Int,
            "i32" => Repr::I32,
            "q8" => Repr::Q8,
            "q16" => Repr::Q16,
            "f32" => Repr::F32,
            other => return Err(format!("unknown repr `{other}` (int/i32/q8/q16/f32)")),
        })
    }
    fn name(self) -> &'static str {
        match self {
            Repr::Int => "int",
            Repr::I32 => "i32",
            Repr::Q8 => "q8",
            Repr::Q16 => "q16",
            Repr::F32 => "f32",
        }
    }
    /// The rendered state-field type: f32 fields are `Ty::F32` (typed bits at the
    /// state boundary); every integer repr stays `u32`.
    fn field_ty(self) -> &'static str {
        match self {
            Repr::F32 => "f32",
            _ => "u32",
        }
    }
}

impl Quantity {
    /// The counterfactual battery's "+1": one raw unit for the integer reprs, a
    /// genuine `+1.0` for f32 (recomputed through host f32 — the battery only
    /// compares answer vectors for equality, so any deterministic nudge works,
    /// but a 1-*bit* nudge would vanish under rounding and stress nothing).
    fn perturbed(&self) -> u64 {
        match self.repr {
            Repr::F32 => (f32::from_bits(self.value) + 1.0).to_bits() as u64,
            // Signed: +1 on the *value*, not the bits — saturating at i32::MAX
            // (the same no-op edge the u32 reprs have at u32::MAX).
            Repr::I32 => (self.value as i32).saturating_add(1) as u32 as u64,
            _ => self.value.saturating_add(1) as u64,
        }
    }
}

/// One arithmetic step: `out = a <op> b`. Op order is semantic and preserved;
/// only *quantity* order is canonicalized.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanOp {
    pub op: String,
    pub a: String,
    pub b: String,
    pub out: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// The value must be non-negative. Free by construction (everything is u32 and
    /// a negative intermediate already escalates) — validated for existence, then
    /// rendered as nothing.
    NonNeg(String),
    /// `a` must divide exactly by `b` — a nonzero remainder where the plan declared
    /// `exact_div` is a *wrong plan*, and kills it.
    ExactDiv(String, String),
}

/// A unit is a tiny exponent vector over the base dimensions
/// `[count, money, time, distance]` — `mul` adds, `div` subtracts, `add`/`sub`
/// require equality. `X_per_Y` composes by subtraction.
type Dim = [i8; 4];

fn base_unit(u: &str) -> Result<Dim, String> {
    Ok(match u {
        "" | "scalar" | "ratio" | "bps" => [0, 0, 0, 0],
        "count" | "items" => [1, 0, 0, 0],
        "cents" | "money" => [0, 1, 0, 0],
        "seconds" | "minutes" | "hours" | "days" | "time" => [0, 0, 1, 0],
        "meters" | "distance" => [0, 0, 0, 1],
        "area" => [0, 0, 0, 2],
        other => return Err(format!("unknown unit `{other}`")),
    })
}

fn unit_dim(u: &str) -> Result<Dim, String> {
    match u.split_once("_per_") {
        Some((num, den)) => {
            let (n, d) = (base_unit(num)?, base_unit(den)?);
            Ok([n[0] - d[0], n[1] - d[1], n[2] - d[2], n[3] - d[3]])
        }
        None => base_unit(u),
    }
}

/// Escalation-band halt codes the rendered checks use (see `ESCALATE_REASONS`).
const NEEDS_WIDER_MATH: u16 = 0xFF05; // overflow / negative intermediate
const OUT_OF_DOMAIN: u16 = 0xFF06; // a declared constraint failed — wrong plan
const FLOAT_OVERFLOW: u16 = 0xFF07; // an f32 target reached ±Inf
const FLOAT_DOMAIN: u16 = 0xFF08; // an f32 target is NaN

impl Plan {
    /// Parse the wire format:
    /// `{"quantities":[{"id","value","unit"}...], "ops":[["mul","a","b","out"]...],
    ///   "target":"...", "constraints":[["nonneg","x"],["exact_div","a","b"]...]}`.
    /// Strict — every parse error here is repair-row material.
    pub fn from_json(text: &str) -> Result<Plan, String> {
        let v: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("plan is not JSON: {e}"))?;
        Self::from_value(&v)
    }

    pub fn from_value(v: &serde_json::Value) -> Result<Plan, String> {
        let obj = v.as_object().ok_or("plan must be a JSON object")?;
        let quantities = obj
            .get("quantities")
            .and_then(|q| q.as_array())
            .ok_or("plan needs a `quantities` array")?
            .iter()
            .map(|q| {
                let id = q
                    .get("id")
                    .and_then(|x| x.as_str())
                    .ok_or("quantity needs an `id`")?
                    .to_string();
                let repr = Repr::parse(q.get("repr").and_then(|x| x.as_str()).unwrap_or(""))
                    .map_err(|e| format!("quantity `{id}`: {e}"))?;
                // f32 quantities take a JSON number and convert f64→f32 (a plan's
                // extracted decimals are well under the ~17-digit zone where that
                // double conversion could differ from a direct decimal→f32 parse);
                // integer reprs take the raw u32 payload exactly as before.
                let value = if repr == Repr::F32 {
                    let f = q
                        .get("value")
                        .and_then(|x| x.as_f64())
                        .ok_or_else(|| format!("quantity `{id}` needs a number `value`"))?;
                    (f as f32).to_bits()
                } else if repr == Repr::I32 {
                    // Symmetric range: i32::MIN has no negation, so it never
                    // enters (the render-time sign discipline relies on this).
                    q.get("value")
                        .and_then(|x| x.as_i64())
                        .filter(|v| v.unsigned_abs() <= i32::MAX as u64)
                        .ok_or_else(|| {
                            format!("quantity `{id}` needs an i32 `value` (|v| <= 2147483647)")
                        })? as i32 as u32
                } else {
                    q.get("value")
                        .and_then(|x| x.as_u64())
                        .filter(|v| *v <= u32::MAX as u64)
                        .ok_or_else(|| format!("quantity `{id}` needs a u32 `value`"))?
                        as u32
                };
                let unit = q
                    .get("unit")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(Quantity {
                    id,
                    value,
                    unit,
                    repr,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let ops = obj
            .get("ops")
            .and_then(|o| o.as_array())
            .ok_or("plan needs an `ops` array")?
            .iter()
            .map(|o| {
                let a = o
                    .as_array()
                    .filter(|a| a.len() == 4)
                    .ok_or("each op is [\"add|sub|mul|div\", \"a\", \"b\", \"out\"]")?;
                let s = |i: usize| -> Result<String, String> {
                    a[i].as_str()
                        .map(str::to_string)
                        .ok_or_else(|| "op fields must be strings".to_string())
                };
                Ok(PlanOp {
                    op: s(0)?,
                    a: s(1)?,
                    b: s(2)?,
                    out: s(3)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let target = obj
            .get("target")
            .and_then(|x| x.as_str())
            .ok_or("plan needs a `target`")?
            .to_string();
        let constraints = obj
            .get("constraints")
            .and_then(|c| c.as_array())
            .map(|cs| {
                cs.iter()
                    .map(|c| {
                        let a = c.as_array().ok_or("each constraint is an array")?;
                        let kind = a
                            .first()
                            .and_then(|x| x.as_str())
                            .ok_or("constraint needs a kind")?;
                        let arg = |i: usize| -> Result<String, String> {
                            a.get(i)
                                .and_then(|x| x.as_str())
                                .map(str::to_string)
                                .ok_or_else(|| format!("`{kind}` constraint: missing field"))
                        };
                        match kind {
                            "nonneg" => Ok(Constraint::NonNeg(arg(1)?)),
                            "exact_div" => Ok(Constraint::ExactDiv(arg(1)?, arg(2)?)),
                            other => {
                                Err(format!("unknown constraint `{other}` (nonneg / exact_div)"))
                            }
                        }
                    })
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Plan {
            quantities,
            ops,
            target,
            constraints,
        })
    }

    /// Normalize every quantity through the compiler's **unit base-scale table**
    /// ([`rustz80::canonical_unit`], versioned by [`rustz80::UNIT_TABLE_VERSION`]):
    /// money → cents, time → seconds, unknown nouns → count, rates → explicit
    /// `numerator_per_denominator`. Values scale by the (integer) factor; every
    /// application is returned as a repair row. A rate whose combined factor is
    /// fractional (e.g. `dollars_per_hour` → cents per second) keeps its original
    /// spelling and value, recorded — escalate-don't-guess, never a misscale.
    pub fn normalize_units(&mut self) -> Result<Vec<String>, String> {
        let mut repairs = Vec::new();
        for q in &mut self.quantities {
            let (canon, nf, df) = rustz80::canonical_unit(&q.unit);
            // A non-integer repr never scales silently: multiplying f32 *bits* by a
            // unit factor is nonsense, and multiplying the float itself would round
            // — escalate-don't-misscale. Canonical-spelling renames (factor 1) are
            // fine for every repr.
            if q.repr != Repr::Int && (nf != 1 || df != 1) {
                return Err(format!(
                    "`{}`: unit `{}` needs a ×{nf}/{df} scale but the quantity is {} — \
                     extract it in the canonical unit ({canon}) instead",
                    q.id,
                    q.unit,
                    q.repr.name()
                ));
            }
            if canon == q.unit && nf == 1 && df == 1 {
                continue;
            }
            if nf % df != 0 {
                repairs.push(format!(
                    "unit_kept: `{}` `{}` (fractional factor {nf}/{df} to `{canon}`)",
                    q.id, q.unit
                ));
                continue;
            }
            let f = nf / df;
            if f > 1 {
                q.value = q
                    .value
                    .checked_mul(f)
                    .ok_or_else(|| format!("`{}`: unit scale ×{f} overflows u32", q.id))?;
                repairs.push(format!(
                    "unit_scaled: `{}` {} -> {canon} factor={f}",
                    q.id, q.unit
                ));
            } else if canon != q.unit {
                repairs.push(format!("unit_normalized: `{}` {} -> {canon}", q.id, q.unit));
            }
            q.unit = canon;
        }
        Ok(repairs)
    }

    /// Render to canonical dialect source. Every failure here happens **before**
    /// compilation: unknown/mismatched units, undefined or reassigned names.
    ///
    /// Canonical means slots (M2.5): ops are **topologically sorted** with a
    /// deterministic tie-break (op kind, then operand slots), quantities become
    /// `q0, q1, …` in dataflow order (first use in that sequence), op outputs
    /// become `v0, v1, …` in emission order. The model's identifiers never reach
    /// the Rust — they survive only in [`Rendered::renames`] — so keyword traps
    /// (`final`, `try`, `union`) are impossible by construction, and two
    /// extractions of the same structure render byte-identically whatever nouns
    /// or op order the model chose.
    pub fn render_canonical(&self) -> Result<Rendered, String> {
        for q in &self.quantities {
            if q.id.is_empty() {
                return Err("empty quantity id".into());
            }
        }
        let mut dims: HashMap<&str, Dim> = HashMap::new();
        let mut reprs: HashMap<&str, Repr> = HashMap::new();
        for q in &self.quantities {
            if dims.insert(&q.id, unit_dim(&q.unit)?).is_some() {
                return Err(format!("duplicate quantity `{}`", q.id));
            }
            reprs.insert(&q.id, q.repr);
        }
        let mut outs = std::collections::HashSet::new();
        for (i, o) in self.ops.iter().enumerate() {
            if dims.contains_key(o.out.as_str()) || !outs.insert(o.out.as_str()) {
                return Err(format!("op {i}: `{}` is assigned twice", o.out));
            }
            if o.out.is_empty() {
                return Err(format!("op {i}: empty output id"));
            }
            if !matches!(o.op.as_str(), "add" | "sub" | "mul" | "div") {
                return Err(format!("op {i}: unknown op `{}` (add/sub/mul/div)", o.op));
            }
        }
        // Topological emission order with a deterministic tie-break: among ready
        // ops, (op kind, operand slot keys). Slots are assigned as ops emit, so
        // the tie-break is noun-independent wherever dataflow pins an order; only
        // genuinely symmetric unslotted operands fall back to (value, id).
        let rank = |op: &str| match op {
            "add" => 0u8,
            "sub" => 1,
            "mul" => 2,
            _ => 3,
        };
        let mut slot: HashMap<&str, String> = HashMap::new();
        let mut q_next = 0usize;
        let mut v_next = 0usize;
        let quantity: HashMap<&str, &Quantity> =
            self.quantities.iter().map(|q| (q.id.as_str(), q)).collect();
        // Key for an operand under the current slot assignment. Assigned slots
        // order first (by kind then index); unassigned quantities by (value, id).
        let okey = |name: &str, slot: &HashMap<&str, String>| -> (u8, u64, String) {
            match slot.get(name) {
                Some(s) => {
                    let idx: u64 = s[1..].parse().unwrap_or(u64::MAX);
                    (if s.starts_with('q') { 0 } else { 1 }, idx, String::new())
                }
                None => match quantity.get(name) {
                    Some(q) => (2, q.value as u64, name.to_string()),
                    None => (3, 0, name.to_string()),
                },
            }
        };
        let mut remaining: Vec<usize> = (0..self.ops.len()).collect();
        let mut order: Vec<usize> = Vec::new();
        let mut defined: std::collections::HashSet<&str> =
            self.quantities.iter().map(|q| q.id.as_str()).collect();
        while !remaining.is_empty() {
            let mut ready: Vec<usize> = remaining
                .iter()
                .copied()
                .filter(|&i| {
                    let o = &self.ops[i];
                    defined.contains(o.a.as_str()) && defined.contains(o.b.as_str())
                })
                .collect();
            if ready.is_empty() {
                let i = remaining[0];
                let o = &self.ops[i];
                let missing = if defined.contains(o.a.as_str()) {
                    &o.b
                } else {
                    &o.a
                };
                return Err(format!("op {i}: `{missing}` is not defined yet"));
            }
            ready.sort_by_key(|&i| {
                let o = &self.ops[i];
                (rank(&o.op), okey(&o.a, &slot), okey(&o.b, &slot))
            });
            let i = ready[0];
            let o = &self.ops[i];
            for operand in [&o.a, &o.b] {
                if !slot.contains_key(operand.as_str()) && quantity.contains_key(operand.as_str()) {
                    slot.insert(operand.as_str(), format!("q{q_next}"));
                    q_next += 1;
                }
            }
            slot.insert(o.out.as_str(), format!("v{v_next}"));
            v_next += 1;
            defined.insert(o.out.as_str());
            order.push(i);
            remaining.retain(|&r| r != i);
        }
        // Unused quantities: slots after the used ones, by (value, id) — they
        // carry state but never appear in an op line.
        let mut unused: Vec<&Quantity> = self
            .quantities
            .iter()
            .filter(|q| !slot.contains_key(q.id.as_str()))
            .collect();
        unused.sort_by(|a, b| (a.value, &a.id).cmp(&(b.value, &b.id)));
        for q in unused {
            slot.insert(&q.id, format!("q{q_next}"));
            q_next += 1;
        }
        // Type-flow the ops in emission order: units lawful, one line per op.
        let mut op_lines = String::new();
        for (n, &i) in order.iter().enumerate() {
            let op = &self.ops[i];
            // Repr type-flow first (orthogonal to dimension): both operands must
            // share a representation — a mixed op is a *wrong plan*, not a value
            // to coerce (the conversions exist, but the model must plan them).
            let ra = reprs[op.a.as_str()];
            let rb = reprs[op.b.as_str()];
            if ra != rb {
                return Err(format!(
                    "op {i}: `{}` mixes {} and {} — representations never convert \
                     implicitly (plan an explicit conversion, or extract both \
                     quantities in one repr)",
                    op.out,
                    ra.name(),
                    rb.name()
                ));
            }
            if matches!(ra, Repr::Q8 | Repr::Q16) && matches!(op.op.as_str(), "mul" | "div") {
                return Err(format!(
                    "op {i}: `{}` — q-repr {} needs the scale-aware fixed-point \
                     kernels (`q_mul`/`q_div` cells), which the renderer doesn't \
                     compose yet; use int repr with explicit scaling, or f32",
                    op.out, op.op
                ));
            }
            reprs.insert(&op.out, ra);
            let da = dims[op.a.as_str()];
            let db = dims[op.b.as_str()];
            let dout = match op.op.as_str() {
                "add" | "sub" => {
                    if da != db {
                        return Err(format!(
                            "op {i}: `{}` — unit mismatch (can't {} different dimensions)",
                            op.out, op.op
                        ));
                    }
                    da
                }
                "mul" => [da[0] + db[0], da[1] + db[1], da[2] + db[2], da[3] + db[3]],
                _ => [da[0] - db[0], da[1] - db[1], da[2] - db[2], da[3] - db[3]],
            };
            dims.insert(&op.out, dout);
            let (a, b, out) = (
                &slot[op.a.as_str()],
                &slot[op.b.as_str()],
                &slot[op.out.as_str()],
            );
            // f32 ops render as plain typed arithmetic — the dialect's operator
            // routing compiles them to the softfloat kernels, and rustc-exact IEEE
            // semantics hold *inside* the cell (overflow → Inf, 0/0 → NaN); the
            // escalation happens at the target boundary (the rendered finite gate),
            // per the amendment's escalate-at-the-boundary contract. No wrap/
            // negative/overflow guards: those are integer diseases.
            if ra == Repr::F32 {
                let line = match op.op.as_str() {
                    "add" => "+",
                    "sub" => "-",
                    "mul" => "*",
                    _ => "/",
                };
                op_lines.push_str(&format!("        self.{out} = self.{a} {line} self.{b};\n"));
                continue;
            }
            // Signed ops: two's-complement bits in u32 fields. add/sub are the
            // wrapping u32 patterns plus the textbook sign-rule escalation;
            // mul/div go through magnitudes (safe: i32::MIN is excluded at parse
            // and escalated below, so `0 - bits` is always the true magnitude)
            // with the result sign reapplied — division truncates toward zero,
            // rustc i32 semantics. Any result landing on the MIN bit pattern
            // escalates too, keeping the range symmetric for downstream ops.
            if ra == Repr::I32 {
                const SIGN: &str = "2147483648u32"; // the i32 sign bit / MIN pattern
                let signs = format!(
                    "        let sa{n} = (self.{a} >= {SIGN}) as u32;\n        let sb{n} = (self.{b} >= {SIGN}) as u32;\n"
                );
                // Branch-free |x|: with mask = 0 - sign (all ones iff negative),
                // (x ^ mask) - mask is x or its two's-complement negation.
                let mags = format!(
                    "        let na{n} = 0u32 - sa{n};\n        let nb{n} = 0u32 - sb{n};\n        \
                     let ma{n} = (self.{a} ^ na{n}) - na{n};\n        \
                     let mb{n} = (self.{b} ^ nb{n}) - nb{n};\n"
                );
                match op.op.as_str() {
                    "add" | "sub" => {
                        let (sym, overflow) = if op.op == "add" {
                            // Same signs in, different sign out ⇒ overflow.
                            ("+", format!("sa{n} == sb{n}"))
                        } else {
                            // Different signs in, result flips from a ⇒ overflow.
                            ("-", format!("sa{n} != sb{n}"))
                        };
                        op_lines.push_str(&format!(
                            "{signs}        let t{n} = self.{a} {sym} self.{b};\n        \
                             if {overflow} && ((t{n} >= {SIGN}) as u32) != sa{n} {{ halt({NEEDS_WIDER_MATH}u16); }}\n        \
                             if t{n} == {SIGN} {{ halt({NEEDS_WIDER_MATH}u16); }}\n        \
                             self.{out} = t{n};\n"
                        ));
                    }
                    "mul" => {
                        op_lines.push_str(&format!(
                            "{signs}{mags}        let p{n} = ma{n} * mb{n};\n        \
                             if ma{n} != 0u32 && p{n} / ma{n} != mb{n} {{ halt({NEEDS_WIDER_MATH}u16); }}\n        \
                             if p{n} >= {SIGN} {{ halt({NEEDS_WIDER_MATH}u16); }}\n        \
                             let nr{n} = 0u32 - (sa{n} ^ sb{n});\n        \
                             self.{out} = (p{n} ^ nr{n}) - nr{n};\n"
                        ));
                    }
                    _ => {
                        // Magnitude quotient ≤ magnitude dividend < 2^31, so the
                        // sign reapplication can't overflow; /0 halts on its own.
                        op_lines.push_str(&format!(
                            "{signs}{mags}        let d{n} = ma{n} / mb{n};\n        \
                             let nr{n} = 0u32 - (sa{n} ^ sb{n});\n        \
                             self.{out} = (d{n} ^ nr{n}) - nr{n};\n"
                        ));
                    }
                }
                continue;
            }
            match op.op.as_str() {
                "add" => {
                    // Wrap detect: a checked add, escalating rather than wrapping.
                    op_lines.push_str(&format!(
                        "        let t{n} = self.{a} + self.{b};\n        if t{n} < self.{a} {{ halt({NEEDS_WIDER_MATH}u16); }}\n        self.{out} = t{n};\n"
                    ));
                }
                "sub" => {
                    // A negative intermediate is the sign-magnitude escalation point.
                    op_lines.push_str(&format!(
                        "        if self.{a} < self.{b} {{ halt({NEEDS_WIDER_MATH}u16); }}\n        self.{out} = self.{a} - self.{b};\n"
                    ));
                }
                "mul" => {
                    // The classic post-hoc overflow check (docs 10 §Arithmetic).
                    op_lines.push_str(&format!(
                        "        let t{n} = self.{a} * self.{b};\n        if self.{a} != 0u32 && t{n} / self.{a} != self.{b} {{ halt({NEEDS_WIDER_MATH}u16); }}\n        self.{out} = t{n};\n"
                    ));
                }
                _ => {
                    // Floor division; a zero divisor halts DivByZero on its own.
                    op_lines.push_str(&format!("        self.{out} = self.{a} / self.{b};\n"));
                }
            }
        }
        // Constraints: trailing checks, canonical order (sorted by rendering).
        let mut checks: Vec<String> = Vec::new();
        for c in &self.constraints {
            match c {
                Constraint::NonNeg(id) => {
                    if !dims.contains_key(id.as_str()) {
                        return Err(format!("nonneg: `{id}` is not defined"));
                    }
                    // Unsigned integer reprs: u32 by construction, sub already
                    // escalates — renders as nothing. f32 has a sign bit, so the
                    // constraint is a real check (NaN compares false and passes
                    // here; a NaN *target* still dies at the finite gate). i32's
                    // sign lives in the top bit of the u32 field.
                    if reprs[id.as_str()] == Repr::F32 {
                        checks.push(format!(
                            "        if self.{} < 0.0f32 {{ halt({OUT_OF_DOMAIN}u16); }}\n",
                            slot[id.as_str()]
                        ));
                    } else if reprs[id.as_str()] == Repr::I32 {
                        checks.push(format!(
                            "        if self.{} >= 2147483648u32 {{ halt({OUT_OF_DOMAIN}u16); }}\n",
                            slot[id.as_str()]
                        ));
                    }
                }
                Constraint::ExactDiv(a, b) => {
                    if !dims.contains_key(a.as_str()) || !dims.contains_key(b.as_str()) {
                        return Err(format!("exact_div: `{a}`/`{b}` must be defined"));
                    }
                    if reprs[a.as_str()] == Repr::F32 || reprs[b.as_str()] == Repr::F32 {
                        return Err(format!(
                            "exact_div: `{a}`/`{b}` — exactness is the integer/fraction \
                             tiers' claim; f32 is correctly_rounded, never exact"
                        ));
                    }
                    let (ia, ib) = (
                        reprs[a.as_str()] == Repr::I32,
                        reprs[b.as_str()] == Repr::I32,
                    );
                    if ia != ib {
                        return Err(format!(
                            "exact_div: `{a}`/`{b}` mixes int and i32 — representations \
                             never convert implicitly"
                        ));
                    }
                    if ia {
                        // Signed: exactness is a magnitude question. Temps are
                        // keyed by the slots (not a counter) so the rendered
                        // check is independent of constraint order — the sort
                        // below stays a true canonicalization.
                        let (sa, sb) = (&slot[a.as_str()], &slot[b.as_str()]);
                        checks.push(format!(
                            "        let e{sa}{sb}a = 0u32 - ((self.{sa} >= 2147483648u32) as u32);\n        \
                             let e{sa}{sb}b = 0u32 - ((self.{sb} >= 2147483648u32) as u32);\n        \
                             if ((self.{sa} ^ e{sa}{sb}a) - e{sa}{sb}a) % ((self.{sb} ^ e{sa}{sb}b) - e{sa}{sb}b) != 0u32 {{ halt({OUT_OF_DOMAIN}u16); }}\n"
                        ));
                    } else {
                        checks.push(format!(
                            "        if self.{} % self.{} != 0u32 {{ halt({OUT_OF_DOMAIN}u16); }}\n",
                            slot[a.as_str()],
                            slot[b.as_str()]
                        ));
                    }
                }
            }
        }
        checks.sort();
        // A repeated constraint must not redeclare its (slot-keyed) temps.
        checks.dedup();
        let target_slot = slot
            .get(self.target.as_str())
            .cloned()
            .ok_or_else(|| format!("target `{}` is never defined", self.target))?;
        let target_repr = reprs[self.target.as_str()];
        // The struct: q slots then v slots, in slot order — each at its repr's
        // field type (`f32` state fields are `Ty::F32`: typed bits at the boundary).
        let slot_repr: HashMap<&str, Repr> = slot
            .iter()
            .map(|(name, s)| (s.as_str(), reprs[name]))
            .collect();
        let field = |s: String| -> String {
            let ty = slot_repr
                .get(s.as_str())
                .copied()
                .unwrap_or_default()
                .field_ty();
            format!("{s}: {ty}")
        };
        let fields: Vec<String> = (0..q_next)
            .map(|i| field(format!("q{i}")))
            .chain((0..v_next).map(|i| field(format!("v{i}"))))
            .collect();
        // The tail: an integer target answers in the low word; an f32 target
        // answers through its state field (read by name), with the **finite gate**
        // at the boundary — NaN is `float_domain`, ±Inf `float_overflow`; the
        // status result is 1. IEEE propagated inside, escalate-not-lie at return.
        let tail = if target_repr == Repr::F32 {
            format!(
                "        if self.{target_slot}.is_nan() {{ halt({FLOAT_DOMAIN}u16); }}\n        \
                 let fin = self.{target_slot}.is_finite();\n        \
                 if !fin {{ halt({FLOAT_OVERFLOW}u16); }}\n        1u16"
            )
        } else {
            format!("        (self.{target_slot} & 65535u32) as u16")
        };
        let src = format!(
            "//! rendered plan\nstruct P {{ {} }}\nimpl P {{\n    fn run(&mut self) -> u16 {{\n{op_lines}{}{tail}\n    }}\n}}\n",
            fields.join(", "),
            checks.concat(),
        );
        let mut renames: Vec<(String, String)> = slot
            .iter()
            .map(|(name, s)| (name.to_string(), s.clone()))
            .collect();
        renames.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(Rendered {
            src,
            renames,
            target_slot,
            target_repr,
        })
    }

    /// [`render_canonical`](Plan::render_canonical), source only.
    pub fn render(&self) -> Result<String, String> {
        self.render_canonical().map(|r| r.src)
    }
}

/// A rendered plan: canonical source plus the metadata the slots displaced.
#[derive(Debug, Clone)]
pub struct Rendered {
    pub src: String,
    /// `(source_name, slot)` for every quantity (`q*`) and op output (`v*`) —
    /// the only place the model's identifiers survive.
    pub renames: Vec<(String, String)>,
    /// The slot the plan's target landed in.
    pub target_slot: String,
    /// The target's representation — an `f32` target's `answer` is its raw
    /// binary32 bits (`f32::from_bits` to read); integer targets are the value.
    pub target_repr: Repr,
}

/// Parse one plan object **or** an array of candidate plans — the shape every
/// surface (CLI, py, MCP) accepts.
pub fn plans_from_json(text: &str) -> Result<Vec<Plan>, String> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("plans are not JSON: {e}"))?;
    match &v {
        serde_json::Value::Array(items) => items.iter().map(Plan::from_value).collect(),
        one => Ok(vec![Plan::from_value(one)?]),
    }
}

/// One plan's fate through the loop.
#[derive(Debug, Clone)]
pub struct PlanOutcome {
    /// Hex artifact hash of the rendered cell (`None` if it never compiled).
    pub artifact: Option<String>,
    /// The full-width answer (the target field, post-run) for a surviving plan.
    /// For an `f32` target (`answer_repr == "f32"`) these are the raw binary32
    /// bits — `f32::from_bits(answer as u32)` to read.
    pub answer: Option<u64>,
    /// The target's representation name (`int`/`q8`/`q16`/`f32`).
    pub answer_repr: &'static str,
    /// Why it died: a render error, a compile error, or the halt that killed it.
    pub kill: Option<String>,
    /// The rendered artifact was already in the catalog — retrieved, not compiled
    /// (the H-M3 precipitation counter).
    pub retrieved: bool,
    /// Deterministic repairs applied before render (unit scaling/normalization),
    /// plus the slot renames — the only place the model's identifiers survive.
    pub repairs: Vec<String>,
    /// `(source_name, slot)` for the rendered cell's state fields.
    pub renames: Vec<(String, String)>,
}

/// What `solve` returns: per-plan outcomes and the consensus answer, if any.
#[derive(Debug, Clone)]
pub struct SolveReport {
    pub outcomes: Vec<PlanOutcome>,
    /// The answer of the winning agreement group (`None` = no survivor consensus —
    /// escalate up the ladder).
    pub answer: Option<u64>,
    /// Whether disagreement forced the counterfactual battery.
    pub battery_ran: bool,
}

impl SolveReport {
    pub fn to_json(&self) -> String {
        use serde_json::json;
        json!({
            "answer": self.answer,
            "battery_ran": self.battery_ran,
            "plans": self.outcomes.iter().map(|o| json!({
                "artifact": o.artifact,
                "answer": o.answer,
                "answer_repr": o.answer_repr,
                "kill": o.kill,
                "retrieved": o.retrieved,
                "repairs": o.repairs,
                "renames": o.renames.iter()
                    .map(|(n, s)| json!({"source_name": n, "slot": s}))
                    .collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
        .to_string()
    }
}

impl CellHost {
    /// The minimal `cell_solve` loop (M2): render each candidate plan → compile
    /// (**the plan is a cell**, catalogued by artifact hash — a re-seen schema is
    /// *retrieved*, not recompiled) → run with the quantities as state fields,
    /// memoized → kill plans that escalate or halt → if survivors disagree, run
    /// the **counterfactual battery** (perturb every quantity by +1; keep the
    /// largest group agreeing across the whole sweep). `None` answer = escalate.
    pub fn solve(&mut self, plans: &[Plan], budget: u64) -> Result<SolveReport, String> {
        let mut outcomes: Vec<PlanOutcome> = Vec::new();
        // (plan idx, handle, normalized plan, id→slot, target slot)
        type Live = (usize, usize, Plan, HashMap<String, String>, String);
        let mut live: Vec<Live> = Vec::new();
        for (i, plan) in plans.iter().enumerate() {
            // The unit base-scale table first (money → cents, unknown nouns →
            // count, …) — deterministic, recorded, versioned in the compiler.
            let mut plan = plan.clone();
            let mut repairs = match plan.normalize_units() {
                Ok(r) => r,
                Err(e) => {
                    outcomes.push(PlanOutcome {
                        artifact: None,
                        answer: None,
                        answer_repr: "int",
                        kill: Some(format!("render: {e}")),
                        retrieved: false,
                        repairs: Vec::new(),
                        renames: Vec::new(),
                    });
                    continue;
                }
            };
            let rendered = match plan.render_canonical() {
                Ok(r) => r,
                Err(e) => {
                    outcomes.push(PlanOutcome {
                        artifact: None,
                        answer: None,
                        answer_repr: "int",
                        kill: Some(format!("render: {e}")),
                        retrieved: false,
                        repairs,
                        renames: Vec::new(),
                    });
                    continue;
                }
            };
            let cart = match Cartridge::compile(
                &rendered.src,
                CellConfig::sandboxed(),
                CartridgeOpts {
                    entry: Some("P::run".into()),
                    summary: format!("rendered plan (target {})", plan.target),
                    tags: vec!["plan".into()],
                    ..Default::default()
                },
            ) {
                Ok(c) => c,
                Err(e) => {
                    outcomes.push(PlanOutcome {
                        artifact: None,
                        answer: None,
                        answer_repr: rendered.target_repr.name(),
                        kill: Some(format!("compile: {e}")),
                        retrieved: false,
                        repairs,
                        renames: rendered.renames,
                    });
                    continue;
                }
            };
            repairs.extend(cart.canon_repairs.iter().map(|r| r.to_string()));
            let hash = crate::facts::hex(&cart.artifact_hash());
            let id = format!("plan.{}", &hash[..16]);
            // Precipitation: same schema ⇒ same hash ⇒ already catalogued.
            let retrieved = self.manifest(&id).is_some();
            if !retrieved {
                let mut cart = cart;
                cart.manifest.id = id.clone();
                self.add(cart);
            }
            let h = self.handle_for(&id)?;
            let slots: HashMap<String, String> = rendered.renames.iter().cloned().collect();
            let fields: Vec<(String, u64)> = plan
                .quantities
                .iter()
                .map(|q| (slots[&q.id].clone(), q.value as u64))
                .collect();
            let (fast, state) = self.run_state_fast(h, &fields, budget)?;
            let answer = state
                .iter()
                .find(|(k, _)| *k == rendered.target_slot)
                .map(|(_, v)| *v);
            match fast.halt {
                Halt::Returned => {
                    outcomes.push(PlanOutcome {
                        artifact: Some(hash),
                        answer,
                        answer_repr: rendered.target_repr.name(),
                        kill: None,
                        retrieved,
                        repairs,
                        renames: rendered.renames.clone(),
                    });
                    live.push((i, h, plan, slots, rendered.target_slot));
                }
                other => {
                    let why = match other {
                        Halt::Escalate(c) => {
                            format!(
                                "escalate:{}",
                                Halt::Escalate(c).escalate_reason().unwrap_or("?")
                            )
                        }
                        Halt::DivByZero => "div_by_zero".into(),
                        Halt::Halted(c) => format!("halt:{c}"),
                        Halt::CycleBudget => "cycle_budget".into(),
                        Halt::MemoryLimit => "memory_limit".into(),
                        Halt::Returned => unreachable!(),
                    };
                    outcomes.push(PlanOutcome {
                        artifact: Some(hash),
                        answer: None,
                        answer_repr: rendered.target_repr.name(),
                        kill: Some(why),
                        retrieved,
                        repairs,
                        renames: rendered.renames,
                    });
                }
            }
        }
        // Consensus / battery.
        let answers: Vec<Option<u64>> = live
            .iter()
            .map(|(i, _, _, _, _)| outcomes[*i].answer)
            .collect();
        let mut battery_ran = false;
        let answer = match live.len() {
            0 => None,
            1 => answers[0],
            _ => {
                // Always perturb when more than one plan survives — even if they
                // already agree pre-perturbation, a coincidental agreement at the
                // given numbers (the same class of bug as the documented
                // min/median3 register-0 coincidence) must not be accepted as
                // consensus without being stress-tested: perturb every quantity
                // by +1 (the field sweep the substrate makes free) and keep the
                // largest group whose whole answer vector agrees.
                battery_ran = true;
                let mut vectors: Vec<Vec<Option<u64>>> = vec![Vec::new(); live.len()];
                let names: Vec<String> = {
                    let mut all: Vec<String> = live
                        .iter()
                        .flat_map(|(_, _, p, _, _)| p.quantities.iter().map(|q| q.id.clone()))
                        .collect();
                    all.sort();
                    all.dedup();
                    all
                };
                for name in &names {
                    for (row, (_, h, plan, slots, target_slot)) in live.iter().enumerate() {
                        let fields: Vec<(String, u64)> = plan
                            .quantities
                            .iter()
                            .map(|q| {
                                let v = if q.id == *name {
                                    q.perturbed()
                                } else {
                                    q.value as u64
                                };
                                (slots[&q.id].clone(), v)
                            })
                            .collect();
                        let out = self.run_state_fast(*h, &fields, budget).ok().and_then(
                            |(fast, state)| {
                                (fast.halt == Halt::Returned)
                                    .then(|| {
                                        state
                                            .iter()
                                            .find(|(k, _)| *k == *target_slot)
                                            .map(|(_, v)| *v)
                                    })
                                    .flatten()
                            },
                        );
                        vectors[row].push(out);
                    }
                }
                let mut groups: HashMap<(Option<u64>, Vec<Option<u64>>), usize> = HashMap::new();
                for (slot, v) in vectors.iter().enumerate() {
                    *groups.entry((answers[slot], v.clone())).or_insert(0) += 1;
                }
                let best = groups.iter().max_by_key(|(_, n)| **n);
                match best {
                    Some(((ans, _), n)) if groups.values().filter(|m| *m == n).count() == 1 => *ans,
                    _ => None, // tie or nothing — escalate
                }
            }
        };
        // Plan cells stay **warm** — one runner per schema, its memo table intact:
        // the next solve retrieves the schema and serves repeats as cache hits,
        // and `export_facts` finds the residue. (Unloading would return the
        // runner to the pool and clear the memo table with it.)
        Ok(SolveReport {
            outcomes,
            answer,
            battery_ran,
        })
    }
}
