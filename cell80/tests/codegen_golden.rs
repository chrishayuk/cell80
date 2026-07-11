//! Byte-identity golden for the rustz80 codegen (roadmap Phase 4).
//!
//! Renders every corpus program — the 100 stdlib cells (Cell target, the full
//! prelude+inline+DCE pipeline), the rustz80 showcase samples (both targets), the
//! `codegen_loop` frame-loop entry, and sources that force each appended software
//! runtime — as its per-fn size report plus the full image hex, and compares against
//! the committed golden.
//!
//! The Phase 4.1 `Ins` seam must keep this file **byte-identical**. Phase 4.2
//! peephole rules change it deliberately: regenerate with `UPDATE_GOLDEN=1` and
//! review the size deltas in the diff — the golden is also the size baseline to beat.
//!
//! Regenerate: `UPDATE_GOLDEN=1 cargo test -p cell80 --test codegen_golden`

use cell80::CellProgram;
use rustz80::{compile_program_for, Target};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const TARGETS: [(Target, &str); 2] = [(Target::Spectrum48, "spectrum"), (Target::Cell, "cell")];

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

/// Sorted `*.rs` paths in `dir` — deterministic corpus order.
fn rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();
    files
}

/// One program's golden block: a header, the per-fn size report, the image hex.
fn render_program(out: &mut String, name: &str, prog: &rustz80::Program) {
    writeln!(out, "program {name} len={}", prog.code.len()).unwrap();
    for f in prog.size_report() {
        writeln!(out, "  fn {} addr={:#06x} size={}", f.name, f.addr, f.size).unwrap();
    }
    writeln!(out, "  image {}", hex(&prog.code)).unwrap();
}

fn render() -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = String::new();

    // 1. The stdlib cells — the CellProgram pipeline (prelude append, cap check,
    //    inline, DCE, Cell target). Cells live in pack subdirectories, discovered
    //    recursively (rs_files below is flat, reused as-is for the showcase samples).
    let mut cell_paths: Vec<PathBuf> =
        cell80::discover_cell_files(manifest.join("cells").to_str().unwrap())
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .collect();
    cell_paths.sort();
    for path in cell_paths {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let src = fs::read_to_string(&path).unwrap();
        // A `//! kernel_bank: on` cell (the resident-bank feature, `docs/09-cell80-abi.md`)
        // calls into the shared softfloat bank instead of inlining the f32 kernel family —
        // some (e.g. excel_rri, the Finance80 batch's Nth-root Newton solver) genuinely
        // can't compile any other way: fully inlining every kernel it touches (sqrt, mul,
        // div, ...) alongside its own locals overruns the scratch region into STATE_BASE.
        // Render those through the same banked path `library_cartridge`/the admission gate
        // use (`cli/meta.rs`), rather than the plain unbanked `CellProgram::compile` every
        // other cell here uses — otherwise this golden can never include such a cell at all.
        let prog = if src.contains("//! kernel_bank: on") {
            CellProgram::compile_with_config_banked(&src, cell80::CellConfig::permissive())
                .unwrap_or_else(|e| panic!("cell {name} failed to compile (banked): {e}"))
        } else {
            CellProgram::compile(&src)
                .unwrap_or_else(|e| panic!("cell {name} failed to compile: {e}"))
        };
        render_program(&mut out, &format!("cell/{name}"), prog.program());
    }

    // 2. The showcase samples — whole-program codegen on both targets (the Spectrum
    //    side exercises the appended software mul/div runtime).
    for path in rs_files(&manifest.join("../rustz80/samples/showcase")) {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let src = fs::read_to_string(&path).unwrap();
        for (target, tname) in TARGETS {
            let prog = compile_program_for(&src, target)
                .unwrap_or_else(|e| panic!("showcase {name} ({tname}) failed: {e}"));
            render_program(&mut out, &format!("showcase/{name}/{tname}"), &prog);
        }
    }

    // 3. The frame-loop entry (`codegen_loop`) — the SDK/game path: state-zeroing
    //    prologue variants and the code-relative scratch placement.
    let loop_src = "
        fn helper(x: u16) -> u16 { x * 3u16 + 1u16 }
        fn update(state: u16, _b: u16, _c: u16) {
            let mut i = 0u16;
            while i < 8u16 {
                poke(state + i, helper(i) as u8);
                i = i + 1u16;
            }
        }
    ";
    let file: syn::File = syn::parse_str(loop_src).unwrap();
    let funcs = rustz80::lower_program(&file, &rustz80::PreludeConfig::default()).unwrap();
    for state_bytes in [6u16, 1, 0] {
        let code =
            rustz80::codegen_loop(&funcs, rustz80::ORG, "update", 0xB000, state_bytes).unwrap();
        writeln!(out, "program loop/state{state_bytes} len={}", code.len()).unwrap();
        writeln!(out, "  image {}", hex(&code)).unwrap();
    }

    // 4. The label-emitted software runtimes — u32 mul/div/rem (`__mul16w`/`__mul32`/
    //    `__divmod32`) and signed div/rem (`__sdivmod16`), on both targets (the Cell
    //    side emits the `ED FE` trap forms instead).
    let runtime_src = "
        fn run(a: u16, b: u16) -> u16 {
            let w = (a as u32) * (b as u32) + 7u32;
            let q = w / 3u32;
            let r = w % 5u32;
            let s = (a as i16) / -3i16;
            let t = (a as i16) % 3i16;
            (q as u16) + (r as u16) + (s as u16) + (t as u16)
        }
    ";
    for (target, tname) in TARGETS {
        let prog = compile_program_for(runtime_src, target)
            .unwrap_or_else(|e| panic!("runtime corpus ({tname}) failed: {e}"));
        render_program(&mut out, &format!("runtime/u32_signed/{tname}"), &prog);
    }

    out
}

#[test]
fn codegen_golden() {
    let golden_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/codegen_golden.txt");
    let got = render();
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        fs::write(&golden_path, &got).unwrap();
        return;
    }
    let want = fs::read_to_string(&golden_path).unwrap_or_else(|e| {
        panic!(
            "no golden at {} ({e}) — generate it with UPDATE_GOLDEN=1",
            golden_path.display()
        )
    });
    // A Windows checkout may materialise the golden with CRLF (git autocrlf);
    // the rendered output is always LF. Normalise so the comparison is about
    // codegen, not line endings.
    let want = want.replace("\r\n", "\n");
    if got != want {
        let mismatch = got
            .lines()
            .zip(want.lines())
            .enumerate()
            .find(|(_, (g, w))| g != w);
        match mismatch {
            Some((i, (g, w))) => panic!(
                "codegen output diverged from the golden at line {}:\n  golden: {}\n  got:    {}\n\
                 (intentional change? regenerate with UPDATE_GOLDEN=1 and review the diff)",
                i + 1,
                &w[..w.len().min(120)],
                &g[..g.len().min(120)]
            ),
            None => panic!(
                "codegen output diverged from the golden in length only \
                 (got {} lines, golden {} lines)",
                got.lines().count(),
                want.lines().count()
            ),
        }
    }
}
