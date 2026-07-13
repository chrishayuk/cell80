//! Byte-identity goldens for the IR→MSL text emission (and the fixed
//! interpreter kernel) — the M0 lock for the CUDA dialect work.
//!
//! The Dialect refactor (one walker, two dialects) must keep the MSL output
//! **byte-identical**; these goldens are the gate. The corpus walks every
//! emission path: both widths of arithmetic and div/rem, literal and runtime
//! shifts at both widths, the `__bits_*` intrinsics, `while`/`for`/`loop`
//! with `break`/`continue`, short-circuit `Logic`, `halt`, local arrays
//! (`Fill`/`Index`/`StoreIndex`), const data (`ConstAddr`), `peek`/`poke`,
//! wide params and returns, tuple returns and destructuring, a typed-state
//! cell, and a multi-cell `compile_library` fusion (per-cell prefixes, the
//! `switch` dispatch, cumulative state offsets).
//!
//! Regenerate: `UPDATE_GOLDEN=1 cargo test -p rustmsl --test codegen_snapshot`
//! and review the diff — an unreviewed regeneration defeats the lock.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// A lowered snippet: its functions and packed const data.
type Lowered = (Vec<(String, cell80_core::ir::Func)>, Vec<(String, Vec<u8>)>);

/// Lower a dialect snippet through the same front door the corner battery
/// uses (no inline/DCE — `compile_library` emits helper calls directly).
fn lower(src: &str) -> Lowered {
    let file: syn::File =
        syn::parse_str(src).unwrap_or_else(|e| panic!("parse failed: {e}\nsrc: {src}"));
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())
        .unwrap_or_else(|e| panic!("lower failed: {e}\nsrc: {src}"));
    let consts = lowered.const_data();
    (lowered.funcs, consts)
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

/// One module's golden block: a header per cell (the shape the executor
/// consumes), the const blob, then the full MSL source.
fn render_module(out: &mut String, name: &str, module: &rustmsl::MslModule) {
    writeln!(out, "==== {name} ====").unwrap();
    for (i, c) in module.cells.iter().enumerate() {
        writeln!(
            out,
            "cell {i} entry={} params={} ret_regs={} wide_ret={} state_len={}",
            c.entry, c.params, c.ret_regs, c.wide_ret, c.state_len
        )
        .unwrap();
    }
    writeln!(out, "consts {}", hex(&module.consts)).unwrap();
    out.push_str(&module.source);
    out.push('\n');
}

