//! Shared pieces of the cost-discovery experiment: the pre-registered trap-surcharge
//! measurement (P) and full-domain tabulation. See `../cell-cost-discovery-preregistration.md`.

// Tabulation fills a table indexed by the run's input value; the index IS the datum.
#![allow(clippy::needless_range_loop)]

use cell80::{Cartridge, CartridgeOpts, CellConfig, Halt, Runner, DEFAULT_CYCLES};

pub const DOMAIN: usize = 1 << 16;

/// Trap-free shift-and-add mul16, fixed 16 iterations — the substrate's own price for a
/// software multiply. Paired with [`TRAP_MUL`], the mean-cycle differential over the same
/// u8×u8 grid is the trap surcharge P (call overhead cancels; the trap's 4 charged cycles
/// are subtracted implicitly).
pub const SOFT_MUL: &str = "fn run(a: u16, b: u16) -> u16 { let mut acc = 0u16; let mut x = a; let mut y = b; let mut i = 0u16; while i < 16u16 { if (y & 1u16) != 0u16 { acc = acc.wrapping_add(x); } x = x << 1u16; y = y >> 1u16; i = i + 1u16; } acc }";
pub const TRAP_MUL: &str = "fn run(a: u16, b: u16) -> u16 { a * b }";

pub fn compile(id: &str, src: &str) -> Result<Cartridge, String> {
    Cartridge::compile(
        src,
        CellConfig::permissive(),
        CartridgeOpts {
            id: Some(id.into()),
            kernel_bank: true,
            ..Default::default()
        },
    )
}

pub fn mean_cycles_u8grid(cart: &Cartridge) -> Result<f64, String> {
    let mut r = Runner::new(cart.z80()?);
    let entry = cart.manifest.entry.clone();
    let mut sum = 0u64;
    for a in 0..=255u16 {
        for b in 0..=255u16 {
            let f = r.run_fast(Some(&entry), &[a, b], DEFAULT_CYCLES)?;
            if !matches!(f.halt, Halt::Returned) {
                return Err(format!(
                    "P-measurement cell stopped at ({a},{b}): {:?}",
                    f.halt
                ));
            }
            sum += f.cycles;
        }
    }
    Ok(sum as f64 / DOMAIN as f64)
}

/// Measure the trap surcharge P (pre-registered differential). Prints the components.
pub fn measure_p() -> u64 {
    let soft = compile("xp_soft_mul16", SOFT_MUL).expect("soft mul16 compiles");
    let trap = compile("xp_trap_mul16", TRAP_MUL).expect("trap mul16 compiles");
    let ms = mean_cycles_u8grid(&soft).expect("soft mul16 runs the grid");
    let mt = mean_cycles_u8grid(&trap).expect("trap mul16 runs the grid");
    let p = ms - mt;
    if p <= 0.0 {
        eprintln!("WARNING: measured P = {p:.1} <= 0; clamping to 0 (soft mul not slower?)");
    }
    println!(
        "P (trap surcharge) = {:.0} T-states  [soft mean {ms:.1} - trap mean {mt:.1}]",
        p.max(0.0).round()
    );
    p.max(0.0).round() as u64
}

/// Full-domain tabulation of a unary (or constant-bound binary) entry: `None` unless total.
/// Returns `(table, mean_repriced, mean_p0)`.
pub fn tabulate(
    cart: &Cartridge,
    fixed: Option<u16>,
    p_surcharge: u64,
) -> Option<(Vec<u16>, f64, f64)> {
    let mut r = Runner::new(cart.z80().ok()?);
    let entry = cart.manifest.entry.clone();
    let mut table = vec![0u16; DOMAIN];
    let mut srp = 0u64;
    let mut sp0 = 0u64;
    for v in 0..DOMAIN {
        let args = match fixed {
            Some(c) => [v as u16, c].to_vec(),
            None => [v as u16].to_vec(),
        };
        let f = r.run_fast(Some(&entry), &args, DEFAULT_CYCLES).ok()?;
        if !matches!(f.halt, Halt::Returned) {
            return None;
        }
        table[v] = f.result;
        sp0 += f.cycles;
        srp += f.cycles + p_surcharge * f.trapped_ops;
    }
    Some((
        table,
        srp as f64 / DOMAIN as f64,
        sp0 as f64 / DOMAIN as f64,
    ))
}
