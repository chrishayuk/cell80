//! Tests for the `rustz80-cell` micro-VM runner (behind `--features cell`). Without the
//! feature this file compiles to nothing.

use cell80::{self as cell, Runner, Ty, DEFAULT_CYCLES};

#[test]
fn cell_program_compile_once_instantiate_cheap() {
    use cell80::{CellConfig, CellProgram};
    // Compile once → a cacheable program; instantiate many cheap runners (no re-parse).
    let prog = CellProgram::compile("fn run(a: u16, b: u16) -> u16 { a + b }").unwrap();
    assert!(prog.program().symbols.contains_key("run"));

    let mut r1 = Runner::new(&prog);
    let mut r2 = Runner::new(&prog); // independent machines from one program
    assert_eq!(r1.run(None, &[20, 22], DEFAULT_CYCLES).unwrap().result, 42);
    assert_eq!(r2.run(None, &[1, 2], DEFAULT_CYCLES).unwrap().result, 3);
    assert_eq!(r1.run(None, &[5, 5], DEFAULT_CYCLES).unwrap().result, 10); // no shared state

    // The policy travels with the compiled program.
    assert!(CellProgram::compile_with_config(
        "fn run() -> u16 { peek(0u16) as u16 }",
        CellConfig::sandboxed()
    )
    .is_err());
}

#[test]
fn state_cell_named_io() {
    use cell80::StateCell;
    // The agent surface: set named inputs → run → read named outputs, no raw addresses.
    let src = "struct State { x: u16, y: u16, score: u16 }
               impl State { fn run(&mut self) -> u16 { self.score = self.x * self.x + self.y; self.score } }";
    let mut cell = StateCell::bind(src, "State", None).unwrap(); // entry defaults to State::run
    cell.set("x", 6).unwrap();
    cell.set("y", 5).unwrap();
    let rep = cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.result, 41); // 6*6 + 5
    assert_eq!(cell.get("score"), Some(41));

    // Reuse with new inputs — no leakage (score re-zeroed by the reset before re-run).
    cell.set("x", 2).unwrap();
    cell.set("y", 3).unwrap();
    cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(cell.get("score"), Some(7)); // 2*2 + 3

    // Unknown / non-existent fields.
    assert!(cell.set("nope", 1).is_err());
    assert_eq!(cell.get("nope"), None);
    let mut names: Vec<&str> = cell.fields().collect();
    names.sort();
    assert_eq!(names, ["score", "x", "y"]);
}

#[test]
fn report_json_is_abi_versioned() {
    // The report schema leads with the ABI version, then the documented keys.
    // (v3: the buffer manifest types — `bytes[N]`/`str[N]` state fields, Phase S.)
    use cell80::ABI_VERSION;
    assert_eq!(ABI_VERSION, 3);
    let mut r = Runner::compile("fn run(a: u16, b: u16) -> u16 { a * b }").unwrap();
    let json = r.run(None, &[6, 7], DEFAULT_CYCLES).unwrap().to_json();
    assert!(
        json.starts_with(&format!("{{\"abi\":{ABI_VERSION},")),
        "got: {json}"
    );
    for key in [
        "\"entry\":\"run\"",
        "\"result\":42",
        "\"regs\":[42,",
        "\"cycles\":",
        "\"trapped_ops\":",
        "\"budget\":",
        "\"halt\":\"returned\"",
        "\"code_bytes\":",
        "\"functions\":",
        "\"symbols\":{",
        "\"memory_touched\":[",
        "\"reads\":{",
    ] {
        assert!(json.contains(key), "v1 schema missing `{key}` in {json}");
    }
}

#[test]
fn report_counts_trapped_ops() {
    // The honest cost companion to `cycles`: `mul`/`div` traps read as ~free in cycles, so
    // count them so a reward function can't be gamed by routing work through traps.
    let mut r = Runner::compile("fn run(a: u16, b: u16) -> u16 { a * b + a * b + a / b }").unwrap();
    let rep = r.run(None, &[6, 2], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.result, 27); // 12 + 12 + 3
    assert_eq!(rep.trapped_ops, 3); // two muls + one div
    assert!(rep.to_json().contains("\"trapped_ops\":3"));

    // Pure add/shift code traps nothing.
    let mut add = Runner::compile("fn run(a: u16, b: u16) -> u16 { a + b + a }").unwrap();
    assert_eq!(
        add.run(None, &[6, 2], DEFAULT_CYCLES).unwrap().trapped_ops,
        0
    );

    // The fast (batch) path reports the same count — input-independent for straight-line.
    let many = r
        .run_many_fast(None, &[&[6, 2], &[3, 3]], DEFAULT_CYCLES)
        .unwrap();
    assert!(many.iter().all(|f| f.trapped_ops == 3));
}

#[test]
fn logical_and_is_short_circuit() {
    // `&&` must not evaluate its right operand once the left decides the result — proven
    // via `trapped_ops`: the guarded `a / b` is a divide trap, so a true short-circuit
    // skips it. `b != 0 && a / b > 1` with `b == 0` must run zero traps (and not divide
    // by zero), while `b != 0` evaluates the divide.
    let mut r = Runner::compile(
        "fn run(b: u16, a: u16) -> u16 { let mut r = 0u16; if b != 0u16 && a / b > 1u16 { r = 1u16; } r }",
    )
    .unwrap();
    let zero = r.run(None, &[0, 10], DEFAULT_CYCLES).unwrap();
    assert_eq!(zero.result, 0); // guard false → body skipped
    assert_eq!(zero.trapped_ops, 0, "`a / b` must be short-circuited away");
    let live = r.run(None, &[3, 10], DEFAULT_CYCLES).unwrap();
    assert_eq!(live.result, 1); // 3 != 0 && 10/3 = 3 > 1
    assert_eq!(
        live.trapped_ops, 1,
        "the divide runs when the guard is true"
    );
}

#[test]
fn variable_shift_saturates_past_word() {
    // A runtime shift amount ≥ 16 shifts a u16 entirely out to 0 (the cell's defined
    // behaviour; rustc would panic, so this lives here, not in the differential suite).
    let mut shl = Runner::compile("fn run(x: u16, s: u16) -> u16 { x << s }").unwrap();
    assert_eq!(shl.run(None, &[1, 0], DEFAULT_CYCLES).unwrap().result, 1);
    assert_eq!(
        shl.run(None, &[1, 15], DEFAULT_CYCLES).unwrap().result,
        32768
    );
    assert_eq!(shl.run(None, &[1, 16], DEFAULT_CYCLES).unwrap().result, 0);
    let mut shr = Runner::compile("fn run(x: u16, s: u16) -> u16 { x >> s }").unwrap();
    assert_eq!(
        shr.run(None, &[0x8000, 15], DEFAULT_CYCLES).unwrap().result,
        1
    );
    assert_eq!(
        shr.run(None, &[0xFFFF, 16], DEFAULT_CYCLES).unwrap().result,
        0
    );
}

#[test]
fn prelude_kernels_are_shared_and_dce_pruned() {
    use cell80::CellProgram;
    // A cell may call a shared kernel (`gcd`) it never defines — it comes from the appended
    // prelude. The cell stays "super modular": it doesn't re-implement the kernel.
    let mut r = Runner::compile("fn run(a: u16, b: u16) -> u16 { gcd(a, b) }").unwrap();
    assert_eq!(r.run(None, &[48, 36], DEFAULT_CYCLES).unwrap().result, 12);
    // DCE pulls in ONLY the reached kernel: `run` + `gcd` = 2 symbols (the other five
    // prelude kernels — imin/imax/iabs_diff/isqrt/clamp_to — are pruned).
    assert_eq!(r.program().symbols.len(), 2);

    // A cell that calls two kernels carries exactly those two (+ its own `run`).
    let two =
        CellProgram::compile("fn run(a: u16, b: u16) -> u16 { imin(a, b) + imax(a, b) }").unwrap();
    assert_eq!(two.program().symbols.len(), 3);

    // A cell that uses no kernel carries none — byte-identical to having no prelude at all.
    let bare = CellProgram::compile("fn run(a: u16) -> u16 { a + 1u16 }").unwrap();
    assert_eq!(bare.program().symbols.len(), 1); // just `run`
}

#[test]
fn struct_field_state_matches_host() {
    // Closes the B3 seam against the host oracle (not against hardcoded literals): run a
    // struct program through the cell, snapshot EVERY field via `struct_layout`, and assert
    // field-by-field equality with the same logic under rustc. This proves the
    // host-vs-cell *field-state* equality through the layout map — the literal B3 claim —
    // the way `diff.rs` proves it for the `HL` return value.
    let src = "struct State { x: u16, y: u16, sum: u16, prod: u16, big: u16 }
               impl State {
                   fn run(&mut self) -> u16 {
                       self.sum = self.x.wrapping_add(self.y);
                       self.prod = self.x.wrapping_mul(self.y);
                       if self.x > self.y { self.big = self.x; } else { self.big = self.y; }
                       self.sum
                   }
               }";
    // The rustc oracle — the identical logic on a host struct.
    #[derive(Default)]
    struct State {
        x: u16,
        y: u16,
        sum: u16,
        prod: u16,
        big: u16,
    }
    impl State {
        fn run(&mut self) -> u16 {
            self.sum = self.x.wrapping_add(self.y);
            self.prod = self.x.wrapping_mul(self.y);
            if self.x > self.y {
                self.big = self.x;
            } else {
                self.big = self.y;
            }
            self.sum
        }
    }

    const BASE: u16 = 0xB000;
    let layout = rustz80::struct_layout(src, "State").unwrap();
    let addr = |f: &str| BASE + layout.iter().find(|l| l.name == f).unwrap().offset * 2;
    let mut r = Runner::compile(src).unwrap();

    for (x, y) in [
        (3u16, 4u16),
        (40000, 40000),
        (7, 7),
        (0, 9),
        (255, 256),
        (12345, 9999),
    ] {
        // cell: set inputs by name, run, read every field back through the layout map.
        let inputs = vec![
            (addr("x"), Ty::U16, x as u64),
            (addr("y"), Ty::U16, y as u64),
        ];
        let result = r
            .run_with_inputs(Some("State::run"), &[BASE], &inputs, DEFAULT_CYCLES)
            .unwrap()
            .result;
        // host: the same program under rustc.
        let mut host = State {
            x,
            y,
            ..Default::default()
        };
        let host_result = host.run();

        assert_eq!(result, host_result, "return value ({x},{y})");
        for (name, hv) in [
            ("x", host.x),
            ("y", host.y),
            ("sum", host.sum),
            ("prod", host.prod),
            ("big", host.big),
        ] {
            assert_eq!(
                r.peek_u16(addr(name)),
                hv,
                "field `{name}` diverged from host on ({x},{y})"
            );
        }
    }
}

#[test]
fn run_many_fast_matches_single() {
    // The batch path (entry resolved once) agrees with per-call run_fast.
    let mut r = Runner::compile("fn run(x: u16, y: u16) -> u16 { x * x + y }").unwrap();
    let sets: [&[u16]; 3] = [&[3, 1], &[6, 5], &[10, 0]];
    let many = r.run_many_fast(None, &sets, DEFAULT_CYCLES).unwrap();
    assert_eq!(
        many.iter().map(|f| f.result).collect::<Vec<_>>(),
        vec![10, 41, 100]
    );
    for (f, s) in many.iter().zip(sets.iter()) {
        let single = r.run_fast(None, s, DEFAULT_CYCLES).unwrap();
        assert_eq!(
            (f.result, f.cycles, f.halt),
            (single.result, single.cycles, single.halt)
        );
    }
}

