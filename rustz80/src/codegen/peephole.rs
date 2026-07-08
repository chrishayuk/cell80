//! The Stage-2 peephole pass — local rewrites over the symbolic [`Ins`] stream.
//!
//! Every rule matches a **contiguous** window of instructions, so a label placement
//! (`Ins::At`) or symbol (`Ins::Def`) inside the window blocks the match by
//! construction — control flow can only enter a matched window by falling in from
//! the top, never mid-window. Correctness arguments per rule are local:
//!
//! - **R1** `PUSH HL; <leaf load HL>; POP DE` → `EX DE,HL; <load>`. Final state is
//!   identical (`DE` = old `HL`, `HL` = loaded value, old `DE` lost either way), no
//!   flags touched by either form, SP balanced. The leaf loads (`LD HL,imm` /
//!   `LD HL,(nn)`) read neither `DE` nor the stack. This is the `Var⊕Var`/`Var⊕Lit`
//!   shape of every binop, `gen_pair` runtime call, and leaf-address store.
//! - **R2** `EX DE,HL; LD HL,imm; ADD HL,DE` → `LD DE,imm; ADD HL,DE`. Commutative
//!   add with the same flags from the final `ADD`; `DE` ends holding the literal
//!   instead of the left operand — safe because codegen treats `DE` as scratch: every
//!   consumer loads it explicitly (results travel in `HL`; tuple returns come from
//!   pops, never from a binop's leftover `DE`).
//! - **R3** `LD (m),HL; LD HL,(m)` → `LD (m),HL`. The reload reads back exactly what
//!   was stored; neither touches flags. `m` is only ever a locals slot or a runtime
//!   scratch word (I/O goes through `LD (HL),E` / `poke`), so the memory is plain RAM.
//! - **R4** `PUSH HL; POP HL` → (nothing). The one-arg call convention's dead pair.
//! - **R5** `PUSH HL; POP DE; POP HL` → `EX DE,HL; POP HL`. The two-arg call tail:
//!   `DE` = old `HL`, then `HL` comes off the stack; old `DE`/old `HL` are dead in
//!   both forms, SP delta identical (net one pop), no flags.
//! - **R6** `EX DE,HL; EX DE,HL` → (nothing); `LD H,0; LD H,0` → one. Cleanups for
//!   shapes earlier rules (or width masking) can leave behind.
//! - **R7** `LD DE,1; ADD HL,DE` → `INC HL`; `LD DE,2; ADD HL,DE` → `INC HL; INC HL`.
//!   Strength-reduction of a `+1`/`+2` value add (3+1 bytes → 1/2). Safe because the
//!   `ADD`'s flags are dead — arithmetic results travel in `HL` as *values*, and every
//!   flag consumer (a comparison / condition) recomputes via its own `SBC` (the R2
//!   invariant); `INC HL` (which sets no flags) therefore observes the same downstream
//!   state. `DE` ends holding its prior value instead of the literal — scratch either
//!   way. Only fires on the `imm ∈ {1,2}` add (the literal `LD DE,base_addr` used for
//!   address arithmetic and the `*2` `ADD HL,HL` are untouched).
//! - **R8** `PUSH HL; <DE-free span ≥2>; POP DE` → `EX DE,HL; <span>`. The window-spanning
//!   generalisation of R1: the span reads/writes only `HL`/`BC`/memory (never `DE`, the
//!   stack, flags, or control flow — [`de_free_span`]), so `EX DE,HL` up front leaves the
//!   same final state (`DE` = the pushed `HL`, `HL` = the span's result) with the stack
//!   balanced. R7 *creates* this shape: a `self.arr[i]` element address is
//!   `PUSH HL; LD HL,(self); INC HL…; POP DE` once R7 reduces the field-offset add. The
//!   span must be ≥2 so R1 keeps the single-leaf case.
//!
//! Rules run to a fixpoint: a rewrite can expose another match (R1 feeds R2 feeds R7,
//! and R7's `INC HL` spans feed R8).

use super::ins::{FxBytes, Imm, Ins, R16};

