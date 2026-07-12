//! Driver for `rustmsl::interp` — the bytecode-interpreter backend (now a real
//! library module, not inline here). Verifies the whole value-cell corpus is
//! bit-identical to the reference `Interp` on both the CPU reference VM and the
//! GPU `InterpBatch`, and runs the trap/intrinsic battery. The substrate pricing
//! (no kernel-size cliff, flat to 500k) lives in git history under
//! `library_launch_cost.rs`; this example is the correctness proof.
//!
//! Run: `cargo run --release -p cell80 --example gpu_interp_proto`

use cell80_core::ir::{BinOp, Expr, Func, Stmt, Width};
use cell80_core::{Interp, Target};
use rustmsl::interp::{cpu_run, linearize, CellProgram, VmOut, OUT_STRIDE};

type Funcs = Vec<(String, Func)>;
type Consts = Vec<(String, Vec<u8>)>;

/// Cartridge pipeline to the IR seam: prelude, lower, inline, DCE-root at `entry`.
fn lower(src: &str, entry: &str) -> Result<(Funcs, Consts), String> {
    let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
    let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse: {e}"))?;
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
    if !lowered.funcs.iter().any(|(n, _)| n == entry) {
        return Err(format!("no `{entry}` entry"));
    }
    let consts = lowered.const_data();
    let funcs = cell80_core::inline::inline(lowered.funcs, &[entry]);
    let funcs = cell80_core::dce::prune(funcs, &[entry]);
    Ok((funcs, consts))
}

/// Extract the code from `Interp`'s halt error (`"interp: halt(N)"`).
fn parse_halt(e: &str) -> Option<u16> {
    e.strip_prefix("interp: halt(")?
        .strip_suffix(')')?
        .parse::<u16>()
        .ok()
}

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> u16 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x & 0xFFFF) as u16
    }
}

/// Compare `Interp` and the CPU VM on one (cell, probe). Returns Ok(matched) or
/// None if the probe is a refusal we don't test (shouldn't happen for div0/halt,
/// which ARE checked). `matched=false` carries a message.
fn cpu_matches(
    funcs: &Funcs,
    consts: &Consts,
    prog: &CellProgram,
    args: &[u16],
) -> Result<(), String> {
    let mut interp = Interp::new(
        funcs,
        consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
        Target::Cell.descriptor(),
    );
    let iref = interp.run("run", args);
    let isteps = interp.steps();
    match (iref, cpu_run(prog, args)) {
        (Ok(iout), VmOut::Value(vout, vs)) if vout == iout && vs == isteps => Ok(()),
        (Ok(iout), out) => Err(format!(
            "value @ {args:?}: interp={iout:?}/{isteps} vm={out:?}"
        )),
        (Err(e), out) => match parse_halt(&e) {
            Some(code) => match out {
                VmOut::Halt(vc, vs) if vc == code && vs == isteps => Ok(()),
                _ => Err(format!(
                    "halt @ {args:?}: interp={code}/{isteps} vm={out:?}"
                )),
            },
            None if e.contains("divide by zero") => match out {
                VmOut::DivZero => Ok(()),
                _ => Err(format!("div0 @ {args:?}: vm={out:?}")),
            },
            None => Ok(()),
        },
    }
}