#[test]
fn cell_pool_reuses_buses() {
    use cell80::{CellPool, CellProgram};
    let p1 = CellProgram::compile("fn run(a: u16) -> u16 { a + 1u16 }").unwrap();
    let p2 = CellProgram::compile("fn run(a: u16) -> u16 { a * 2u16 }").unwrap();
    let mut pool = CellPool::new();
    assert_eq!(pool.idle_count(), 0);

    let mut r = pool.acquire(&p1);
    assert_eq!(r.run(None, &[10], DEFAULT_CYCLES).unwrap().result, 11);
    pool.release(r);
    assert_eq!(pool.idle_count(), 1);

    // A different program reuses the pooled bus — no leakage from p1, correct result.
    let mut r = pool.acquire(&p2);
    assert_eq!(pool.idle_count(), 0); // the idle bus was taken, not a new alloc
    assert_eq!(r.run(None, &[10], DEFAULT_CYCLES).unwrap().result, 20);
    pool.release(r);
    assert_eq!(pool.idle_count(), 1);

    // Two concurrent cells → pool grows to the high-water mark of 2.
    let a = pool.acquire(&p1);
    let b = pool.acquire(&p2);
    pool.release(a);
    pool.release(b);
    assert_eq!(pool.idle_count(), 2);
}

#[test]
fn run_many_fast_fast_path_matches_authentic() {
    // A straight-line cell exercising much of the fast executor — mul/div/rem traps, the
    // 8-bit bitwise path, const shift-add — must match the authentic interpreter exactly
    // (result + cycles + halt) on every input. This is the differential guard on the fast
    // engine: any divergence fails here.
    let mut r = Runner::compile(
        "fn run(a: u16, b: u16) -> u16 { (a * b + a / b) % 100u16 + (a & b) + a * 3u16 }",
    )
    .unwrap();
    let sets: [&[u16]; 5] = [&[60, 7], &[1000, 3], &[7, 7], &[40000, 123], &[3, 9]];
    let many = r.run_many_fast(None, &sets, DEFAULT_CYCLES).unwrap();
    for (f, s) in many.iter().zip(sets.iter()) {
        let auth = r.run_fast(None, s, DEFAULT_CYCLES).unwrap();
        assert_eq!(
            (f.result, f.cycles, f.halt),
            (auth.result, auth.cycles, auth.halt),
            "fast vs authentic diverged on {s:?}"
        );
    }
}

#[test]
fn fast_executor_matches_authentic_across_ops() {
    // Drive a spread of straight-line cells through the fast path and assert each matches
    // the authentic interpreter (result + cycles + halt) — covering and validating the
    // executor's opcode arms: traps, bitwise, const-mul, array indexing (HL loads + INC),
    // tuples (BC), and raw memory.
    let cells = [
        "fn run(a: u16, b: u16) -> u16 { a * b + a / b + a % b }",
        "fn run(a: u16, b: u16) -> u16 { (a & b) + (a | b) + (a ^ b) }",
        "fn run(a: u16, b: u16) -> u16 { a * 7u16 + b * 3u16 }",
        "fn run(i: u16, a: u16, b: u16) -> u16 { let arr = [a, b, a + b]; arr[i as usize] }",
        "fn run(a: u16, b: u16) -> (u16, u16, u16) { (a * b, a + b, a) }",
        "fn run(a: u16, b: u16) -> u16 { let arr = [a; 4]; arr[0] + b }", // [v; N] fill → fallback
        "fn run(a: u16, b: u16) -> u16 { halt(a); b }",                   // halt trap → fallback
    ];
    // last input has b = 0 → exercises the divide-by-zero arm (both engines agree).
    let inputs: [&[u16]; 5] = [
        &[2, 3, 5],
        &[60, 4, 9],
        &[1, 1000, 7],
        &[2, 40000, 255],
        &[5, 0, 2],
    ];
    for src in cells {
        let mut r = Runner::compile(src).unwrap();
        let many = r.run_many_fast(None, &inputs, DEFAULT_CYCLES).unwrap();
        for (f, inp) in many.iter().zip(inputs.iter()) {
            let auth = r.run_fast(None, inp, DEFAULT_CYCLES).unwrap();
            assert_eq!(
                (f.result, f.cycles, f.halt),
                (auth.result, auth.cycles, auth.halt),
                "fast vs authentic diverged: `{src}` on {inp:?}"
            );
        }
    }
}

#[test]
fn run_many_fast_falls_back_for_branches() {
    // A looping cell is not straight-line → run_many_fast transparently falls back to the
    // authentic interpreter, still correct per input.
    let mut r = Runner::compile(
        "fn run(n: u16) -> u16 {
             let mut s = 0u16; let mut i = 0u16;
             while i < n { s = s + i; i = i + 1u16; } s
         }",
    )
    .unwrap();
    let sets: [&[u16]; 3] = [&[0], &[5], &[100]];
    let many = r.run_many_fast(None, &sets, DEFAULT_CYCLES).unwrap();
    for (f, s) in many.iter().zip(sets.iter()) {
        let auth = r.run_fast(None, s, DEFAULT_CYCLES).unwrap();
        assert_eq!(
            (f.result, f.cycles, f.halt),
            (auth.result, auth.cycles, auth.halt)
        );
    }
    assert_eq!(many[1].result, 10); // 0+1+2+3+4
    assert_eq!(many[2].result, 4950); // sum 0..100
}

#[test]
fn cartridge_roundtrip_and_inspect() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig, ABI_VERSION};
    let src = "fn run(a: u16, b: u16) -> u16 { a * b }";
    let cart = Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("mul.v1".into()),
            summary: "product".into(),
            tags: vec!["math".into(), "demo".into()],
            entry: None, // resolves to `run`
            limits: Vec::new(),
            scale: None,
            ..Default::default()
        },
    )
    .unwrap();

    // Round-trip through bytes: manifest + program survive, and it still runs.
    let bytes = cart.to_bytes();
    let back = Cartridge::from_bytes(&bytes).unwrap();
    assert_eq!(back.manifest, cart.manifest);
    assert_eq!(back.manifest.id, "mul.v1");
    assert_eq!(back.manifest.entry, "run");
    assert_eq!(back.manifest.abi_version, ABI_VERSION);
    assert!(!back.manifest.compiler_version.is_empty());
    assert_eq!(
        Runner::new(&back.program)
            .run(None, &[6, 7], DEFAULT_CYCLES)
            .unwrap()
            .result,
        42
    );

    // Inspection surfaces the manifest for a tool index.
    let j = back.to_json();
    for key in [
        "\"id\":\"mul.v1\"",
        "\"entry\":\"run\"",
        "\"tags\":[\"math\",\"demo\"]",
        "\"abi\":3",
    ] {
        assert!(j.contains(key), "inspect json missing {key}: {j}");
    }
    assert!(back.to_human().contains("mul.v1"));

    // Foreign / truncated bytes are rejected, not panicked; bad entry errors.
    assert!(Cartridge::from_bytes(b"nope!!").is_err());
    assert!(Cartridge::from_bytes(&bytes[..bytes.len() - 4]).is_err());
    assert!(Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            entry: Some("missing".into()),
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn cell_host_warm_session() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost};
    let scalar = |id: &str, src: &str, summary: &str, tags: Vec<&str>| {
        Cartridge::compile(
            src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.into()),
                summary: summary.into(),
                tags: tags.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
        )
        .unwrap()
    };

    let mut host = CellHost::new();
    assert!(host.is_empty());
    host.add(scalar(
        "add",
        "fn run(a: u16, b: u16) -> u16 { a + b }",
        "add two",
        vec!["math"],
    ));
    host.add(scalar(
        "mul",
        "fn run(a: u16, b: u16) -> u16 { a * b }",
        "multiply two",
        vec!["math"],
    ));
    host.add(
        Cartridge::compile(
            "struct S { x: u16, y: u16, sum: u16 }
             impl S { fn run(&mut self) -> u16 { self.sum = self.x + self.y; self.sum } }",
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some("accum".into()),
                entry: Some("S::run".into()),
                summary: "sum two fields".into(),
                tags: vec!["state".into()],
                limits: Vec::new(),
                scale: None,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    assert_eq!(host.len(), 3);

    // Discover + inspect.
    assert_eq!(host.search("multiply", 5)[0].id, "mul");
    assert_eq!(host.manifest("add").unwrap().signature.params.len(), 2);
    assert!(host.manifest("ghost").is_none());

    // Load once → run MANY on the warm handle (reused, deterministic — no re-instantiate).
    let h = host.load("mul").unwrap();
    assert_eq!(host.live_count(), 1);
    assert_eq!(
        host.run_fast(h, &[6, 7], DEFAULT_CYCLES).unwrap().result,
        42
    );
    assert_eq!(host.run_fast(h, &[3, 3], DEFAULT_CYCLES).unwrap().result, 9);
    assert_eq!(
        host.run(h, &[10, 2], &[], DEFAULT_CYCLES).unwrap().result,
        20
    );

    // A loaded state cell: typed inputs in, named field out — by handle.
    let hs = host.load("accum").unwrap();
    let base = 0xB000u16;
    let rep = host
        .run(
            hs,
            &[base],
            &[(base, Ty::U16, 4), (base + 2, Ty::U16, 5)],
            DEFAULT_CYCLES,
        )
        .unwrap();
    assert_eq!(rep.result, 9);
    assert_eq!(
        host.read_named(hs, &[("sum".into(), base + 4, Ty::U16)])
            .unwrap(),
        vec![("sum".into(), 9u64)]
    );
    assert_eq!(host.live_count(), 2);

    // The same state cell, driven BY NAME (run_state) — the JSON↔state surface, no raw
    // addresses: write named inputs, run, read every field back in declaration order.
    let (rep, out) = host
        .run_state(hs, &[("x".into(), 4), ("y".into(), 5)], DEFAULT_CYCLES)
        .unwrap();
    assert_eq!(rep.result, 9);
    assert_eq!(
        out,
        vec![("x".into(), 4u64), ("y".into(), 5u64), ("sum".into(), 9u64)]
    );
    // Unknown field, and a free-fn cell (no named state), both error rather than panic.
    assert!(host
        .run_state(hs, &[("nope".into(), 1)], DEFAULT_CYCLES)
        .is_err());
    assert!(host
        .run_state(h, &[("a".into(), 1)], DEFAULT_CYCLES)
        .is_err());

    // Discover by BEHAVIOUR: examples only `add` reproduces pick it over `mul`, and vice
    // versa — the phrasing-independent signal text search can't give. No match → empty.
    assert_eq!(
        host.route_by_examples(&[(vec![4, 5], 9), (vec![10, 2], 12)], 5)[0].id,
        "add"
    );
    assert_eq!(
        host.route_by_examples(&[(vec![6, 7], 42), (vec![3, 3], 9)], 5)[0].id,
        "mul"
    );
    assert!(host.route_by_examples(&[(vec![2, 2], 100)], 5).is_empty());

    // Unload returns the bus to the pool; the freed handle slot is reused next load.
    host.unload(h).unwrap();
    assert_eq!(host.live_count(), 1);
    let h2 = host.load("mul").unwrap();
    assert_eq!(h2, h, "freed handle slot reused");
    assert_eq!(
        host.run_fast(h2, &[5, 5], DEFAULT_CYCLES).unwrap().result,
        25
    );

    // Bad id / bad handle error, not panic.
    assert!(host.load("ghost").is_err());
    assert!(host.run_fast(999, &[1, 1], DEFAULT_CYCLES).is_err());
    assert!(host.unload(999).is_err());
}

#[test]
fn cli_exec_runs_a_compiled_cartridge() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig};
    let dir = std::env::temp_dir().join("rustz80_exec_test");
    std::fs::create_dir_all(&dir).unwrap();
    let cellfile = dir.join("ws.cell");
    let cart = Cartridge::compile(
        "fn run(a: u16, b: u16, c: u16) -> u16 { a + b * 2u16 + c * 3u16 }",
        CellConfig::sandboxed(),
        CartridgeOpts::default(),
    )
    .unwrap();
    std::fs::write(&cellfile, cart.to_bytes()).unwrap();
    let cf = cellfile.to_str().unwrap().to_string();

    // exec a precompiled cell (no recompile); the entry defaults to the manifest's.
    let out = cell::run_cli(&[
        "exec".into(),
        cf.clone(),
        "--args".into(),
        "5,1,9".into(),
        "--json".into(),
    ])
    .unwrap();
    assert!(out.contains("\"result\":34"), "got: {out}"); // 5 + 1*2 + 9*3
    assert!(
        cell::run_cli(&["exec".into(), cf.clone(), "--args".into(), "1,1,1".into()])
            .unwrap()
            .contains("result")
    ); // human format

    // A state cell, exec'd by explicit entry with --cycles + typed --set/--read.
    let scfile = dir.join("state.cell");
    let state = Cartridge::compile(
        "struct S { x: u16, y: u16, sum: u16 }
         impl S { fn run(&mut self) -> u16 { self.sum = self.x + self.y; self.sum } }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            entry: Some("S::run".into()),
            ..Default::default()
        },
    )
    .unwrap();
    std::fs::write(&scfile, state.to_bytes()).unwrap();
    let scf = scfile.to_str().unwrap().to_string();
    let out = cell::run_cli(&[
        "exec".into(),
        scf,
        "--entry".into(),
        "S::run".into(),
        "--cycles".into(),
        "100000".into(),
        "--args".into(),
        "0xB000".into(),
        "--set".into(),
        "0xB000:u16=4,0xB002:u16=5".into(),
        "--read".into(),
        "sum@0xB004:u16".into(),
        "--json".into(),
    ])
    .unwrap();
    assert!(out.contains("\"sum\":9"), "got: {out}"); // 4 + 5

    // errors: missing file, unknown option.
    assert!(cell::run_cli(&["exec".into(), "/no/such.cell".into()]).is_err());
    assert!(cell::run_cli(&["exec".into(), cf, "--bogus".into()]).is_err());
}

