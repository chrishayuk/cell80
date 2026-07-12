//! Confirmation step pre-registered in `../../cell-cost-discovery-preregistration.md`:
//! each search winner, hand-composed into ONE single-source cell (the chain's stages
//! inlined, constants folded), recompiled, verified full-domain identical to its target,
//! and re-costed — the chain-sum cost the search used carries d call overheads, so the
//! inlined cell should only be cheaper. A candidate failing equality or costing more than
//! the chain-sum would be reported, not hidden.

use cell80::{discover_cell_files, find_cell_file};
use cell_cost_discovery::{compile, measure_p, tabulate, DOMAIN};
use std::path::Path;

/// (target id, candidate id, hand-composed single source, the chain it inlines)
const CANDIDATES: &[(&str, &str, &str, &str)] = &[
    (
        "isqrt",
        "isqrt_bitwise",
        // geomean2[b=1]: `prod = n as u32 * 1` folds to `n as u32`; the rest is geomean2's
        // division-free bitwise integer sqrt, verbatim.
        "fn run(n: u16) -> u16 { let mut val = n as u32; let mut res = 0u32; let mut bit = 1u32 << 30u32; while bit > val { bit = bit >> 2u32; } while bit != 0u32 { if val >= res + bit { val = val - (res + bit); res = (res >> 1u32) + bit; } else { res = res >> 1u32; } bit = bit >> 2u32; } res as u16 }",
        "geomean2[b=1]",
    ),
    (
        "bit_length",
        "bit_length_clz",
        // leading_zeros |> abs_diff[b=16]: clz <= 16 always, so |c - 16| folds to 16 - c.
        "fn run(x: u16) -> u16 { let mut v = x; let mut c = 0u16; while c < 16u16 && (v & 0x8000u16) == 0u16 { v = v << 1u16; c = c + 1u16; } 16u16 - c }",
        "leading_zeros |> abs_diff[b=16]",
    ),
    (
        "is_weekend",
        "is_weekend_le",
        "fn run(dow: u16) -> u16 { (dow <= 1u16) as u16 }",
        "is_le[b=1]",
    ),
    (
        "is_odd",
        "is_odd_and",
        "fn run(x: u16) -> u16 { x & 1u16 }",
        "mask_intersection[b=1]",
    ),
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cells_dir = args.get(1).map(String::as_str).unwrap_or("cell80/cells");
    let _ = discover_cell_files(cells_dir).expect("library dir");
    let p = measure_p();

    for (target_id, cand_id, cand_src, chain) in CANDIDATES {
        let tf = find_cell_file(Path::new(cells_dir), target_id).expect("target source");
        let tsrc = std::fs::read_to_string(&tf).expect("read target");
        let tcart = compile(target_id, &tsrc).expect("target compiles");
        let (tt, t_rp, t_p0) = tabulate(&tcart, None, p).expect("target total");

        let ccart = compile(cand_id, cand_src).expect("candidate compiles");
        let (ct, c_rp, c_p0) = tabulate(&ccart, None, p).expect("candidate total");

        let equal = (0..DOMAIN).all(|v| tt[v] == ct[v]);
        let verdict = if !equal {
            "FULL-DOMAIN MISMATCH — candidate rejected"
        } else if c_rp < t_rp {
            "CONFIRMED: identical on all 65536 inputs, strictly cheaper inlined"
        } else {
            "identical but NOT cheaper once inlined — chain win was overhead artifact"
        };
        println!(
            "{cand_id} (inlines {chain}) vs {target_id}: {verdict}\n    repriced mean {c_rp:.1} vs {t_rp:.1} ({:.2}x), raw {c_p0:.1} vs {t_p0:.1} ({:.2}x)",
            t_rp / c_rp,
            t_p0 / c_p0,
        );
    }
}
