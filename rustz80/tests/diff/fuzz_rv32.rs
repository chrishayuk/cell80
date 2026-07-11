//! The rustrv32 determinism-fuzz battery (WS-B/B2, the owed `cell_fuzz`
//! discipline at the family level): seeded random straight-line programs over the
//! full width lattice (u16/i16/u32/i32, casts across the explicit bridges),
//! asserted for **cross-system agreement** — random shapes no hand-written test
//! picked — and for the RV32 determinism fingerprint (result registers, exact
//! cycles, stop, and the entire data window) across fresh executor instances.
//!
//! No rustc oracle here: the generated source stays inside the dialect's total
//! fragment (division guarded positive, constant shifts in range), and the claim
//! under test is *self-consistency of the family* — `run_program` already asserts
//! Spectrum48 ≡ Cell ≡ interpreter ≡ RV32 per program, so one call per case runs
//! the whole matrix. Divide-by-zero is excluded by construction (the one
//! pre-registered per-target divergence, docs 13 §2.1).

use crate::harness::run_program;

/// The `cell_fuzz` xorshift — fixed seeds, no `rand`, fully reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Ty {
    U16,
    I16,
    U32,
    I32,
}

impl Ty {
    fn name(self) -> &'static str {
        match self {
            Ty::U16 => "u16",
            Ty::I16 => "i16",
            Ty::U32 => "u32",
            Ty::I32 => "i32",
        }
    }
    fn wide(self) -> bool {
        matches!(self, Ty::U32 | Ty::I32)
    }
}

/// A literal of `ty` (negatives via the folding `-` prefix; ranges small enough
/// to keep division interesting without caring about overflow — everything wraps
/// identically everywhere).
fn lit(rng: &mut Rng, ty: Ty) -> String {
    match ty {
        Ty::U16 => format!("{}u16", rng.below(1000)),
        Ty::I16 => format!("{}i16", rng.below(1000) as i64 - 500),
        Ty::U32 => format!("{}u32", rng.below(100_000)),
        Ty::I32 => format!("{}i32", rng.below(200_000) as i64 - 100_000),
    }
}

/// A leaf of `ty`: a literal, a same-typed local, or a cast from another local
/// (the explicit width bridges — zero-extend, sign-extend, truncate, bit-cast).
fn leaf(rng: &mut Rng, ty: Ty, locals: &[(String, Ty)]) -> String {
    let same: Vec<&(String, Ty)> = locals.iter().filter(|(_, t)| *t == ty).collect();
    match rng.below(3) {
        0 if !same.is_empty() => same[rng.below(same.len() as u64) as usize].0.clone(),
        1 if !locals.is_empty() => {
            let (name, from) = &locals[rng.below(locals.len() as u64) as usize];
            if *from == ty {
                name.clone()
            } else {
                // `i16 as u16` takes the bits; wide→16 truncates; 16→wide extends
                // by the source's signedness; wide↔wide is a bit identity.
                format!("({name} as {})", ty.name())
            }
        }
        _ => lit(rng, ty),
    }
}

/// A binary expression of `ty` over leaves. Division is guarded strictly positive
/// (`& mask | 1`), so `/ 0` — and the i32 `MIN / -1` corner — never occur; shifts
/// are constant and in range.
fn expr(rng: &mut Rng, ty: Ty, locals: &[(String, Ty)]) -> String {
    let l = leaf(rng, ty, locals);
    let r = leaf(rng, ty, locals);
    let t = ty.name();
    match rng.below(10) {
        0 => format!("({l} + {r})"),
        1 => format!("({l} - {r})"),
        2 => format!("({l} * {r})"),
        3 => format!("({l} / (({r} & 63{t}) | 1{t}))"),
        4 => format!("({l} % (({r} & 63{t}) | 1{t}))"),
        5 => format!("({l} & {r})"),
        6 => format!("({l} | {r})"),
        7 => format!("({l} ^ {r})"),
        8 => {
            let k = rng.below(if ty.wide() { 31 } else { 15 });
            if rng.below(2) == 0 {
                format!("({l} << {k})")
            } else {
                format!("({l} >> {k})")
            }
        }
        _ => {
            // A comparison materialised and widened back into the lane — the
            // signed/unsigned ordering paths under random operands.
            let cmp = ["<", "<=", ">", ">=", "==", "!="][rng.below(6) as usize];
            format!("((({l} {cmp} {r}) as u16) as {t})")
        }
    }
}