#[test]
fn cell_index_search_ranks_by_relevance() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig, CellIndex};
    let mut idx = CellIndex::new();
    assert!(idx.is_empty());
    for (id, tags) in [
        ("manhattan", vec!["grid", "distance", "score"]),
        ("range_check", vec!["validation", "range", "bounds"]),
        ("gcd", vec!["math", "bench"]),
    ] {
        let c = Cartridge::compile(
            "fn run(a: u16, b: u16) -> u16 { a + b }",
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.into()),
                summary: format!("the {id} cell"),
                tags: tags.iter().map(|s| s.to_string()).collect(),
                entry: None,
                limits: Vec::new(),
                scale: None,
                ..Default::default()
            },
        )
        .unwrap();
        idx.add(c.manifest);
    }
    assert_eq!(idx.len(), 3);
    assert_eq!(idx.search("grid distance", 5)[0].id, "manhattan"); // tag hits win
    assert!(idx.search("xyzzy", 5).is_empty()); // no match → empty
    assert!(idx.search("cell", 1).len() <= 1); // limit respected ("cell" is in every summary)
}

#[test]
fn cli_index_and_search_the_seed_library() {
    let dir = format!("{}/cells", env!("CARGO_MANIFEST_DIR"));
    let listing = cell::run_cli(&["index".into(), dir.clone()]).unwrap();
    assert!(listing.contains("manhattan") && listing.contains("Pts::run() -> u16"));
    assert!(listing.contains("range_check") && listing.contains("288 cells"));

    // search surfaces the most relevant cell first (line 0 is the header). A bare "grid
    // distance" now hits the whole distance family (manhattan/chebyshev/euclid_sq), so the
    // cell-specific name disambiguates.
    let g = cell::run_cli(&[
        "search".into(),
        "manhattan distance to a target".into(),
        dir.clone(),
    ])
    .unwrap();
    assert!(g.lines().nth(1).unwrap().contains("manhattan"), "got: {g}");
    let v = cell::run_cli(&["search".into(), "validate a value is in range".into(), dir]).unwrap();
    assert!(
        v.lines().nth(1).unwrap().contains("range_check"),
        "got: {v}"
    );

    // error paths.
    assert!(cell::run_cli(&["search".into(), "q".into()]).is_err()); // no dir
    assert!(cell::run_cli(&["index".into(), "/no/such/dir".into()]).is_err());
    assert!(cell::run_cli(&["serve".into()]).is_err()); // serve needs a dir
}

#[test]
fn cli_index_mixed_dir_with_compiled_cells() {
    // A library dir may hold both `.rs` sources and pre-compiled `.cell` cartridges
    // (and ignores anything else) — covers loading a `.cell` from the index.
    use cell80::{Cartridge, CartridgeOpts, CellConfig};
    let dir = std::env::temp_dir().join("rustz80_lib_mixed");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("doubler.rs"),
        "//! Double a value.\n//! tags: math, scale\nfn run(a: u16) -> u16 { a * 2u16 }",
    )
    .unwrap();
    let cart = Cartridge::compile(
        "fn run(a: u16, b: u16) -> u16 { a + b }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("adder".into()),
            summary: "Add two numbers.".into(),
            tags: vec!["math".into(), "add".into()],
            ..Default::default()
        },
    )
    .unwrap();
    std::fs::write(dir.join("adder.cell"), cart.to_bytes()).unwrap();
    std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
    // a source with no `//!` header → renders as "(no summary)".
    std::fs::write(dir.join("bare.rs"), "fn run(a: u16) -> u16 { a }").unwrap();

    let out = cell::run_cli(&["index".into(), dir.to_str().unwrap().to_string()]).unwrap();
    assert!(out.contains("3 cells"), "got: {out}"); // 2 .rs + 1 .cell, not .txt
    assert!(out.contains("doubler") && out.contains("adder"));
    assert!(out.contains("bare — (no summary)")); // header-less source
}

#[test]
fn cli_index_without_gate_is_unchanged() {
    // Locks the existing no-flag contract: `--gate` must be strictly additive.
    let dir = format!("{}/cells", env!("CARGO_MANIFEST_DIR"));
    let listing = cell::run_cli(&["index".into(), dir]).unwrap();
    assert!(listing.contains("manhattan") && listing.contains("288 cells"));
    assert!(!listing.contains("REFUSED"));
}