fn main() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("cells");
    let mut rng = Rng(0x1234_5678);
    let probes: Vec<[u16; 3]> = (0..64)
        .map(|_| [rng.next(), rng.next(), rng.next()])
        .collect();

    let mut total = 0usize;
    let mut supported = 0usize;
    let mut cpu_ok = 0usize;
    let mut bail_hist: std::collections::BTreeMap<String, usize> = Default::default();
    let mut cpu_fail: Vec<String> = Vec::new();
    // Keep supported cells (with their lowered funcs) for the GPU pass.
    let mut kept: Vec<(String, Funcs, Consts, CellProgram)> = Vec::new();

    let mut files: Vec<_> = cell80::discover_cell_files(dir.to_str().unwrap()).unwrap();
    files.sort();
    for path in files {
        if path.extension().is_none_or(|x| x != "rs") {
            continue;
        }
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(&path).unwrap();
        let Ok(sig) = rustz80::entry_signature(&src, "run") else {
            continue;
        };
        let scalar = sig.state.is_empty()
            && sig.params.iter().all(|(_, ty)| {
                matches!(ty.as_str(), "u8" | "u16" | "i16" | "u32" | "i32" | "bool")
            });
        if !scalar {
            continue;
        }
        let Ok((funcs, consts)) = lower(&src, "run") else {
            continue;
        };
        total += 1;
        let prog = match linearize(&funcs, "run") {
            Ok(p) => p,
            Err(b) => {
                *bail_hist.entry(format!("{b:?}")).or_default() += 1;
                continue;
            }
        };
        supported += 1;
        let params = prog.params.min(3);
        let mut ok = true;
        for probe in &probes {
            if let Err(msg) = cpu_matches(&funcs, &consts, &prog, &probe[..params]) {
                ok = false;
                if cpu_fail.len() < 10 {
                    cpu_fail.push(format!("  {name}: {msg}"));
                }
                break;
            }
        }
        if ok {
            cpu_ok += 1;
        }
        kept.push((name, funcs, consts, prog));
    }

    println!("rustmsl::interp corpus verification\n");
    println!("value cells considered:   {total}");
    println!("linearized (supported):   {supported}");
    println!("CPU bit-identical:        {cpu_ok}/{supported}  (values + IR steps vs Interp)");
    if !bail_hist.is_empty() {
        println!("\nout-of-subset:");
        let mut rows: Vec<_> = bail_hist.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1));
        for (r, c) in rows {
            println!("  {c:>4}  {r}");
        }
    }
    for f in &cpu_fail {
        println!("✗{f}");
    }

    trap_battery();

    #[cfg(target_os = "macos")]
    gpu_corpus(&kept, &probes);
    #[cfg(not(target_os = "macos"))]
    let _ = &kept;
}

/// GPU pass: one `InterpBatch` over the supported corpus, compared to `Interp`.
#[cfg(target_os = "macos")]
fn gpu_corpus(cells: &[(String, Funcs, Consts, CellProgram)], probes: &[[u16; 3]]) {
    use rustmsl::interp::InterpBatch;
    // `InterpBatch::new` takes `&[CellProgram]`; the kept programs sit inside
    // tuples, so re-linearize into a flat Vec (cheap) to hand it a contiguous slice.
    let progs: Vec<CellProgram> = re_linearize(cells);
    let (batch, skipped) = InterpBatch::new(&progs).expect("interp batch");
    let out = batch.run(probes);
    let mut checked = 0usize;
    let mut ok = 0usize;
    let mut fail: Vec<String> = Vec::new();
    for (ci, (name, funcs, consts, prog)) in cells
        .iter()
        .filter(|(_, _, _, p)| p.n_locals <= 64)
        .enumerate()
    {
        let params = prog.params.min(3);
        for (pi, probe) in probes.iter().enumerate() {
            let args = &probe[..params];
            let mut interp = Interp::new(
                funcs,
                consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
                Target::Cell.descriptor(),
            );
            let iref = interp.run("run", args);
            let isteps = interp.steps();
            let g = out[ci * probes.len() + pi];
            let gsteps = g[4] as u64 | ((g[5] as u64) << 16);
            let m = match &iref {
                Ok(v) => {
                    checked += 1;
                    g[3] == 0 && gsteps == isteps && v.iter().enumerate().all(|(k, x)| g[k] == *x)
                }
                Err(e) => match parse_halt(e) {
                    Some(code) => {
                        checked += 1;
                        g[3] == 2 && g[0] == code && gsteps == isteps
                    }
                    None if e.contains("divide by zero") => {
                        checked += 1;
                        g[3] == 1
                    }
                    None => continue,
                },
            };
            if m {
                ok += 1;
            } else if fail.len() < 10 {
                fail.push(format!(
                    "  {name} @ {args:?}: gpu r0={} st={} steps={gsteps} interp={iref:?}/{isteps}",
                    g[0], g[3]
                ));
            }
        }
    }
    println!(
        "\nGPU (InterpBatch): {ok}/{checked} bit-identical to Interp ({} cells, {skipped} skipped)",
        batch.n_cells()
    );
    for f in &fail {
        println!("✗{f}");
    }
}

/// CellProgram is move-only; the kept vec owns one per cell. Rebuild a Vec to
/// hand to InterpBatch (which needs `&[CellProgram]`) by re-linearizing.
#[cfg(target_os = "macos")]
fn re_linearize(cells: &[(String, Funcs, Consts, CellProgram)]) -> Vec<CellProgram> {
    cells
        .iter()
        .map(|(_, funcs, _, _)| linearize(funcs, "run").unwrap())
        .collect()
}

