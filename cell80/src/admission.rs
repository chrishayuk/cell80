//! Per-cell admission gate (roadmap 2.2): a cell enters the library only if it survives
//! its own paraphrase + adversarial query set. Two rules from `docs/library-growth.md`,
//! enforced at ingest instead of by author discipline alone:
//!
//!   1. **no behavioural duplicates** — [`Fingerprint::agreement`] against every
//!      already-admitted cell of the **same shape**; an exact match (`1.0`, the
//!      fingerprint module's own definition of "indistinguishable on this bank")
//!      refuses, with "alias it in metadata" as the remedy. The two historical
//!      exemptions are **lifted**: the probe bank now supplies all three convention
//!      registers (so `clamp`/`min3`/`between_exclusive` no longer collapse to the
//!      same degenerate constant), and **state cells** are driven through their
//!      named scalar fields (`field i ← probe[i % 3]`, declaration order — so
//!      identical-layout duplicates, the real copy-paste risk, fingerprint
//!      identically). Comparison is guarded to the same *shape* — value cells
//!      compare against value cells of the same arity, state cells against state
//!      cells with the same scalar-field count — because cross-shape agreement is
//!      coincidence about the assignment pattern, not evidence about behaviour.
//!   2. **pay the eval tax per cell** — a candidate with zero rows in the retrieval
//!      dataset can't have "survived" a query set it doesn't have.
//!
//! Even within the fingerprintable (arity ≤ 2) class, `agreement == 1.0` means "these two
//! cells agree on every probe in [`crate::DEFAULT_PROBES`]," not "these are provably
//! identical for all `u16` inputs" — a finite bank can coincidentally agree for two cells
//! that differ elsewhere. Two real examples surfaced this way and were fixed by widening
//! the bank rather than touching the colliding cell: `luhn_check`/`is_zero` (no probe was
//! Luhn-valid) and `snap_down`/`round_to_multiple` (they agreed on the whole ten-probe bank
//! but diverge at e.g. `x=8, step=5` — fixed as a side effect of widening for a *different*
//! pair, `sign_i16`/`nonzero`, once `DEFAULT_PROBES` gained a negative-`i16`-domain value).
//! A refusal is strong evidence, not proof; a maintainer reviewing a `REFUSED` report should
//! treat it as "no probe has ever told these apart yet," and widening the probe bank is the
//! honest fix if a pair keeps false-positiving.
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
/// The comparison-guard shape: value cells compare within an arity class, state
/// cells within an **ordered field-type** class (the type codes, not just the
/// count — `(u32, u16, u32)` and `(u32, u32, u32)` cells aren't interchangeable
/// to any caller, and comparing them invites agree-on-the-narrow-domain false
/// positives: `cents_mul_qty` (qty: u16) computes identically to
/// `mul_checked_u32` on every probe a u16 can hold, while their domains differ).
#[derive(Debug, Clone, PartialEq, Eq)]
enum CellShape {
    Value(usize),
    State(Vec<u8>),
}

fn cell_shape(cart: &crate::Cartridge) -> CellShape {
    // The full wire encoding of every drivable field (code; + elem/len for arrays):
    // scalar-only cells keep exactly their pre-array shape vector (codes alone), a
    // `u16[8]` and a `u16[4]` cell land in different classes, and an array cell is
    // never compared against a scalar-only one. Buffers stay out (undrivable until
    // Phase S3 — they can't contribute to a fingerprint either).
    let mut field_tys: Vec<u8> = Vec::new();
    for (_, _, ty) in &cart.manifest.state_addrs {
        if let Some((elem, len)) = ty.array_dims() {
            field_tys.push(ty.code());
            field_tys.push(elem.code());
            field_tys.extend_from_slice(&len.to_le_bytes());
        } else if ty.capacity().is_none() {
            field_tys.push(ty.code());
        }
    }
    if field_tys.is_empty() {
        CellShape::Value(cart.manifest.signature.params.len())
    } else {
        CellShape::State(field_tys)
    }
}

