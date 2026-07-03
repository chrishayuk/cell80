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
//!
//! Rules run to a fixpoint: a rewrite can expose another match (R1 feeds R2).

use super::ins::{Ins, R16};

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
}

/// Is `i` a leaf load into `HL` — reads neither `DE`, flags, nor the stack?
fn leaf_load_hl(i: &Ins) -> bool {
    matches!(i, Ins::LdImm(R16::Hl, _) | Ins::LdHlMem(_))
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
        let file: syn::File = syn::parse_str(src).unwrap();
        let funcs = crate::lower::lower_program(&file, &crate::lower::PreludeConfig::default())
            .unwrap_or_else(|e| panic!("lower failed: {e}\nsrc: {src}"));
        let mut a = Asm::new(0x8000, target);
        let mut base = 0u16;
        for (name, f) in &funcs {
            a.define(name);
            a.base = base;
            emit_func(&mut a, f);
            base += f.n_locals as u16;
        }
        a.seal();
        a.peep
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

    /// Corpus-wide site counts over the cell80 stdlib — run explicitly:
    /// `cargo test -p rustz80 --lib corpus_rule_ranking -- --ignored --nocapture`.
    /// (Ignored: the cells live in the sibling `cell80` crate, absent in a packaged
    /// crates.io build.)
    #[test]
    #[ignore]
    fn corpus_rule_ranking() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../cell80/cells");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            eprintln!("no cells corpus at {} — skipping", dir.display());
            return;
        };
        let mut total = PeepholeCounts::default();
        let mut files = 0u32;
        for e in entries {
            let p = e.unwrap().path();
            if p.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&p).unwrap();
            let c = counts_for(&src, Target::Cell);
            total.leaf_pair += c.leaf_pair;
            total.lit_add += c.lit_add;
            total.store_reload += c.store_reload;
            total.dead_push_pop += c.dead_push_pop;
            total.call_tail += c.call_tail;
            total.cleanup += c.cleanup;
            files += 1;
        }
        println!("peephole sites across {files} cells: {total:?}");
        assert!(total.leaf_pair > 0);
    }
}
