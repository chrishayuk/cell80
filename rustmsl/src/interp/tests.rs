use super::*;
use cell80_core::ir::{BinOp, Expr, Func, Stmt, Width};
use cell80_core::{Interp, Target};

/// Build a one-function library whose `run` returns `ret`.
fn cell(params: usize, n_locals: usize, body: Vec<Stmt>, ret: Expr) -> Vec<(String, Func)> {
    vec![(
        "run".into(),
        Func {
            params,
            n_locals,
            body,
            ret: vec![ret],
            wide_param: false,
            wide_second: false,
            wide_ret: false,
        },
    )]
}

/// cpu_run must match Interp bit-for-bit (values AND steps) on `args`.
fn assert_parity(funcs: &[(String, Func)], args: &[u16]) {
    let prog = linearize(funcs, "run").expect("linearizes");
    let mut interp = Interp::new(
        funcs,
        Vec::<(&str, &[u8])>::new(),
        Target::Cell.descriptor(),
    );
    let iref = interp.run("run", args);
    let isteps = interp.steps();
    match (iref, cpu_run(&prog, args)) {
        (Ok(v), VmOut::Value(o, s)) => {
            assert_eq!(v, o, "values @ {args:?}");
            assert_eq!(isteps, s, "steps @ {args:?}");
        }
        (Err(e), out) if e.contains("divide by zero") => assert!(matches!(out, VmOut::DivZero)),
        (a, b) => panic!("mismatch @ {args:?}: interp={a:?} vm={b:?}"),
    }
}

#[test]
fn arithmetic_and_steps() {
    // run(x, y) = (x + y) * x   over Word
    let add = Expr::Bin(
        BinOp::Add,
        Box::new(Expr::Var(0)),
        Box::new(Expr::Var(1)),
        Width::Word,
    );
    let mul = Expr::Bin(
        BinOp::Mul,
        Box::new(add),
        Box::new(Expr::Var(0)),
        Width::Word,
    );
    let c = cell(2, 2, vec![], mul);
    for args in [[3u16, 4], [0, 0], [65535, 1], [12345, 6789]] {
        assert_parity(&c, &args);
    }
}

#[test]
fn div_by_zero_traps() {
    // run(x) = x / (x - x)  — always divide by zero
    let z = Expr::Bin(
        BinOp::Sub,
        Box::new(Expr::Var(0)),
        Box::new(Expr::Var(0)),
        Width::Word,
    );
    let d = Expr::Bin(BinOp::Div, Box::new(Expr::Var(0)), Box::new(z), Width::Word);
    assert_parity(&cell(1, 1, vec![], d), &[7]);
}

#[test]
fn signed_min_div_neg_one_wraps() {
    // i16::MIN / -1 wraps to i16::MIN (0x8000), not a trap.
    let d = Expr::Bin(
        BinOp::Div,
        Box::new(Expr::Lit(0x8000)),
        Box::new(Expr::Lit(0xFFFF)),
        Width::SWord,
    );
    assert_parity(&cell(1, 1, vec![], d), &[0]);
}

#[test]
fn loop_and_control_flow() {
    // run(n): s=0; i=0; while i<n { s = s + i; i = i + 1 } ; return s
    use cell80_core::ir::{Cmp, Cond};
    let cond = Cond {
        cmp: Cmp::Lt,
        lhs: Expr::Var(2),
        rhs: Expr::Var(0),
        signed: false,
    };
    let body = vec![
        Stmt::Assign(
            1,
            Expr::Bin(
                BinOp::Add,
                Box::new(Expr::Var(1)),
                Box::new(Expr::Var(2)),
                Width::Word,
            ),
        ),
        Stmt::Assign(
            2,
            Expr::Bin(
                BinOp::Add,
                Box::new(Expr::Var(2)),
                Box::new(Expr::Lit(1)),
                Width::Word,
            ),
        ),
    ];
    let c = cell(
        1,
        3,
        vec![
            Stmt::Assign(1, Expr::Lit(0)),
            Stmt::Assign(2, Expr::Lit(0)),
            Stmt::While(cond, body),
        ],
        Expr::Var(1),
    );
    for n in [0u16, 1, 5, 100] {
        assert_parity(&c, &[n]);
    }
}