/// A straight-line program: `n` typed lets over the growing scope, folded to one
/// u16 (every local reaches the result, so nothing generated is dead).
fn gen_program(rng: &mut Rng, n: usize, tys: &[Ty]) -> String {
    let mut locals: Vec<(String, Ty)> = Vec::new();
    let mut body = String::new();
    for i in 0..n {
        let ty = tys[rng.below(tys.len() as u64) as usize];
        let name = format!("v{i}");
        body.push_str(&format!(
            "    let {name}: {} = {};\n",
            ty.name(),
            expr(rng, ty, &locals)
        ));
        locals.push((name, ty));
    }
    let fold = locals
        .iter()
        .map(|(name, ty)| match ty {
            Ty::U16 => name.clone(),
            Ty::I16 => format!("({name} as u16)"),
            _ => format!("(({name}) as u16)"),
        })
        .collect::<Vec<_>>()
        .join(" ^ ");
    format!("fn run() -> u16 {{\n{body}    {fold}\n}}")
}

/// Z80-legal lanes (no i32): every program runs the full five-system matrix —
/// Spectrum48, Cell, the IR interpreter, and RV32 must all agree per program
/// (`run_program` asserts the matrix internally).
#[test]
fn family_agreement_fuzz() {
    for seed in 1..=120u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let src = gen_program(&mut rng, 8, &[Ty::U16, Ty::I16, Ty::U32]);
        run_program(&src, "run"); // panics with the diverging pair + source
    }
}

/// The i32 lane: backend zero refuses these (the signed-32 gate), so agreement is
/// interpreter ≡ RV32 — random signed-32 shapes on the one machine backend that
/// has them.
#[test]
fn signed32_agreement_fuzz() {
    for seed in 1..=80u64 {
        let mut rng = Rng(seed.wrapping_mul(0xD134_2543_DE82_EF95) | 1);
        let src = gen_program(&mut rng, 8, &[Ty::U16, Ty::I16, Ty::U32, Ty::I32]);
        let ir = rustz80::interp_program(&src, "run")
            .unwrap_or_else(|e| panic!("interp failed: {e}\nsrc: {src}"));
        let rv = crate::harness::run_rv32(&src, "run", &[], &[]);
        assert_eq!(
            rv, ir[0],
            "RV32 vs IR interpreter diverged\nsrc: {src}\n  rv32={rv} ir={}",
            ir[0]
        );
    }
}

/// The determinism fingerprint at RV32: same image on a fresh executor ⇒ identical
/// result registers, **exact** cycle count, stop reason, and the entire 64 KiB
/// data window — the `Snapshot` discipline (`cell_fuzz.rs`) at the new backend.
#[test]
fn rv32_determinism_fingerprint_fuzz() {
    for seed in 1..=40u64 {
        let mut rng = Rng(seed.wrapping_mul(0xA076_1D64_78BD_642F) | 1);
        let src = gen_program(&mut rng, 8, &[Ty::U16, Ty::I16, Ty::U32, Ty::I32]);
        let file: syn::File = syn::parse_str(&src).unwrap();
        let lowered =
            rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default()).unwrap();
        let image = rustrv32::compile(&lowered.funcs, &lowered.const_data())
            .unwrap_or_else(|e| panic!("rv32 compile failed: {e}\nsrc: {src}"));
        let entry = image.symbols["run"];
        let first = rustrv32::run_cell(&image.code, &image.consts, entry, &[], &[], 10_000_000);
        for _ in 0..2 {
            let again = rustrv32::run_cell(&image.code, &image.consts, entry, &[], &[], 10_000_000);
            assert_eq!(first.0, again.0, "result registers drifted\nsrc: {src}");
            assert_eq!(first.1, again.1, "cycle count drifted\nsrc: {src}");
            assert_eq!(first.2, again.2, "stop reason drifted\nsrc: {src}");
            assert_eq!(first.3, again.3, "data window drifted\nsrc: {src}");
        }
    }
}