/// How many times each rule fired — the measured ranking the roadmap asks for
/// (`size_report` deltas carry the byte prize; this carries the site counts).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PeepholeCounts {
    /// R1: leaf-operand `PUSH`/`POP` pair → `EX DE,HL`.
    pub leaf_pair: u64,
    /// R2: literal right operand of a commutative add loaded straight into `DE`.
    pub lit_add: u64,
    /// R3: store-then-reload elision.
    pub store_reload: u64,
    /// R4: dead `PUSH HL; POP HL`.
    pub dead_push_pop: u64,
    /// R5: two-arg call tail `PUSH HL; POP DE; POP HL` → `EX DE,HL; POP HL`.
    pub call_tail: u64,
    /// R6: `EX;EX` / doubled `LD H,0` cleanups.
    pub cleanup: u64,
    /// R7: `+1`/`+2` value add strength-reduced to `INC HL`(`; INC HL`).
    pub inc_dec: u64,
    /// R8: window-spanning `PUSH HL … POP DE` collapsed to `EX DE,HL`.
    pub window_span: u64,
}

/// `INC HL` (0x23) as a one-byte fixed instruction — the target of R7.
fn inc_hl() -> Ins {
    Ins::Fx(FxBytes::new(&[0x23]))
}

/// A span instruction safe to carry across a `PUSH HL … POP DE` → `EX DE,HL` rewrite
/// (R8): reads/writes only `HL`/`BC`/memory — never `DE`, the stack, flags, or control
/// flow. `INC HL`/`DEC HL` (the `0x23`/`0x2B` fixed bytes R7 leaves) are the shape that
/// makes R8 fire on element-address code.
fn de_free_span(i: &Ins) -> bool {
    matches!(
        i,
        Ins::LdHlMem(_)
            | Ins::StHlMem(_)
            | Ins::LdImm(R16::Hl | R16::Bc, _)
            | Ins::LdImmSym(R16::Hl | R16::Bc, _)
            | Ins::AddHl(R16::Hl | R16::Bc)
    ) || matches!(i, Ins::Fx(fx) if fx.bytes() == [0x23] || fx.bytes() == [0x2B])
}

/// R8 match: if the instructions after a `PUSH HL` form a **non-trivial** DE-free span
/// (`≥2`, so R1 keeps the single-leaf case) terminated by `POP DE`, the span length.
fn r8_span_len(rest: &[Ins]) -> Option<usize> {
    let n = rest.iter().take_while(|i| de_free_span(i)).count();
    (n >= 2 && matches!(rest.get(n), Some(Ins::Pop(R16::De)))).then_some(n)
}

/// Is `i` a leaf load into `HL` — reads neither `DE`, flags, nor the stack?
/// (`LdImmSym` is `LdImm` with the operand resolved later — same instruction.)
fn leaf_load_hl(i: &Ins) -> bool {
    matches!(
        i,
        Ins::LdImm(R16::Hl, _) | Ins::LdImmSym(R16::Hl, _) | Ins::LdHlMem(_)
    )
}

/// The doubled-`LD H,0` byte pattern.
fn ld_h_0(i: &Ins) -> bool {
    matches!(i, Ins::Fx(fx) if fx.bytes() == [0x26, 0x00])
}

