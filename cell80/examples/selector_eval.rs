//! Cell-selection experiment — does routing belong *in the cell layer* (learned + verifier),
//! and can it beat plain retrieval on the confusable tail?
//!
//! Run: `cargo run --example selector_eval -p cell80`
//!
//! Setup (honest, deterministic, no LLM): 11 real **two-argument** cells from confusable
//! families (min/max, gcd/lcm, the comparators, divides/safe_mod…). Each has one **direct**
//! query (training), one **paraphrase**, and one **adversarial** query (the held-out tail —
//! the adversarial one is phrased to token-collide with a confusable sibling). We compare, by
//! P@1 on the tail:
//!
//! * token-overlap — cell80's current `CellIndex`
//! * tf-idf — the new `TfidfIndex` (idf tokens + char-3-grams)
//! * learned slot — a frozen per-cell MLP slot (V3b/V3-CP), trained on the direct query
//! * behavioural — `rank_by_examples`: the request carries desired I/O examples, and the
//!   *verifier* picks the cell whose behaviour matches
//!
//! KILL GATE (stated up front, reported honestly — a tie is a tie): the learned slot must beat
//! tf-idf on the adversarial row to have earned anything over retrieval. With one training
//! query per cell it may well tie — the expected honest outcome, which motivates query scaling.
//! The behavioural row is the separate point: where the *request carries behaviour*, selection
//! grounded in the cells is exact even when text is a coin-flip.
//!
//! (Two-arg cells only: the fingerprint probe bank passes two arguments, so probing a 4-arg
//! cell like `chebyshev` with two args is degenerate — that's a probe-arity limitation, not a
//! method one. The confusable families that matter survive in the 2-arg subset.)

use cell80::{
    rank_by_examples, Cartridge, CartridgeOpts, CellConfig, CellIndex, Encoder, Fingerprint, Rng,
    Runner, SlotRouter, TfidfIndex, DEFAULT_CYCLES,
};

struct Spec {
    id: &'static str,
    src: &'static str,
    direct: &'static str,
    paraphrase: &'static str,
    adversarial: &'static str,
}

fn fixture() -> Vec<Spec> {
    macro_rules! s {
        ($id:literal, $file:literal, $d:literal, $p:literal, $a:literal) => {
            Spec {
                id: $id,
                src: include_str!($file),
                direct: $d,
                paraphrase: $p,
                adversarial: $a,
            }
        };
    }
    vec![
        s!(
            "min",
            "../cells/ranking-stats/min.rs",
            "the smaller of two numbers",
            "which of two values is lower",
            "pick the lesser of two numbers, not the larger"
        ),
        s!(
            "max",
            "../cells/ranking-stats/max.rs",
            "the larger of two numbers",
            "which of two values is higher",
            "the bigger of two numbers, not the smaller one"
        ),
        s!(
            "gcd",
            "../cells/number-theory/gcd.rs",
            "the greatest common divisor of two numbers",
            "the largest integer that divides both numbers evenly",
            "the common factor shared by two numbers"
        ),
        s!(
            "lcm",
            "../cells/number-theory/lcm.rs",
            "the least common multiple of two numbers",
            "the smallest number both values divide into",
            "the common multiple of two numbers"
        ),
        s!(
            "divides",
            "../cells/number-theory/divides.rs",
            "does the first number divide evenly by the second",
            "is one number a whole multiple of another",
            "can two numbers be divided with no remainder"
        ),
        s!(
            "abs_diff",
            "../cells/distance/abs_diff.rs",
            "the absolute difference between two numbers",
            "how far apart two values are",
            "the distance between two numbers on a line"
        ),
        s!(
            "eq",
            "../cells/predicates/eq.rs",
            "are two numbers equal",
            "do two values match exactly",
            "compare two numbers for equality, not order"
        ),
        s!(
            "is_lt",
            "../cells/predicates/is_lt.rs",
            "is the first number less than the second",
            "does one value come before another in order",
            "compare two numbers, is the first the smaller"
        ),
        s!(
            "is_gt",
            "../cells/predicates/is_gt.rs",
            "is the first number greater than the second",
            "does one value exceed another",
            "compare two numbers, is the first the larger"
        ),
        s!(
            "avg2",
            "../cells/safe-arith/avg2.rs",
            "the average of two numbers",
            "add two numbers and halve the result",
            "the value in the middle of two numbers"
        ),
        s!(
            "safe_mod",
            "../cells/safe-arith/safe_mod.rs",
            "the remainder after dividing two numbers",
            "what is left over when one number is divided by another",
            "divide two numbers and give the leftover, not the quotient"
        ),
    ]
}

