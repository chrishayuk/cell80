//! Scratch spot-check (not part of C0): does the `next_pow2` fan-out win
//! (`cell-fanout-gate-findings.md` §5, 3.25x under the IR-step proxy) survive under
//! real Z80 T-states? Hand-composes the winning chain into one source (the same
//! technique `experiments/cell-cost-discovery/src/bin/confirm.rs` uses), sweeps the
//! full 65,536-input domain on the real Z80 body, and reports the domain-wide mean
//! plus where in the domain the ratio crosses over — **both raw and P-repriced**
//! (`cell-cost-discovery`'s own mandatory repricing discipline: the runner's mul/div
//! host trap is a flat 4 T-states, ~36x under the real software-routine cost on a
//! chip with no hardware MUL/DIV, so any candidate using `/` or `*` must be repriced
//! or the comparison silently favors it). `snap_up` (inside the composed candidate)
//! divides and multiplies; the reference `next_pow2` loop does neither.

use cell80::{find_cell_file, CartridgeOpts, CellConfig, Halt, Runner};

const DOMAIN: usize = 1 << 16;

/// `cell-cost-discovery`'s own P-measurement, reproduced verbatim (not imported —
/// separate crate): the mean-cycle differential between a trap-free shift-and-add
/// mul16 and a plain `a * b` trap cell over the full u8x8 grid, at the SAME
/// `CartridgeOpts` (`kernel_bank: true`) this spot-check and that experiment both use.
const SOFT_MUL: &str = "fn run(a: u16, b: u16) -> u16 { let mut acc = 0u16; let mut x = a; let mut y = b; let mut i = 0u16; while i < 16u16 { if (y & 1u16) != 0u16 { acc = acc.wrapping_add(x); } x = x << 1u16; y = y >> 1u16; i = i + 1u16; } acc }";
const TRAP_MUL: &str = "fn run(a: u16, b: u16) -> u16 { a * b }";

fn measure_p() -> f64 {
    let soft = compile("xp_soft_mul16", SOFT_MUL);
    let trap = compile("xp_trap_mul16", TRAP_MUL);
    let mean = |cart: &cell80::Cartridge| {
        let mut r = Runner::new(cart.z80().unwrap());
        let entry = cart.manifest.entry.clone();
        let mut sum = 0u64;
        for a in 0..=255u16 {
            for b in 0..=255u16 {
                let f = r
                    .run_fast(Some(&entry), &[a, b], cell80::DEFAULT_CYCLES)
                    .unwrap();
                assert!(matches!(f.halt, Halt::Returned));
                sum += f.cycles;
            }
        }
        sum as f64 / (256 * 256) as f64
    };
    (mean(&soft) - mean(&trap)).max(0.0)
}

// Hand-inlined next_pow2 <- snap_up(mask_xor(x, is_zero(x)), highest_set_bit(x)),
// each stage's body copied verbatim from cell80/cells/{predicates/is_zero,
// bit-mask/mask_xor,bit-mask/highest_set_bit,bounds/snap_up}.rs.
const COMPOSED: &str = "fn run(x: u16) -> u16 { \
    let iz = (x == 0u16) as u16; \
    let mx = x ^ iz; \
    let mut v = x; \
    v = v | (v >> 1u16); \
    v = v | (v >> 2u16); \
    v = v | (v >> 4u16); \
    v = v | (v >> 8u16); \
    let hsb = v - (v >> 1u16); \
    if hsb != 0u16 && mx != 0u16 { ((mx - 1u16) / hsb + 1u16) * hsb } else { mx } \
}";

