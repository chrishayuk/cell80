//! Runs the protocol pre-registered in `../evolved-cells-preregistration.md`. For each fixed
//! arity-1 target: search for a chain of real, existing stdlib cells (`cell80::synthesize` —
//! A* — for every target; also `cell_synth_evolve::{evolve, mcts, portfolio}` — the *actual*
//! GA/MCTS code from `cell-synth-evolve`, reused via its lib, not duplicated — for the
//! deliberately harder targets, `Target::harder`), validate the chain over the *entire* u16
//! domain (not just the probes used to search), hand-compose it into one candidate cell
//! source (general codegen — parses each op's real source text and substitutes/renames,
//! regex-based rather than a full `syn` AST transform; see `ParsedCell`'s doc comment for why
//! that's a stated limit, not an oversight), and check that candidate against the real
//! admission-gate mechanism (`cell80::{Fingerprint, DEFAULT_PROBES}`) against every cell
//! currently in `cell80/cells/*.rs` — not a reimplementation of the gate, the actual
//! fingerprint code.
//!
//! The pre-registration's original "smooth targets, A* suffices, skip GA/MCTS" scope
//! reduction turned out not to hold in general: `mystery_bits_2` (a deliberately deep,
//! Hamming-deceptive target over a broadened real-cell op pool) breaks A* outright — no chain
//! found within budget — while GA and MCTS both find one reliably. See
//! `../evolved-cells-findings.md` for the full account.
use cell80::{synthesize, Cartridge, CartridgeOpts, CellConfig, Fingerprint, Op};
use cell_synth_evolve::{evolve, mcts, portfolio, summarize};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const DUPLICATE_AGREEMENT: f32 = 1.0; // mirrors cell80/src/admission.rs's own threshold
const BUDGET: usize = 500_000;

fn cell_source(cells_dir: &Path, name: &str) -> String {
    fs::read_to_string(cells_dir.join(format!("{name}.rs")))
        .unwrap_or_else(|e| panic!("reading {name}: {e}"))
}

fn compile(id: &str, src: &str) -> Cartridge {
    Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(id.into()),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("compiling {id}: {e}"))
}

// --- Reference oracles: independent of the cell library, used both to generate search
// examples and to validate a discovered chain over the full domain. ---

fn digital_root_ref(n: u16) -> u16 {
    if n == 0 {
        0
    } else {
        (1 + (n as u32 - 1) % 9) as u16
    }
}
fn low_byte_popcount_ref(x: u16) -> u16 {
    (x & 0xFF).count_ones() as u16
}
fn high_byte_popcount_ref(x: u16) -> u16 {
    (x >> 8).count_ones() as u16
}
fn rotated_low_byte_popcount_ref(x: u16) -> u16 {
    (x.rotate_left(4) & 0xFF).count_ones() as u16
}
/// A deliberately harder, Hamming-deceptive target in the same spirit as `cell-synth-evolve`'s
/// "lossy" benchmarks (OR/rotate/XOR/mask combinations) — but built from real library ops
/// (`mask_union`, `rotl16`, `mask_xor`, `popcount`) instead of synthetic toy cells. Tests
/// whether the pre-registration's "smooth targets, plain A* suffices" scope reduction actually
/// holds, or whether this pool/depth needs GA/MCTS the way the earlier experiment's larger
/// lossy benchmarks did.
fn mystery_bits_ref(x: u16) -> u16 {
    ((x | 0xAAAA).rotate_left(8) ^ 0x5555).count_ones() as u16
}
/// A second, deeper (6-step) escalation of `mystery_bits_ref` — mirrors
/// `cell-synth-evolve`'s own step from a 4-step to a 6-step lossy benchmark, over the same
/// broadened real-cell op pool, to test whether depth *and* pool size together (not just one)
/// is what it takes to actually break A* rather than just strain it.
fn mystery_bits_2_ref(x: u16) -> u16 {
    let a = x | 0x0F0F;
    let b = a.rotate_left(10);
    let c = b & 0xAAAA;
    let d = c.rotate_left(6);
    let e = d ^ 0x5A5A;
    e.count_ones() as u16
}
/// True (1) iff n is a product of exactly two primes counted with multiplicity (p*q, both
/// prime, p possibly == q). A deliberate negative control: no existing cell captures anything
/// like prime-factorization structure, so this is expected to fail to find a chain — that's
/// the point, confirming the experiment doesn't just report success on everything.
fn is_semiprime_ref(n: u16) -> u16 {
    let mut v = n as u32;
    if v < 4 {
        return 0;
    }
    let mut count = 0u32;
    let mut d = 2u32;
    while d * d <= v {
        while v % d == 0 {
            v /= d;
            count += 1;
            if count > 2 {
                return 0;
            }
        }
        d += 1;
    }
    if v > 1 {
        count += 1;
    }
    (count == 2) as u16
}