#[test]
fn cli_index_json_lists_every_manifest() {
    // The plain (non-gate) listing's --json path — feeds docs/cell-index.md's generator.
    let dir = format!("{}/cells", env!("CARGO_MANIFEST_DIR"));
    let out = cell::run_cli(&["index".into(), dir, "--json".into()]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let cells = v["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 288, "got: {out}");
    let manhattan = cells.iter().find(|c| c["id"] == "manhattan").unwrap();
    assert_eq!(manhattan["signature"], "Pts::run() -> u16");
    assert!(manhattan["tags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t == "grid"));
}

#[test]
fn cli_index_gate_over_the_real_library() {
    // The admission gate against the real 203-cell library + its own retrieval dataset — the
    // true end-to-end proof, not a synthetic fixture. Wave 3's calendrical/checksum pack
    // found (and fixed at the root) a `luhn_check`/`is_zero` false positive by widening
    // `DEFAULT_PROBES` (fingerprint.rs) rather than touching luhn_check; every pack since
    // (Q8.8, agentic-runtime, running-stats, spatial/grid, the Phase 2.3 pilot batch, and
    // the GSM8K checked-arithmetic + money/bps packs — all state cells, exempt from the
    // fingerprint check entirely) added no new collisions. The units pack (same_unit_check,
    // unit_mul, unit_div, unit_cancel_check) is the campaign's first *free-fn* pack — its
    // arity-2 u16 cells go through the fingerprint check for real, and still collided with
    // nothing (including its later wage-rate dimension-code extension). The verifier/ranker
    // pack's sum_equals/diff_equals are arity-3 (exempt by design — admission.rs only
    // fingerprints arity-≤2 free-fns) and its product_equals_u32/quotient_equals_exact_u32
    // are state cells (exempt too), and so is the stateful/RNG pack (lcg_next, xorshift16,
    // counter_step — all state cells). The signed-deltas pack (sign_i16, abs_i16, clamp_i16,
    // apply_delta_clamped) found a second real fingerprint gap: every DEFAULT_PROBES value
    // was non-negative as `i16`, so `sign_i16` degenerated to `nonzero` on this bank — fixed
    // the same way as luhn_check, by widening the bank with `[65531, 3]` (`-5` as an `i16`
    // bit pattern). That widening had a welcome side effect: it also separated the
    // long-standing `snap_down`/`round_to_multiple` false positive (they now diverge on the
    // new probe too), so the gate is fully clean. The scoring/choice pack (weighted_sum2/3
    // state cells, choose_best3 state cell, is_clear_winner arity-3 free-fn — all exempt
    // from the fingerprint check) added no new collisions either. The fractions pack (M1
    // 5/5, all 10 state cells) closed out M1's first pass — still 0 refusals. The second
    // slice (checked-arithmetic +18, money-bps +2, verifier-ranker +11, fractions +9)
    // plus the units wage-rate extension added 40 more cells and still 0 refusals —
    // and since the fingerprint exemptions were lifted (state cells digest their
    // post-run fields; the probe bank drives all three registers; shapes compare by
    // ordered field types), that 0 covers *every* cell, not just the arity-≤2 slice.
    let dir = format!("{}/cells", env!("CARGO_MANIFEST_DIR"));
    let retrieval = format!(
        "{}/../cell-eval/datasets/retrieval.jsonl",
        env!("CARGO_MANIFEST_DIR")
    );
    let out = cell::run_cli(&["index".into(), dir, "--gate".into(), retrieval]).unwrap();
    assert!(out.contains("288 admitted, 0 refused"), "got: {out}");
}

#[test]
fn cli_index_gate_refuses_a_planted_duplicate() {
    let dir = std::env::temp_dir().join("cell80_cli_gate_duplicate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = "//! Smaller of two values.\n//! tags: math\nfn run(a: u16, b: u16) -> u16 { let mut m = a; if b < a { m = b; } m }";
    std::fs::write(dir.join("min.rs"), src).unwrap();
    std::fs::write(dir.join("min2.rs"), src).unwrap(); // exact behavioural duplicate
    let retrieval = dir.join("retrieval.jsonl");
    std::fs::write(
        &retrieval,
        "{\"id\": \"min-1\", \"query\": \"minimum of two values\", \"expected\": \"min\", \"category\": \"direct\"}\n\
         {\"id\": \"min2-1\", \"query\": \"the smaller of two numbers\", \"expected\": \"min2\", \"category\": \"paraphrase\"}\n",
    )
    .unwrap();
    let out = cell::run_cli(&[
        "index".into(),
        dir.to_str().unwrap().to_string(),
        "--gate".into(),
        retrieval.to_str().unwrap().to_string(),
    ])
    .unwrap();
    assert!(out.contains("1 admitted, 1 refused"), "got: {out}");
    assert!(
        out.contains("min2 — behavioural duplicate of `min`"),
        "got: {out}"
    );
}

#[test]
fn cli_index_gate_refuses_missing_eval_rows() {
    let dir = std::env::temp_dir().join("cell80_cli_gate_norows");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("novel.rs"),
        "//! A genuinely new cell.\n//! tags: math\nfn run(a: u16) -> u16 { a * 3u16 }",
    )
    .unwrap();
    let retrieval = dir.join("retrieval.jsonl"); // empty — no rows for `novel`
    std::fs::write(&retrieval, "").unwrap();
    let out = cell::run_cli(&[
        "index".into(),
        dir.to_str().unwrap().to_string(),
        "--gate".into(),
        retrieval.to_str().unwrap().to_string(),
    ])
    .unwrap();
    assert!(out.contains("0 admitted, 1 refused"), "got: {out}");
    assert!(
        out.contains("novel — no retrieval.jsonl rows"),
        "got: {out}"
    );
}

#[test]
fn cartridge_carries_typed_signature() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig};
    // fn-args signature, surviving the round-trip + surfaced in inspect.
    let c = Cartridge::compile(
        "fn run(a: u16, b: u16) -> u16 { a + b }",
        CellConfig::sandboxed(),
        CartridgeOpts::default(),
    )
    .unwrap();
    assert_eq!(
        c.manifest.signature.params,
        vec![("a".into(), "u16".into()), ("b".into(), "u16".into())]
    );
    assert_eq!(c.manifest.signature.ret, "u16");
    assert!(c.manifest.signature.state.is_empty());
    let back = Cartridge::from_bytes(&c.to_bytes()).unwrap();
    assert_eq!(back.manifest, c.manifest); // signature round-trips
    assert!(back
        .to_human()
        .contains("signature: run(a: u16, b: u16) -> u16"));
    assert!(back.to_json().contains(
        "\"signature\":{\"params\":[[\"a\",\"u16\"],[\"b\",\"u16\"]],\"ret\":\"u16\",\"state\":[]}"
    ));

    // `&mut self` method → the named typed state (struct fields with types).
    let src = "struct State { x: u16, y: u16, score: u16 }
               impl State { fn run(&mut self) -> u16 { self.score = self.x + self.y; self.score } }";
    let s = Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            entry: Some("State::run".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(s.manifest.signature.params.is_empty());
    assert_eq!(
        s.manifest.signature.state,
        vec![
            ("x".into(), "u16".into()),
            ("y".into(), "u16".into()),
            ("score".into(), "u16".into())
        ]
    );
    assert!(Cartridge::from_bytes(&s.to_bytes())
        .unwrap()
        .to_human()
        .contains("state: { x: u16, y: u16, score: u16 }"));
}

#[test]
fn cartridge_permissive_and_empty_manifest_branches() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig};
    // Permissive (no ceilings → ∞/null) + empty summary/tags (→ "(no summary)" / "—").
    let cart = Cartridge::compile(
        "fn run() -> u16 { 0u16 }",
        CellConfig::permissive(),
        CartridgeOpts::default(),
    )
    .unwrap();
    let human = cart.to_human();
    assert!(
        human.contains("(no summary)") && human.contains("tags: —"),
        "got: {human}"
    );
    assert!(human.contains("max_code=∞") && human.contains("max_touched=∞"));
    assert!(cart
        .to_json()
        .contains("\"max_code\":null,\"max_touched\":null"));
    assert_eq!(cart.manifest.id, "run"); // defaulted to the entry name

    // Neither `run` nor `main`, no explicit entry → error.
    assert!(Cartridge::compile(
        "fn helper() -> u16 { 1u16 }",
        CellConfig::permissive(),
        CartridgeOpts::default()
    )
    .is_err());
}

#[test]
fn cli_compile_inspect_and_errors() {
    let dir = std::env::temp_dir().join("rustz80_cart_test");
    std::fs::create_dir_all(&dir).unwrap();
    let rs = dir.join("c.rs");
    let cellfile = dir.join("c.cell");
    std::fs::write(
        &rs,
        "fn run(a: u16, b: u16) -> u16 { poke(40000u16, a as u8); a + b }",
    )
    .unwrap();
    let (rs, cellfile) = (
        rs.to_str().unwrap().to_string(),
        cellfile.to_str().unwrap().to_string(),
    );

    // compile exercising most options (poke needs --allow-raw-memory).
    let out = cell::run_cli(&[
        "compile".into(),
        rs.clone(),
        "-o".into(),
        cellfile.clone(),
        "--entry".into(),
        "run".into(),
        "--id".into(),
        "add.v1".into(),
        "--summary".into(),
        "adds two".into(),
        "--tags".into(),
        "math, demo".into(),
        "--allow-raw-memory".into(),
        "--max-touched".into(),
        "256".into(),
    ])
    .unwrap();
    assert!(
        out.contains("wrote") && out.contains("add.v1"),
        "got: {out}"
    );

    // inspect: human (no --json) and json.
    assert!(cell::run_cli(&["inspect".into(), cellfile.clone()])
        .unwrap()
        .contains("add.v1"));
    assert!(
        cell::run_cli(&["inspect".into(), cellfile, "--json".into()])
            .unwrap()
            .contains("\"tags\":[\"math\",\"demo\"]")
    );

    // error paths: unknown command, no args, compile without -o, unknown option, missing file.
    assert!(cell::run_cli(&["frobnicate".into()]).is_err());
    assert!(cell::run_cli(&[]).is_err());
    assert!(cell::run_cli(&["compile".into(), rs.clone()]).is_err());
    assert!(cell::run_cli(&[
        "compile".into(),
        rs,
        "-o".into(),
        "/x".into(),
        "--bogus".into()
    ])
    .is_err());
    assert!(cell::run_cli(&["inspect".into(), "/no/such.cell".into()]).is_err());
}

#[test]
fn cell_image_roundtrip() {
    use cell80::{CellConfig, CellProgram};
    let src = "fn run(a: u16, b: u16) -> u16 { a * b }";
    let prog = CellProgram::compile_with_config(src, CellConfig::sandboxed()).unwrap();
    let bytes = prog.to_bytes();
    assert!(
        bytes.len() < 128,
        "image should be tiny (got {})",
        bytes.len()
    );

    // Reload without re-parsing — same code + symbols, runs to the same result, policy kept.
    let back = CellProgram::from_bytes(&bytes).unwrap();
    assert_eq!(back.program().code, prog.program().code);
    assert_eq!(back.program().symbols, prog.program().symbols);
    assert_eq!(
        Runner::new(&back)
            .run(None, &[6, 7], DEFAULT_CYCLES)
            .unwrap()
            .result,
        42
    );

    // Foreign / truncated bytes are rejected, not panicked.
    assert!(CellProgram::from_bytes(b"nope").is_err());
    assert!(CellProgram::from_bytes(&bytes[..bytes.len() - 3]).is_err());
}

#[test]
fn cell80_halt_with_code() {
    use cell80::Halt;
    // `halt(code)` stops the run early with a status code.
    let mut r = Runner::compile(
        "fn run(n: u16) -> u16 {
             let mut i = 0u16;
             while i < 1000u16 { if i == n { halt(7u16); } i = i + 1u16; }
             0u16
         }",
    )
    .unwrap();
    let early = r.run(None, &[5], DEFAULT_CYCLES).unwrap();
    assert_eq!(early.halt, Halt::Halted(7));
    assert!(!early.returned);

    // n never hit → the loop completes and returns normally.
    let full = r.run(None, &[2000], DEFAULT_CYCLES).unwrap();
    assert_eq!(full.halt, Halt::Returned);
    assert_eq!(full.result, 0);
    assert!(early.cycles < full.cycles, "halt(5) should stop far sooner");

    // `halt` compiles for the authentic Spectrum target too (a no-op `ED FE` there).
    assert!(rustz80::compile_program("fn run() -> u16 { halt(1u16); 0u16 }").is_ok());
}

#[test]
fn cell80_array_init_is_a_block_op() {
    use cell80::{CellProgram, Runner};
    // A big `[v; N]` init is one block op, not N unrolled stores — so the code stays tiny
    // (it would be ~hundreds of bytes unrolled). Result still correct.
    let src = "fn run() -> u16 { let a = [9u16; 256]; a[0] + a[255] }";
    let cp = CellProgram::compile(src).unwrap();
    assert!(
        cp.program().code.len() < 64,
        "256-element fill should not unroll (got {} bytes)",
        cp.program().code.len()
    );
    assert_eq!(
        Runner::new(&cp)
            .run(None, &[], DEFAULT_CYCLES)
            .unwrap()
            .result,
        18
    ); // 9 + 9
}

#[test]
fn cell80_traps_mul_div_natively() {
    use cell80::{CellProgram, Runner};
    let src = "fn run(a: u16, b: u16) -> u16 { a * b + a / b + a % b }";

    // Cell mode: `*`/`/`/`%` lower to ED FE host traps — no software runtime appended.
    let cp = CellProgram::compile(src).unwrap();
    assert!(
        !cp.program().symbols.contains_key("__mul16"),
        "cell mode shouldn't append __mul16"
    );
    assert!(!cp.program().symbols.contains_key("__divmod16"));
    let got = Runner::new(&cp)
        .run(None, &[60, 7], DEFAULT_CYCLES)
        .unwrap()
        .result;
    assert_eq!(got, 60u16 * 7 + 60 / 7 + 60 % 7); // 420 + 8 + 4 = 432 (matches rustc)

    // Authentic Spectrum compile still uses (and appends) the software routines.
    let spec = rustz80::compile_program(src).unwrap();
    assert!(spec.symbols.contains_key("__mul16") && spec.symbols.contains_key("__divmod16"));
}

#[test]
fn run_fast_matches_run() {
    use cell80::Halt;
    // The hot path must agree with the full Report on result/regs/cycles/halt.
    let mut r = Runner::compile("fn run(a: u16, b: u16) -> (u16, u16) { (a * a + b, a) }").unwrap();
    let full = r.run(None, &[6, 5], DEFAULT_CYCLES).unwrap();
    let fast = r.run_fast(None, &[6, 5], DEFAULT_CYCLES).unwrap();
    assert_eq!(fast.result, full.result); // 6*6 + 5 = 41
    assert_eq!(fast.regs, full.regs);
    assert_eq!(fast.cycles, full.cycles);
    assert_eq!(fast.halt, full.halt);
    assert_eq!(fast.halt, Halt::Returned);

    // Budget overrun is reported, not hung, on the fast path too.
    let mut spin =
        Runner::compile("fn run() -> u16 { let mut i = 0u16; loop { i = i + 1u16; } }").unwrap();
    assert_eq!(
        spin.run_fast(None, &[], 1000).unwrap().halt,
        Halt::CycleBudget
    );
}

