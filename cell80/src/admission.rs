//! Per-cell admission gate (roadmap 2.2): a cell enters the library only if it survives
//! its own paraphrase + adversarial query set. Two rules from `docs/library-growth.md`,
//! enforced at ingest instead of by author discipline alone:
//!
//!   1. **no behavioural duplicates** — [`Fingerprint::agreement`] against every
//!      already-admitted cell of comparable arity; an exact match (`1.0`, the fingerprint
//!      module's own definition of "indistinguishable on this bank") refuses, with "alias
//!      it in metadata" as the remedy. Two classes of cell are exempt because
//!      [`Fingerprint::compute`] only ever drives the plain **2-register** calling
//!      convention: **state cells** (entry takes no scalar params — its fields live at
//!      [`STATE_BASE`](crate::STATE_BASE)) and **3-argument free-fn cells** (the calling
//!      convention allows up to 3: `weighted_sum`, `clamp`, `min3`, ...). Verified against
//!      the real library: every arity-3 cell's unset third register silently
//!      defaults, collapsing many of them to the *same* degenerate constant output and
//!      producing spurious matches against unrelated cells (`clamp` vs `between_exclusive`,
//!      `min3` vs `between_exclusive`, ...). Extending `Fingerprint` with a wider probe
//!      bank (and driving named state fields) would lift both exemptions; not built here.
//!   2. **pay the eval tax per cell** — a candidate with zero rows in the retrieval
//!      dataset can't have "survived" a query set it doesn't have.
//!
//! Even within the fingerprintable (arity ≤ 2) class, `agreement == 1.0` means "these two
//! cells agree on every probe in [`crate::DEFAULT_PROBES`]," not "these are provably
//! identical for all `u16` inputs" — ten probes can coincidentally agree for two cells that
//! differ elsewhere (e.g. `snap_down`/`round_to_multiple` agree on the whole default probe
//! bank but diverge at `x=8, step=5`). A refusal is strong evidence, not proof; a maintainer
//! reviewing a `REFUSED` report should treat it as "no probe has ever told these apart yet,"
//! and widening the probe bank is the honest fix if a pair keeps false-positiving.
//!
//! **Query collision is corroborating evidence on (1), not an independent gate.** The DoD
//! names it as "queries collide with an existing cell's fingerprint" — but treating "this
//! candidate's own query ranks a *different* cell #1 against the whole library" as its own
//! hard rule would refuse most legitimate new family members: plain lexical search already
//! only reaches paraphrase P@1 ≈ 0.45 across the accepted 98-cell library (see
//! `docs/library-growth.md`), so roughly half of *today's own admitted* paraphrase rows
//! don't rank their cell #1 once the whole confusable family exists. So instead: when a
//! candidate *is* refused as a behavioural duplicate, the report additionally names which
//! of its own queries also currently resolve to the cell it duplicates — the literal query
//! evidence, attached to the one signal that can gate hard without contradicting the
//! library's own accepted behaviour.

use super::{Cartridge, Fingerprint, Manifest, TfidfIndex};
use std::collections::HashMap;
use std::path::Path;

/// Exact fingerprint agreement counts as a behavioural duplicate. Not a softer band: real
/// confusable-but-distinct siblings sit well under this (`min`/`max` agree on only 4/11 of
/// [`crate::DEFAULT_PROBES`]), so `1.0` is the only value that can't false-positive a
/// genuine new family member.
pub const DUPLICATE_AGREEMENT: f32 = 1.0;

/// One retrieval-eval row scoped to a single candidate id (its `expected` named this id).
#[derive(Debug, Clone)]
pub struct RetrievalRow {
    pub case_id: String,
    pub query: String,
    pub category: String,
}

/// Why a candidate was refused admission.
#[derive(Debug, Clone)]
pub enum RefusalReason {
    /// Behaviourally indistinguishable from an already-admitted cell on `DEFAULT_PROBES`.
    BehaviouralDuplicate {
        /// The id of the already-admitted cell this candidate duplicates.
        of: String,
        agreement: f32,
        /// This candidate's own retrieval rows that *also* currently rank `of` #1 — the
        /// query-collision evidence, attached as corroboration (see the module doc).
        colliding_queries: Vec<(String, String, String)>, // (case_id, query, category)
    },
    /// No retrieval-dataset rows found for this candidate id.
    NoRetrievalRows,
}