struct Target {
    name: &'static str,
    oracle: fn(u16) -> u16,
    max_depth: usize,
    /// `//! summary` / `//! tags:` for a passing candidate — real cell sources carry these,
    /// and `library_cartridge` (`cell80/src/cli.rs`) parses them into the manifest, so a
    /// candidate without them wouldn't look like a real contribution to the actual gate.
    summary: &'static str,
    tags: &'static str,
    /// Also run GA/MCTS/portfolio (not just A*) — reserved for the deliberately harder
    /// targets, to bound runtime rather than running the full search-method comparison on
    /// every easy target too.
    harder: bool,
}

/// One real cell's source, parsed just enough to re-emit its body with substitutions: the
/// parameter names in declaration order, the statements before the tail expression (may be
/// empty), and the tail expression itself. Regex/text-based, not a full `syn` AST transform —
/// simpler, and sufficient for this library's cell bodies (no closures, no nested nested
/// nested items), but noted plainly as the reason this isn't a fully general Rust-source
/// codegen tool.
struct ParsedCell {
    params: Vec<String>,
    stmts: String,
    tail: String,
}

/// Split a function body's inner text into (leading statements, final tail expression).
/// Depth-tracked so a `;`/`}` inside a nested `{ }`/`( )` doesn't count. A top-level `;` is
/// always a statement boundary; a top-level `}` is *also* one *unless* it's immediately
/// followed by `else` (still part of the same if/else chain) — needed because this library's
/// loop-based cells (`digit_sum`, `popcount`, ...) end in a `while { ... }` with no trailing
/// semicolon before the bare tail variable, and treating only `;` as a boundary (the first
/// version of this function) swallowed the whole loop into the "tail expression," producing
/// `let out = while ... { ... } s;` — not valid syntax, caught by the first real compile
/// attempt, not by reasoning about it in advance. Known remaining gap: a cell whose *entire*
/// tail is itself an if/else chain with no leading statements (e.g. `clamp.rs`) would still
/// split wrong here — none of the ops actually in this pool have that shape, but a genuinely
/// general version would need real `syn`-based parsing, not text scanning, to handle it.
fn split_tail(body: &str) -> (String, String) {
    let mut depth = 0i32;
    let mut last_boundary = None;
    for (i, ch) in body.char_indices() {
        match ch {
            '{' | '(' => depth += 1,
            '}' | ')' => {
                depth -= 1;
                if depth == 0 && ch == '}' && !body[i + 1..].trim_start().starts_with("else") {
                    last_boundary = Some(i + 1);
                }
            }
            ';' if depth == 0 => last_boundary = Some(i + 1),
            _ => {}
        }
    }
    match last_boundary {
        Some(pos) => (body[..pos].to_string(), body[pos..].trim().to_string()),
        None => (String::new(), body.trim().to_string()),
    }
}

fn parse_cell(src: &str) -> ParsedCell {
    let sig_re = Regex::new(r"fn run\(([^)]*)\)").unwrap();
    let m = sig_re
        .captures(src)
        .unwrap_or_else(|| panic!("no `fn run(...)` found in:\n{src}"));
    let params: Vec<String> = m[1]
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|p| p.trim().split(':').next().unwrap().trim().to_string())
        .collect();

    let sig_end = sig_re.find(src).unwrap().end();
    let brace_rel = src[sig_end..]
        .find('{')
        .unwrap_or_else(|| panic!("no fn body found in:\n{src}"));
    let body_start = sig_end + brace_rel;
    let last_brace = src
        .rfind('}')
        .unwrap_or_else(|| panic!("no closing brace in:\n{src}"));
    let inner = src[body_start + 1..last_brace].trim();

    let (stmts, tail) = split_tail(inner);
    ParsedCell {
        params,
        stmts,
        tail,
    }
}

/// Rename every whole-word occurrence of each `name` in `locals` to `{name}_{uid}` — so
/// applying the same op twice in one chain (e.g. `digit_sum` on `digit_sum`'s own output)
/// doesn't redeclare the same local variable name twice in the composed function body.
fn rename_locals(text: &str, locals: &[String], uid: usize) -> String {
    let mut out = text.to_string();
    for name in locals {
        let re = Regex::new(&format!(r"\b{}\b", regex::escape(name))).unwrap();
        out = re.replace_all(&out, format!("{name}_{uid}")).to_string();
    }
    out
}

