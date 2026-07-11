//! Property/fuzz tests for the cell's headline guarantees — determinism and reset
//! completeness — stated adversarially rather than as a few hand-written samples
//! (behind `--features cell`). Seeded + reproducible (a fixed corpus, no external deps).

use cell80::{CellConfig, CellPool, CellProgram, Halt, Runner, StateCell, DEFAULT_CYCLES};

/// A tiny deterministic xorshift PRNG — so the corpus is reproducible (and `cargo test`
/// stays free of `rand`).
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

/// Generate a random **straight-line** arithmetic body over `a`, `b`, and constants — the
/// fast-path-eligible shape. Cell arithmetic wraps (it's raw Z80), and `/0`/`%0` are defined
/// (`0xFFFF`), so every generated program is valid and total; we test self-consistency, not
/// equivalence to rustc (that's `diff.rs`).
fn gen_expr(rng: &mut Rng, depth: u32) -> String {
    if depth == 0 || rng.below(3) == 0 {
        return match rng.below(3) {
            0 => "a".into(),
            1 => "b".into(),
            _ => format!("{}u16", rng.below(1000)),
        };
    }
    let l = gen_expr(rng, depth - 1);
    let r = gen_expr(rng, depth - 1);
    match rng.below(8) {
        0 => format!("({l}).wrapping_add({r})"),
        1 => format!("({l}).wrapping_sub({r})"),
        2 => format!("({l}).wrapping_mul({r})"),
        3 => format!("(({l}) / (({r}) | 1u16))"),
        4 => format!("(({l}) % (({r}) | 1u16))"),
        5 => format!("(({l}) & ({r}))"),
        6 => format!("(({l}) | ({r}))"),
        _ => format!("(({l}) ^ ({r}))"),
    }
}

fn gen_program(rng: &mut Rng) -> String {
    format!("fn run(a: u16, b: u16) -> u16 {{ {} }}", gen_expr(rng, 4))
}

/// `(result, cycles, halt, touched)` — the full deterministic fingerprint of a run.
type Snapshot = (u16, u64, Halt, Vec<(u16, u16)>);
fn snapshot(r: &mut Runner, args: &[u16]) -> Snapshot {
    let rep = r.run(None, args, DEFAULT_CYCLES).unwrap();
    (rep.result, rep.cycles, rep.halt, rep.touched)
}

