//! One-off re-check of C1's single stage-2 candidate (`is_weekend ← is_le(x, 1)`)
//! against the fixed `compose_source`/`Inliner` in `gpu_superopt.rs` — the fix
//! (true expression-level inlining, no separate-function subroutine-call
//! overhead) landed after the full 68-target run in
//! `../../experiments/c1_superopt_run.log` already completed and was found to
//! reject this candidate (0.93x, reference wins) using the *buggy*
//! separate-function composer. Stage 1 (the GPU search) doesn't depend on
//! `compose_source` at all, so re-running the full ~2.6h search is unneeded —
//! only this one candidate's stage-2 hand-composition + real-Z80 cost needs
//! re-verifying. Logic duplicated (not imported) from `gpu_superopt.rs`
//! because that file is a `mod macos` inside a binary example, not a library.
//!
//! Run: `cargo run --release -p cell80 --example c1_stage2_recheck` (macOS)

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("c1_stage2_recheck needs macOS — real-Z80 profiling only, no GPU needed here");
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use cell80::{Cartridge, CartridgeOpts, CellConfig, Halt, Runner};
    use cell80_core::ir::Expr;
    use std::collections::HashMap;

    const DOMAIN: usize = 1 << 16;

    fn z80_compile(id: &str, src: &str) -> Option<Cartridge> {
        Cartridge::compile(
            src,
            CellConfig::permissive(),
            CartridgeOpts {
                id: Some(id.into()),
                kernel_bank: true,
                ..Default::default()
            },
        )
        .ok()
    }

    fn z80_profile_unary(cart: &Cartridge) -> Option<(f64, f64, Vec<u16>, bool)> {
        let mut r = Runner::new(cart.z80().ok()?);
        let entry = cart.manifest.entry.clone();
        let mut cyc = 0u64;
        let mut trp = 0u64;
        let mut table = vec![0u16; DOMAIN];
        let mut total = true;
        for v in 0..DOMAIN {
            let f = r
                .run_fast(Some(&entry), &[v as u16], cell80::DEFAULT_CYCLES)
                .ok()?;
            cyc += f.cycles;
            trp += f.trapped_ops;
            if matches!(f.halt, Halt::Returned) {
                table[v] = f.result;
            } else {
                total = false;
            }
        }
        Some((
            cyc as f64 / DOMAIN as f64,
            trp as f64 / DOMAIN as f64,
            table,
            total,
        ))
    }

    fn z80_profile_binary(cart: &Cartridge) -> Option<(f64, f64)> {
        let mut r = Runner::new(cart.z80().ok()?);
        let entry = cart.manifest.entry.clone();
        let mut cyc = 0u64;
        let mut trp = 0u64;
        let mut n = 0u64;
        for a in 0..=255u16 {
            for b in 0..=255u16 {
                let f = r.run_fast(Some(&entry), &[a, b], cell80::DEFAULT_CYCLES).ok()?;
                cyc += f.cycles;
                trp += f.trapped_ops;
                n += 1;
            }
        }
        Some((cyc as f64 / n as f64, trp as f64 / n as f64))
    }

    const SOFT_MUL: &str = "fn run(a: u16, b: u16) -> u16 { let mut acc = 0u16; let mut x = a; let mut y = b; let mut i = 0u16; while i < 16u16 { if (y & 1u16) != 0u16 { acc = acc.wrapping_add(x); } x = x << 1u16; y = y >> 1u16; i = i + 1u16; } acc }";
    const TRAP_MUL: &str = "fn run(a: u16, b: u16) -> u16 { a * b }";

    fn measure_p_t() -> f64 {
        let soft = z80_compile("xp_soft_mul16", SOFT_MUL).expect("soft mul16 compiles");
        let trap = z80_compile("xp_trap_mul16", TRAP_MUL).expect("trap mul16 compiles");
        let (sm, _) = z80_profile_binary(&soft).expect("soft mul16 profiles");
        let (tm, _) = z80_profile_binary(&trap).expect("trap mul16 profiles");
        (sm - tm).max(0.0)
    }

    fn extract_params_and_body(src: &str) -> Option<(Vec<String>, String)> {
        let stripped: String = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n");
        let item: syn::ItemFn = syn::parse_str(&stripped).ok()?;
        let params: Vec<String> = item
            .sig
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                syn::FnArg::Typed(pt) => match &*pt.pat {
                    syn::Pat::Ident(id) => Some(id.ident.to_string()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let open = stripped.find('{')?;
        let close = stripped.rfind('}')?;
        if close <= open {
            return None;
        }
        Some((params, stripped[open + 1..close].trim().to_string()))
    }

    fn word_replace(text: &str, name: &str, replacement: &str) -> String {
        let is_ident = |c: char| c.is_alphanumeric() || c == '_';
        let chars: Vec<char> = text.chars().collect();
        let needle: Vec<char> = name.chars().collect();
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < chars.len() {
            if chars[i..].starts_with(needle.as_slice()) {
                let before_ok = i == 0 || !is_ident(chars[i - 1]);
                let after = i + needle.len();
                let after_ok = after >= chars.len() || !is_ident(chars[after]);
                if before_ok && after_ok {
                    out.push_str(replacement);
                    i = after;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    }

    struct Inliner<'a> {
        sources: &'a HashMap<String, String>,
        counter: usize,
    }
    impl<'a> Inliner<'a> {
        fn fresh(&mut self) -> String {
            self.counter += 1;
            format!("__t{}", self.counter)
        }
        fn inline(&mut self, e: &Expr, prelude: &mut Vec<String>) -> Option<String> {
            match e {
                Expr::Var(_) => Some("x".to_string()),
                Expr::Lit(n) => Some(format!("{n}")),
                Expr::Call(name, args) => {
                    let arg_texts: Vec<String> = args
                        .iter()
                        .map(|a| self.inline(a, prelude))
                        .collect::<Option<Vec<_>>>()?;
                    let (params, body) = extract_params_and_body(self.sources.get(name)?)?;
                    let mut body_sub = body;
                    for (p, a) in params.iter().zip(&arg_texts) {
                        let is_leaf = a == "x" || a.parse::<i64>().is_ok();
                        if is_leaf {
                            body_sub = word_replace(&body_sub, p, a);
                        } else {
                            let fresh = self.fresh();
                            prelude.push(format!("let {fresh}: u16 = {a};"));
                            body_sub = word_replace(&body_sub, p, &fresh);
                        }
                    }
                    Some(body_sub)
                }
                _ => None,
            }
        }
    }

    fn compose_source(e: &Expr, sources: &HashMap<String, String>) -> Option<String> {
        let mut inliner = Inliner { sources, counter: 0 };
        let mut prelude = Vec::new();
        let tail = inliner.inline(e, &mut prelude)?;
        Some(format!(
            "fn run(x: u16) -> u16 {{ {} {tail} }}",
            prelude.join(" ")
        ))
    }

    pub fn run() {
        let is_le_src = "fn run(a: u16, b: u16) -> u16 { (a <= b) as u16 }";
        let is_weekend_src = "fn run(dow: u16) -> u16 { (dow == 0u16 || dow == 1u16) as u16 }";

        let mut sources = HashMap::new();
        sources.insert("is_le".to_string(), is_le_src.to_string());

        let candidate_expr = Expr::Call("is_le".into(), vec![Expr::Var(0), Expr::Lit(1)]);
        let composed = compose_source(&candidate_expr, &sources).expect("compose_source");
        println!("composed candidate source: {composed}");

        let cand_cart = z80_compile("stage2_recheck_candidate", &composed).expect("candidate compiles");
        let ref_cart = z80_compile("stage2_recheck_reference", is_weekend_src).expect("reference compiles");

        let (cand_cyc, cand_trp, cand_table, cand_total) =
            z80_profile_unary(&cand_cart).expect("candidate profiles");
        let (ref_cyc, ref_trp, ref_table, ref_total) =
            z80_profile_unary(&ref_cart).expect("reference profiles");

        assert!(cand_total, "candidate must return on every input");
        assert!(ref_total, "reference must return on every input");
        assert_eq!(
            cand_table, ref_table,
            "candidate and reference must be exactly equivalent over the full u16 domain"
        );
        println!("full-domain equivalence: CONFIRMED (65536/65536 inputs match)");

        let p_t = measure_p_t();
        println!("P_T = {p_t}");

        let cand_repriced = cand_cyc + p_t * cand_trp;
        let ref_repriced = ref_cyc + p_t * ref_trp;
        let ratio_repriced = ref_repriced / cand_repriced;
        println!(
            "Z80 repriced: candidate {cand_repriced:.1} vs reference {ref_repriced:.1} ({ratio_repriced:.2}x)"
        );

        let ratio_raw = ref_cyc / cand_cyc;
        println!(
            "Z80 raw (P_T=0): candidate {cand_cyc:.1} vs reference {ref_cyc:.1} ({ratio_raw:.2}x)"
        );

        if ratio_repriced > 1.0 && ratio_raw > 1.0 {
            println!("==> stage 2 CONFIRMED win (P_T=0-robust)");
        } else {
            println!("==> stage 2 REJECTED (reference at least as cheap on one lane)");
        }
    }
}