/// Run the peephole rules over `ins` to a fixpoint. Returns the per-rule fire counts.
pub(super) fn optimize(ins: &mut Vec<Ins>) -> PeepholeCounts {
    let mut counts = PeepholeCounts::default();
    loop {
        let mut changed = false;
        let mut out: Vec<Ins> = Vec::with_capacity(ins.len());
        let mut i = 0;
        while i < ins.len() {
            match &ins[i..] {
                // R4 — dead push/pop (before R1/R5: it is the most specific 2-window).
                [Ins::Push(R16::Hl), Ins::Pop(R16::Hl), ..] => {
                    counts.dead_push_pop += 1;
                    changed = true;
                    i += 2;
                }
                // R5 — two-arg call tail.
                [Ins::Push(R16::Hl), Ins::Pop(R16::De), Ins::Pop(R16::Hl), ..] => {
                    counts.call_tail += 1;
                    changed = true;
                    out.push(Ins::ExDeHl);
                    out.push(Ins::Pop(R16::Hl));
                    i += 3;
                }
                // R1 — leaf-operand push/pop pair.
                [Ins::Push(R16::Hl), l, Ins::Pop(R16::De), ..] if leaf_load_hl(l) => {
                    counts.leaf_pair += 1;
                    changed = true;
                    out.push(Ins::ExDeHl);
                    out.push(l.clone());
                    i += 3;
                }
                // R8 — window-spanning leaf pair (the ≥2 span R1 can't take).
                [Ins::Push(R16::Hl), ..] if r8_span_len(&ins[i + 1..]).is_some() => {
                    let span = r8_span_len(&ins[i + 1..]).unwrap();
                    counts.window_span += 1;
                    changed = true;
                    out.push(Ins::ExDeHl);
                    out.extend(ins[i + 1..i + 1 + span].iter().cloned());
                    i += span + 2; // PUSH + span + POP DE
                }
                // R2 — literal add straight into DE (runs on R1's output next pass).
                [Ins::ExDeHl, Ins::LdImm(R16::Hl, m), Ins::AddHl(R16::De), ..] => {
                    counts.lit_add += 1;
                    changed = true;
                    out.push(Ins::LdImm(R16::De, *m));
                    out.push(Ins::AddHl(R16::De));
                    i += 3;
                }
                // R3 — store-then-reload.
                [Ins::StHlMem(m), Ins::LdHlMem(m2), ..] if m == m2 => {
                    counts.store_reload += 1;
                    changed = true;
                    out.push(Ins::StHlMem(*m));
                    i += 2;
                }
                // R7 — INC strength reduction (runs on R2's `LD DE,imm; ADD HL,DE`).
                [Ins::LdImm(R16::De, Imm::Abs(n @ (1 | 2))), Ins::AddHl(R16::De), ..] => {
                    counts.inc_dec += 1;
                    changed = true;
                    for _ in 0..*n {
                        out.push(inc_hl());
                    }
                    i += 2;
                }
                // R6 — cleanups.
                [Ins::ExDeHl, Ins::ExDeHl, ..] => {
                    counts.cleanup += 1;
                    changed = true;
                    i += 2;
                }
                [a, b, ..] if ld_h_0(a) && ld_h_0(b) => {
                    counts.cleanup += 1;
                    changed = true;
                    out.push(a.clone());
                    i += 2;
                }
                [first, ..] => {
                    out.push(first.clone());
                    i += 1;
                }
                [] => unreachable!(),
            }
        }
        *ins = out;
        if !changed {
            return counts;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ins::Imm;
    use super::super::{emit_func, Asm, Target};
    use super::*;

    /// Emit `src` (whole pipeline: lower → emit → seal) and return the fire counts.
    fn counts_for(src: &str, target: Target) -> PeepholeCounts {
        try_counts_for(src, target).expect("hand-written test source must lower")
    }

    /// Like [`counts_for`] but `None` if `src` doesn't lower standalone (some cells
    /// reference cross-cell wide signatures resolved only by the cell layer).
    fn try_counts_for(src: &str, target: Target) -> Option<PeepholeCounts> {
        let file: syn::File = syn::parse_str(src).ok()?;
        let funcs =
            crate::lower::lower_program(&file, &crate::lower::PreludeConfig::default()).ok()?;
        let mut a = Asm::new(0x8000, target);
        // Mirror `codegen_program_c`: without the wide-call signature map, a call
        // to a wide fn (an auto-appended f32 kernel — the first corpus cells that
        // both lower standalone *and* make wide calls) emits 16-bit call plumbing
        // and trips the wide-node-in-16-bit-context unreachable.
        a.wide_sigs = crate::codegen::wide_sig_map(&funcs);
        let mut base = 0u16;
        for (name, f) in &funcs {
            a.define(name);
            a.base = base;
            emit_func(&mut a, f);
            base += f.n_locals as u16;
        }
        a.seal();
        Some(a.peep)
    }

    /// The sealed (post-peephole) `Ins` stream for `src` — for measuring *candidate*
    /// sites of not-yet-built rules. `None` if the source doesn't lower standalone (some
    /// cells reference cross-cell wide signatures resolved only by the cell layer).
    fn sealed_ins(src: &str, target: Target) -> Option<Vec<Ins>> {
        let file: syn::File = syn::parse_str(src).ok()?;
        let funcs =
            crate::lower::lower_program(&file, &crate::lower::PreludeConfig::default()).ok()?;
        let mut a = Asm::new(0x8000, target);
        // Mirror `codegen_program_c`: without the wide-call signature map, a call
        // to a wide fn (an auto-appended f32 kernel — the first corpus cells that
        // both lower standalone *and* make wide calls) emits 16-bit call plumbing
        // and trips the wide-node-in-16-bit-context unreachable.
        a.wide_sigs = crate::codegen::wide_sig_map(&funcs);
        let mut base = 0u16;
        for (name, f) in &funcs {
            a.define(name);
            a.base = base;
            emit_func(&mut a, f);
            base += f.n_locals as u16;
        }
        a.seal();
        Some(a.ins.clone())
    }

    #[test]
    fn hand_stream_rewrites() {
        // R6's LD H,0 dedup — currently only reachable via hand-built streams (the
        // frontend never emits two adjacent masks today; the counts test keeps that
        // claim honest), plus the EX;EX cancellation R1 can feed.
        let h0 = || Ins::Fx(super::super::ins::FxBytes::new(&[0x26, 0x00]));
        let mut s = vec![h0(), h0(), Ins::ExDeHl, Ins::ExDeHl];
        let c = optimize(&mut s);
        assert_eq!(s, vec![h0()]);
        assert_eq!(c.cleanup, 2);

        // R1 then R2 cascade across fixpoint passes: PUSH; LD HL,lit; POP DE; ADD
        // → EX; LD HL,lit; ADD → LD DE,lit; ADD.
        let mut s = vec![
            Ins::Push(R16::Hl),
            Ins::LdImm(R16::Hl, Imm::Abs(7)),
            Ins::Pop(R16::De),
            Ins::AddHl(R16::De),
        ];
        let c = optimize(&mut s);
        assert_eq!(
            s,
            vec![Ins::LdImm(R16::De, Imm::Abs(7)), Ins::AddHl(R16::De)]
        );
        assert_eq!((c.leaf_pair, c.lit_add), (1, 1));

        // A label inside the window fences the rewrite — control could enter there.
        let mut s = vec![
            Ins::Push(R16::Hl),
            Ins::At(0),
            Ins::LdImm(R16::Hl, Imm::Abs(7)),
            Ins::Pop(R16::De),
        ];
        let before = s.clone();
        let c = optimize(&mut s);
        assert_eq!(s, before, "a window containing a label must not rewrite");
        assert_eq!(c, PeepholeCounts::default());
    }

    #[test]
    fn r8_fires_on_element_address() {
        // `self.arr[i]` with a non-zero field offset produces `PUSH HL; LD HL,(self);
        // INC HL…; POP DE` (R7 supplies the `INC HL`s), the DE-free span R8 collapses to
        // `EX DE,HL`. Fired-proof: the counter is non-zero on both targets.
        let src = "
            struct S { first: u16, arr: [u16; 4] }
            impl S { fn at(&self, i: u16) -> u16 { self.arr[i] } }
            fn run() -> u16 { let s = S { first: 1u16, arr: [2u16, 3u16, 4u16, 5u16] }; s.at(2u16) }
        ";
        for target in [Target::Spectrum48, Target::Cell] {
            let c = counts_for(src, target);
            assert!(
                c.window_span >= 1,
                "R8 should fire on the element-address shape ({target:?}), got {c:?}"
            );
        }
    }

    /// The measured rule ranking on a representative body (the DoD asks for counted
    /// sites, not an assumed ranking): the leaf-operand pair rule dominates.
    #[test]
    fn leaf_pair_dominates_on_representative_code() {
        let src = "
            fn clamp(x: u16, lo: u16, hi: u16) -> u16 {
                let mut r = x;
                if x < lo { r = lo; }
                if x > hi { r = hi; }
                r
            }
            fn run(a: u16, b: u16) -> u16 {
                let mut acc = 0u16;
                let mut i = 0u16;
                while i < 10u16 {
                    let t = clamp(a + i, b - 4u16, b + 9u16);
                    acc = acc + t;
                    i = i + 1u16;
                }
                acc
            }
        ";
        for target in [Target::Spectrum48, Target::Cell] {
            let c = counts_for(src, target);
            assert!(
                c.leaf_pair >= c.lit_add
                    && c.leaf_pair >= c.store_reload
                    && c.leaf_pair >= c.dead_push_pop
                    && c.leaf_pair >= c.call_tail,
                "expected the leaf-pair rule to dominate, got {c:?}"
            );
        }
    }

    /// Every `.rs` file under `dir`, discovered recursively (`rustz80` stays
    /// dependency-free, so this doesn't reuse `cell80::discover_cell_files` — the cells
    /// corpus now lives in pack subdirectories, not flat).
    fn rs_files_recursive(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return out;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(rs_files_recursive(&p));
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
        out
    }

    /// Corpus-wide site counts over the cell80 stdlib — run explicitly:
    /// `cargo test -p rustz80 --lib corpus_rule_ranking -- --ignored --nocapture`.
    /// (Ignored: the cells live in the sibling `cell80` crate, absent in a packaged
    /// crates.io build.)
    #[test]
    #[ignore]
    fn corpus_rule_ranking() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../cell80/cells");
        if !dir.is_dir() {
            eprintln!("no cells corpus at {} — skipping", dir.display());
            return;
        }
        let mut total = PeepholeCounts::default();
        let mut files = 0u32;
        for p in rs_files_recursive(&dir) {
            let src = std::fs::read_to_string(&p).unwrap();
            let Some(c) = try_counts_for(&src, Target::Cell) else {
                continue; // cross-cell cell that doesn't lower standalone
            };
            total.leaf_pair += c.leaf_pair;
            total.lit_add += c.lit_add;
            total.store_reload += c.store_reload;
            total.dead_push_pop += c.dead_push_pop;
            total.call_tail += c.call_tail;
            total.cleanup += c.cleanup;
            total.inc_dec += c.inc_dec;
            total.window_span += c.window_span;
            files += 1;
        }
        println!("peephole sites across {files} cells: {total:?}");
        assert!(total.leaf_pair > 0);
    }

    /// Size the prize for the *next* rules before building them: count candidate sites
    /// across the cell80 corpus on the post-peephole stream.
    /// `cargo test -p rustz80 --lib measure_next_rules -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn measure_next_rules() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../cell80/cells");
        if !dir.is_dir() {
            eprintln!("no cells corpus — skipping");
            return;
        }
        let (mut span_safe, mut reload, mut files) = (0u64, 0u64, 0u32);
        for p in rs_files_recursive(&dir) {
            let src = std::fs::read_to_string(&p).unwrap();
            let Some(ins) = sealed_ins(&src, Target::Cell) else {
                continue;
            };
            // R8 candidate: PUSH HL, then a DE-free span ≥2, then POP DE (post-seal this is
            // 0 — R8 already consumed them; the count lives in `corpus_rule_ranking`).
            for (k, i) in ins.iter().enumerate() {
                if matches!(i, Ins::Push(R16::Hl)) && r8_span_len(&ins[k + 1..]).is_some() {
                    span_safe += 1;
                }
            }
            // Reload-elision ceiling (the HL/DE tracker's max): a store to slot `m` and a
            // later reload of the same `m` with no intervening store to it — non-adjacent
            // (adjacent is R3). Upper bound only: most spans clobber HL, so the realisable
            // subset is smaller.
            for (k, i) in ins.iter().enumerate() {
                let Ins::StHlMem(m) = i else { continue };
                for later in &ins[k + 2..] {
                    match later {
                        Ins::StHlMem(m2) if m2 == m => break,
                        Ins::LdHlMem(m2) if m2 == m => {
                            reload += 1;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            files += 1;
        }
        println!(
            "next-rule candidates across {files} cells: window_span_leaf(R8, safe, saves 1B each)={span_safe}  reload_elision_ceiling={reload}"
        );
    }
}