fn compile(id: &str, src: &str) -> cell80::Cartridge {
    cell80::Cartridge::compile(
        src,
        CellConfig::permissive(),
        CartridgeOpts {
            id: Some(id.into()),
            kernel_bank: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("{id} failed to compile: {e}"))
}

fn main() {
    let p = measure_p();
    println!("P (trap surcharge) = {p:.1} T-states\n");

    let cells_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cells");
    let ref_path = find_cell_file(&cells_dir, "next_pow2").expect("next_pow2 source");
    let ref_src = std::fs::read_to_string(&ref_path).unwrap();

    let ref_cart = compile("next_pow2_ref", &ref_src);
    let comp_cart = compile("next_pow2_composed", COMPOSED);

    let mut r_ref = Runner::new(ref_cart.z80().unwrap());
    let mut r_comp = Runner::new(comp_cart.z80().unwrap());
    let ref_entry = ref_cart.manifest.entry.clone();
    let comp_entry = comp_cart.manifest.entry.clone();

    let mut ref_total = 0u64;
    let mut comp_total = 0u64;
    let mut ref_trapped_total = 0u64;
    let mut comp_trapped_total = 0u64;
    let mut mismatches = 0usize;
    let mut crossover: Option<u32> = None; // first x where composed <= reference (raw)
    let mut ref_leq_comp_count = 0usize; // domain fraction where reference is cheaper (raw)

    for x in 0..DOMAIN {
        let x = x as u16;
        let rf = r_ref
            .run_fast(Some(&ref_entry), &[x], cell80::DEFAULT_CYCLES)
            .unwrap();
        let cf = r_comp
            .run_fast(Some(&comp_entry), &[x], cell80::DEFAULT_CYCLES)
            .unwrap();
        if !matches!(rf.halt, Halt::Returned) || !matches!(cf.halt, Halt::Returned) {
            panic!(
                "non-Returned halt at x={x}: ref={:?} comp={:?}",
                rf.halt, cf.halt
            );
        }
        if rf.result != cf.result {
            mismatches += 1;
            if mismatches <= 5 {
                println!("MISMATCH x={x} ref={} comp={}", rf.result, cf.result);
            }
        }
        ref_total += rf.cycles;
        comp_total += cf.cycles;
        ref_trapped_total += rf.trapped_ops;
        comp_trapped_total += cf.trapped_ops;
        if rf.cycles <= cf.cycles {
            ref_leq_comp_count += 1;
        }
        if crossover.is_none() && cf.cycles <= rf.cycles {
            crossover = Some(x as u32);
        }
    }

    println!("mismatches: {mismatches}/{DOMAIN}");
    let ref_mean = ref_total as f64 / DOMAIN as f64;
    let comp_mean = comp_total as f64 / DOMAIN as f64;
    let ref_trapped_mean = ref_trapped_total as f64 / DOMAIN as f64;
    let comp_trapped_mean = comp_trapped_total as f64 / DOMAIN as f64;
    println!("reference next_pow2 mean cycles (raw):  {ref_mean:.1}  (mean trapped_ops: {ref_trapped_mean:.3})");
    println!("composed  next_pow2 mean cycles (raw):  {comp_mean:.1}  (mean trapped_ops: {comp_trapped_mean:.3})");
    println!(
        "RAW domain-mean ratio (ref/comp): {:.3}x {}",
        ref_mean / comp_mean,
        if ref_mean > comp_mean {
            "(composed wins)"
        } else {
            "(reference wins)"
        }
    );
    println!(
        "fraction of domain where reference is cheaper-or-equal (raw): {:.2}% ({}/{})",
        100.0 * ref_leq_comp_count as f64 / DOMAIN as f64,
        ref_leq_comp_count,
        DOMAIN
    );
    println!("first x where composed <= reference (raw): {crossover:?}\n");

    // P-repriced: cell-cost-discovery's mandatory correction — mul/div traps
    // charged at their measured real-substrate cost, not the model's flat 4
    // T-states. The reference has zero mul/div anywhere in its body; the composed
    // candidate's snap_up divides then multiplies whenever step != 0 && x != 0.
    let ref_repriced = ref_mean + p * ref_trapped_mean;
    let comp_repriced = comp_mean + p * comp_trapped_mean;
    println!("reference next_pow2 mean cycles (P-repriced): {ref_repriced:.1}");
    println!("composed  next_pow2 mean cycles (P-repriced): {comp_repriced:.1}");
    println!(
        "REPRICED domain-mean ratio (ref/comp): {:.3}x {}",
        ref_repriced / comp_repriced,
        if ref_repriced > comp_repriced {
            "(composed wins)"
        } else {
            "(reference wins)"
        }
    );
}
