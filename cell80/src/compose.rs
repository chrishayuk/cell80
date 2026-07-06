//! M2.9 — **`cell80 compose`**: parse → canonicalize (`Full`, wide lane) → **link**
//! (resolve unknown call targets against the library by search + arity, inline the
//! resolved cell's source, recompile) → run, with the N-derivation **agreement gate**
//! as a first-class solve mode (`docs/math-campaign-amendment.md` §M2.9).
//!
//! The Python retry loop in `experiments/planfix/compose_link.py` is the reference
//! implementation this productionizes; the loop shape is the same — the compiler's own
//! `unknown call target` rejection (typed as `E0504`) is the linker's cue, one
//! resolution per iteration, no progress ⇒ a named failure, never a guess.
//!
//! Only **free-fn** cells inline (a state cell has no callable `fn` surface — its
//! `run(&mut self)` reads fields). When the widened lane needs an overload that exists
//! only as a state cell, the failure is typed and names the cell: that's an
//! escalation, and the honest fix is a free-fn wide sibling in the library (legal
//! since two-u32-params landed), not a silent narrow call.
//!
//! Accounting discipline (settled input 3): an answer exists only when a composed
//! cell **compiled, ran, and returned** — `correct_via_solve`, never a stated number.

use super::{Cartridge, CartridgeOpts, CellConfig, CellHost, Halt};
use std::collections::HashSet;
use std::path::Path;
use syn::visit::Visit;

/// One linker resolution: the model's named intent and the cell that answered it.
#[derive(Debug, Clone)]
pub struct Resolution {
    pub name: String,
    pub cell_id: String,
    pub arity: usize,
}

/// A linked, compiled composed cell — not yet run.
pub struct Composition {
    pub cart: Cartridge,
    /// The final linked canonical source (what the artifact hash covers).
    pub source: String,
    pub resolutions: Vec<Resolution>,
    /// Canonicalization repairs, stringified with their `E*` codes.
    pub repairs: Vec<String>,
    pub widened: bool,
}

/// One derivation's fate. `answer` is `Some` **only** for a clean `Returned` run.
#[derive(Debug, Clone)]
pub struct DerivationOutcome {
    pub answer: Option<u64>,
    pub kill: Option<String>,
    pub artifact: Option<String>,
    pub resolutions: Vec<(String, String)>,
    pub repairs: Vec<String>,
    /// The composed schema was already catalogued (H-M3 precipitation counter).
    pub retrieved: bool,
}

const LINK_BUDGET: usize = 12;

fn unknown_target(err: &str) -> Option<String> {
    let rest = err.split("unknown call target `").nth(1)?;
    rest.split('`').next().map(str::to_string)
}

/// Arity of the first call to `name` in `src` (syn-level, not a regex).
fn call_arity(src: &str, name: &str) -> Option<usize> {
    struct Finder<'a> {
        name: &'a str,
        arity: Option<usize>,
    }
    impl<'ast, 'a> Visit<'ast> for Finder<'a> {
        fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
            if self.arity.is_none() {
                if let syn::Expr::Path(p) = &*c.func {
                    if p.path.is_ident(self.name) {
                        self.arity = Some(c.args.len());
                    }
                }
            }
            syn::visit::visit_expr_call(self, c);
        }
    }
    let file: syn::File = syn::parse_str(src).ok()?;
    let mut f = Finder { name, arity: None };
    f.visit_file(&file);
    f.arity
}

/// A manifest is inlinable iff it is a free-fn cell (state cells have no callable fn).
fn free_fn_arity(m: &super::Manifest) -> Option<usize> {
    m.signature
        .state
        .is_empty()
        .then_some(m.signature.params.len())
}

/// The minimum `search_scored` magnitude for a hit to link **without** lexical
/// support. Measured against the 261-cell library: genuine name matches score
/// ≥ 0.66 (`is_gt` 0.84, `gcd3` 0.82, `max` 0.66); the best hit for a nonsense
/// name peaks at 0.46 on shared trigrams. The floor sits between those bands.
/// (The magnitude is the index's calibrated scale — gate on it, never rescale it.)
const LINK_CONFIDENCE_FLOOR: f32 = 0.6;

