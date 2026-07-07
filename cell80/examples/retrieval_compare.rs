//! **Token-overlap vs TF-IDF on the real library** — the banked retrieval baseline.
//!
//! [`TfidfIndex`] (IDF-weighted word + char-3-gram cosine) is now the *live* search path
//! (`CellHost` / CLI / MCP); [`CellIndex`] (token overlap) is the previous default, kept as
//! the comparison baseline. This is the honest number on the *authoritative* set — the full
//! `cell80/cells` library indexed against the `cell-eval` retrieval dataset, split by
//! category — that justified the swap: TF-IDF lifts **direct** and **paraphrase** P@1 a few
//! points and is neutral on hit@5, at the cost of a ~1-query **adversarial** P@1 wobble.
//! Re-run it as a regression guard before changing the index or the library; the lever for
//! the still-coin-flip **paraphrase** row is the type-led index, not this.
//!
//! Deterministic, no model. Run: `cargo run --release --example retrieval_compare -p cell80`.

use cell80::{Cartridge, CartridgeOpts, CellConfig, CellIndex, Manifest, TfidfIndex, TypeLedIndex};
use std::path::PathBuf;

/// A named ranking method: a label and a `query → ranked-cell-ids` function.
type Method<'a> = (&'a str, &'a dyn Fn(&str) -> Vec<String>);

/// Parse a library cell's `//!` header (summary / `tags:` / `entry:`) — mirrors the CLI's
/// `parse_meta` so the manifests here are identical to what `index <dir>` builds.
fn parse_meta(src: &str) -> (String, Vec<String>, Option<String>, Vec<String>) {
    let (mut summary, mut tags, mut entry, mut limits) =
        (String::new(), Vec::new(), None, Vec::new());
    let csv = |s: &str| -> Vec<String> {
        s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    for line in src.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("//!") {
            let rest = rest.trim();
            if let Some(t) = rest.strip_prefix("tags:") {
                tags = csv(t);
            } else if let Some(e) = rest.strip_prefix("entry:") {
                entry = Some(e.trim().to_string());
            } else if let Some(m) = rest.strip_prefix("limits:") {
                limits = csv(m);
            } else if summary.is_empty() {
                summary = rest.to_string();
            }
        } else if !l.is_empty() && !l.starts_with("//") {
            break; // first code line — header done
        }
    }
    (summary, tags, entry, limits)
}

/// Compile every `cell80/cells/*.rs` into a cartridge (id = file stem). Cartridges, not just
/// manifests, because the type-led index runs each cell to learn its behavioural shape.
fn load_carts() -> Vec<Cartridge> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
    let paths: Vec<_> = cell80::discover_cell_files(dir.to_str().unwrap())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("rs"))
        .collect();
    let mut out = Vec::new();
    for p in &paths {
        let src = std::fs::read_to_string(p).unwrap();
        let (summary, tags, entry, limits) = parse_meta(&src);
        let id = p.file_stem().and_then(|s| s.to_str()).unwrap().to_string();
        match Cartridge::compile(
            &src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.clone()),
                entry,
                summary,
                tags,
                limits,
                scale: None,
                ..Default::default()
            },
        ) {
            Ok(c) => out.push(c),
            Err(e) => eprintln!("skip {id}: {e}"),
        }
    }
    out
}

struct Row {
    query: String,
    expected: Vec<String>,
    category: String,
}

/// Load the `cell-eval` retrieval dataset (`{id, query, expected, category}` per line;
/// `expected` is one id or a list; `#` lines are comments).
fn load_dataset() -> Vec<Row> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cell-eval/datasets/retrieval.jsonl");
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let mut rows = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line).expect("bad dataset row");
        let expected = match &v["expected"] {
            serde_json::Value::String(s) => vec![s.clone()],
            serde_json::Value::Array(a) => a
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect(),
            _ => vec![],
        };
        rows.push(Row {
            query: v["query"].as_str().unwrap_or_default().to_string(),
            expected,
            category: v["category"].as_str().unwrap_or("direct").to_string(),
        });
    }
    rows
}

/// A cell's structural shape — what a type-led signal could discriminate on: value cell by
/// arity (`v2`, `v3`), state cell by field count (`s4`). Two cells with the same shape are
/// invisible to any arity/predicate-style structural signal.
fn shape(m: &Manifest) -> String {
    if m.signature.state.is_empty() {
        format!("v{}", m.signature.params.len())
    } else {
        format!("s{}", m.signature.state.len())
    }
}