/// Value-cell corpus: every snippet is known-good dialect (most are the
/// corner battery's, verbatim — the runtime-validated shapes).
const VALUE_CELLS: &[(&str, &str)] = &[
    (
        "e1_arith_widths",
        "fn run(x: u16, y: u16) -> u16 { let a = x as u8; let b = y as u8; ((a + b) as u16) | (((a * b) as u16) << 8) ^ (x & y) }",
    ),
    (
        "e1_div_signed16",
        "fn run(a: i16, b: i16) -> i16 { (a / b) + (a % b) }",
    ),
    (
        "e1_div_unsigned16",
        "fn run(a: u16, b: u16) -> u16 { (a / b) * b + (a % b) }",
    ),
    (
        "e1_u32_arith",
        "fn run(a: u16, b: u16, c: u16) -> u32 { let x = ((a as u32) << 16) | (b as u32); let d = c as u32; (x / (d + 1)) ^ (x * d) ^ (x >> 5) }",
    ),
    (
        "e1_i32_div",
        "fn run(a: i16, b: i16) -> i32 { let x = (a as i32) << 16; let y = b as i32; (x / y) + (x % y) }",
    ),
    (
        "e1_sign_bridges",
        "fn run(a: i16, b: u16) -> u32 { (a as u32) + ((b as u32) << 3) }",
    ),
    (
        "e1_shift_runtime",
        "fn run(x: u16, n: u16) -> u16 { (x << n) | (x >> n) }",
    ),
    (
        "e1_shift_runtime_signed",
        "fn run(x: i16, n: u16) -> i16 { x >> n }",
    ),
    (
        "e1_shift_literal",
        "fn run(x: u16) -> u16 { (x << 3) ^ (x >> 15) ^ (x << 15) }",
    ),
    (
        "e1_shift32_both",
        "fn run(a: u16, b: u16) -> u32 { let x = ((a as u32) << 16) | (b as u32); (x << 3) ^ (x >> 7) ^ (((x as i32) >> 5) as u32) }",
    ),
    (
        "e1_bits_intrinsics",
        "fn run(x: u16) -> u16 { x.count_ones() + (x.leading_zeros() << 5) + (x.trailing_zeros() << 10) }",
    ),
    (
        "e1_signed_compare",
        "fn run(a: i16, b: i16) -> u16 { ((a < b) as u16) | (((a >= b) as u16) << 1) | (((a == b) as u16) << 2) }",
    ),
    (
        "e1_logic_shortcircuit",
        "fn run(a: u16, b: u16) -> u16 { if a > 0 && b / a > 2 { 1 } else { 0 } }",
    ),
    (
        "e1_halt",
        "fn run(x: u16) -> u16 { if x > 40000 { halt(7); } x + 1 }",
    ),
    (
        "e1_tuple_return",
        "fn run(a: u16, b: u16) -> (u16, u16) { let hi = if a > b { a } else { b }; let lo = if a > b { b } else { a }; (hi - lo, (a == b) as u16) }",
    ),
    (
        "e1_tuple_destructure",
        "fn pair(a: u16) -> (u16, u16) { (a + 1u16, a * 2u16) }\n\
         fn run(x: u16) -> u16 { let (p, q) = pair(x); p + q }",
    ),
    (
        "e1_helper_calls",
        "fn diff(a: i16, b: i16) -> i16 { if a > b { a - b } else { b - a } }\n\
         fn run(x: i16, y: i16, z: i16) -> i16 { diff(diff(x, y), z) }",
    ),
    (
        "e1_wide_param_call",
        "fn wide(x: u32, k: u16) -> u32 { x + (k as u32) }\n\
         fn run(a: u16, b: u16) -> u32 { wide(((a as u32) << 16) | (b as u32), a) }",
    ),
    (
        "e2_gcd_while",
        "fn run(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0 { let t = x % y; x = y; y = t; } x }",
    ),
    (
        "e2_for_continue",
        "fn run(n: u16, m: u16) -> u16 { let mut s = 0; for i in 0..(n & 127) { if i % 3 == 0 { continue; } if i == m { continue; } s = s + i; } s }",
    ),
    (
        "e2_nested_break",
        "fn run(a: u16, b: u16) -> u16 { let mut s = 0; for i in 0..(a & 31) { for j in 0..(b & 31) { if j == i { continue; } if j > 20 { break; } s = s + 1; } if s > 400 { break; } } s }",
    ),
    (
        "e2_loop_collatz",
        "fn run(x: u16) -> u16 { let mut v = x | 1; let mut n = 0; loop { if v == 1 { break; } if v % 2 == 0 { v = v / 2; } else { v = (v & 8191) * 3 + 1; } n = n + 1; if n > 400 { break; } } n }",
    ),
    (
        "e2_byte_loop_wrap",
        "fn run(n: u16) -> u16 { let mut s = 0; let mut i: u8 = 250; while i != 4 { i = i + 1; s = s + 1; if s > 300 { break; } } s + (n & 0) }",
    ),
    (
        "mem_array_sort",
        "fn run(n: u16) -> u16 {\n\
             let mut v = n;\n\
             let mut digits: [u16; 5] = [0u16; 5];\n\
             let mut count = 0u16;\n\
             while v != 0u16 { digits[count as usize] = v % 10u16; v = v / 10u16; count = count + 1u16; }\n\
             let mut i = 0u16;\n\
             while i < count {\n\
                 let mut j = 0u16;\n\
                 while j + 1u16 < count - i {\n\
                     if digits[j as usize] < digits[(j + 1u16) as usize] {\n\
                         let tmp = digits[j as usize];\n\
                         digits[j as usize] = digits[(j + 1u16) as usize];\n\
                         digits[(j + 1u16) as usize] = tmp;\n\
                     }\n\
                     j = j + 1u16;\n\
                 }\n\
                 i = i + 1u16;\n\
             }\n\
             let mut result = 0u32;\n\
             let mut k = 0u16;\n\
             while k < count { result = result * 10u32 + digits[k as usize] as u32; k = k + 1u16; }\n\
             if result > 65535u32 { halt(0xFF05u16); }\n\
             result as u16\n\
         }",
    ),
    (
        "mem_const_table",
        "const T: [u16; 4] = [3u16, 1u16, 4u16, 1u16];\n\
         fn run(i: u16) -> u16 { T[(i & 3u16) as usize] + T[0] }",
    ),
    (
        "mem_peek_poke",
        "fn run(a: u16, b: u16) -> u16 { poke(0x9000u16 + 40u16, b as u8); (peek(0x9000u16 + 40u16) as u16) + a }",
    ),
];

