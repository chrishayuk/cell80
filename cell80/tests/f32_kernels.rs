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
///
/// Banked negative (2026-07-07): a barrel-decomposed `f32_shr_jam` (test-and-shift by
/// 16/8/4/2/1) measured *worse* than the per-bit loop on the typical profile — fadd
/// 12,406 vs 10,854 T and +636 B — because real alignments are small (same-magnitude
/// adds shift 0-2) and the `n > 31` early-out already caps the tail. The loop stays.
#[test]
fn f32_kernel_cost_envelope() {
    let pi = std::f32::consts::PI.to_bits();
    let e = std::f32::consts::E.to_bits();
    // Baseline driver shape: same call/return plumbing, no kernel.
    let base_src = "fn run() -> u16 { let x = 42u32; (x >> 16u32) as u16 }";
    let mut runner = Runner::compile(base_src).unwrap();
    let base = runner.run(None, &[], BUDGET).unwrap();
    let table: [(&str, String, u64); 9] = [
        ("fadd", format!("fadd({pi}u32, {e}u32)"), 14_000),
        ("fsub", format!("fsub({pi}u32, {e}u32)"), 16_000),
        ("fmul", format!("fmul({pi}u32, {e}u32)"), 15_000),
        ("fdiv", format!("fdiv({pi}u32, {e}u32)"), 46_000),
        ("fsqrt", format!("fsqrt({pi}u32)"), 66_000),
        ("ftrunc", format!("ftrunc({pi}u32)"), 6_000),
        ("ffloor", format!("ffloor({pi}u32)"), 6_000),
        ("fround", format!("fround({pi}u32)"), 6_000),
        ("fmin", format!("fmin({pi}u32, {e}u32)"), 8_000),
    ];
    // The typed conversions drive through the typed surface (their args/returns are
    // f32-typed by interception); `.is_finite()` is inline bit-compares, ~noise.
    let typed: [(&str, &str, u64); 3] = [
        (
            "int_to_f32",
            "fn run() -> u16 { let v = int_to_f32(123456789u32); let mut r = 0u16; if v.is_finite() { r = 1u16; } r }",
            14_000, // the normalize loop is value-dependent: up to 31 shift-by-1 steps
        ),
        (
            "q16_to_f32",
            "fn run() -> u16 { let v = q16_to_f32(98304u32); let mut r = 0u16; if v.is_finite() { r = 1u16; } r }",
            14_000,
        ),
        (
            "f32_to_int",
            "fn run() -> u16 { (f32_to_int_trunc(3.140625f32) >> 16u32) as u16 }",
            18_000, // value-dependent: up to 23 shift-by-1 steps to strip the fraction
        ),
    ];
    println!(
        "kernel  T-states  traps  image-bytes   (baseline {} T)",
        base.cycles
    );
    let typed_rows = typed
        .iter()
        .map(|(n, src, c)| (*n, src.to_string(), *c))
        .collect::<Vec<_>>();
    let all = table
        .into_iter()
        .map(|(n, expr, c)| {
            (
                n,
                format!("fn run() -> u16 {{ ({expr} >> 16u32) as u16 }}"),
                c,
            )
        })
        .chain(typed_rows);
    for (name, src, ceiling) in all {
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

/// The F0.4 boundary contract end-to-end (`finite_result`, `.cell` v8): an
/// f32-returning cell whose result is non-finite escalates typed through the host —
/// Inf never arrives wearing an answer's clothes — while `finite_result: off` opts
/// an IEEE-plumbing cell out, non-f32 entries are inert, and the flag survives the
/// wire format round-trip.
#[test]
fn finite_result_boundary_contract() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost, Halt};
    let mk = |id: &str, src: &str, finite: Option<bool>| {
        Cartridge::compile(
            src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.to_string()),
                summary: format!("{id} (finite_result test)"),
                finite_result: finite,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{id}: {e}"))
    };
    let mut host = CellHost::new();
    // 3e38 * 10 overflows to +Inf; 0/0 is NaN; both propagate IEEE *inside* the cell.
    host.add(mk(
        "inf",
        "fn run() -> f32 { 300000000000000000000000000000000000000.0f32 * 10.0f32 }",
        None,
    ));
    host.add(mk("nan", "fn run() -> f32 { 0.0f32 / 0.0f32 }", None));
    host.add(mk("fine", "fn run() -> f32 { 1.5f32 + 2.0f32 }", None));
    host.add(mk(
        "nan_ok",
        "fn run() -> f32 { 0.0f32 / 0.0f32 }",
        Some(false),
    ));
    host.add(mk("int_inert", "fn run() -> u16 { 0x7FC0u16 }", None));
    let cases = [
        ("inf", Halt::Escalate(0xFF07)),
        ("nan", Halt::Escalate(0xFF08)),
        ("fine", Halt::Returned),
        ("nan_ok", Halt::Returned), // opted out: the canonical NaN bits ARE the answer
        ("int_inert", Halt::Returned),
    ];
    for (id, want) in cases {
        let h = host.load(id).unwrap();
        let r = host.run(h, &[], &[], BUDGET).unwrap();
        assert_eq!(r.halt, want, "{id}");
        if id == "fine" {
            let bits = r.regs[0] as u32 | (r.regs[1] as u32) << 16;
            assert_eq!(bits, 3.5f32.to_bits());
        }
        if id == "nan_ok" {
            let bits = r.regs[0] as u32 | (r.regs[1] as u32) << 16;
            assert_eq!(
                bits, 0x7FC0_0000,
                "opt-out returns the canonical NaN itself"
            );
        }
    }
    // the flag survives the .cell round-trip (v8)
    let cart = mk("rt", "fn run() -> f32 { 1.0f32 + 1.0f32 }", Some(false));
    let back = Cartridge::from_bytes(&cart.to_bytes()).unwrap();
    assert!(!back.manifest.finite_result);
    let cart = mk("rt2", "fn run() -> f32 { 1.0f32 + 1.0f32 }", None);
    let back = Cartridge::from_bytes(&cart.to_bytes()).unwrap();
    assert!(back.manifest.finite_result, "default is on");
}

/// The conversion pair's typed domain halt (`0xFF08 float_domain`): out-of-range and
/// NaN inputs refuse instead of wrapping — deliberate boundary behaviour, *not*
/// rustc's saturating cast (the one documented divergence in the family).
#[test]
fn conversion_domain_halts() {
    let cases = [
        "fn run() -> u16 { (f32_to_int_trunc(100000000000000000000.0f32) >> 16u32) as u16 }",
        "fn run() -> u16 { (f32_to_int_trunc(0.0f32 / 0.0f32) >> 16u32) as u16 }",
        "fn run() -> u16 { (f32_to_int_trunc(-2.5f32) >> 16u32) as u16 }",
        "fn run() -> u16 { (f32_to_q16(65536.0f32) >> 16u32) as u16 }",
    ];
    for src in cases {
        let mut runner = Runner::compile(src).unwrap();
        let r = runner.run(None, &[], BUDGET).unwrap();
        assert_eq!(
            r.halt,
            cell80::Halt::Escalate(0xFF08),
            "expected float_domain: {src}"
        );
    }
}
