//! Shared pieces reused by both `main.rs` (the pre-registered run) and `bin/boundary_sweep.rs`
//! (mapping where A* actually starts failing, instead of the one data point the first pass
//! found): op-pool construction from real stdlib cells, the reference oracles, and codegen —
//! now `syn`-based, not regex-based, for the part that actually needed real parsing (finding a
//! cell's true tail expression). Renaming/substitution stays regex-based on the rendered
//! text: that part was already correct, only the tail-detection heuristic was the known gap.
use cell80::{Cartridge, CartridgeOpts, CellConfig, Op};
use quote::quote;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn cell_source(cells_dir: &Path, name: &str) -> String {
    let path =
        cell80::find_cell_file(cells_dir, name).unwrap_or_else(|e| panic!("finding {name}: {e}"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {name}: {e}"))
}

pub fn compile(id: &str, src: &str) -> Cartridge {
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

pub fn digital_root_ref(n: u16) -> u16 {
    if n == 0 {
        0
    } else {
        (1 + (n as u32 - 1) % 9) as u16
    }
}
pub fn low_byte_popcount_ref(x: u16) -> u16 {
    (x & 0xFF).count_ones() as u16
}
pub fn high_byte_popcount_ref(x: u16) -> u16 {
    (x >> 8).count_ones() as u16
}
pub fn rotated_low_byte_popcount_ref(x: u16) -> u16 {
    (x.rotate_left(4) & 0xFF).count_ones() as u16
}
/// A deliberately harder, Hamming-deceptive target in the same spirit as `cell-synth-evolve`'s
/// "lossy" benchmarks (OR/rotate/XOR/mask combinations) — but built from real library ops.
pub fn mystery_bits_ref(x: u16) -> u16 {
    ((x | 0xAAAA).rotate_left(8) ^ 0x5555).count_ones() as u16
}
/// A second, deeper (6-step) escalation — the one that actually breaks A* at the full pool
/// size/depth (see `evolved-cells-findings.md`). Needs `or_0f0f`, `rotl10`, `and_aaaa`,
/// `rotl6`, `xor_5a5a`, `popcount` all present in the pool to be solvable at all — kept as the
/// fixed "core" prefix of [`build_ops`]'s pool ordering so every sweep size stays solvable.
pub fn mystery_bits_2_ref(x: u16) -> u16 {
    let a = x | 0x0F0F;
    let b = a.rotate_left(10);
    let c = b & 0xAAAA;
    let d = c.rotate_left(6);
    let e = d ^ 0x5A5A;
    e.count_ones() as u16
}
/// True (1) iff n is a product of exactly two primes counted with multiplicity. A deliberate
/// negative control: no existing cell captures anything like prime-factorization structure.
pub fn is_semiprime_ref(n: u16) -> u16 {
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

/// Build an op pool of exactly `pool_size` ops (clamped to `[MIN_POOL, MAX_POOL]`), as a
/// **prefix** of one fixed, deterministic ordering: the 13 arity-1 base ops, then the 5
/// "core" combos `mystery_bits_2` actually needs (so every valid pool size stays solvable for
/// it), then 16 "extra" distractor combos appended last purely to grow branching factor. This
/// is what makes a pool-size sweep meaningful: shrinking the pool changes branching factor,
/// not solvability, so a transition from found to not-found is really about search difficulty.
pub const MIN_POOL: usize = 18;
pub const MAX_POOL: usize = 34;

pub fn build_ops(cells_dir: &Path, pool_size: usize) -> (Vec<Op>, HashMap<String, (String, u16)>) {
    let pool_size = pool_size.clamp(MIN_POOL, MAX_POOL);

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

    let mask_src = cell_source(cells_dir, "mask_intersection");
    let mask_cart = compile("mask_intersection", &mask_src);
    let union_src = cell_source(cells_dir, "mask_union");
    let union_cart = compile("mask_union", &union_src);
    let xor_src = cell_source(cells_dir, "mask_xor");
    let xor_cart = compile("mask_xor", &xor_src);
    let rotl_src = cell_source(cells_dir, "rotl16");
    let rotl_cart = compile("rotl16", &rotl_src);

    // (label, cell-cart, cell-source, fixed-arg) — core (mystery_bits_2's real dependencies)
    // first, then extras, in this exact fixed order.
    let core: Vec<(&str, &Cartridge, &str, u16)> = vec![
        ("or_0f0f", &union_cart, &union_src, 0x0F0F),
        ("rotl10", &rotl_cart, &rotl_src, 10),
        ("and_aaaa", &mask_cart, &mask_src, 0xAAAA),
        ("rotl6", &rotl_cart, &rotl_src, 6),
        ("xor_5a5a", &xor_cart, &xor_src, 0x5A5A),
    ];
    let extra: Vec<(&str, &Cartridge, &str, u16)> = vec![
        ("and_00ff", &mask_cart, &mask_src, 0x00FF),
        ("and_ff00", &mask_cart, &mask_src, 0xFF00),
        ("and_5555", &mask_cart, &mask_src, 0x5555),
        ("rotl2", &rotl_cart, &rotl_src, 2),
        ("rotl4", &rotl_cart, &rotl_src, 4),
        ("rotl8", &rotl_cart, &rotl_src, 8),
        ("rotl12", &rotl_cart, &rotl_src, 12),
        ("rotl14", &rotl_cart, &rotl_src, 14),
        ("or_aaaa", &union_cart, &union_src, 0xAAAA),
        ("or_5555", &union_cart, &union_src, 0x5555),
        ("or_00ff", &union_cart, &union_src, 0x00FF),
        ("or_ff00", &union_cart, &union_src, 0xFF00),
        ("xor_5555", &xor_cart, &xor_src, 0x5555),
        ("xor_aaaa", &xor_cart, &xor_src, 0xAAAA),
        ("xor_00ff", &xor_cart, &xor_src, 0x00FF),
        ("xor_ff00", &xor_cart, &xor_src, 0xFF00),
    ];

    let mut ops: Vec<Op> = Vec::new();
    let mut op_meta: HashMap<String, (String, u16)> = HashMap::new();

    for name in arity1 {
        let src = cell_source(cells_dir, name);
        let cart = compile(name, &src);
        ops.push(Op::from_cell(name, &cart, 0));
        op_meta.insert(name.to_string(), (src, 0));
    }

    for (label, cart, src, k) in core
        .into_iter()
        .chain(extra)
        .take(pool_size.saturating_sub(arity1.len()))
    {
        ops.push(Op::from_cell(label, cart, k));
        op_meta.insert(label.to_string(), (src.to_string(), k));
    }

    (ops, op_meta)
}

/// One real cell's source, parsed with `syn` (not text scanning) just enough to re-emit its
/// body with substitutions: the parameter names in declaration order, the leading statements
/// (rendered text, may be empty), and the tail expression (rendered text). Using `syn` fixes
/// the previous regex-based version's known gap — a cell whose *entire* tail is itself a
/// leading if/else chain with no statements before it (e.g. `clamp.rs`) split wrong under text
/// heuristics, because "is this `}` a statement boundary or part of the tail" is genuinely
/// ambiguous from text alone. `syn::Stmt::Expr(_, None)` (an expression with no trailing
/// semicolon) is unambiguous by construction: it can only be the last statement in a block,
/// and it's exactly the tail — no heuristic needed once the real grammar does the splitting.
/// Renaming/substitution below stays regex/text-based on the rendered pieces: that part was
/// already correct, only tail-detection needed real parsing.
struct ParsedCell {
    params: Vec<String>,
    stmts: String,
    tail: String,
}

fn parse_cell(src: &str) -> ParsedCell {
    let file = syn::parse_file(src).unwrap_or_else(|e| panic!("parsing cell source: {e}\n{src}"));
    let func = file
        .items
        .into_iter()
        .find_map(|item| match item {
            syn::Item::Fn(f) if f.sig.ident == "run" => Some(f),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no `fn run` found in:\n{src}"));

    let params: Vec<String> = func
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pt) => match &*pt.pat {
                syn::Pat::Ident(pi) => Some(pi.ident.to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect();

    let mut stmts = func.block.stmts;
    let tail_expr = match stmts.pop() {
        Some(syn::Stmt::Expr(e, None)) => e,
        Some(other) => {
            panic!("cell body's last statement isn't a tail expression (ends in `;`) in:\n{src}\n(saw: {})", quote!(#other))
        }
        None => panic!("empty cell body in:\n{src}"),
    };

    let stmts_text: String = stmts
        .iter()
        .map(|s| quote!(#s).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let tail_text = quote!(#tail_expr).to_string();

    ParsedCell {
        params,
        stmts: stmts_text,
        tail: tail_text,
    }
}

/// Rename every whole-word occurrence of each `name` in `locals` to `{name}_{uid}` — so
/// applying the same op twice in one chain doesn't redeclare the same local variable name
/// twice in the composed function body.
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

/// General codegen: parse each step's *real* source text via `syn` (not a hand-written
/// template, not text-scanning for the tail), rename its locals to avoid cross-step
/// collisions, substitute its parameter(s) — the first with the running input expression, a
/// second (for fixed-second-arg ops like `and_00ff`/`rotl4`) with the literal constant baked
/// into that `Op` — and chain the results via `let out{i} = ...;` bindings. Emits flat
/// statements throughout (never a block bound to a variable), since the dialect rejects
/// `let x = { ... };`.
pub fn codegen(steps: &[String], op_meta: &HashMap<String, (String, u16)>) -> String {
    let mut body = String::new();
    let mut cur = "x".to_string();
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

pub fn indent(s: &str) -> String {
    s.lines()
        .map(|l| format!("    {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}