fn substitute_param(text: &str, param: &str, replacement: &str) -> String {
    let re = Regex::new(&format!(r"\b{}\b", regex::escape(param))).unwrap();
    re.replace_all(text, format!("({replacement})")).to_string()
}

/// General codegen: parse each step's *real* source text (not a hand-written template),
/// rename its locals to avoid cross-step collisions, substitute its parameter(s) — the first
/// with the running input expression, a second (for the fixed-second-arg ops like
/// `and_00ff`/`rotl4`) with the literal constant baked into that `Op` — and chain the results
/// via `let out{i} = ...;` bindings. Emits flat statements throughout (never a block bound to
/// a variable), since the dialect rejects `let x = { ... };` — caught by actually trying to
/// compile the first candidate under the old hand-written version of this, not assumed.
fn codegen(steps: &[String], op_meta: &std::collections::HashMap<String, (String, u16)>) -> String {
    let mut body = String::new();
    let mut cur = "x".to_string();
    // Compiled once — clippy's regex-in-loop lint is right, this is per-call hot.
    let let_re = Regex::new(r"let\s+(?:mut\s+)?(\w+)").unwrap();
    for (i, step) in steps.iter().enumerate() {
        let (src, fixed_arg) = &op_meta[step];
        let parsed = parse_cell(src);
        let locals: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            let_re
                .captures_iter(&parsed.stmts)
                .map(|c| c[1].to_string())
                .filter(|n| seen.insert(n.clone()))
                .collect()
        };

        let mut stmts = rename_locals(&parsed.stmts, &locals, i);
        let mut tail = rename_locals(&parsed.tail, &locals, i);
        stmts = substitute_param(&stmts, &parsed.params[0], &cur);
        tail = substitute_param(&tail, &parsed.params[0], &cur);
        if parsed.params.len() > 1 {
            let konst = format!("{fixed_arg}u16");
            stmts = substitute_param(&stmts, &parsed.params[1], &konst);
            tail = substitute_param(&tail, &parsed.params[1], &konst);
        }

        let out = format!("out{i}");
        body.push_str(&format!("    {stmts}\n    let {out} = {tail};\n"));
        cur = out;
    }
    format!("fn run(x: u16) -> u16 {{\n{body}    {cur}\n}}\n")
}