#[test]
fn captures_all_result_registers() {
    // A tuple return leaves the values in HL/DE/BC — read them all back.
    let mut r = Runner::compile("fn run(a: u16, b: u16) -> (u16, u16) { (a / b, a % b) }").unwrap();
    let rep = r.run(None, &[47, 5], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.result, 9); // HL = quotient
    assert_eq!(rep.regs[0], 9); // HL
    assert_eq!(rep.regs[1], 2); // DE = remainder
}

#[test]
fn typed_state_read_back() {
    // A program that writes known bytes; read them back typed from post-run memory.
    let mut r = Runner::compile(
        "fn run() -> u16 {
             poke(40000u16, 0x34u8); poke(40001u16, 0x12u8);  // u16 0x1234 @ 40000
             poke(40002u16, 0x78u8); poke(40003u16, 0x56u8);  // u32 high word
             0u16
         }",
    )
    .unwrap();
    r.run(None, &[], DEFAULT_CYCLES).unwrap();
    assert_eq!(r.peek_u8(40000), 0x34);
    assert_eq!(r.peek_u16(40000), 0x1234);
    assert_eq!(r.peek_u32(40000), 0x5678_1234);
    let vals = r.read_named(&[
        ("a".into(), 40000, Ty::U16),
        ("b".into(), 40000, Ty::U32),
        ("c".into(), 40003, Ty::U8),
    ]);
    assert_eq!(
        vals,
        vec![
            ("a".into(), 0x1234u64),
            ("b".into(), 0x5678_1234u64),
            ("c".into(), 0x56u64),
        ]
    );
}

#[test]
fn struct_layout_offsets() {
    let src = "struct State { x: u16, y: u16, arr: [u16; 4], total: u32, score: u16 }";
    let l = rustz80::struct_layout(src, "State").unwrap();
    assert_eq!(
        l[0],
        rustz80::FieldLayout {
            name: "x".into(),
            offset: 0,
            slots: 1,
            dword: false,
            f32: false,
            bytes: None
        }
    );
    assert_eq!(l[1].offset, 1); // y
    assert_eq!((l[2].offset, l[2].slots), (2, 4)); // arr — 4 slots
    assert_eq!((l[3].offset, l[3].slots, l[3].dword), (6, 2, true)); // total — a wide u32
    assert_eq!(l[4].offset, 8); // score, after the u32's two slots
    assert!(rustz80::struct_layout(src, "Nope").is_err());
}

#[test]
fn typed_io_named_loop() {
    // The full agent loop by NAME: resolve field addresses from the layout, set typed
    // inputs, run, read typed outputs — the caller never touches raw addresses directly.
    let src = "struct State { x: u16, y: u16, score: u16 }
               impl State { fn run(&mut self) -> u16 { self.score = self.x + self.y * 10u16; self.score } }";
    const BASE: u16 = 0xB000;
    let layout = rustz80::struct_layout(src, "State").unwrap();
    let addr = |f: &str| BASE + layout.iter().find(|l| l.name == f).unwrap().offset * 2;

    let mut r = Runner::compile(src).unwrap();
    let inputs = vec![(addr("x"), Ty::U16, 3u64), (addr("y"), Ty::U16, 4u64)];
    let rep = r
        .run_with_inputs(Some("State::run"), &[BASE], &inputs, DEFAULT_CYCLES)
        .unwrap();
    assert_eq!(rep.result, 43); // 3 + 4*10
    let out = r.read_named(&[("score".into(), addr("score"), Ty::U16)]);
    assert_eq!(out, vec![("score".into(), 43u64)]);

    // Different inputs, same compiled cell (warm) — no leakage from the prior run.
    let rep2 = r
        .run_with_inputs(
            Some("State::run"),
            &[BASE],
            &[(addr("x"), Ty::U16, 100), (addr("y"), Ty::U16, 0)],
            DEFAULT_CYCLES,
        )
        .unwrap();
    assert_eq!(rep2.result, 100);
}

#[test]
fn ty_parse() {
    assert_eq!(Ty::parse("u8").unwrap(), Ty::U8);
    assert_eq!(Ty::parse("u16").unwrap(), Ty::U16);
    assert_eq!(Ty::parse("u32").unwrap(), Ty::U32);
    assert!(Ty::parse("u9").is_err());
}

#[test]
fn runner_reuse_is_deterministic() {
    // Compile once, run many: each run must reset the bus, so repeated runs (same args)
    // are bit-identical — same result, same T-states, same touched memory — and changing
    // args changes the result, with no leakage between runs.
    let mut r = Runner::compile(
        "fn run(n: u16) -> u16 { let mut a = [0u16; 8]; let mut s = 0u16;
             let mut i = 0u16; while i < 8u16 { a[i as usize] = i + n; i = i + 1u16; }
             let mut j = 0u16; while j < 8u16 { s = s + a[j as usize]; j = j + 1u16; } s }",
    )
    .expect("compile");

    assert!(r.program().symbols.contains_key("run")); // the compiled program is reachable
    let first = r.run(None, &[0], DEFAULT_CYCLES).unwrap(); // 0+1+..+7 = 28
    let again = r.run(None, &[0], DEFAULT_CYCLES).unwrap();
    assert_eq!(first.result, 28);
    assert_eq!(first.result, again.result, "reuse must be deterministic");
    assert_eq!(first.cycles, again.cycles, "same path → same T-states");
    assert_eq!(
        first.touched, again.touched,
        "same writes → same memory diff"
    );

    let bumped = r.run(None, &[10], DEFAULT_CYCLES).unwrap(); // (0..7)+8*10 = 28+80 = 108
    assert_eq!(bumped.result, 108);
    // Back to the original args still gives the original answer (no accumulated state).
    assert_eq!(r.run(None, &[0], DEFAULT_CYCLES).unwrap().result, 28);
}

#[test]
fn runs_and_reports() {
    // A small program: sum 1..=n. Run with an arg, check result/cost/symbols/memory.
    let src = "
        fn run(n: u16) -> u16 {
            let mut s = 0u16;
            let mut i = 1u16;
            while i <= n { s = s + i; i = i + 1u16; }
            s
        }
    ";
    let r = cell::run(src, None, &[10], DEFAULT_CYCLES).expect("run");
    assert_eq!(r.entry, "run"); // defaulted to `run`
    assert_eq!(r.entry_addr, rustz80::ORG);
    assert_eq!(r.result, 55); // 1+..+10
    assert!(r.returned, "should return within budget");
    assert!(r.cycles > 0 && r.cycles < DEFAULT_CYCLES);
    assert!(r.code_bytes > 0 && r.fn_count >= 1);
    assert!(r
        .symbols
        .iter()
        .any(|(n, a)| n == "run" && *a == rustz80::ORG));
    // The loop counter/accumulator live in the scratch "register file"; some RAM is hit.
    assert!(!r.touched.is_empty());
}

#[test]
fn budget_exceeded_is_reported_not_panicked() {
    // An infinite loop must stop at the budget and report `returned = false`.
    let src = "fn run() -> u16 { let mut i = 0u16; loop { i = i + 1u16; } }";
    let r = cell::run(src, None, &[], 1000).expect("run");
    assert!(!r.returned, "infinite loop should hit the budget");
    assert!(r.cycles >= 1000);
}

#[test]
fn monomorphic_instances_appear_in_symbols() {
    // Two capacities → two instances in the symbol map.
    let src = include_str!("../../rustz80/samples/showcase/entities.rs");
    let r = cell::run(src, None, &[], DEFAULT_CYCLES).expect("run");
    assert_eq!(r.result, 2530);
    assert!(r.symbols.iter().any(|(n, _)| n == "Entities$4::add"));
    assert!(r.symbols.iter().any(|(n, _)| n == "Entities$8::add"));
}

#[test]
fn missing_entry_errors_with_available_names() {
    let src = "fn run() -> u16 { 1u16 }";
    let err = cell::run(src, Some("nope"), &[], DEFAULT_CYCLES).unwrap_err();
    assert!(err.contains("nope") && err.contains("run"), "got: {err}");
}

#[test]
fn parse_args_decimal_and_hex() {
    assert_eq!(cell::parse_args("1,0x10,255").unwrap(), vec![1, 16, 255]);
    assert_eq!(cell::parse_args("").unwrap(), Vec::<u16>::new());
    assert!(cell::parse_args("notanum").is_err());
}

#[test]
fn report_formats_human_and_json() {
    let r = cell::run("fn run() -> u16 { 7u16 }", None, &[], DEFAULT_CYCLES).unwrap();
    let human = r.to_human();
    assert!(human.contains("result     7") && human.contains("returned"));
    let json = r.to_json();
    assert!(
        json.starts_with('{')
            && json.contains("\"result\":7")
            && json.contains("\"halt\":\"returned\"")
    );
}

#[test]
fn run_cli_end_to_end() {
    // Write a source to a temp file and drive the full CLI path (run → format).
    let dir = std::env::temp_dir().join("rustz80_cell_cli_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.rs");
    std::fs::write(&path, "fn run(a: u16, b: u16) -> u16 { a + b }").unwrap();
    let p = path.to_str().unwrap().to_string();

    // --json with args
    let out = cell::run_cli(&[
        "run".into(),
        p.clone(),
        "--args".into(),
        "20,22".into(),
        "--json".into(),
    ])
    .unwrap();
    assert!(out.contains("\"result\":42"), "got: {out}");

    // human form, default budget
    let out = cell::run_cli(&["run".into(), p.clone(), "--args".into(), "1,2".into()]).unwrap();
    assert!(out.contains("result     3"));

    // a tiny budget reports overshoot rather than hanging/panicking
    let loopsrc = dir.join("loop.rs");
    std::fs::write(
        &loopsrc,
        "fn run() -> u16 { let mut i = 0u16; loop { i = i + 1u16; } }",
    )
    .unwrap();
    let out = cell::run_cli(&[
        "run".into(),
        loopsrc.to_str().unwrap().into(),
        "--cycles".into(),
        "500".into(),
    ])
    .unwrap();
    assert!(out.contains("BUDGET EXCEEDED"));

    // error paths
    assert!(cell::run_cli(&[]).is_err());
    assert!(cell::run_cli(&["wat".into()]).is_err());
    assert!(cell::run_cli(&["run".into(), p, "--bogus".into()]).is_err());
    assert!(cell::run_cli(&["run".into(), "/no/such/file.rs".into()]).is_err());
}