/// P@1 and hit@5 over the rows in one category, for a ranking fn.
fn score(rows: &[&Row], rank: &dyn Fn(&str) -> Vec<String>) -> (f32, f32) {
    if rows.is_empty() {
        return (0.0, 0.0);
    }
    let (mut p1, mut h5) = (0u32, 0u32);
    for r in rows {
        let top = rank(&r.query);
        if top
            .first()
            .is_some_and(|id| r.expected.iter().any(|e| e == id))
        {
            p1 += 1;
        }
        if top
            .iter()
            .take(5)
            .any(|id| r.expected.iter().any(|e| e == id))
        {
            h5 += 1;
        }
    }
    let n = rows.len() as f32;
    (p1 as f32 / n, h5 as f32 / n)
}

fn main() {
    let carts = load_carts();
    let manifests: Vec<Manifest> = carts.iter().map(|c| c.manifest.clone()).collect();
    let rows = load_dataset();

    let mut token = CellIndex::new();
    for m in &manifests {
        token.add(m.clone());
    }
    let tfidf = TfidfIndex::build(manifests.clone());
    let typed = TypeLedIndex::build(carts);

    let rank = |hits: Vec<&Manifest>| hits.into_iter().map(|m| m.id.clone()).collect::<Vec<_>>();
    let methods: [Method; 3] = [
        ("token-overlap", &|q| rank(token.search(q, 5))),
        ("tf-idf (live)", &|q| rank(tfidf.search(q, 5))),
        ("type-led", &|q| rank(typed.search(q, 5))),
    ];

    let cats = ["direct", "paraphrase", "adversarial"];
    let bucket = |cat: &str| -> Vec<&Row> { rows.iter().filter(|r| r.category == cat).collect() };

    println!(
        "Retrieval P@1 / hit@5 — {} cells, {} queries (cell-eval/datasets/retrieval.jsonl)\n",
        manifests.len(),
        rows.len()
    );
    print!("  {:<15}", "method");
    for cat in cats {
        print!("{:>16}", format!("{cat} (n={})", bucket(cat).len()));
    }
    println!("{:>16}", "overall");
    println!("  {}", "-".repeat(15 + 16 * 4));
    for (name, rank_fn) in methods {
        print!("  {name:<15}");
        for cat in cats {
            let (p1, h5) = score(&bucket(cat), rank_fn);
            print!("{:>16}", format!("{:.0}% / {:.0}%", p1 * 100.0, h5 * 100.0));
        }
        let (p1, h5) = score(&rows.iter().collect::<Vec<_>>(), rank_fn);
        println!("{:>16}", format!("{:.0}% / {:.0}%", p1 * 100.0, h5 * 100.0));
    }
    println!(
        "\n  Read: the paraphrase column is the headline. type-led re-ranks tf-idf by a\n  \
         behavioural predicate signal with a corpus-learned (not hardcoded) query intent —\n  \
         the lift it adds there is the case for it over plain text."
    );

    // Why structural re-ranking has limited headroom: of tf-idf's wrong top-1s on the hard
    // (non-direct) queries, how many name a cell of the *same* shape as the right answer? A
    // structural (arity/predicate) signal can only ever rescue the different-shape misses.
    let by_id: std::collections::HashMap<&str, &Manifest> =
        manifests.iter().map(|m| (m.id.as_str(), m)).collect();
    let (mut same, mut diff) = (0u32, 0u32);
    for r in rows.iter().filter(|r| r.category != "direct") {
        let got = tfidf.search(&r.query, 1);
        let got_id = got.first().map(|m| m.id.as_str());
        if got_id.is_some_and(|g| r.expected.iter().any(|e| e == g)) {
            continue; // hit
        }
        let exp_shape = r
            .expected
            .iter()
            .filter_map(|e| by_id.get(e.as_str()))
            .map(|m| shape(m))
            .next();
        let got_shape = got_id.and_then(|g| by_id.get(g)).map(|m| shape(m));
        match (exp_shape, got_shape) {
            (Some(e), Some(g)) if e == g => same += 1,
            (Some(_), Some(_)) => diff += 1,
            _ => {}
        }
    }
    println!(
        "\n  tf-idf hard (paraphrase+adversarial) top-1 misses: same-shape {same} / \
         different-shape {diff}.\n  A structural signal can only rescue the different-shape \
         ones; the same-shape sibling misses need behaviour (I/O examples) or better semantics."
    );
}
