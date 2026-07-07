//! The F0 softfloat oracle bank (H-F1): every basic-op kernel in
//! `rustz80::F32_KERNELS` is **bit-identical** to rustc's f32 — on both compile
//! targets, post NaN-canonicalization, across an enumerated edge bank and a seeded
//! random bank. Bit equality, not tolerance: correctly-rounded ops don't need a
//! band, and a band is how float testing lies to itself. A reproducible mismatch
//! here is a kernel bug (or a rustc bug); no f32 cell is admitted while one is open.

use crate::harness::run_program_pruned;

/// NaN-class canonicalization: any NaN → the blessed quiet NaN. The kernels only
/// ever *produce* this pattern; the host side needs the fold because hardware NaN
/// payload propagation differs across x86/ARM.
fn canon(x: u32) -> u32 {
    if x & 0x7FFF_FFFF > 0x7F80_0000 {
        0x7FC0_0000
    } else {
        x
    }
}

/// The enumerated edge bank: zeros, subnormal min/max/mid, normal boundary and its
/// neighbours, mantissa-LSB neighbours of 1.0, halfway/tie generators against 1.0
/// (`0x3380_0000` = 2^-24), exponent extremes, ±Inf, NaN payload variants — the
/// Berkeley TestFloat case families, enumerated harness-side.
const EDGES: [u32; 40] = [
    0x00000000, 0x80000000, 0x00000001, 0x80000001, 0x00000002, 0x007FFFFF, 0x807FFFFF, 0x00400000,
    0x00800000, 0x80800000, 0x00800001, 0x00FFFFFF, 0x33800000, 0x33800001, 0xB3800000, 0x34000000,
    0x3F000000, 0x3F7FFFFF, 0x3F800000, 0xBF800000, 0x3F800001, 0x3F800002, 0x3FC00000, 0x40000000,
    0x40490FDB, 0x3EAAAAAB, 0x4B800000, 0x4F000000, 0x5F800000, 0x7F000000, 0x7F7FFFFF, 0xFF7FFFFF,
    0x7F800000, 0xFF800000, 0x7FC00000, 0xFFC00000, 0x7F800001, 0x7FFFFFFF, 0x4CBEBC20, 0x501502F9,
];

const OPS: [&str; 7] = ["fadd", "fsub", "fmul", "fdiv", "feq", "flt", "fle"];

/// The rustc oracle: the same operation on host f32, canonicalized. This is the
/// whole point of owning binary32 — the reference is bit-specified and portable.
fn expect(op: &str, a: u32, b: u32) -> u32 {
    let (fa, fb) = (f32::from_bits(a), f32::from_bits(b));
    // The comparison trio returns 0/1 with Rust semantics (NaN false, -0 == +0);
    // `>`/`>=` lower as swapped `flt`/`fle`, so the trio covers all six.
    match op {
        "feq" => return (fa == fb) as u32,
        "flt" => return (fa < fb) as u32,
        "fle" => return (fa <= fb) as u32,
        _ => {}
    }
    canon(match op {
        "fadd" => (fa + fb).to_bits(),
        "fsub" => (fa - fb).to_bits(),
        "fmul" => (fa * fb).to_bits(),
        "fdiv" => (fa / fb).to_bits(),
        "fsqrt" => fa.sqrt().to_bits(),
        _ => unreachable!("unknown op {op}"),
    })
}

fn case_expr(op: &str, a: u32, b: u32) -> String {
    if op == "fsqrt" {
        format!("fsqrt({a}u32)")
    } else {
        format!("{op}({a}u32, {b}u32)")
    }
}

