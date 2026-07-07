//! F0 smoke + cost for the owned-softfloat prelude family
//! (`docs/real-valued-cells-amendment.md`).
//!
//! Correctness lives in `rustz80/tests/diff/f32_ops.rs` (the H-F1 bit-equality bank
//! against rustc, both targets, edge + random). This side asserts the *cell* story:
//! the kernels are reachable through `CELL_PRELUDE`, they prune when unused, and the
//! measured T-state/byte costs stay inside the registered H-F2 envelope. `cycles`
//! caveat: `fmul`'s four u32 multiplies ride `ED FE` traps charged ~4 T-states each,
//! so its honest cost is `cycles` *paired with* `trapped_ops`; fadd/fdiv/fsqrt are
//! pure shifts/adds — their cycles are authentic.

use cell80::Runner;

const BUDGET: u64 = 1_000_000;

fn run_result(src: &str) -> u16 {
    let mut runner = Runner::compile(src).unwrap_or_else(|e| panic!("compile failed: {e}"));
    let r = runner.run(None, &[], BUDGET).expect("run failed");
    assert!(r.returned, "cell did not return cleanly");
    r.result
}

/// Each kernel is callable from a cell via the prelude and produces rustc-f32 bits.
/// (Values chosen off the special-case paths; the exhaustive bank is rustz80-side.)
#[test]
fn f32_kernels_reachable_from_cells() {
    let pi = std::f32::consts::PI.to_bits();
    let e = std::f32::consts::E.to_bits();
    let checks = [
        (
            format!("fadd({pi}u32, {e}u32)"),
            (std::f32::consts::PI + std::f32::consts::E).to_bits(),
        ),
        (
            format!("fsub({pi}u32, {e}u32)"),
            (std::f32::consts::PI - std::f32::consts::E).to_bits(),
        ),
        (
            format!("fmul({pi}u32, {e}u32)"),
            (std::f32::consts::PI * std::f32::consts::E).to_bits(),
        ),
        (
            format!("fdiv({pi}u32, {e}u32)"),
            (std::f32::consts::PI / std::f32::consts::E).to_bits(),
        ),
        (
            format!("fsqrt({pi}u32)"),
            std::f32::consts::PI.sqrt().to_bits(),
        ),
    ];
    for (expr, want) in checks {
        let src = format!(
            "fn run() -> u16 {{ let mut ok = 0u16; if {expr} == {want}u32 {{ ok = 1u16; }} ok }}"
        );
        assert_eq!(run_result(&src), 1, "{expr} != rustc bits 0x{want:08X}");
    }
}

/// Multi-kernel f32 cells fit since scratch relocation (the locals region places
/// above the code when it outgrows the classic `0x9000` window; the Cell ceiling is
/// `STATE_BASE` = `0xB000`): a lerp chains fsub → fmul → fadd in one cell and stays
/// bit-identical to rustc.
#[test]
fn f32_multi_kernel_cell() {
    let (a, b, t) = ((-3.5f32).to_bits(), 7.25f32.to_bits(), 0.75f32.to_bits());
    let want = (-3.5f32 + 0.75f32 * (7.25f32 - (-3.5f32))).to_bits();
    let src = format!(
        "fn run() -> u16 {{ let mut ok = 0u16; \
         if fadd({a}u32, fmul({t}u32, fsub({b}u32, {a}u32))) == {want}u32 {{ ok = 1u16; }} ok }}"
    );
    assert_eq!(run_result(&src), 1, "lerp cell diverged from rustc bits");
}

/// A cell that calls no f32 kernel stays byte-identical — the appended family is
/// fully pruned (the "composition costs zero bytes" property extends to F0).
#[test]
fn f32_kernels_prune_when_unused() {
    let mut runner = Runner::compile("fn run() -> u16 { 42u16 }").unwrap();
    let r = runner.run(None, &[], BUDGET).unwrap();
    assert_eq!(r.result, 42);
    // The tiny cell's whole image must stay tiny — kernels would add kilobytes.
    assert!(
        r.code_bytes < 64,
        "unused f32 kernels leaked into the image ({}B)",
        r.code_bytes
    );
}

/// H-F2, measured: per-kernel T-states and image bytes, printed as the cost table
/// (`--nocapture`) and pinned ~25-30% above the 2026-07-07 measurement (fadd 10,854 /
/// fsub 12,586 / fmul 11,227+4 traps / fdiv 36,644 / fsqrt 53,219 T-states). The
/// amendment's pre-registered prediction ("fadd/fmul low thousands") missed ~3x —
/// recorded, not hidden; the measured table is published in the dialect doc. A breach
/// here is a *pricing* regression, not a tweak-the-ceiling signal.
#[test]
fn f32_kernel_cost_envelope() {
    let pi = std::f32::consts::PI.to_bits();
    let e = std::f32::consts::E.to_bits();
    // Baseline driver shape: same call/return plumbing, no kernel.
    let base_src = "fn run() -> u16 { let x = 42u32; (x >> 16u32) as u16 }";
    let mut runner = Runner::compile(base_src).unwrap();
    let base = runner.run(None, &[], BUDGET).unwrap();
    let table: [(&str, String, u64); 5] = [
        ("fadd", format!("fadd({pi}u32, {e}u32)"), 14_000),
        ("fsub", format!("fsub({pi}u32, {e}u32)"), 16_000),
        ("fmul", format!("fmul({pi}u32, {e}u32)"), 15_000),
        ("fdiv", format!("fdiv({pi}u32, {e}u32)"), 46_000),
        ("fsqrt", format!("fsqrt({pi}u32)"), 66_000),
    ];
    println!(
        "kernel  T-states  traps  image-bytes   (baseline {} T)",
        base.cycles
    );
    for (name, expr, ceiling) in table {
        let src = format!("fn run() -> u16 {{ ({expr} >> 16u32) as u16 }}");
        let mut runner = Runner::compile(&src).unwrap();
        let r = runner.run(None, &[], BUDGET).unwrap();
        assert!(r.returned);
        let t = r.cycles - base.cycles;
        println!(
            "{name:6} {t:>9} {traps:>6} {code:>12}",
            traps = r.trapped_ops,
            code = r.code_bytes
        );
        assert!(
            t <= ceiling,
            "{name}: {t} T-states blew the {ceiling} ceiling — re-price, don't hide (H-F2)"
        );
    }
}