#[test]
fn run_cli_typed_read() {
    let dir = std::env::temp_dir().join("rustz80_cell_read_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.rs");
    std::fs::write(
        &path,
        "fn run() -> u16 { poke(40000u16, 42u8); poke(40001u16, 7u8); 0u16 }",
    )
    .unwrap();
    let p = path.to_str().unwrap().to_string();

    // This cell uses `poke`, so it needs --allow-raw-memory (sandboxed by default).
    let out = cell::run_cli(&[
        "run".into(),
        p.clone(),
        "--allow-raw-memory".into(),
        "--set".into(),
        "0x9c42:u16=0x00ff,40004:u16=9".into(), // hex addr/value AND decimal — both parse paths
        "--read".into(),
        "score@40000:u8,lives@0x9c41:u8,extra@0x9c42:u16,dec@40004:u16".into(), // 0x9c41 = 40001
        "--json".into(),
    ])
    .unwrap();
    assert!(
        out.contains("\"reads\":{\"score\":42,\"lives\":7,\"extra\":255,\"dec\":9}"),
        "got: {out}"
    );

    let human = cell::run_cli(&[
        "run".into(),
        p.clone(),
        "--allow-raw-memory".into(),
        "--read".into(),
        "score@40000:u8".into(),
    ])
    .unwrap();
    assert!(human.contains("reads      score=42"), "got: {human}");

    // bad specs
    assert!(cell::run_cli(&[
        "run".into(),
        p.clone(),
        "--allow-raw-memory".into(),
        "--read".into(),
        "noaddr".into()
    ])
    .is_err());
    assert!(cell::run_cli(&[
        "run".into(),
        p,
        "--allow-raw-memory".into(),
        "--read".into(),
        "x@40000:u9".into()
    ])
    .is_err());
}

#[test]
fn capabilities_gate_raw_memory_and_ports() {
    use cell80::CellConfig;
    // `poke`/`peek` need raw memory; `inport` needs ports — denied by default.
    let pokes = "fn run() -> u16 { poke(40000u16, 1u8); peek(40000u16) as u16 }";
    let ports = "fn run() -> u16 { inport(0xFEu16) as u16 }";
    assert!(Runner::compile_with_config(pokes, CellConfig::sandboxed()).is_err());
    assert!(Runner::compile_with_config(ports, CellConfig::sandboxed()).is_err());
    // Explicitly allowed → compiles.
    assert!(Runner::compile_with_config(pokes, CellConfig::permissive()).is_ok());
    let mut cfg = CellConfig::sandboxed();
    cfg.allow_ports = true;
    assert!(Runner::compile_with_config(ports, cfg).is_ok());
    // A pure-compute cell needs no capabilities — fine sandboxed.
    assert!(Runner::compile_with_config(
        "fn run(a: u16) -> u16 { a * 2u16 }",
        CellConfig::sandboxed()
    )
    .is_ok());
}

#[test]
fn safety_config_defaults_and_cli_flags() {
    use cell80::{CellConfig, Halt};
    // default() is the sandboxed policy.
    let d = CellConfig::default();
    assert!(!d.allow_raw_memory && !d.allow_ports && d.max_code_bytes.is_some());

    // A memory-limit run formats in both modes.
    let mut cfg = CellConfig::sandboxed();
    cfg.max_touched = Some(2);
    let mut r = Runner::compile_with_config(
        "fn run() -> u16 { let mut a = [0u16; 32]; let mut i = 0u16;
             while i < 32u16 { a[i as usize] = i; i = i + 1u16; } a[0] }",
        cfg,
    )
    .unwrap();
    let rep = r.run(None, &[], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.halt, Halt::MemoryLimit);
    assert!(rep.to_human().contains("MEMORY LIMIT"));
    assert!(rep.to_json().contains("\"halt\":\"memory_limit\""));

    // CLI safety flags parse + apply.
    let dir = std::env::temp_dir().join("rustz80_cell_safety_cli");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ok.rs");
    std::fs::write(&path, "fn run(a: u16) -> u16 { a + 1u16 }").unwrap();
    let p = path.to_str().unwrap().to_string();
    let out = cell::run_cli(&[
        "run".into(),
        p.clone(),
        "--max-code-bytes".into(),
        "8192".into(),
        "--max-touched".into(),
        "8192".into(),
        "--allow-ports".into(),
        "--args".into(),
        "41".into(),
    ])
    .unwrap();
    assert!(out.contains("result     42"), "got: {out}");
    // A too-tight code-size limit rejects.
    assert!(cell::run_cli(&["run".into(), p, "--max-code-bytes".into(), "2".into()]).is_err());
}

#[test]
fn limits_code_size_and_memory() {
    use cell80::{CellConfig, Halt};
    // A tiny code-size ceiling rejects at compile.
    let mut cfg = CellConfig::sandboxed();
    cfg.max_code_bytes = Some(4);
    assert!(Runner::compile_with_config("fn run() -> u16 { let mut s = 0u16; let mut i = 0u16; while i < 100u16 { s = s + i; i = i + 1u16; } s }", cfg).is_err());

    // A memory-touched ceiling aborts the run with Halt::MemoryLimit.
    let mut cfg = CellConfig::sandboxed();
    cfg.max_touched = Some(4);
    let mut r = Runner::compile_with_config(
        "fn run() -> u16 { let mut a = [0u16; 64]; let mut i = 0u16;
             while i < 64u16 { a[i as usize] = i; i = i + 1u16; } a[0] }",
        cfg,
    )
    .unwrap();
    let rep = r.run(None, &[], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.halt, Halt::MemoryLimit);
    assert!(!rep.returned);
}

#[test]
fn run_cli_typed_set() {
    let dir = std::env::temp_dir().join("rustz80_cell_set_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.rs");
    std::fs::write(
        &path,
        "struct State { x: u16, y: u16, score: u16 }
         impl State { fn run(&mut self) -> u16 { self.score = self.x + self.y; self.score } }",
    )
    .unwrap();
    let p = path.to_str().unwrap().to_string();

    let out = cell::run_cli(&[
        "run".into(),
        p.clone(),
        "--entry".into(),
        "State::run".into(),
        "--args".into(),
        "0xB000".into(),
        "--set".into(),
        "0xB000:u16=20,0xB002:u16=22".into(),
        "--read".into(),
        "score@0xB004:u16".into(),
        "--json".into(),
    ])
    .unwrap();
    assert!(
        out.contains("\"result\":42") && out.contains("\"score\":42"),
        "got: {out}"
    );

    // bad --set specs
    assert!(cell::run_cli(&["run".into(), p.clone(), "--set".into(), "noeq".into()]).is_err());
    assert!(cell::run_cli(&["run".into(), p, "--set".into(), "0xB000:u9=1".into()]).is_err());
}

#[test]
fn wide_u32_state_field_end_to_end() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost, StateCell};
    // A `u32` state field driven and read BY NAME, wide, at every layer of the stack:
    // StateCell, the `.cell` manifest round-trip (v4 carries a width per field), and
    // the warm host's `run_state`. The values only exist past the u16 ceiling.
    let src = "struct Acc { n: u16, total: u32 }
               impl Acc {
                   fn run(&mut self) -> u16 {
                       self.total = self.total + self.n as u32 * self.n as u32;
                       (self.total >> 16u32) as u16
                   }
               }";

    // StateCell: set a 32-bit value into the wide field, read the exact wide result.
    let mut cell = StateCell::bind(src, "Acc", None).unwrap();
    cell.set("n", 300).unwrap();
    cell.set("total", 4_000_000_000).unwrap(); // representable only in the wide field
    cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(cell.get("total"), Some(4_000_090_000)); // 4e9 + 300²
    assert_eq!(cell.get("n"), Some(300));

    // The manifest carries the width — and it survives the byte round-trip.
    let cart = Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("acc".into()),
            entry: Some("Acc::run".into()),
            summary: "wide accumulator".into(),
            tags: vec!["state".into()],
            limits: Vec::new(),
            scale: None,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        cart.manifest.state_addrs,
        vec![
            ("n".into(), 0xB000, Ty::U16),
            ("total".into(), 0xB002, Ty::U32),
        ]
    );
    let back = Cartridge::from_bytes(&cart.to_bytes()).unwrap();
    assert_eq!(back.manifest.state_addrs, cart.manifest.state_addrs);

    // The warm host drives the wide field by name — the JSON↔state agent surface.
    let mut host = CellHost::new();
    host.add(back);
    let h = host.load("acc").unwrap();
    let (rep, state) = host
        .run_state(
            h,
            &[("n".into(), 300), ("total".into(), 4_000_000_000)],
            DEFAULT_CYCLES,
        )
        .unwrap();
    assert_eq!(rep.result, (4_000_090_000u64 >> 16) as u16);
    assert_eq!(
        state,
        vec![("n".into(), 300u64), ("total".into(), 4_000_090_000u64)]
    );
    host.unload(h).unwrap();
}

#[test]
fn div_by_zero_halts_by_default() {
    use cell80::Halt;
    // The determinism contract: a garbage quotient must not flow onward — `/ 0` and
    // `% 0` stop the run with a typed halt (both the 16-bit and the wide trap).
    let mut r = Runner::compile("fn run(a: u16, b: u16) -> u16 { a / b }").unwrap();
    let rep = r.run(None, &[9, 0], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.halt, Halt::DivByZero);
    assert!(!rep.returned);

    let mut m = Runner::compile("fn run(a: u16, b: u16) -> u16 { a % b }").unwrap();
    assert_eq!(
        m.run(None, &[9, 0], DEFAULT_CYCLES).unwrap().halt,
        Halt::DivByZero
    );

    let mut w =
        Runner::compile("fn run(a: u16, b: u16) -> u16 { (a as u32 / b as u32) as u16 }").unwrap();
    assert_eq!(
        w.run(None, &[9, 0], DEFAULT_CYCLES).unwrap().halt,
        Halt::DivByZero
    );

    // A guarded divide (the library's `safe_div` shape) still returns cleanly.
    let mut s = Runner::compile(
        "fn run(a: u16, b: u16) -> u16 { let mut r = 0u16; if b != 0u16 { r = a / b; } r }",
    )
    .unwrap();
    let rep = s.run(None, &[9, 0], DEFAULT_CYCLES).unwrap();
    assert_eq!((rep.result, rep.halt), (0, Halt::Returned));
}

#[test]
fn div_by_zero_saturates_under_the_legacy_policy() {
    use cell80::{CellProgram, DivByZero, Halt};
    // The opt-in keeps the old bounded-garbage behaviour: q = 0xFFFF, run continues.
    let mut cfg = cell::CellConfig::sandboxed();
    cfg.div_by_zero = DivByZero::Saturate;
    let mut r = Runner::compile_with_config("fn run(a: u16, b: u16) -> u16 { a / b }", cfg.clone())
        .unwrap();
    let rep = r.run(None, &[9, 0], DEFAULT_CYCLES).unwrap();
    assert_eq!((rep.result, rep.halt), (0xFFFF, Halt::Returned));

    // The policy is part of the artifact: it survives the image round-trip (flag bit 4),
    // and a pre-policy image (bit absent) loads with the safe default.
    let prog =
        CellProgram::compile_with_config("fn run(a: u16, b: u16) -> u16 { a / b }", cfg).unwrap();
    let back = CellProgram::from_bytes(&prog.to_bytes()).unwrap();
    assert_eq!(back.cfg().div_by_zero, DivByZero::Saturate);
    let mut r2 = Runner::new(&back);
    assert_eq!(
        r2.run(None, &[9, 0], DEFAULT_CYCLES).unwrap().result,
        0xFFFF
    );
}