/// Pull `summary` (first `//!` line) and `tags:` out of a cell's source header — the same
/// metadata cell80's real index sees, so the baselines aren't crippled.
fn parse_meta(src: &str) -> (String, Vec<String>) {
    let mut summary = String::new();
    let mut tags = Vec::new();
    for line in src.lines() {
        let Some(rest) = line.trim().strip_prefix("//!") else {
            continue;
        };
        let rest = rest.trim();
        if let Some(t) = rest.strip_prefix("tags:") {
            tags = t
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if summary.is_empty() && !rest.is_empty() {
            summary = rest.to_string();
        }
    }
    (summary, tags)
}

fn compile_all(specs: &[Spec]) -> Vec<Cartridge> {
    specs
        .iter()
        .map(|s| {
            let (summary, tags) = parse_meta(s.src);
            Cartridge::compile(
                s.src,
                CellConfig::sandboxed(),
                CartridgeOpts {
                    id: Some(s.id.into()),
                    summary,
                    tags,
                    ..Default::default()
                },
            )
            .unwrap_or_else(|e| panic!("compile {}: {e}", s.id))
        })
        .collect()
}

/// P@1: fraction of test cases whose top-ranked id equals the expected id.
fn p_at_1(tests: &[(&str, &str)], top: &dyn Fn(&str) -> Option<String>) -> f32 {
    let hit = tests
        .iter()
        .filter(|(q, want)| top(q).as_deref() == Some(*want))
        .count();
    hit as f32 / tests.len().max(1) as f32
}

fn main() {
    let specs = fixture();
    let carts = compile_all(&specs);
    let manifests: Vec<_> = carts.iter().map(|c| c.manifest.clone()).collect();
    let fps: Vec<Fingerprint> = carts.iter().map(Fingerprint::of).collect();

    // Encoder fit over manifests + the (training-visible) direct queries.
    let corpus: Vec<String> = manifests
        .iter()
        .map(|m| format!("{} {} {}", m.id, m.summary, m.tags.join(" ")))
        .chain(specs.iter().map(|s| s.direct.to_string()))
        .collect();
    let enc = Encoder::fit(256, &corpus);

    // Baselines.
    let mut token = CellIndex::new();
    for m in &manifests {
        token.add(m.clone());
    }
    let tfidf = TfidfIndex::build(manifests.clone());

    // Learned slot-router: one frozen slot per cell. Positives = its direct query AND its
    // manifest text (the same words the baselines index, so the comparison is fair); negatives
    // = every other cell's direct query + manifest (confusable siblings are the hard ones).
    let man_text: Vec<String> = manifests
        .iter()
        .map(|m| format!("{} {} {}", m.id, m.summary, m.tags.join(" ")))
        .collect();
    let mut rng = Rng::new(0xC0FFEE);
    let mut router = SlotRouter::new(256, 16);
    for (i, s) in specs.iter().enumerate() {
        let pos = vec![enc.encode(s.direct), enc.encode(&man_text[i])];
        let negs: Vec<Vec<f32>> = specs
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .flat_map(|(j, t)| [enc.encode(t.direct), enc.encode(&man_text[j])])
            .collect();
        router.add_cell(s.id, Some(fps[i].clone()), &pos, &negs, 300, 0.2, &mut rng);
    }

    // Test splits. `direct` is a sanity check (the cell's own words — every method should
    // ace it; if not, the harness is broken). `paraphrase`/`adversarial` are the held-out tail.
    let direct: Vec<(&str, &str)> = specs.iter().map(|s| (s.direct, s.id)).collect();
    let para: Vec<(&str, &str)> = specs.iter().map(|s| (s.paraphrase, s.id)).collect();
    let adv: Vec<(&str, &str)> = specs.iter().map(|s| (s.adversarial, s.id)).collect();
    let token_top = |q: &str| token.search(q, 1).first().map(|m| m.id.clone());
    let tfidf_top = |q: &str| tfidf.search(q, 1).first().map(|m| m.id.clone());
    let learned_top = |q: &str| router.top(&enc.encode(q)).map(str::to_string);

    let rows = [
        (
            "token-overlap (CellIndex)",
            p_at_1(&direct, &token_top),
            p_at_1(&para, &token_top),
            p_at_1(&adv, &token_top),
        ),
        (
            "tf-idf (TfidfIndex)",
            p_at_1(&direct, &tfidf_top),
            p_at_1(&para, &tfidf_top),
            p_at_1(&adv, &tfidf_top),
        ),
        (
            "learned slot-router",
            p_at_1(&direct, &learned_top),
            p_at_1(&para, &learned_top),
            p_at_1(&adv, &learned_top),
        ),
    ];

    println!(
        "\nCell selection — P@1 ({} confusable 2-arg cells)\n",
        specs.len()
    );
    println!(
        "  {:<28} {:>8} {:>11} {:>13}",
        "method (text query)", "direct", "paraphrase", "adversarial"
    );
    println!("  {}", "-".repeat(64));
    for (name, d, p, a) in &rows {
        println!(
            "  {:<28} {:>7.0}% {:>10.0}% {:>12.0}%",
            name,
            100.0 * d,
            100.0 * p,
            100.0 * a
        );
    }

    // Behavioural routing — phrasing-independent: the request carries desired I/O examples
    // (generated by running the true cell on a clean positive probe bank), and the *verifier*
    // selects by behaviour. Models the realistic case: an agent can state desired I/O even
    // when its wording is ambiguous. Several probes break single-input ties.
    let probes: &[[u16; 2]] = &[[3, 7], [7, 3], [10, 3], [100, 4], [12, 8], [9, 6], [8, 8]];
    let behavioural_top = |want: &str| -> Option<String> {
        let cart = carts.iter().find(|c| c.manifest.id == want)?;
        let mut r = Runner::new(
            cart.z80()
                .expect("the selector eval probes z80-cell bodies"),
        );
        let mut examples = Vec::new();
        for p in probes {
            let out = r.run(Some(&cart.manifest.entry), p, DEFAULT_CYCLES).ok()?;
            if !out.returned {
                return None;
            }
            examples.push((p.to_vec(), out.result));
        }
        rank_by_examples(&carts, &examples, DEFAULT_CYCLES)
            .first()
            .map(|m| m.id.clone())
    };
    let beh_hits = specs
        .iter()
        .filter(|s| behavioural_top(s.id).as_deref() == Some(s.id))
        .count();
    let beh = beh_hits as f32 / specs.len() as f32;

    // Kill-gate verdict (honest).
    let (tf_a, lrn_a) = (rows[1].3, rows[2].3);
    let margin = lrn_a - tf_a;
    let verdict = if margin > 0.02 {
        "learned WINS — beats retrieval on the confusable tail"
    } else if margin < -0.02 {
        "learned LOSES to retrieval — honest null at one query/cell"
    } else {
        "TIE — not demonstrated at one query/cell (needs query scaling); reported honestly"
    };
    println!("\n  KILL GATE — learned slot vs tf-idf on adversarial:");
    println!(
        "    learned {:.0}% vs tf-idf {:.0}%  ->  {verdict}",
        100.0 * lrn_a,
        100.0 * tf_a
    );
    println!(
        "\n  Behavioural route-by-I/O (phrasing-independent): {:.0}% of cells ({}/{})",
        100.0 * beh,
        beh_hits,
        specs.len()
    );
    println!("  Where text routing is a coin-flip, a desired I/O example routes by the free");
    println!("  verifier — selection grounded in the cells, not the wording or the LLM.");
}
