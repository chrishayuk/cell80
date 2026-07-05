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
//! is deterministic and canonical (sorted quantity order, fixed formatting) — the
//! detail the precipitation story hangs on: identical schemas must hash
//! identically, or H-M3 is unfalsifiable.

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
    pub value: u32,
    pub unit: String,
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
                let value = q
                    .get("value")
                    .and_then(|x| x.as_u64())
                    .filter(|v| *v <= u32::MAX as u64)
                    .ok_or_else(|| format!("quantity `{id}` needs a u32 `value`"))?
                    as u32;
                let unit = q
                    .get("unit")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(Quantity { id, value, unit })
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

    /// Render to canonical dialect source. Every failure here happens **before**
    /// compilation: bad identifiers, unknown/mismatched units, undefined or
    /// reassigned names. Output is deterministic: quantities sort by id, ops keep
    /// their (semantic) order, formatting is fixed — same plan modulo quantity
    /// order ⇒ byte-identical source ⇒ identical artifact hash.
    pub fn render(&self) -> Result<String, String> {
        // Identifier discipline: everything becomes a field name.
        let ident_ok = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                && !matches!(
                    s,
                    // Renderer-specific collisions (the entry point, the receiver).
                    "self" | "run"
                    // Rust's strict keywords reachable in this lowercase+digit+underscore
                    // charset (Self/Crate/etc capitalized forms can't match — the charset
                    // check above already excludes them).
                    | "as" | "break" | "const" | "continue" | "crate" | "dyn" | "else"
                    | "enum" | "extern" | "false" | "fn" | "for" | "if" | "impl" | "in"
                    | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref"
                    | "return" | "static" | "struct" | "super" | "trait" | "true" | "type"
                    | "unsafe" | "use" | "where" | "while" | "async" | "await"
                    // Reserved for future use — not compile errors *today* by accident, but
                    // exactly the trap `final` fell into: syn accepts them as keyword tokens
                    // regardless of whether the current grammar defines a meaning for them.
                    | "abstract" | "become" | "box" | "do" | "final" | "macro"
                    | "override" | "priv" | "typeof" | "unsized" | "virtual" | "yield"
                    | "try" | "union"
                )
        };
        let mut dims: HashMap<&str, Dim> = HashMap::new();
        let mut quantities: Vec<&Quantity> = self.quantities.iter().collect();
        quantities.sort_by(|a, b| a.id.cmp(&b.id));
        for q in &quantities {
            if !ident_ok(&q.id) {
                return Err(format!("bad quantity id `{}`", q.id));
            }
            if dims.insert(&q.id, unit_dim(&q.unit)?).is_some() {
                return Err(format!("duplicate quantity `{}`", q.id));
            }
        }
        // Type-flow the ops: inputs defined, outputs fresh, units lawful.
        let mut op_lines = String::new();
        for (i, op) in self.ops.iter().enumerate() {
            if !ident_ok(&op.out) {
                return Err(format!("bad op output id `{}`", op.out));
            }
            let da = *dims
                .get(op.a.as_str())
                .ok_or_else(|| format!("op {i}: `{}` is not defined yet", op.a))?;
            let db = *dims
                .get(op.b.as_str())
                .ok_or_else(|| format!("op {i}: `{}` is not defined yet", op.b))?;
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
                "div" => [da[0] - db[0], da[1] - db[1], da[2] - db[2], da[3] - db[3]],
                other => return Err(format!("op {i}: unknown op `{other}` (add/sub/mul/div)")),
            };
            if dims.insert(&op.out, dout).is_some() {
                return Err(format!("op {i}: `{}` is assigned twice", op.out));
            }
            let (a, b, out) = (&op.a, &op.b, &op.out);
            match op.op.as_str() {
                "add" => {
                    // Wrap detect: a checked add, escalating rather than wrapping.
                    op_lines.push_str(&format!(
                        "        let t{i} = self.{a} + self.{b};\n        if t{i} < self.{a} {{ halt({NEEDS_WIDER_MATH}u16); }}\n        self.{out} = t{i};\n"
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
                        "        let t{i} = self.{a} * self.{b};\n        if self.{a} != 0u32 && t{i} / self.{a} != self.{b} {{ halt({NEEDS_WIDER_MATH}u16); }}\n        self.{out} = t{i};\n"
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
                    // u32 by construction; sub already escalates. Renders as nothing.
                }
                Constraint::ExactDiv(a, b) => {
                    if !dims.contains_key(a.as_str()) || !dims.contains_key(b.as_str()) {
                        return Err(format!("exact_div: `{a}`/`{b}` must be defined"));
                    }
                    checks.push(format!(
                        "        if self.{a} % self.{b} != 0u32 {{ halt({OUT_OF_DOMAIN}u16); }}\n"
                    ));
                }
            }
        }
        checks.sort();
        if !dims.contains_key(self.target.as_str()) {
            return Err(format!("target `{}` is never defined", self.target));
        }
        // The struct: sorted quantities first, then op outputs in op order.
        let mut fields: Vec<String> = quantities
            .iter()
            .map(|q| format!("{}: u32", q.id))
            .collect();
        fields.extend(self.ops.iter().map(|o| format!("{}: u32", o.out)));
        let target = &self.target;
        Ok(format!(
            "//! rendered plan — target `{target}`\nstruct P {{ {} }}\nimpl P {{\n    fn run(&mut self) -> u16 {{\n{op_lines}{}        (self.{target} & 65535u32) as u16\n    }}\n}}\n",
            fields.join(", "),
            checks.concat(),
        ))
    }
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
    pub answer: Option<u64>,
    /// Why it died: a render error, a compile error, or the halt that killed it.
    pub kill: Option<String>,
    /// The rendered artifact was already in the catalog — retrieved, not compiled
    /// (the H-M3 precipitation counter).
    pub retrieved: bool,
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
                "kill": o.kill,
                "retrieved": o.retrieved,
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
        let mut live: Vec<(usize, usize, Plan)> = Vec::new(); // (plan idx, handle, plan)
        for (i, plan) in plans.iter().enumerate() {
            let src = match plan.render() {
                Ok(s) => s,
                Err(e) => {
                    outcomes.push(PlanOutcome {
                        artifact: None,
                        answer: None,
                        kill: Some(format!("render: {e}")),
                        retrieved: false,
                    });
                    continue;
                }
            };
            let cart = match Cartridge::compile(
                &src,
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
                        kill: Some(format!("compile: {e}")),
                        retrieved: false,
                    });
                    continue;
                }
            };
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
            let fields: Vec<(String, u64)> = plan
                .quantities
                .iter()
                .map(|q| (q.id.clone(), q.value as u64))
                .collect();
            let (fast, state) = self.run_state_fast(h, &fields, budget)?;
            let answer = state
                .iter()
                .find(|(k, _)| *k == plan.target)
                .map(|(_, v)| *v);
            match fast.halt {
                Halt::Returned => {
                    outcomes.push(PlanOutcome {
                        artifact: Some(hash),
                        answer,
                        kill: None,
                        retrieved,
                    });
                    live.push((i, h, plan.clone()));
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
                        kill: Some(why),
                        retrieved,
                    });
                }
            }
        }
        // Consensus / battery.
        let answers: Vec<Option<u64>> = live.iter().map(|(i, _, _)| outcomes[*i].answer).collect();
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
                        .flat_map(|(_, _, p)| p.quantities.iter().map(|q| q.id.clone()))
                        .collect();
                    all.sort();
                    all.dedup();
                    all
                };
                for name in &names {
                    for (slot, (_, h, plan)) in live.iter().enumerate() {
                        let fields: Vec<(String, u64)> = plan
                            .quantities
                            .iter()
                            .map(|q| {
                                let v = if q.id == *name {
                                    q.value.saturating_add(1) as u64
                                } else {
                                    q.value as u64
                                };
                                (q.id.clone(), v)
                            })
                            .collect();
                        let out = self.run_state_fast(*h, &fields, budget).ok().and_then(
                            |(fast, state)| {
                                (fast.halt == Halt::Returned)
                                    .then(|| {
                                        state
                                            .iter()
                                            .find(|(k, _)| *k == plan.target)
                                            .map(|(_, v)| *v)
                                    })
                                    .flatten()
                            },
                        );
                        vectors[slot].push(out);
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