// ── the memoization cache (roadmap 3.3) ────────────────────────────────────────────

#[test]
fn cache_replays_identical_outcomes_and_counts_hits() {
    let mut r = Runner::compile("fn run(a: u16, b: u16) -> u16 { a * b + 1 }").unwrap();
    r.enable_cache();
    let first = r.run_fast(None, &[6, 7], DEFAULT_CYCLES).unwrap();
    let hit = r.run_fast(None, &[6, 7], DEFAULT_CYCLES).unwrap();
    // Byte-for-byte the same outcome: result, cycles, trapped_ops, halt.
    assert_eq!(hit.result, first.result);
    assert_eq!(hit.cycles, first.cycles);
    assert_eq!(hit.trapped_ops, first.trapped_ops);
    assert_eq!(hit.halt, first.halt);
    assert_eq!(r.cache_stats(), Some((1, 2)));

    // A different argument set is its own entry (miss, then hit).
    assert_eq!(r.run_fast(None, &[2, 3], DEFAULT_CYCLES).unwrap().result, 7);
    assert_eq!(r.run_fast(None, &[2, 3], DEFAULT_CYCLES).unwrap().result, 7);
    assert_eq!(r.cache_stats(), Some((2, 4)));
}

#[test]
fn cache_respects_the_budget_and_never_stores_budget_stops() {
    // A loop that takes real cycles: cached at a generous budget, then re-asked with a
    // budget smaller than the stored cycles — must NOT replay (a live run would have
    // stopped with CycleBudget first).
    let src = "fn run(n: u16) -> u16 { let mut s = 0u16; let mut i = 0u16;
               while i < n { s = s + i; i = i + 1; } s }";
    let mut r = Runner::compile(src).unwrap();
    r.enable_cache();
    let full = r.run_fast(None, &[100], DEFAULT_CYCLES).unwrap();
    assert_eq!(full.halt, cell80::Halt::Returned);
    assert!(full.cycles > 50);

    let tight = r.run_fast(None, &[100], 50).unwrap();
    assert_eq!(tight.halt, cell80::Halt::CycleBudget); // fresh run, not the cached return
                                                       // The budget stop itself must not have been stored: a full-budget ask still succeeds.
    let again = r.run_fast(None, &[100], DEFAULT_CYCLES).unwrap();
    assert_eq!(again.halt, cell80::Halt::Returned);
    assert_eq!(again.result, full.result);
}

#[test]
fn cache_disabled_by_default_and_cleared_on_pool_reuse() {
    use cell80::{CellPool, CellProgram};
    let mut r = Runner::compile("fn run() -> u16 { 1 }").unwrap();
    r.run_fast(None, &[], DEFAULT_CYCLES).unwrap();
    assert_eq!(r.cache_stats(), None); // opt-in, off by default

    // A pooled bus re-pointed at a different program must not replay the old one.
    let p1 = CellProgram::compile("fn run() -> u16 { 11 }").unwrap();
    let p2 = CellProgram::compile("fn run() -> u16 { 22 }").unwrap();
    let mut pool = CellPool::new();
    let mut a = pool.acquire(&p1);
    a.enable_cache();
    assert_eq!(a.run_fast(None, &[], DEFAULT_CYCLES).unwrap().result, 11);
    assert_eq!(a.run_fast(None, &[], DEFAULT_CYCLES).unwrap().result, 11);
    assert_eq!(a.cache_stats(), Some((1, 2)));
    pool.release(a);
    let mut b = pool.acquire(&p2);
    assert_eq!(b.run_fast(None, &[], DEFAULT_CYCLES).unwrap().result, 22);
    assert_eq!(b.cache_stats(), Some((0, 1))); // entries and counters both reset
}

#[test]
fn cache_stats_ride_the_report_and_its_json() {
    let mut r = Runner::compile("fn run(a: u16) -> u16 { a + 1 }").unwrap();
    let rep = r.run(None, &[1], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.cache_stats, None);
    assert!(!rep.to_json().contains("\"cache\""));

    r.enable_cache();
    r.run_fast(None, &[5], DEFAULT_CYCLES).unwrap();
    r.run_fast(None, &[5], DEFAULT_CYCLES).unwrap();
    let rep = r.run(None, &[1], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.cache_stats, Some((1, 2)));
    assert!(rep
        .to_json()
        .contains("\"cache\":{\"hits\":1,\"lookups\":2}"));
}

#[test]
fn host_enables_caching_on_load() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost};
    let cart = Cartridge::compile(
        "fn run(a: u16, b: u16) -> u16 { a + b }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("adder".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let mut host = CellHost::new();
    host.add(cart);
    host.set_cache(true);
    let h = host.load("adder").unwrap();
    host.run_fast(h, &[3, 4], DEFAULT_CYCLES).unwrap();
    host.run_fast(h, &[3, 4], DEFAULT_CYCLES).unwrap();
    assert_eq!(host.cache_stats(h).unwrap(), Some((1, 2)));
    host.unload(h).unwrap();
}

// ── the escalation contract (roadmap 3.2) ──────────────────────────────────────────

#[test]
fn escalation_band_decodes_as_a_typed_handoff() {
    use cell80::{Halt, ESCALATE_BASE};
    // A cell that answers small inputs and escalates past its domain.
    let src = "fn run(n: u16) -> u16 { if n > 1000 { halt(0xFF06u16); } n * 2 }";
    let mut r = Runner::compile(src).unwrap();
    assert_eq!(r.run(None, &[21], DEFAULT_CYCLES).unwrap().result, 42);

    let rep = r.run(None, &[2000], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.halt, Halt::Escalate(0xFF06));
    assert_eq!(rep.halt.escalate_reason(), Some("out_of_domain"));
    assert!(rep.to_json().contains("\"halt\":\"escalate\""));
    assert!(rep.to_json().contains("\"escalate\":\"out_of_domain\""));
    assert!(rep.to_json().contains("\"escalate_code\":65286"));
    assert!(rep.to_human().contains("escalated (out_of_domain"));

    // Below the band it's an ordinary explicit stop, exactly as before.
    let src = "fn run() -> u16 { halt(7u16); 0 }";
    let rep = cell80::run(src, None, &[], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.halt, Halt::Halted(7));
    assert_eq!(rep.halt.escalate_reason(), None);

    // The band floor itself escalates as "unspecified"; unnamed codes are "custom".
    assert_eq!(
        Halt::Escalate(ESCALATE_BASE).escalate_reason(),
        Some("unspecified")
    );
    assert_eq!(Halt::Escalate(0xFFAA).escalate_reason(), Some("custom"));
}

#[test]
fn manifest_limits_round_trip_and_render() {
    use cell80::{Cartridge, CartridgeOpts, CellConfig};
    let cart = Cartridge::compile(
        "fn run(a: u16, b: u16) -> u16 { a + b }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("adder".into()),
            limits: vec!["floats".into(), "sums > 65535".into()],
            ..Default::default()
        },
    )
    .unwrap();
    let bytes = cart.to_bytes();
    let back = Cartridge::from_bytes(&bytes).unwrap();
    assert_eq!(back.manifest.limits, vec!["floats", "sums > 65535"]);
    assert!(back.to_human().contains("limits: floats, sums > 65535"));
    assert!(back
        .to_json()
        .contains("\"limits\":[\"floats\",\"sums > 65535\"]"));

    // No declared limits → empty list, absent from the human view.
    let plain = Cartridge::compile(
        "fn run() -> u16 { 1 }",
        CellConfig::sandboxed(),
        CartridgeOpts::default(),
    )
    .unwrap();
    let back = Cartridge::from_bytes(&plain.to_bytes()).unwrap();
    assert!(back.manifest.limits.is_empty());
    assert!(!back.to_human().contains("limits:"));
}

#[test]
fn escalation_caches_like_any_deterministic_stop() {
    // An escalation is a property of (entry, args) — the memoization cache may replay it.
    let src = "fn run(n: u16) -> u16 { if n > 10 { halt(0xFF01u16); } n }";
    let mut r = Runner::compile(src).unwrap();
    r.enable_cache();
    let first = r.run_fast(None, &[99], DEFAULT_CYCLES).unwrap();
    let hit = r.run_fast(None, &[99], DEFAULT_CYCLES).unwrap();
    assert_eq!(first.halt, cell80::Halt::Escalate(0xFF01));
    assert_eq!(hit.halt, first.halt);
    assert_eq!(r.cache_stats(), Some((1, 2)));
}

// ── content-addressed, signed artifacts (roadmap 3.1) ─────────────────────────────