pub fn admit(dir: &str, retrieval_jsonl: &Path) -> Result<AdmissionReport, String> {
    let by_id = load_retrieval_rows(retrieval_jsonl)?;

    let paths = crate::discover::discover_cell_files(dir)?;

    let mut admitted: Vec<Cartridge> = Vec::new();
    let mut fp_cache: Vec<(String, CellShape, Fingerprint)> = Vec::new();
    let mut report = AdmissionReport::default();

    for path in paths {
        let Some(cart_res) = crate::cli::library_cartridge(&path) else {
            continue;
        };
        let cart = cart_res?; // compile/load errors stay hard CLI failures, unrelated to gating
        let id = cart.manifest.id.clone();
        let mut reasons = Vec::new();

        // Every cell fingerprints now (the probe bank drives all three convention
        // registers; state cells are driven through their named scalar fields — see
        // the module doc). Comparison is guarded to the same *shape*: value-vs-value
        // at equal arity, state-vs-state at equal scalar-field count — cross-shape
        // agreement says nothing about behaviour, only about the assignment pattern.
        let shape = cell_shape(&cart);
        let fp = Fingerprint::of(&cart);

        let dup = fp_cache
            .iter()
            .find_map(|(other_id, other_shape, other_fp)| {
                if *other_shape != shape {
                    return None;
                }
                let a = fp.agreement(other_fp);
                (a >= DUPLICATE_AGREEMENT).then(|| (other_id.clone(), a))
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
            fp_cache.push((id, shape, fp));
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
    fn state_cells_are_fingerprinted_and_distinguished() {
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

    #[test]
    fn report_renders_json_and_bad_rows_reject() {
        // The JSON rendering of a duplicate refusal (the CI-able report shape),
        // and a retrieval row whose `expected` is neither string nor array.
        let dir = scratch_dir("json_refusal");
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
        let j = report.to_json().to_string();
        assert!(j.contains("behavioural_duplicate"), "{j}");
        assert!(j.contains("colliding_queries"), "{j}");

        let bad = dir.join("bad.jsonl");
        std::fs::write(
            &bad,
            "{\"id\": \"x\", \"query\": \"q\", \"expected\": 5, \"category\": \"direct\"}\n",
        )
        .unwrap();
        let err = admit(dir.to_str().unwrap(), &bad).unwrap_err();
        assert!(err.contains("must be a string or array"), "{err}");

        // The array form of `expected` parses.
        let arr = dir.join("arr.jsonl");
        std::fs::write(&arr, "{\"id\": \"x\", \"query\": \"smaller of two\", \"expected\": [\"min\", \"min2\"], \"category\": \"direct\"}\n").unwrap();
        assert!(admit(dir.to_str().unwrap(), &arr).is_ok());
    }

    #[test]
    fn identical_layout_state_duplicate_is_refused() {
        // The lifted exemption's payoff: a copy-paste state cell under a new id
        // fingerprints identically (same layout, same field-driving) and refuses.
        let dir = scratch_dir("state_dup");
        let src = "//! Manhattan distance.\n//! tags: grid\n//! entry: Pts::run\nstruct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }\nimpl Pts { fn run(&mut self) -> u16 { let dx = if self.x1 > self.x2 { self.x1 - self.x2 } else { self.x2 - self.x1 }; let dy = if self.y1 > self.y2 { self.y1 - self.y2 } else { self.y2 - self.y1 }; self.dist = dx + dy; self.dist } }";
        write_cell(&dir, "manhattan", src);
        write_cell(
            &dir,
            "taxicab",
            &src.replace("Pts", "Taxi"), // new id + struct name, same behaviour/layout
        );
        let retrieval = write_retrieval(
            &dir,
            &[
                ("m-1", "manhattan grid distance", "manhattan", "direct"),
                (
                    "t-1",
                    "taxicab distance between points",
                    "taxicab",
                    "direct",
                ),
            ],
        );
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        assert_eq!(report.admitted.len(), 1, "{:?}", report.refused);
        assert_eq!(report.refused.len(), 1);
        assert!(matches!(
            &report.refused[0].1[0],
            RefusalReason::BehaviouralDuplicate { of, .. } if of == "manhattan"
        ));
    }

    #[test]
    fn array_state_cells_admit_dedupe_and_class_by_length() {
        // The v11 array surface through the gate: an array-state cell admits, its
        // identical-layout copy refuses as a behavioural duplicate, and a
        // different window LENGTH is a different shape class (the wire encoding
        // in the shape vector) — never compared, both admit.
        let dir = scratch_dir("array_state");
        let sma = |s: &str, n: u8| {
            format!(
                "//! Trailing-{n} window sum ({s}).\n//! tags: window\n//! entry: {s}::run\n\
                 struct {s} {{ value: u16, w: [u16; {n}], head: u16, out: u16 }}\n\
                 impl {s} {{ fn run(&mut self) -> u16 {{\n\
                     self.w[self.head as usize] = self.value;\n\
                     self.head = (self.head + 1u16) % {n}u16;\n\
                     let mut s = 0u16; let mut i = 0u16;\n\
                     while i < {n}u16 {{ s = s + self.w[i as usize]; i = i + 1u16; }}\n\
                     self.out = s; self.out }} }}"
            )
        };
        write_cell(&dir, "wsum4", &sma("Wa", 4));
        write_cell(&dir, "wsum4_copy", &sma("Wb", 4)); // same layout+behaviour, new names
        write_cell(&dir, "wsum8", &sma("Wc", 8)); // different length ⇒ different class
        let retrieval = write_retrieval(
            &dir,
            &[
                ("w4-1", "sum of a trailing window of four", "wsum4", "direct"),
                ("w4c-1", "rolling four sample total", "wsum4_copy", "direct"),
                ("w8-1", "sum of a trailing window of eight", "wsum8", "direct"),
            ],
        );
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        let admitted: Vec<&str> = report.admitted.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(admitted, ["wsum4", "wsum8"], "refused: {:?}", report.refused);
        assert_eq!(report.refused.len(), 1);
        assert!(matches!(
            &report.refused[0].1[0],
            RefusalReason::BehaviouralDuplicate { of, .. } if of == "wsum4"
        ));
    }

    #[test]
    fn arity3_cells_are_distinguished_and_deduped() {
        // The third probe column retires the arity-3 exemption: clamp and min3 no
        // longer collapse to the same degenerate constant (both admit), while an
        // exact arity-3 duplicate refuses.
        let dir = scratch_dir("arity3");
        write_cell(
            &dir,
            "clamp",
            "//! Clamp x into [lo, hi].\n//! tags: bounds\nfn run(x: u16, lo: u16, hi: u16) -> u16 { let mut r = x; if x < lo { r = lo; } if x > hi { r = hi; } r }",
        );
        write_cell(
            &dir,
            "min3",
            "//! Smallest of three.\n//! tags: math\nfn run(a: u16, b: u16, c: u16) -> u16 { let mut m = a; if b < m { m = b; } if c < m { m = c; } m }",
        );
        write_cell(
            &dir,
            "min3_again",
            "//! Minimum of three values.\n//! tags: math\nfn run(x: u16, y: u16, z: u16) -> u16 { let mut m = x; if y < m { m = y; } if z < m { m = z; } m }",
        );
        let retrieval = write_retrieval(
            &dir,
            &[
                ("c-1", "clamp into bounds", "clamp", "direct"),
                ("m-1", "smallest of three", "min3", "direct"),
                ("m-2", "minimum of three values", "min3_again", "direct"),
            ],
        );
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        let admitted: Vec<&str> = report.admitted.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(admitted, ["clamp", "min3"], "{:?}", report.refused);
        assert!(matches!(
            &report.refused[0].1[0],
            RefusalReason::BehaviouralDuplicate { of, .. } if of == "min3"
        ));
    }

    #[test]
    fn shape_guard_blocks_cross_kind_comparison() {
        // A 1-arg value cell and a 1-field state cell that coincidentally agree on
        // every probe must NOT collide — different shapes never compare.
        let dir = scratch_dir("shape_guard");
        write_cell(
            &dir,
            "double",
            "//! Twice the value.\n//! tags: math\nfn run(a: u16) -> u16 { a.wrapping_mul(2u16) }",
        );
        write_cell(
            &dir,
            "double_state",
            "//! Doubler with state.\n//! tags: math\n//! entry: D::run\nstruct D { x: u16 }\nimpl D { fn run(&mut self) -> u16 { self.x.wrapping_mul(2u16) } }",
        );
        let retrieval = write_retrieval(
            &dir,
            &[
                ("d-1", "double a value", "double", "direct"),
                ("d-2", "stateful doubler", "double_state", "direct"),
            ],
        );
        let report = admit(dir.to_str().unwrap(), &retrieval).unwrap();
        assert_eq!(report.admitted.len(), 2, "{:?}", report.refused);
    }
}
