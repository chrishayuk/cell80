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

use cell80::{Cartridge, CartridgeOpts, CellConfig, CellIndex, Manifest, TfidfIndex};
use std::path::PathBuf;

/// Parse a library cell's `//!` header (summary / `tags:` / `entry:`) — mirrors the CLI's
/// `parse_meta` so the manifests here are identical to what `index <dir>` builds.
fn parse_meta(src: &str) -> (String, Vec<String>, Option<String>) {
    let (mut summary, mut tags, mut entry) = (String::new(), Vec::new(), None);
    for line in src.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("//!") {
            let rest = rest.trim();
            if let Some(t) = rest.strip_prefix("tags:") {
                tags = t
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else if let Some(e) = rest.strip_prefix("entry:") {
                entry = Some(e.trim().to_string());
            } else if summary.is_empty() {
                summary = rest.to_string();
            }
        } else if !l.is_empty() && !l.starts_with("//") {
            break; // first code line — header done
        }
    }
    (summary, tags, entry)
}

/// Compile every `cell80/cells/*.rs` into its manifest (id = file stem).
fn load_manifests() -> Vec<Manifest> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("rs"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for p in &paths {
        let src = std::fs::read_to_string(p).unwrap();
        let (summary, tags, entry) = parse_meta(&src);
        let id = p.file_stem().and_then(|s| s.to_str()).unwrap().to_string();
        match Cartridge::compile(
            &src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.clone()),
                entry,
                summary,
                tags,
            },
        ) {
            Ok(c) => out.push(c.manifest),
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
    let manifests = load_manifests();
    let rows = load_dataset();

    let mut token = CellIndex::new();
    for m in &manifests {
        token.add(m.clone());
    }
    let tfidf = TfidfIndex::build(manifests.clone());

    let token_rank = |q: &str| {
        token
            .search(q, 5)
            .into_iter()
            .map(|m| m.id.clone())
            .collect::<Vec<_>>()
    };
    let tfidf_rank = |q: &str| {
        tfidf
            .search(q, 5)
            .into_iter()
            .map(|m| m.id.clone())
            .collect::<Vec<_>>()
    };

    let cats = ["direct", "paraphrase", "adversarial"];
    println!(
        "Retrieval — {} cells, {} queries (cell-eval/datasets/retrieval.jsonl)\n",
        manifests.len(),
        rows.len()
    );
    println!("                       token-overlap (prev)       tf-idf (live)");
    println!("  category      n        P@1      hit@5            P@1      hit@5");
    println!("  --------------------------------------------------------------------");
    for cat in cats {
        let sub: Vec<&Row> = rows.iter().filter(|r| r.category == cat).collect();
        let (tp1, th5) = score(&sub, &token_rank);
        let (fp1, fh5) = score(&sub, &tfidf_rank);
        println!(
            "  {cat:<12} {:>3}      {:>5.0}%     {:>5.0}%          {:>5.0}%     {:>5.0}%",
            sub.len(),
            tp1 * 100.0,
            th5 * 100.0,
            fp1 * 100.0,
            fh5 * 100.0
        );
    }
    let all: Vec<&Row> = rows.iter().collect();
    let (tp1, th5) = score(&all, &token_rank);
    let (fp1, fh5) = score(&all, &tfidf_rank);
    println!("  --------------------------------------------------------------------");
    println!(
        "  {:<12} {:>3}      {:>5.0}%     {:>5.0}%          {:>5.0}%     {:>5.0}%",
        "overall",
        all.len(),
        tp1 * 100.0,
        th5 * 100.0,
        fp1 * 100.0,
        fh5 * 100.0
    );
    println!(
        "\n  Read: does tf-idf lift P@1 without regressing the paraphrase row? That, not the\n  \
         direct row, decides whether it should be the live default."
    );
}