#[test]
fn bits_intrinsic() {
    let call = Expr::Call("__bits_count_ones".into(), vec![Expr::Var(0)]);
    for x in [0u16, 1, 0xF0F0, 0xFFFF] {
        assert_parity(&cell(1, 1, vec![], call.clone()), &[x]);
    }
}

#[test]
fn wide_returning_inlined_call() {
    // helper(x: u16) -> u32 { (x as u32) * 2 }
    // run(x)          -> u32 { helper(x) + 1 }   — a wide call inlined in u32 position
    let mul = Expr::Bin32(
        BinOp::Mul,
        Box::new(Expr::Widen(Box::new(Expr::Var(0)))),
        Box::new(Expr::Lit32(2)),
        false,
    );
    let helper = Func {
        params: 1,
        n_locals: 1,
        body: vec![],
        ret: vec![mul],
        wide_param: false,
        wide_second: false,
        wide_ret: true,
    };
    let call = Expr::Bin32(
        BinOp::Add,
        Box::new(Expr::Call("helper".into(), vec![Expr::Var(0)])),
        Box::new(Expr::Lit32(1)),
        false,
    );
    let run = Func {
        params: 1,
        n_locals: 1,
        body: vec![],
        ret: vec![call],
        wide_param: false,
        wide_second: false,
        wide_ret: true,
    };
    let funcs = vec![("run".to_string(), run), ("helper".to_string(), helper)];
    for x in [0u16, 5, 1000, 40000] {
        assert_parity(&funcs, &[x]);
    }
}

/// `InterpBatch::run` must match `cpu_run` bit-for-bit on **every** probe, at a
/// probe count (65,536) that exceeds `max_total_threads_per_threadgroup()` on
/// any Metal device this runs on (a single-digit-thousands cap is typical;
/// deliberately overkill so this still catches a regression on hardware with a
/// higher cap than the one this bug was found on).
///
/// Found live via an evolutionary search (`cell80/examples/gpu_fanout_gate.rs`,
/// `experiments/cell-fanout-gate-preregistration.md`'s InterpBatch amendment): a
/// single dispatch assigns one thread per probe with no internal loop, so every
/// probe beyond `max_tpg` went unwritten and read back as a false
/// `status=0, r0=0` ("succeeded, value 0") — silently wrong, not an error. This
/// is exactly the defect class `msl_battery.rs`'s "no silent caps" discipline
/// exists to catch, except that battery only covers `GpuBatch` (the codegen
/// path, `rustmsl::runtime`); `InterpBatch` (`rustmsl::interp::gpu`, the
/// bytecode interpreter every dynamic/evolved-candidate search in this codebase
/// uses) had no equivalent coverage until this test. The cell shape below —
/// `while c<16 && (v & 0x8000)!=0 { v<<=1; c+=1 }; c` (leading-ones count) — is
/// the exact one that exposed it, its whole-domain upper half (x >= 0x8000)
/// being where the bug lived.
#[test]
#[cfg(target_os = "macos")]
fn interp_batch_matches_cpu_run_beyond_one_threadgroup() {
    let src = "fn run(x: u16) -> u16 { \
        let mut v = x; let mut c = 0u16; \
        while c < 16u16 && (v & 0x8000u16) != 0u16 { v = v << 1u16; c = c + 1u16; } \
        c }";
    let file: syn::File = syn::parse_str(src).expect("parses");
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())
        .unwrap_or_else(|e| panic!("lower failed: {e}"));
    let prog = linearize(&lowered.funcs, "run").expect("linearizes");
    let (batch, _skipped) = InterpBatch::new(std::slice::from_ref(&prog)).expect("metal device");
    let probes: Vec<[u16; IN_STRIDE]> = (0..=u16::MAX).map(|v| [v, 0, 0]).collect();
    let out = batch.run(&probes);
    let np = probes.len();
    for v in 0..=u16::MAX {
        let cpu = match cpu_run(&prog, &[v]) {
            VmOut::Value(r, _) => r.first().copied(),
            other => panic!("cpu_run non-value @ {v}: {other:?}"),
        };
        let o = out[v as usize % np]; // n_cells == 1, so cell-major == probe-major
        let gpu = if o[3] == 0 { Some(o[0]) } else { None };
        assert_eq!(cpu, gpu, "InterpBatch vs cpu_run mismatch @ x={v:#06x}");
    }
}