#[cfg(test)]
fn adder_cart() -> cell80::Cartridge {
    use cell80::{Cartridge, CartridgeOpts, CellConfig};
    Cartridge::compile(
        "fn run(a: u16, b: u16) -> u16 { a + b }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("adder".into()),
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn artifact_hash_round_trips_and_pins_content() {
    use cell80::Cartridge;
    let cart = adder_cart();
    let bytes = cart.to_bytes();
    let back = Cartridge::from_bytes(&bytes).unwrap();
    assert_eq!(back.artifact_hash(), cart.artifact_hash());
    assert!(back.to_json().contains("\"artifact_hash\":\"sha256:"));
    assert!(back.to_json().contains("\"signed\":false"));
    assert!(back.to_human().contains("(unsigned)"));

    // A different program is a different address; so is a different manifest.
    use cell80::{Cartridge as C, CartridgeOpts, CellConfig};
    let other = C::compile(
        "fn run(a: u16, b: u16) -> u16 { a - b }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("adder".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_ne!(other.artifact_hash(), cart.artifact_hash());
}

#[test]
fn scale_annotation_round_trips() {
    // The optional fixed-point scale (v7) survives serialization and shows up in the
    // inspect output; a scale-less cell reads back as `None`.
    use cell80::{Cartridge, CartridgeOpts, CellConfig};
    let cart = Cartridge::compile(
        "fn run(a: u16, b: u16) -> u16 { ((a as u32 * b as u32) >> 8u32) as u16 }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("q".into()),
            scale: Some(8),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(cart.manifest.scale, Some(8));
    let back = Cartridge::from_bytes(&cart.to_bytes()).unwrap();
    assert_eq!(back.manifest.scale, Some(8));
    assert_eq!(back.artifact_hash(), cart.artifact_hash());
    assert!(back.to_json().contains("\"scale\":8"));
    assert!(back.to_human().contains("scale: Q·8"));

    // A scale-less cell → None everywhere (the common case).
    let plain = Cartridge::compile(
        "fn run(a: u16) -> u16 { a }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("id".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(plain.manifest.scale, None);
    assert_eq!(
        Cartridge::from_bytes(&plain.to_bytes())
            .unwrap()
            .manifest
            .scale,
        None
    );
    assert!(plain.to_json().contains("\"scale\":null"));
    // A different scale is a different artifact (the hash covers it).
    assert_ne!(back.artifact_hash(), plain.artifact_hash());
}

#[test]
fn tampered_bytes_are_refused_by_default_and_loadable_unverified() {
    use cell80::Cartridge;
    let bytes = adder_cart().to_bytes();

    // Flip a bit in the image (the last byte): parses fine, so only the hash refuses it.
    let mut evil = bytes.clone();
    let n = bytes.len();
    evil[n - 1] ^= 0x01;
    let err = Cartridge::from_bytes(&evil)
        .err()
        .expect("tampered load must fail");
    assert!(err.contains("hash mismatch"), "{err}");

    // A manifest-region flip must not load cleanly either (structural error or hash
    // mismatch, depending on which byte the flip lands in — refusal is the contract).
    let mut evil = bytes.clone();
    evil[10] ^= 0x01;
    assert!(Cartridge::from_bytes(&evil).is_err());

    // The dev escape hatch parses (structure permitting) without the hash check.
    let mut evil = bytes.clone();
    let n = evil.len();
    evil[n - 1] ^= 0x01;
    assert!(Cartridge::from_bytes_unverified(&evil).is_ok());
}

#[test]
fn signing_round_trips_and_forgeries_fail() {
    use cell80::Cartridge;
    let mut cart = adder_cart();
    let seed = [7u8; 32];
    cart.sign(&seed);
    let bytes = cart.to_bytes();

    let back = Cartridge::from_bytes(&bytes).unwrap(); // hash + signature both verify
    assert!(back.signature.is_some());
    assert!(back.to_json().contains("\"signed\":true"));
    assert!(back.to_human().contains("signed, key ed25519:"));

    // Corrupt one signature byte: the hash still matches, the signature must not.
    // Layout: [manifest][hash 32][marker 1][vk 32][sig 64][img_len 4][img] — the
    // signature block ends 4 + img_len bytes before the end.
    let img_len = cart.program.to_bytes().len();
    let sig_last = bytes.len() - 4 - img_len - 1;
    let mut forged = bytes.clone();
    forged[sig_last] ^= 0x01;
    let err = Cartridge::from_bytes(&forged)
        .err()
        .expect("forged load must fail");
    assert!(err.contains("signature"), "{err}");

    // An unsigned re-serialization of the same content keeps the same address.
    let unsigned = adder_cart();
    assert_eq!(unsigned.artifact_hash(), cart.artifact_hash());
}

#[test]
fn pre_v5_cartridges_still_load() {
    use cell80::Cartridge;
    // A v4 image has no limits list, no hash, no signature block: rebuild one from a
    // v5 byte stream by splicing those fields out and rewriting the version byte.
    let cart = adder_cart();
    let v5 = cart.to_bytes();
    let img = cart.program.to_bytes();
    let img_block_len = 4 + img.len();
    let manifest_end = v5.len() - img_block_len - 33; // hash(32) + unsigned marker(1)
    let mut v4 = Vec::new();
    // Drop the trailing v5/v7/v8 manifest fields a v4 stream never had: the empty
    // limits u16 (2 bytes), the v7 scale presence byte (1), and the v8
    // finite_result byte (1).
    v4.extend_from_slice(&v5[..manifest_end - 4]);
    v4.extend_from_slice(&v5[v5.len() - img_block_len..]);
    v4[4] = 4; // version byte
    let back = Cartridge::from_bytes(&v4).unwrap(); // no hash → grandfathered, verified load path
    assert_eq!(back.manifest.id, "adder");
    assert!(back.manifest.limits.is_empty());
    assert!(back.signature.is_none());
}

#[test]
fn abi_v3_bytes_field_manifest() {
    // A byte-packed `[u8; N]` state field is declared in the manifest as
    // `bytes[N]` (ABI v3 / .cell v6): name-addressed with its capacity, surviving
    // the byte round-trip — while the scalar set/get paths reject it cleanly
    // until the S3 byte-I/O surface.
    use cell80::{Cartridge, CartridgeOpts, CellConfig, StateCell, Ty};
    let src = "
        struct Out { len: u16, buf: [u8; 8], score: u32 }
        impl Out {
            fn run(&mut self) -> u16 {
                self.buf[0] = b'h';
                self.len = 1u16;
                self.len
            }
        }
    ";
    let cart = Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            entry: Some("Out::run".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let addrs = &cart.manifest.state_addrs;
    let by_name = |n: &str| addrs.iter().find(|(name, ..)| name == n).unwrap();
    assert_eq!(by_name("buf").2, Ty::Bytes(8));
    assert_eq!(by_name("len").2, Ty::U16);
    assert_eq!(by_name("score").2, Ty::U32);
    // buf sits after len (1 slot): STATE_BASE + 2; score after buf's 4 slots.
    assert_eq!(by_name("buf").1, cell80::STATE_BASE + 2);
    assert_eq!(by_name("score").1, cell80::STATE_BASE + 2 + 8);

    // Round-trip: the capacity survives the v6 wire format.
    let back = Cartridge::from_bytes(&cart.to_bytes()).unwrap();
    assert_eq!(back.manifest.state_addrs, cart.manifest.state_addrs);

    // Scalar paths reject the buffer with a steering message.
    let mut cell = StateCell::bind(src, "Out", None).unwrap();
    let err = cell.set("buf", 1).unwrap_err();
    assert!(err.contains("bytes[8]"), "unexpected: {err}");
    cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(cell.get("buf"), None); // no scalar misread
    assert_eq!(cell.get("len"), Some(1));
}

#[test]
fn abi_v3_ty_parse_display() {
    use cell80::Ty;
    for (s, ty) in [
        ("u8", Ty::U8),
        ("u16", Ty::U16),
        ("u32", Ty::U32),
        ("bytes[64]", Ty::Bytes(64)),
        ("str[1024]", Ty::Str(1024)),
    ] {
        assert_eq!(Ty::parse(s).unwrap(), ty);
        assert_eq!(ty.to_string(), s);
    }
    assert!(Ty::parse("bytes[x]").is_err());
    assert!(Ty::parse("float").is_err());
}

#[test]
fn run_state_fast_caches_the_scoring_workhorse() {
    // docs/12 §2 (delta two): state cells — the scoring family — go through the
    // memo table. The cached outcome is byte-for-byte the live one, repeats are
    // hash lookups, and a different field set is a different fact.
    use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost};
    let src = "
        struct Score { wx: u16, wy: u16, x: u16, y: u16, total: u32 }
        impl Score {
            fn run(&mut self) -> u16 {
                self.total = (self.wx as u32) * (self.x as u32)
                    + (self.wy as u32) * (self.y as u32);
                (self.total >> 16) as u16
            }
        }
    ";
    let cart = Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("score.v1".into()),
            entry: Some("Score::run".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let mut host = CellHost::new();
    host.set_cache(true);
    host.add(cart);
    let h = host.load("score.v1").unwrap();

    let fields = vec![
        ("wx".into(), 3u64),
        ("wy".into(), 5),
        ("x".into(), 17),
        ("y".into(), 40),
    ];
    let (f1, s1) = host.run_state_fast(h, &fields, DEFAULT_CYCLES).unwrap();
    let (f2, s2) = host.run_state_fast(h, &fields, DEFAULT_CYCLES).unwrap();
    // Identical outcome, and the repeat was a hit.
    assert_eq!(
        (f1.result, f1.regs, f1.cycles, f1.trapped_ops),
        (f2.result, f2.regs, f2.cycles, f2.trapped_ops)
    );
    assert_eq!(s1, s2);
    let by = |s: &Vec<(String, u64)>, n: &str| s.iter().find(|(k, _)| k == n).unwrap().1;
    assert_eq!(by(&s1, "total"), 3 * 17 + 5 * 40);
    assert_eq!(host.cache_stats(h).unwrap(), Some((1, 2)));

    // Matches the uncached rich path field-for-field.
    let (_, live) = host.run_state(h, &fields, DEFAULT_CYCLES).unwrap();
    assert_eq!(s1, live);

    // A different field set misses (a different fact).
    let fields2 = vec![
        ("wx".into(), 4u64),
        ("wy".into(), 5),
        ("x".into(), 17),
        ("y".into(), 40),
    ];
    let (_f3, s3) = host.run_state_fast(h, &fields2, DEFAULT_CYCLES).unwrap();
    assert_eq!(by(&s3, "total"), 4 * 17 + 5 * 40);
    assert_ne!(by(&s3, "total"), by(&s1, "total"));
    assert_eq!(host.cache_stats(h).unwrap(), Some((1, 3)));
}

#[test]
fn run_state_fast_budget_and_order_rules() {
    // The strict-replay rule holds for state facts, and field order doesn't
    // change the fact (canonical key: sorted by address).
    use cell80::StateCell;
    let src = "
        struct S { a: u16, b: u16, out: u16 }
        impl S { fn run(&mut self) -> u16 { self.out = self.a + self.b; self.out } }
    ";
    let mut r = Runner::compile(src).unwrap();
    r.enable_cache();
    let layout = rustz80::struct_layout(src, "S").unwrap();
    let addr =
        |n: &str| cell80::STATE_BASE + layout.iter().find(|f| f.name == n).unwrap().offset * 2;
    let reads = vec![("out".to_string(), addr("out"), cell80::Ty::U16)];
    let ab = vec![
        (addr("a"), cell80::Ty::U16, 30u64),
        (addr("b"), cell80::Ty::U16, 12u64),
    ];
    let ba = vec![ab[1], ab[0]];
    let (f1, s1) = r
        .run_state_fast(Some("S::run"), &ab, &reads, DEFAULT_CYCLES)
        .unwrap();
    assert_eq!(s1, vec![("out".to_string(), 42u64)]);
    // Same fields, different order → the same fact (a hit).
    let (_, s2) = r
        .run_state_fast(Some("S::run"), &ba, &reads, DEFAULT_CYCLES)
        .unwrap();
    assert_eq!(s1, s2);
    assert_eq!(r.cache_stats(), Some((1, 2)));
    // The replay rule is strict `<`: at budget == stored cycles the cache is
    // skipped — and the live run *completes* (the final instruction starts while
    // cycles < budget), so equality is a conservative miss with the same outcome.
    let (f3, _) = r
        .run_state_fast(Some("S::run"), &ab, &reads, f1.cycles)
        .unwrap();
    assert_eq!(f3.halt, cell80::Halt::Returned);
    assert_eq!(f3.cycles, f1.cycles);
    // Well under the recorded cost, the live run is out of budget. (Near-misses
    // can still return: the guard is checked at loop top, so the final
    // instruction may finish up to one instruction past the budget.)
    let (f4, _) = r.run_state_fast(Some("S::run"), &ab, &reads, 8).unwrap();
    assert_eq!(f4.halt, cell80::Halt::CycleBudget);
    let _ = StateCell::bind(src, "S", None).unwrap(); // still binds (sanity)
}

#[test]
fn u32_param_entry_drives_from_host_as_two_words() {
    // The one-wide-param convention meets the host args convention exactly: a
    // `fn run(w: u32)` entry takes `args = [low, high]` (HL:DE *is* the u32),
    // and a wide return comes back as regs[0] (low) / regs[1] (high).
    let mut r = Runner::compile("fn run(w: u32) -> u32 { w + w }").unwrap();
    let w: u32 = 0x0001_8003; // 98307
    let f = r
        .run_fast(
            None,
            &[(w & 0xFFFF) as u16, (w >> 16) as u16],
            DEFAULT_CYCLES,
        )
        .unwrap();
    let got = f.regs[0] as u32 | (f.regs[1] as u32) << 16;
    assert_eq!(got, w + w);
}