/// Typed-state corpus: `(name, src, entry, state_len)` — the
/// `impl State { fn run(&mut self) }` window shapes the corner battery pins.
const STATE_CELLS: &[(&str, &str, &str, usize)] = &[
    (
        "state_fields_mutate",
        "struct S { x: u16, score: u16 }\n\
         impl S { fn run(&mut self) -> u16 { self.score = self.x * 2u16 + self.score; self.score } }",
        "S::run",
        4,
    ),
    (
        "state_u32_field_arg",
        "struct Acc { total: u32 }\n\
         impl Acc { fn run(&mut self, x: u16) -> u16 {\n\
             self.total = self.total + (x as u32);\n\
             (self.total >> 16u32) as u16 } }",
        "Acc::run",
        4,
    ),
    (
        "state_array_loop",
        "struct W { buf: [u16; 8], n: u16 }\n\
         impl W { fn run(&mut self) -> u16 {\n\
             let mut s = 0u16;\n\
             for i in 0..8u16 { s = s + self.buf[i]; }\n\
             self.n = self.n + 1u16;\n\
             s } }",
        "W::run",
        18,
    ),
];

fn render() -> String {
    let mut out = String::new();

    for (name, src) in VALUE_CELLS {
        let (funcs, consts) = lower(src);
        let module = rustmsl::compile(&funcs, &consts, "run")
            .unwrap_or_else(|e| panic!("msl compile failed for {name}: {e}"));
        render_module(&mut out, &format!("cell {name}"), &module);
    }

    for (name, src, entry, state_len) in STATE_CELLS {
        let (funcs, consts) = lower(src);
        let module = rustmsl::compile_library(&[rustmsl::LibraryCell {
            funcs: &funcs,
            consts: &consts,
            entry,
            state_len: *state_len,
        }])
        .unwrap_or_else(|e| panic!("msl compile failed for {name}: {e}"));
        render_module(&mut out, &format!("state {name}"), &module);
    }

    // The fusion shape: value + state + const cells in one translation unit —
    // per-cell prefixes, the switch dispatch, cumulative state offsets, and a
    // fused const blob all in one block.
    {
        let (f0, c0) = lower(
            VALUE_CELLS
                .iter()
                .find(|(n, _)| *n == "e2_gcd_while")
                .unwrap()
                .1,
        );
        let (f1, c1) = lower(STATE_CELLS[0].1);
        let (f2, c2) = lower(
            VALUE_CELLS
                .iter()
                .find(|(n, _)| *n == "mem_const_table")
                .unwrap()
                .1,
        );
        let module = rustmsl::compile_library(&[
            rustmsl::LibraryCell {
                funcs: &f0,
                consts: &c0,
                entry: "run",
                state_len: 0,
            },
            rustmsl::LibraryCell {
                funcs: &f1,
                consts: &c1,
                entry: STATE_CELLS[0].2,
                state_len: STATE_CELLS[0].3,
            },
            rustmsl::LibraryCell {
                funcs: &f2,
                consts: &c2,
                entry: "run",
                state_len: 0,
            },
        ])
        .unwrap_or_else(|e| panic!("msl library fusion failed: {e}"));
        render_module(&mut out, "library fusion_3", &module);
    }

    out
}

fn check_golden(golden_path: &Path, got: &str) {
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        fs::write(golden_path, got).unwrap();
        return;
    }
    let want = fs::read_to_string(golden_path).unwrap_or_else(|e| {
        panic!(
            "no golden at {} ({e}) — generate it with UPDATE_GOLDEN=1",
            golden_path.display()
        )
    });
    // Normalise a CRLF checkout (git autocrlf) — the comparison is about
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
                "emitted source diverged from the golden at line {}:\n  golden: {}\n  got:    {}\n\
                 (intentional change? regenerate with UPDATE_GOLDEN=1 and review the diff)",
                i + 1,
                &w[..w.len().min(160)],
                &g[..g.len().min(160)]
            ),
            None => panic!(
                "emitted source diverged from the golden in length only \
                 (got {} lines, golden {} lines)",
                got.lines().count(),
                want.lines().count()
            ),
        }
    }
}

#[test]
fn msl_codegen_snapshot() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/codegen_msl.txt");
    check_golden(&golden, &render());
}

/// The interpreter kernel's MSL source — a portable builder (shared body +
/// generated constant block + the Metal signature) whose bytes are locked to
/// the pre-split kernel string.
#[test]
fn interp_kernel_snapshot() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/interp_msl.txt");
    check_golden(&golden, &rustmsl::interp::interp_source_msl());
}