/// The result of gating a directory: admitted manifests and refused ones with reasons, in
/// ingest (sorted-path) order.
#[derive(Debug, Default)]
pub struct AdmissionReport {
    pub admitted: Vec<Manifest>,
    pub refused: Vec<(Manifest, Vec<RefusalReason>)>,
}

impl AdmissionReport {
    /// `{admitted: [id,...], refused: [{id, reasons: [...]}]}`.
    pub fn to_json(&self) -> String {
        use serde_json::json;
        let reason_json = |r: &RefusalReason| match r {
            RefusalReason::BehaviouralDuplicate {
                of,
                agreement,
                colliding_queries,
            } => json!({
                "kind": "behavioural_duplicate",
                "of": of,
                "agreement": agreement,
                "colliding_queries": colliding_queries.iter().map(|(case_id, query, category)| json!({
                    "case_id": case_id, "query": query, "category": category,
                })).collect::<Vec<_>>(),
            }),
            RefusalReason::NoRetrievalRows => json!({ "kind": "no_retrieval_rows" }),
        };
        json!({
            "admitted": self.admitted.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
            "refused": self.refused.iter().map(|(m, reasons)| json!({
                "id": m.id,
                "reasons": reasons.iter().map(reason_json).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
        .to_string()
    }
}

/// Load a retrieval-dataset JSONL, grouping `(id, query, category)` rows by every id their
/// `expected` field names (a string or an array of strings). Blank and `#`-comment lines
/// are skipped, mirroring `cell_eval.datasets.load_jsonl`.
fn load_retrieval_rows(path: &Path) -> Result<HashMap<String, Vec<RetrievalRow>>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut by_id: HashMap<String, Vec<RetrievalRow>> = HashMap::new();
    for (i, line) in text.lines().enumerate() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(s)
            .map_err(|e| format!("{}:{}: bad JSON: {e}", path.display(), i + 1))?;
        let case_id = v
            .get("id")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let query = v
            .get("query")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let category = v
            .get("category")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let expected = v
            .get("expected")
            .ok_or_else(|| format!("{}:{}: missing `expected`", path.display(), i + 1))?;
        let ids: Vec<String> = if let Some(s) = expected.as_str() {
            vec![s.to_string()]
        } else if let Some(arr) = expected.as_array() {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        } else {
            return Err(format!(
                "{}:{}: `expected` must be a string or array of strings",
                path.display(),
                i + 1
            ));
        };
        for id in ids {
            by_id.entry(id).or_default().push(RetrievalRow {
                case_id: case_id.clone(),
                query: query.clone(),
                category: category.clone(),
            });
        }
    }
    Ok(by_id)
}

/// Gate every cell in `dir` (same `.rs`/`.cell` sorted-path walk `cmd_index` uses) against
/// `retrieval_jsonl`'s query set, admitting each candidate against the already-admitted set
/// built up so far (so of two behavioural duplicates, the first in sorted order is admitted
/// and the second is refused).
pub fn admit(dir: &str, retrieval_jsonl: &Path) -> Result<AdmissionReport, String> {
    let by_id = load_retrieval_rows(retrieval_jsonl)?;

    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{dir}: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    paths.sort();

    let mut admitted: Vec<Cartridge> = Vec::new();
    let mut fp_cache: Vec<(String, Fingerprint)> = Vec::new(); // free-fn cells only
    let mut report = AdmissionReport::default();

    for path in paths {
        let Some(cart_res) = crate::cli::library_cartridge(&path) else {
            continue;
        };
        let cart = cart_res?; // compile/load errors stay hard CLI failures, unrelated to gating
        let id = cart.manifest.id.clone();
        let mut reasons = Vec::new();

        // `Fingerprint::DEFAULT_PROBES` supplies only two scalar inputs per probe, so a
        // state-cell entry (no scalar params — see the module doc) or a free-fn cell that
        // takes 3 args (the calling convention allows up to 3: `weighted_sum`, `clamp`,
        // `min3`, ...) has its unset register silently defaulted. Verified against the real
        // library: every arity-3 cell degenerates to the same constant under this probing
        // and spuriously "agrees" with unrelated cells (e.g. `clamp` and `between_exclusive`
        // both collapse to a constant once their third argument defaults away) — so only
        // cells with at most 2 scalar params are safe to fingerprint-compare.
        let is_state_cell = !cart.manifest.state_addrs.is_empty();
        let arity = cart.manifest.signature.params.len();
        let fingerprintable = !is_state_cell && arity <= 2;
        let fp = fingerprintable.then(|| Fingerprint::of(&cart));

        let dup = fp.as_ref().and_then(|fp| {
            fp_cache.iter().find_map(|(other_id, other_fp)| {
                let a = fp.agreement(other_fp);
                (a >= DUPLICATE_AGREEMENT).then(|| (other_id.clone(), a))
            })
        });

        let rows = by_id.get(&id).cloned().unwrap_or_default();
        if rows.is_empty() {
            reasons.push(RefusalReason::NoRetrievalRows);
        }
        if let Some((of, agreement)) = dup {
            let colliding_queries = if rows.is_empty() {
                Vec::new()
            } else {
                let mut manifests: Vec<Manifest> =
                    admitted.iter().map(|c| c.manifest.clone()).collect();
                manifests.push(cart.manifest.clone());
                let idx = TfidfIndex::build(manifests);
                rows.iter()
                    .filter(|r| {
                        idx.search(&r.query, 1)
                            .first()
                            .map(|m| m.id == of)
                            .unwrap_or(false)
                    })
                    .map(|r| (r.case_id.clone(), r.query.clone(), r.category.clone()))
                    .collect()
            };
            reasons.push(RefusalReason::BehaviouralDuplicate {
                of,
                agreement,
                colliding_queries,
            });
        }

        if reasons.is_empty() {
            if let Some(fp) = fp {
                fp_cache.push((id, fp));
            }
            report.admitted.push(cart.manifest.clone());
            admitted.push(cart);
        } else {
            report.refused.push((cart.manifest.clone(), reasons));
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_cell(dir: &Path, id: &str, src: &str) {
        std::fs::write(dir.join(format!("{id}.rs")), src).unwrap();
    }

    fn write_retrieval(dir: &Path, rows: &[(&str, &str, &str, &str)]) -> std::path::PathBuf {
        let path = dir.join("retrieval.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        for (id, query, expected, category) in rows {
            writeln!(
                f,
                r#"{{"id": "{id}", "query": "{query}", "expected": "{expected}", "category": "{category}"}}"#
            )
            .unwrap();
        }
        path
    }

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cell80_admission_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn confusable_but_distinct_siblings_are_admitted() {
        let dir = scratch_dir("confusable");
        write_cell(
            &dir,
            "min",
            "//! Smaller of two values.\n//! tags: math\nfn run(a: u16, b: u16) -> u16 { let mut m = a; if b < a { m = b; } m }",
        );
        write_cell(
            &dir,
            "max",
            "//! Larger of two values.\n//! tags: math\nfn run(a: u16, b: u16) -> u16 { let mut m = a; if b > a { m = b; } m }",
        );
        let retrieval = write_retrieval(
            &dir,
            &[
                ("min-1", "minimum of two values", "min", "direct"),
                ("max-1", "maximum of two values", "max", "direct"),
            ],
        );
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        assert_eq!(report.admitted.len(), 2, "{:?}", report.refused);
        assert!(report.refused.is_empty());
    }

    #[test]
    fn exact_behavioural_duplicate_is_refused() {
        let dir = scratch_dir("duplicate");
        // "min" and "min2" are byte-identical in behaviour under a different id.
        let src = "//! Smaller of two values.\n//! tags: math\nfn run(a: u16, b: u16) -> u16 { let mut m = a; if b < a { m = b; } m }";
        write_cell(&dir, "min", src);
        write_cell(&dir, "min2", src);
        let retrieval = write_retrieval(
            &dir,
            &[
                ("min-1", "minimum of two values", "min", "direct"),
                ("min2-1", "the smaller of two numbers", "min2", "paraphrase"),
            ],
        );
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        assert_eq!(report.admitted.len(), 1);
        assert_eq!(report.admitted[0].id, "min"); // sorted-path order: min before min2
        assert_eq!(report.refused.len(), 1);
        let (m, reasons) = &report.refused[0];
        assert_eq!(m.id, "min2");
        match &reasons[0] {
            RefusalReason::BehaviouralDuplicate { of, agreement, .. } => {
                assert_eq!(of, "min");
                assert_eq!(*agreement, 1.0);
            }
            other => panic!("expected BehaviouralDuplicate, got {other:?}"),
        }
    }

    #[test]
    fn missing_retrieval_rows_refuses() {
        let dir = scratch_dir("norows");
        write_cell(
            &dir,
            "square",
            "//! Square a value.\n//! tags: math\nfn run(a: u16) -> u16 { a * a }",
        );
        let retrieval = write_retrieval(&dir, &[]);
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        assert!(report.admitted.is_empty());
        assert_eq!(report.refused.len(), 1);
        assert!(matches!(
            report.refused[0].1[0],
            RefusalReason::NoRetrievalRows
        ));
    }

    #[test]
    fn state_cells_are_exempt_from_fingerprint_check() {
        let dir = scratch_dir("state");
        write_cell(
            &dir,
            "manhattan",
            "//! Manhattan distance.\n//! tags: grid\n//! entry: Pts::run\nstruct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }\nimpl Pts { fn run(&mut self) -> u16 { let dx = if self.x1 > self.x2 { self.x1 - self.x2 } else { self.x2 - self.x1 }; let dy = if self.y1 > self.y2 { self.y1 - self.y2 } else { self.y2 - self.y1 }; self.dist = dx + dy; self.dist } }",
        );
        write_cell(
            &dir,
            "chebyshev",
            "//! Chebyshev distance.\n//! tags: grid\n//! entry: Cheb::run\nstruct Cheb { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }\nimpl Cheb { fn run(&mut self) -> u16 { let dx = if self.x1 > self.x2 { self.x1 - self.x2 } else { self.x2 - self.x1 }; let dy = if self.y1 > self.y2 { self.y1 - self.y2 } else { self.y2 - self.y1 }; let d = if dx > dy { dx } else { dy }; self.dist = d; self.dist } }",
        );
        let retrieval = write_retrieval(
            &dir,
            &[
                (
                    "manhattan-1",
                    "manhattan grid distance",
                    "manhattan",
                    "direct",
                ),
                (
                    "chebyshev-1",
                    "chebyshev grid distance",
                    "chebyshev",
                    "direct",
                ),
            ],
        );
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        assert_eq!(report.admitted.len(), 2, "{:?}", report.refused);
        assert!(report.refused.is_empty());
    }

    #[test]
    fn colliding_queries_are_populated_for_a_duplicate_with_rows() {
        let dir = scratch_dir("colliding");
        let src = "//! Smaller of two values.\n//! tags: math\nfn run(a: u16, b: u16) -> u16 { let mut m = a; if b < a { m = b; } m }";
        write_cell(&dir, "min", src);
        write_cell(&dir, "min2", src);
        let retrieval = write_retrieval(
            &dir,
            &[
                ("min-1", "minimum of two values", "min", "direct"),
                ("min2-1", "minimum of two values", "min2", "direct"),
            ],
        );
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        let (_, reasons) = &report.refused[0];
        match &reasons[0] {
            RefusalReason::BehaviouralDuplicate {
                colliding_queries, ..
            } => assert!(
                !colliding_queries.is_empty(),
                "identical query text must collide"
            ),
            other => panic!("expected BehaviouralDuplicate, got {other:?}"),
        }
    }

    #[test]
    fn to_json_is_well_formed() {
        let dir = scratch_dir("json");
        write_cell(
            &dir,
            "square",
            "//! Square a value.\n//! tags: math\nfn run(a: u16) -> u16 { a * a }",
        );
        let retrieval = write_retrieval(&dir, &[("sq-1", "square a value", "square", "direct")]);
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        let v: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
        assert_eq!(v["admitted"][0], "square");
        assert!(v["refused"].as_array().unwrap().is_empty());
    }
}