#[test]
fn state_named_roundtrip_fuzz() {
    // The B3 seam, end to end through the NAMED layer — the field-name↔Z80-memory mapping the
    // JSON/MCP path rides on. For a struct cell over 500 random inputs: set inputs *by name*,
    // confirm they round-trip through memory *by name* (the input half), run, then read outputs
    // *by name* and check against a host oracle (the output half) — the whole loop as one
    // property, not the two halves separately. Also re-bind-free reuse (no leak between runs).
    let src = "struct S { a: u16, b: u16, sum: u16, diff: u16, prod: u16, big: u16 }
               impl S {
                   fn run(&mut self) -> u16 {
                       self.sum = self.a.wrapping_add(self.b);
                       self.diff = self.a.wrapping_sub(self.b);
                       self.prod = self.a.wrapping_mul(self.b);
                       if self.a > self.b { self.big = self.a; } else { self.big = self.b; }
                       self.sum
                   }
               }";
    let mut cell = StateCell::bind(src, "S", None).unwrap();
    let mut rng = Rng(0x5747_29f3_1b2d_c4e5);
    for _ in 0..500 {
        let a = (rng.next() & 0xFFFF) as u16;
        let b = (rng.next() & 0xFFFF) as u16;
        cell.set("a", a as u64).unwrap(); // queued; applied to memory by run()
        cell.set("b", b as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        // input half: the queued inputs landed in memory and read back by name unchanged
        assert_eq!(
            cell.get("a"),
            Some(a as u64),
            "input `a` did not round-trip by name"
        );
        assert_eq!(
            cell.get("b"),
            Some(b as u64),
            "input `b` did not round-trip by name"
        );
        // output half: every output (incl. one written last) read back by name == host oracle
        assert_eq!(
            cell.get("sum"),
            Some(a.wrapping_add(b) as u64),
            "sum ({a},{b})"
        );
        assert_eq!(
            cell.get("diff"),
            Some(a.wrapping_sub(b) as u64),
            "diff ({a},{b})"
        );
        assert_eq!(
            cell.get("prod"),
            Some(a.wrapping_mul(b) as u64),
            "prod ({a},{b})"
        );
        assert_eq!(cell.get("big"), Some(a.max(b) as u64), "big ({a},{b})");
    }
}

#[test]
fn array_state_named_roundtrip_fuzz() {
    // The array sibling of `state_named_roundtrip_fuzz` (`.cell` v11): mixed
    // `[u16; 4]` / `[u32; 2]` / scalar state over 500 random inputs — set arrays
    // *by name*, confirm the input half round-trips element-exactly, run, then
    // check the cell's outputs against a host oracle. Short arrays must zero-fill
    // their envelope; the wide array must keep full 32-bit elements.
    let src = "struct A { xs: [u16; 4], ws: [u32; 2], k: u16, sum: u32, mixed: u32 }
               impl A {
                   fn run(&mut self) -> u16 {
                       self.sum = (self.xs[0] as u32) + (self.xs[1] as u32)
                           + (self.xs[2] as u32) + (self.xs[3] as u32);
                       self.mixed = self.ws[0] ^ self.ws[1] ^ (self.k as u32);
                       (self.sum & 0xFFFFu32) as u16
                   }
               }";
    let mut cell = StateCell::bind(src, "A", None).unwrap();
    let mut rng = Rng(0x0be1_77ab_4451_9d03);
    for i in 0..500 {
        let n = 1 + (rng.below(4) as usize); // 1..=4 supplied elements
        let xs: Vec<u64> = (0..n).map(|_| rng.next() & 0xFFFF).collect();
        let ws: Vec<u64> = (0..2).map(|_| rng.next() & 0xFFFF_FFFF).collect();
        let k = rng.next() & 0xFFFF;
        cell.set_array("xs", &xs).unwrap();
        cell.set_array("ws", &ws).unwrap();
        cell.set("k", k).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        // Input half: the envelope reads back element-exactly, short arrays
        // zero-filled (the reset makes unsupplied elements 0 by construction).
        let mut want_xs = xs.clone();
        want_xs.resize(4, 0);
        assert_eq!(cell.get_array("xs"), Some(want_xs.clone()), "iter {i}");
        assert_eq!(cell.get_array("ws"), Some(ws.clone()), "iter {i}");
        // Output half vs the host oracle.
        assert_eq!(
            cell.get("sum"),
            Some(want_xs.iter().sum::<u64>()),
            "sum, iter {i}"
        );
        assert_eq!(
            cell.get("mixed"),
            Some(ws[0] ^ ws[1] ^ k),
            "mixed, iter {i}"
        );
    }
}

#[test]
fn determinism_fuzz() {
    // For random programs × random inputs, the fingerprint `(result, cycles, halt, touched)`
    // must be bit-identical across: (a) re-run on the same Runner, (b) a fresh Runner, (c) a
    // Runner from an image round-tripped through to_bytes/from_bytes; and the fast executor
    // (run_fast / run_many_fast) must agree with the authentic interpreter on result/cycles/halt.
    let inputs: [[u16; 2]; 7] = [
        [0, 0],
        [1, 1],
        [7, 3],
        [0xFFFF, 1],
        [0x8000, 0x8000],
        [40000, 9999],
        [255, 256],
    ];
    for seed in 1..=40u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let src = gen_program(&mut rng);
        let prog = CellProgram::compile(&src).unwrap();
        let image = prog.to_bytes();
        let reloaded = CellProgram::from_bytes(&image).unwrap();

        let mut r1 = Runner::new(&prog);
        for inp in &inputs {
            let base = snapshot(&mut r1, inp);
            // (a) re-run on the same warm Runner.
            assert_eq!(
                snapshot(&mut r1, inp),
                base,
                "rerun diverged\n{src}\n{inp:?}"
            );
            // (b) a fresh Runner.
            assert_eq!(
                snapshot(&mut Runner::new(&prog), inp),
                base,
                "fresh runner diverged\n{src}\n{inp:?}"
            );
            // (c) a Runner from a round-tripped image.
            assert_eq!(
                snapshot(&mut Runner::new(&reloaded), inp),
                base,
                "image round-trip diverged\n{src}\n{inp:?}"
            );
            // (d) fast executor (single + batch) vs the authentic interpreter.
            let f = r1.run_fast(None, inp, DEFAULT_CYCLES).unwrap();
            assert_eq!(
                (f.result, f.cycles, f.halt),
                (base.0, base.1, base.2),
                "run_fast diverged\n{src}\n{inp:?}"
            );
            let many = r1.run_many_fast(None, &[inp], DEFAULT_CYCLES).unwrap();
            assert_eq!(
                (many[0].result, many[0].cycles, many[0].halt),
                (base.0, base.1, base.2),
                "run_many_fast diverged\n{src}\n{inp:?}"
            );
        }
    }
}

#[test]
fn reset_completeness_across_programs() {
    // The Runner resets only the bytes a run *wrote*. Prove that's complete across **all**
    // 64K when a pooled bus is reused by a *different* program: a writer scribbles three
    // high addresses, then a probe (on the recycled bus) reads them back — it must see the
    // same clean zeros as on a fresh bus. A reset leak would surface here as a non-zero sum.
    let writer = CellProgram::compile_with_config(
        "fn run() -> u16 { poke(0xC000u16, 0xABu8); poke(0xD000u16, 0xCDu8); poke(0xE000u16, 0xEFu8); 0u16 }",
        CellConfig::permissive(),
    )
    .unwrap();
    let probe = CellProgram::compile_with_config(
        "fn run() -> u16 { peek(0xC000u16) as u16 + peek(0xD000u16) as u16 + peek(0xE000u16) as u16 }",
        CellConfig::permissive(),
    )
    .unwrap();

    // Fresh-bus baseline: nothing was ever written there → 0.
    let fresh = Runner::new(&probe)
        .run(None, &[], DEFAULT_CYCLES)
        .unwrap()
        .result;
    assert_eq!(fresh, 0);

    // Reuse the *same* bus across many writer→probe alternations.
    let mut pool = CellPool::new();
    for _ in 0..8 {
        let mut w = pool.acquire(&writer);
        assert_eq!(w.run(None, &[], DEFAULT_CYCLES).unwrap().result, 0);
        pool.release(w);

        let mut p = pool.acquire(&probe); // recycles the bus the writer just used
        let reused = p.run(None, &[], DEFAULT_CYCLES).unwrap().result;
        pool.release(p);
        assert_eq!(
            reused, fresh,
            "reset leaked the writer's high-memory writes into the probe"
        );
    }
}