fn indent(s: &str) -> String {
    s.lines()
        .map(|l| format!("    {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() {
    let cells_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells");

    let arity1 = [
        "digit_sum",
        "popcount",
        "high_byte",
        "low_byte",
        "swap_bytes",
        "is_even",
        "is_odd",
        "is_pow2",
        "nonzero",
        "bit_length",
        "leading_zeros",
        "trailing_zeros",
        "reverse_bits",
    ];
    // `op_meta` keeps each op's real source text + fixed second arg (0 for arity-1 ops, where
    // it's ignored) — what `codegen` needs to regenerate a step from the real cell, instead of
    // a hand-written template.
    let mut ops: Vec<Op> = Vec::new();
    let mut op_meta: HashMap<String, (String, u16)> = HashMap::new();
    for name in arity1 {
        let src = cell_source(&cells_dir, name);
        let cart = compile(name, &src);
        ops.push(Op::from_cell(name, &cart, 0));
        op_meta.insert(name.to_string(), (src, 0));
    }
    let mask_src = cell_source(&cells_dir, "mask_intersection");
    let mask_cart = compile("mask_intersection", &mask_src);
    for (label, k) in [
        ("and_00ff", 0x00FFu16),
        ("and_ff00", 0xFF00),
        ("and_0f0f", 0x0F0F),
        ("and_aaaa", 0xAAAA),
        ("and_5555", 0x5555),
    ] {
        ops.push(Op::from_cell(label, &mask_cart, k));
        op_meta.insert(label.to_string(), (mask_src.clone(), k));
    }
    let rotl_src = cell_source(&cells_dir, "rotl16");
    let rotl_cart = compile("rotl16", &rotl_src);
    for (label, n) in [
        ("rotl2", 2u16),
        ("rotl4", 4),
        ("rotl6", 6),
        ("rotl8", 8),
        ("rotl10", 10),
        ("rotl12", 12),
        ("rotl14", 14),
    ] {
        ops.push(Op::from_cell(label, &rotl_cart, n));
        op_meta.insert(label.to_string(), (rotl_src.clone(), n));
    }
    // Broadened deliberately (more constants per family, more rotate amounts) to mirror
    // cell-synth-evolve's own escalation (11->18 ops) that's what actually found A*'s failure
    // point there — testing a genuinely harder, Hamming-deceptive target (mystery_bits/
    // mystery_bits_2, below) needs a comparably richer branching factor, not just a deeper
    // target over a narrow pool.
    let union_src = cell_source(&cells_dir, "mask_union");
    let union_cart = compile("mask_union", &union_src);
    for (label, k) in [
        ("or_aaaa", 0xAAAAu16),
        ("or_5555", 0x5555),
        ("or_0f0f", 0x0F0F),
        ("or_00ff", 0x00FF),
        ("or_ff00", 0xFF00),
    ] {
        ops.push(Op::from_cell(label, &union_cart, k));
        op_meta.insert(label.to_string(), (union_src.clone(), k));
    }
    let xor_src = cell_source(&cells_dir, "mask_xor");
    let xor_cart = compile("mask_xor", &xor_src);
    for (label, k) in [
        ("xor_5555", 0x5555u16),
        ("xor_aaaa", 0xAAAA),
        ("xor_5a5a", 0x5A5A),
        ("xor_00ff", 0x00FF),
        ("xor_ff00", 0xFF00),
    ] {
        ops.push(Op::from_cell(label, &xor_cart, k));
        op_meta.insert(label.to_string(), (xor_src.clone(), k));
    }
    println!("Op pool: {} ops built from real stdlib cells.\n", ops.len());

    // Fingerprint every currently-real library cell once, up front, for comparison — this is
    // the actual mechanism `cell80::admission::admit` uses, applied here to a candidate
    // outside the gate (this never calls or touches admission.rs itself).
    let library_fps: Vec<(String, Fingerprint)> = fs::read_dir(&cells_dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", cells_dir.display()))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .filter_map(|e| {
            e.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .filter_map(|name| {
            let src = fs::read_to_string(cells_dir.join(format!("{name}.rs"))).ok()?;
            let cart = Cartridge::compile(
                &src,
                CellConfig::sandboxed(),
                CartridgeOpts {
                    id: Some(name.clone()),
                    ..Default::default()
                },
            )
            .ok()?;
            Some((name, Fingerprint::of(&cart)))
        })
        .collect();
    println!(
        "Fingerprinted {} existing library cells for comparison.\n",
        library_fps.len()
    );

    let targets: Vec<Target> = vec![
        Target {
            name: "digital_root",
            oracle: digital_root_ref,
            max_depth: 5,
            summary: "Digital root of n: repeatedly sum digits until one digit remains.",
            tags: "number, digits, digital-root, decimal, reduce, math",
            harder: false,
        },
        Target {
            name: "low_byte_popcount",
            oracle: low_byte_popcount_ref,
            max_depth: 3,
            summary: "Population count of just the low byte of x (high byte ignored).",
            tags: "bits, popcount, byte, low, count, ones, bitcount",
            harder: false,
        },
        Target {
            name: "high_byte_popcount",
            oracle: high_byte_popcount_ref,
            max_depth: 3,
            summary: "Population count of just the high byte of x (low byte ignored).",
            tags: "bits, popcount, byte, high, count, ones, bitcount",
            harder: false,
        },
        Target {
            name: "is_semiprime",
            oracle: is_semiprime_ref,
            max_depth: 5,
            summary: "1 if n is a product of exactly two primes (with multiplicity), else 0.",
            tags: "number, prime, semiprime, factorization, predicate",
            harder: false,
        },
        Target {
            name: "rotated_low_byte_popcount",
            oracle: rotated_low_byte_popcount_ref,
            max_depth: 4,
            summary: "Population count of the low byte of x after rotating its bits left by 4.",
            tags: "bits, popcount, rotate, byte, count, ones",
            harder: false,
        },
        Target {
            name: "mystery_bits",
            oracle: mystery_bits_ref,
            max_depth: 6,
            summary: "Popcount of x, OR'd with 0xAAAA, rotated left 8, XOR'd with 0x5555.",
            tags: "bits, popcount, mask, rotate, xor, experimental",
            harder: true,
        },
        Target {
            name: "mystery_bits_2",
            oracle: mystery_bits_2_ref,
            max_depth: 8,
            summary: "Popcount of a 6-step OR/rotate/AND/rotate/XOR mask chain over x.",
            tags: "bits, popcount, mask, rotate, xor, experimental",
            harder: true,
        },
    ];

    // 39999 is load-bearing, not decorative: digit_sum(39999)=39, digit_sum(39)=12,
    // digit_sum(12)=3 — a chain of only 2 digit_sum applications gives 12 here, not the
    // correct 3, which is exactly the gap the first run's full-domain check caught (a 2-pass
    // chain matched every other probe below but was wrong for 8,075/65,536 real inputs).
    const PROBES: &[u16] = &[
        0, 1, 4, 6, 9, 10, 99, 255, 256, 0x0F0F, 0xAAAA, 0x5555, 0xFF00, 0x00FF, 9999, 39999,
        59999, 65535,
    ];

    const HARDER_SEEDS: &[u64] = &[1, 2, 3, 4, 5];

    let mut passes = 0usize;
    for t in &targets {
        println!("=== {} (max_depth={}) ===", t.name, t.max_depth);
        let examples: Vec<(u16, u16)> = PROBES.iter().map(|&x| (x, (t.oracle)(x))).collect();

        let astar_plan = synthesize(&examples, &ops, t.max_depth, BUDGET);
        match &astar_plan {
            Some(p) => println!("  A*:   found {:?} ({} nodes tested)", p.steps, p.tested),
            None => println!(
                "  A*:   no chain found (budget {BUDGET}, depth {})",
                t.max_depth
            ),
        }

        let plan = if t.harder {
            // Also run GA/MCTS/portfolio — not just A* — to test whether they find something
            // A* can't at this pool size (or find it far more cheaply), instead of only
            // reporting whatever A* alone returns.
            let ga_results: Vec<_> = HARDER_SEEDS
                .iter()
                .map(|&seed| evolve(&examples, &ops, t.max_depth, BUDGET, seed))
                .collect();
            let mcts_results: Vec<_> = HARDER_SEEDS
                .iter()
                .map(|&seed| mcts(&examples, &ops, t.max_depth, BUDGET, seed))
                .collect();
            println!("  GA:   {}", summarize(&ga_results));
            println!("  MCTS: {}", summarize(&mcts_results));
            let portfolio_results: Vec<_> = (0..HARDER_SEEDS.len())
                .map(|i| {
                    portfolio(&[
                        astar_plan.clone(),
                        ga_results[i].clone(),
                        mcts_results[i].clone(),
                    ])
                })
                .collect();
            println!(
                "  Portfolio (best of A*/GA/MCTS): {}",
                summarize(&portfolio_results)
            );

            // Prefer A*'s plan for the downstream full-domain/codegen/fingerprint steps if it
            // found one (matching the non-harder targets' behaviour); otherwise fall back to
            // whichever method actually succeeded, so a target A* can't solve still gets
            // checked all the way through instead of being silently skipped.
            astar_plan
                .or_else(|| ga_results.into_iter().flatten().next())
                .or_else(|| mcts_results.into_iter().flatten().next())
        } else {
            astar_plan
        };

        let Some(plan) = plan else {
            println!("  no method found a chain within budget — see note below\n");
            continue;
        };

        let mut wrong = 0u32;
        for x in 0..=u16::MAX {
            let mut v = x;
            for name in &plan.steps {
                let op = ops.iter().find(|o| &o.name == name).unwrap();
                v = op.apply(v);
            }
            if v != (t.oracle)(x) {
                wrong += 1;
            }
        }
        if wrong > 0 {
            println!(
                "  FULL-DOMAIN CHECK FAILED: {wrong}/65536 mismatches — matched the probes but \
                 not the real function\n"
            );
            continue;
        }
        println!("  full-domain check: PASS (all 65,536 inputs correct)");

        let body = codegen(&plan.steps, &op_meta);
        let src = format!("//! {}\n//! tags: {}\n{body}", t.summary, t.tags);

        let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("candidates");
        fs::create_dir_all(&out_dir)
            .unwrap_or_else(|e| panic!("creating {}: {e}", out_dir.display()));
        let out_path = out_dir.join(format!("{}.rs", t.name));
        fs::write(&out_path, &src)
            .unwrap_or_else(|e| panic!("writing {}: {e}", out_path.display()));
        let cart = compile(&format!("candidate_{}", t.name), &src);
        let fp = Fingerprint::of(&cart);
        let (best_agree, closest) = library_fps
            .iter()
            .map(|(name, other)| (fp.agreement(other), name.as_str()))
            .fold(
                (f32::MIN, ""),
                |best, cur| if cur.0 > best.0 { cur } else { best },
            );
        let is_dup = best_agree >= DUPLICATE_AGREEMENT;
        println!(
            "  fingerprint: closest existing cell = `{closest}` (agreement {best_agree:.3}) -> {}",
            if is_dup {
                "DUPLICATE — would be refused"
            } else {
                "NOVEL — would pass the fingerprint check"
            }
        );
        if !is_dup {
            passes += 1;
        }
        println!("  generated candidate source:\n{}\n", indent(&src));
    }

    println!(
        "=== {passes}/{} reachable targets passed (full-domain-correct AND non-duplicate) ===",
        targets.len() - 1 // is_semiprime is the negative control, not counted toward the bar
    );
}