/// A candidate is credible for linking if its score clears the floor or its id and
/// the call name contain one another (the `gcd3` → `gcd` arity-rescue class, which
/// scores low on trigrams but is lexically unambiguous).
fn credible(name: &str, id: &str, score: f32) -> bool {
    if score >= LINK_CONFIDENCE_FLOOR {
        return true;
    }
    let (n, i) = (name.trim_end_matches("_u32"), id.trim_end_matches("_u32"));
    n.len() >= 3 && i.len() >= 3 && (n.contains(i) || i.contains(n))
}

/// Resolve `name` to a library cell: an exact `{name}_u32` id first when the lane is
/// wide, then search + arity (the `call_match.py` result: arity rescues the wrong
/// overload at text-top), then the text-top free-fn — each gated by [`credible`], so
/// a nonsense name is a typed refusal, never a silent guess. State-cell-only matches
/// are a typed failure, not a fallback.
fn resolve(
    host: &CellHost,
    name: &str,
    arity: Option<usize>,
    widened: bool,
) -> Result<String, String> {
    if widened {
        let wide_id = format!("{name}_u32");
        if let Some(m) = host.manifest(&wide_id) {
            if free_fn_arity(m).is_some_and(|a| arity.is_none_or(|want| a == want)) {
                return Ok(wide_id);
            }
        }
    }
    let hits = host.search_scored(name, 6);
    if hits.is_empty() {
        return Err(format!(
            "[E0504 unknown_call_target] no library match for `{name}`"
        ));
    }
    if let Some(want) = arity {
        for (s, m) in &hits {
            if free_fn_arity(m) == Some(want) && credible(name, &m.id, *s) {
                return Ok(m.id.clone());
            }
        }
    }
    if let Some((_, m)) = hits
        .iter()
        .find(|(s, m)| free_fn_arity(m).is_some() && credible(name, &m.id, *s))
    {
        return Ok(m.id.clone());
    }
    if hits
        .iter()
        .any(|(s, m)| credible(name, &m.id, *s) && free_fn_arity(m).is_none())
    {
        return Err(format!(
            "[E0503 wide_call] `{name}`: only state-cell matches ({}) — not inlinable; \
             needs a free-fn sibling in the library",
            hits.iter()
                .map(|(_, m)| m.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Err(format!(
        "[E0504 unknown_call_target] no credible library match for `{name}` \
         (best: {})",
        hits.iter()
            .take(3)
            .map(|(s, m)| format!("{} {:.2}", m.id, s))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// Load a library cell's source for inlining: strip the `//!` header, rename its
/// entry `fn run` to the call-site name.
fn cell_fn_source(cells_dir: &Path, id: &str, as_name: &str) -> Result<String, String> {
    let path = cells_dir.join(format!("{id}.rs"));
    let src = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let body: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//!"))
        .collect::<Vec<_>>()
        .join("\n");
    match body.find("fn run") {
        Some(i) => Ok(format!(
            "{}fn {as_name}{}",
            &body[..i],
            &body[i + "fn run".len()..]
        )),
        None => Err(format!("`{id}`: no `fn run` to rename")),
    }
}

/// Canonicalize (Full, wide default), then drive the compiler's own name resolution:
/// each `unknown call target` rejection resolves one library cell, inlines it, and
/// recompiles. No progress ⇒ a named failure.
pub fn compose(host: &CellHost, cells_dir: &Path, src: &str) -> Result<Composition, String> {
    // Width is decided by the constants (the fold-based detection), not forced:
    // the library's inlinable free-fns are u16, so a blanket-wide lane would make
    // every composed call unlinkable. When folding *does* widen, `_u32` overloads
    // are preferred and a state-cell-only match is a typed escalation.
    let canon = rustz80::canonicalize_source(
        src,
        &rustz80::CanonOptions {
            mode: rustz80::CanonMode::Full,
            hints: Vec::new(),
            wide_default: false,
        },
    )
    .map_err(|d| d.to_string())?;
    let repairs: Vec<String> = canon.repairs.iter().map(|r| r.to_string()).collect();
    let mut linked = canon.source.clone();
    let mut resolutions: Vec<Resolution> = Vec::new();
    let mut tried: HashSet<String> = HashSet::new();
    for _ in 0..LINK_BUDGET {
        match Cartridge::compile(
            &linked,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some("composed".into()),
                summary: "composed cell".into(),
                tags: vec!["composed".into()],
                ..Default::default()
            },
        ) {
            Ok(cart) => {
                return Ok(Composition {
                    cart,
                    source: linked,
                    resolutions,
                    repairs,
                    widened: canon.widened,
                })
            }
            Err(e) => {
                let Some(name) = unknown_target(&e) else {
                    return Err(e); // a real compile error, not a link cue
                };
                if !tried.insert(name.clone()) {
                    return Err(format!(
                        "link stalled: `{name}` resolved but still unknown ({e})"
                    ));
                }
                let arity = call_arity(&linked, &name);
                let id = resolve(host, &name, arity, canon.widened)?;
                let inlined = cell_fn_source(cells_dir, &id, &name)?;
                linked.push('\n');
                linked.push_str(&inlined);
                linked.push('\n');
                resolutions.push(Resolution {
                    name,
                    cell_id: id,
                    arity: arity.unwrap_or(0),
                });
            }
        }
    }
    Err(format!("link budget exhausted ({LINK_BUDGET} iterations)"))
}

/// Catalogue (by artifact hash — a re-seen composition is *retrieved*) and run one
/// composition. The answer exists only for a clean `Returned`; every other halt is a
/// named kill.
pub fn run_composed(
    host: &mut CellHost,
    comp: Composition,
    args: &[u16],
    budget: u64,
) -> Result<DerivationOutcome, String> {
    let hash = crate::facts::hex(&comp.cart.artifact_hash());
    let id = format!("composed.{}", &hash[..16]);
    let retrieved = host.manifest(&id).is_some();
    let wide_ret = comp.cart.manifest.signature.ret == "u32";
    let resolutions: Vec<(String, String)> = comp
        .resolutions
        .iter()
        .map(|r| (r.name.clone(), r.cell_id.clone()))
        .collect();
    if !retrieved {
        let mut cart = comp.cart;
        cart.manifest.id = id.clone();
        host.add(cart);
    }
    let h = host.handle_for(&id)?;
    let fast = host.run_fast(h, args, budget)?;
    let (answer, kill) = match fast.halt {
        Halt::Returned => {
            let a = if wide_ret {
                fast.regs[0] as u64 | ((fast.regs[1] as u64) << 16)
            } else {
                fast.result as u64
            };
            (Some(a), None)
        }
        Halt::Escalate(c) => (
            None,
            Some(format!(
                "escalate:{}",
                Halt::Escalate(c).escalate_reason().unwrap_or("?")
            )),
        ),
        Halt::DivByZero => (None, Some("div_by_zero".into())),
        Halt::Halted(c) => (None, Some(format!("halt:{c}"))),
        Halt::CycleBudget => (None, Some("cycle_budget".into())),
        Halt::MemoryLimit => (None, Some("memory_limit".into())),
    };
    Ok(DerivationOutcome {
        answer,
        kill,
        artifact: Some(hash),
        resolutions,
        repairs: comp.repairs,
        retrieved,
    })
}

/// The registered acceptance rule (M2.7): N-way agreement → accept (`unanimous`);
/// a strict majority → accept **and flag** (reported separately so precision can be
/// audited at both strictness levels); anything else → `escalate`. A lone derivation
/// is `single` — compose without a gate, not a gated accept.
///
/// **Zero-guard (registered amendment 2026-07-06):** an agreed answer of `0` never
/// accepts — it escalates as `degenerate_zero`. Zero is the collapse value of broken
/// derivations (a verify-not-compute else-arm); unrelated broken programs agreeing
/// on it is coincidence, not consensus. Counterfactually verified over every captured
/// campaign run: removes the only accepted-and-wrong at zero yield cost.
pub fn agreement(answers: &[Option<u64>]) -> (Option<u64>, &'static str, bool) {
    let valid: Vec<u64> = answers.iter().flatten().copied().collect();
    if answers.len() == 1 {
        if valid.first() == Some(&0) {
            return (None, "degenerate_zero", false);
        }
        return (valid.first().copied(), "single", false);
    }
    let Some(&first) = valid.first() else {
        return (None, "escalate", false);
    };
    let mut best = (first, 0usize);
    for &v in &valid {
        let n = valid.iter().filter(|&&x| x == v).count();
        if n > best.1 {
            best = (v, n);
        }
    }
    let (top, n) = best;
    if top == 0 && n * 2 > answers.len() {
        return (None, "degenerate_zero", false);
    }
    if n == answers.len() {
        (Some(top), "unanimous", false)
    } else if n * 2 > answers.len() {
        (Some(top), "majority", true)
    } else {
        (None, "escalate", false)
    }
}