/// Run `(op, a, b)` cases chunked — each chunk one compiled program counting
/// mismatches against rustc-computed expected bits, run on both targets. Chunks are
/// grouped by op and DCE-pruned so each image carries only the kernel under test
/// (the code window is 4KB; the full kernel set doesn't fit beside a driver). A bad
/// chunk re-runs its cases singly to name the exact divergence.
fn run_bank(cases: &[(&str, u32, u32)]) {
    let mut by_op: std::collections::BTreeMap<&str, Vec<(&str, u32, u32)>> = Default::default();
    for c in cases {
        by_op.entry(c.0).or_default().push(*c);
    }
    for chunk in by_op.values().flat_map(|v| v.chunks(8)) {
        let mut body = String::from("fn f() -> u16 { let mut bad = 0u16;\n");
        for (op, a, b) in chunk {
            let e = expect(op, *a, *b);
            body.push_str(&format!(
                "if {} != {e}u32 {{ bad = bad + 1u16; }}\n",
                case_expr(op, *a, *b)
            ));
        }
        body.push_str("bad }\n");
        let src = format!("{body}{}", rustz80::F32_KERNELS);
        if run_program_pruned(&src, "f") != 0 {
            for (op, a, b) in chunk {
                let e = expect(op, *a, *b);
                let single = format!(
                    "fn f() -> u16 {{ let mut bad = 0u16; if {} != {e}u32 {{ bad = 1u16; }} bad }}\n{}",
                    case_expr(op, *a, *b),
                    rustz80::F32_KERNELS
                );
                assert_eq!(
                    run_program_pruned(&single, "f"),
                    0,
                    "{op}(0x{a:08X}, 0x{b:08X}) diverged from rustc f32 (expected bits 0x{e:08X})"
                );
            }
            panic!("chunk counted failures but every single passed — chunking bug");
        }
    }
}

/// Edge × edge, exhaustive, all five ops.
#[test]
fn f32_edge_bank() {
    let mut cases = Vec::new();
    for &a in &EDGES {
        for &b in &EDGES {
            for op in OPS {
                cases.push((op, a, b));
            }
        }
        cases.push(("fsqrt", a, 0));
    }
    run_bank(&cases);
}

/// Multi-kernel composition: `lerp(a, b, t) = a + t*(b - a)` chains three kernels
/// (fsub → fmul → fadd, ~8KB with helpers) in one program — the shape the classic
/// fixed-`0x9000` scratch window rejected. Scratch now places above the code, so the
/// chain must compile, run on both targets, and stay bit-identical to rustc.
#[test]
fn f32_multi_kernel_chain() {
    let cases: [(f32, f32, f32); 6] = [
        (0.0, 1.0, 0.5),
        (1.0, 2.0, 0.25),
        (-3.5, 7.25, 0.75),
        (1.0e-38, 1.0, 0.125),  // subnormal-adjacent low end
        (3.0e38, -3.0e38, 0.5), // overflow-adjacent high end
        (2.5, 2.5, 1.0),
    ];
    for (a, b, t) in cases {
        let want = canon((a + t * (b - a)).to_bits());
        let (ab, bb, tb) = (a.to_bits(), b.to_bits(), t.to_bits());
        let src = format!(
            "fn f() -> u16 {{ let mut bad = 0u16; \
             if fadd({ab}u32, fmul({tb}u32, fsub({bb}u32, {ab}u32))) != {want}u32 \
             {{ bad = 1u16; }} bad }}\n{}",
            rustz80::F32_KERNELS
        );
        assert_eq!(
            run_program_pruned(&src, "f"),
            0,
            "lerp({a}, {b}, {t}) diverged from rustc (expected bits 0x{want:08X})"
        );
    }
}

/// Seeded random bank: full-random pairs, nearby-exponent pairs (cancellation
/// stress), and subnormal pairs. Deterministic LCG — same bank every run.
#[test]
fn f32_random_bank() {
    let mut state = 0x5EED_CAFE_F00D_D00Du64;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 32) as u32
    };
    let mut cases = Vec::new();
    for _ in 0..96 {
        // full-random
        let (a, b) = (next(), next());
        for op in OPS {
            cases.push((op, a, b));
        }
        cases.push(("fsqrt", a, 0));
        // nearby exponents — the subtract path's cancellation + normalize stress
        let a = next();
        let ea = ((a >> 23) & 0xFF) as i32;
        let eb = (ea + (next() % 7) as i32 - 3).clamp(0, 254) as u32;
        let b = (next() & 0x807F_FFFF) | (eb << 23);
        for op in OPS {
            cases.push((op, a, b));
        }
        // subnormal / tiny-exponent operands — gradual under/overflow paths
        let (a, b) = (next() & 0x80FF_FFFF, next() & 0x80FF_FFFF);
        for op in OPS {
            cases.push((op, a, b));
        }
        cases.push(("fsqrt", a, 0));
    }
    run_bank(&cases);
}