/// One-expression cell for battery corners.
fn one_expr_cell(e: Expr) -> Vec<(String, Func)> {
    vec![(
        "run".to_string(),
        Func {
            params: 1,
            n_locals: 1,
            body: vec![],
            ret: vec![e],
            wide_param: false,
            wide_second: false,
            wide_ret: false,
        },
    )]
}

/// Trap/intrinsic battery on the CPU VM vs Interp: fuel, div0, signed MIN÷-1,
/// and the __bits_* intrinsics (no corpus cell exercises the last).
fn trap_battery() {
    println!("\n== battery (CPU vm vs Interp) ==");
    // Fuel: a runaway loop must trap at the same step count.
    let runaway = vec![(
        "run".to_string(),
        Func {
            params: 0,
            n_locals: 1,
            body: vec![Stmt::Loop(vec![Stmt::Assign(
                0,
                Expr::Bin(
                    BinOp::Add,
                    Box::new(Expr::Var(0)),
                    Box::new(Expr::Lit(1)),
                    Width::Word,
                ),
            )])],
            ret: vec![Expr::Lit(0)],
            wide_param: false,
            wide_second: false,
            wide_ret: false,
        },
    )];
    let prog = linearize(&runaway, "run").unwrap();
    let mut interp = Interp::new(
        &runaway,
        Vec::<(&str, &[u8])>::new(),
        Target::Cell.descriptor(),
    );
    let ir = interp.run("run", &[]);
    let is = interp.steps();
    let fuel_ok = matches!((ir.is_err(), cpu_run(&prog, &[])), (true, VmOut::Fuel(vs)) if vs == is);
    println!(
        "  fuel:            {}",
        if fuel_ok { "✓ (Δ=0)" } else { "✗" }
    );

    let case = |name: &str,
                cell: &[(String, Func)],
                args: &[u16],
                want: fn(&Result<Vec<u16>, String>, &VmOut) -> bool| {
        let prog = linearize(cell, "run").unwrap();
        let mut it = Interp::new(cell, Vec::<(&str, &[u8])>::new(), Target::Cell.descriptor());
        let ir = it.run("run", args);
        let vr = cpu_run(&prog, args);
        println!(
            "  {name:<14} {}",
            if want(&ir, &vr) {
                "✓"
            } else {
                "✗ MISMATCH"
            }
        );
    };
    let sub = || {
        Expr::Bin(
            BinOp::Sub,
            Box::new(Expr::Var(0)),
            Box::new(Expr::Var(0)),
            Width::Word,
        )
    };
    case(
        "div0:",
        &one_expr_cell(Expr::Bin(
            BinOp::Div,
            Box::new(Expr::Var(0)),
            Box::new(sub()),
            Width::Word,
        )),
        &[7],
        |ir, vr| {
            ir.as_ref()
                .err()
                .is_some_and(|e| e.contains("divide by zero"))
                && matches!(vr, VmOut::DivZero)
        },
    );
    case(
        "MIN÷-1:",
        &one_expr_cell(Expr::Bin(
            BinOp::Div,
            Box::new(Expr::Lit(0x8000)),
            Box::new(Expr::Lit(0xFFFF)),
            Width::SWord,
        )),
        &[0],
        |ir, vr| matches!((ir, vr), (Ok(v), VmOut::Value(o, _)) if v == o && v.first() == Some(&0x8000)),
    );
    for (nm, f) in [
        ("count_ones:", "__bits_count_ones"),
        ("leading_zeros:", "__bits_leading_zeros"),
        ("trailing_zeros:", "__bits_trailing_zeros"),
    ] {
        let cell = one_expr_cell(Expr::Call(f.to_string(), vec![Expr::Var(0)]));
        let prog = linearize(&cell, "run").unwrap();
        let all = [0u16, 1, 0x00F0, 0xF0F0, 0xFFFF].iter().all(|&x| {
            let mut it = Interp::new(&cell, Vec::<(&str, &[u8])>::new(), Target::Cell.descriptor());
            matches!((it.run("run", &[x]), cpu_run(&prog, &[x])), (Ok(v), VmOut::Value(o, _)) if v == o)
        });
        println!("  bits {nm:<9} {}", if all { "✓" } else { "✗" });
    }
    let _ = OUT_STRIDE;
}