/// The typed surface, single-source: real f32 programs — literals (compile-time
/// RNE decimal→binary32, same bits rustc gives the token), operator routing to the
/// kernels, comparisons with NaN/zero semantics, unary neg, `.sqrt()`/`.abs()`,
/// f32 params and returns — run on both targets and match a rustc-computed oracle.
/// The kernels auto-append: no prelude text in these sources.
#[test]
fn f32_typed_surface() {
    // (source, expected) — expected computed by the same expressions in host f32.
    #[allow(clippy::excessive_precision)]
    let host = {
        let a = 1.5f32;
        let b = 2.5f32;
        let p = a * b;
        let s = p + -0.75f32;
        let q = s.sqrt();
        (q > 1.0f32 && p == 3.75f32) as u16
    };
    let src = "fn f() -> u16 {
        let a = 1.5f32;
        let b = 2.5f32;
        let p = a * b;
        let s = p + -0.75f32;
        let q = s.sqrt();
        let mut r = 0u16;
        if q > 1.0f32 && p == 3.75f32 { r = 1u16; }
        r
    }";
    assert_eq!(run_program_pruned(src, "f"), host);

    // f32 params + f32 return: the wide convention carries bits, typed
    let src = "fn lerp(a: f32, t: f32) -> f32 { a + t * (10.0f32 - a) }
               fn f() -> u16 { let mut ok = 0u16; if lerp(2.0f32, 0.25f32) == 4.0f32 { ok = 1u16; } ok }";
    assert_eq!(run_program_pruned(src, "f"), 1);

    // the rounding-folklore witness: in binary32 `0.1 + 0.2 == 0.3` happens to be
    // *true* (unlike f64) — what matters is that the kernels agree with rustc on
    // it, whichever way the rounding falls
    let host = ((0.1f32 + 0.2f32).to_bits() == 0.3f32.to_bits()) as u16;
    let src = "fn f() -> u16 { let mut r = 0u16; if 0.1f32 + 0.2f32 == 0.3f32 { r = 1u16; } r }";
    assert_eq!(run_program_pruned(src, "f"), host);

    // abs + division + <= ; also -0.0 == 0.0 (feq's signed-zero rule)
    let host = {
        let x = (-7.5f32).abs() / 2.0f32;
        (x <= 3.75f32 && -0.0f32 == 0.0f32) as u16
    };
    let src = "fn f() -> u16 {
        let x = (-7.5f32).abs() / 2.0f32;
        let mut r = 0u16;
        if x <= 3.75f32 && -0.0f32 == 0.0f32 { r = 1u16; }
        r
    }";
    assert_eq!(run_program_pruned(src, "f"), host);
}

/// The repr discipline: f32 and integers never mix silently — every cross is a
/// clean compile error naming the rule, never a silent bit-pattern operation.
/// (This is the type-flow gate model-composed float cells depend on.)
#[test]
fn f32_never_mixes_with_integers() {
    let rejects = [
        "fn f(a: f32) -> f32 { a + 1u32 }",          // int rhs
        "fn f(a: f32) -> f32 { a + 1u16 }",          // 16-bit rhs
        "fn f(a: u32) -> u32 { a + 1.5f32 }",        // float rhs on u32
        "fn f(a: f32) -> u16 { a as u16 }",          // cast out
        "fn f(a: u16) -> f32 { a as f32 }",          // cast in
        "fn f(a: f32) -> f32 { a % 2.0f32 }",        // % undefined
        "fn f(a: f32) -> f32 { a << 1u32 }",         // shifts undefined
        "fn f(a: f32) -> f32 { a.wrapping_add(a) }", // integer method
        "fn f(a: f32) -> u32 { a }",                 // f32 bits posing as u32
        "fn f(a: u32) -> f32 { a }",                 // u32 posing as f32
        "fn f(a: f32) -> f32 { let x: u32 = a; x }", // annotation cross
        "fn f(a: u32) -> f32 { let x: f32 = a; x }", // annotation cross
        "fn f(a: f32) -> u16 { let mut r = 0u16; if a == 1u32 { r = 1u16; } r }", // cmp cross
        "fn f() -> f32 { 1.5f64 }",                  // f64 literal
        "fn g(x: u32) -> u32 { x } fn f(a: f32) -> u32 { g(a) }", // f32 into u32 param
    ];
    for src in rejects {
        for target in crate::harness::TARGETS {
            let file: syn::File = syn::parse_str(src).expect("parses");
            assert!(
                rustz80::compile_file_pruned(&file, target, &["f"]).is_err(),
                "expected a clean rejection ({target:?}): {src}"
            );
        }
    }
}
