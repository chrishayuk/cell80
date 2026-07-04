//! The appended mul/div micro-runtime (Spectrum target) + the Cell80 `ED FE` trap ids.
use super::asm::Asm;
use super::ins::{Imm, R16};
use super::Target;

/// `__mul16`: HL = HL * DE (low 16). Shift-add, **multiplier-terminated**: loops once
/// per bit up to the multiplier's (DE's) top set bit, then returns — so small operands
/// finish in a few iterations instead of a fixed 16. Clobbers AF/BC/DE.
pub(super) const MUL16: &[u8] = &[
    0x44, 0x4D, // ld b,h ; ld c,l   (BC = multiplicand)
    0x21, 0x00, 0x00, // ld hl,0     (product)
    // loop:
    0x7B, 0xB2, 0xC8, // ld a,e ; or d ; ret z   (no multiplier bits left → done)
    0xCB, 0x3A, 0xCB, 0x1B, // srl d ; rr e       (DE >>= 1, low bit -> CF)
    0x30, 0x01, // jr nc,+1
    0x09, // add hl,bc                            (product += multiplicand)
    // skip:
    0xCB, 0x21, 0xCB, 0x10, // sla c ; rl b       (BC <<= 1)
    0x18, 0xF0, // jr loop  (-16)
];

/// `__divmod16`: HL/DE -> HL=quotient, DE=remainder (divisor < 0x8000).
/// Fast path: `dividend < divisor` → quotient 0, remainder = dividend (returns at once).
/// Else restoring division. Clobbers AF/BC.
pub(super) const DIVMOD16: &[u8] = &[
    // Fast path: if HL (dividend) < DE (divisor), q=0, r=dividend.
    0x7C, 0xBA, // ld a,h ; cp d
    0x38, 0x06, // jr c, less        (H < D → HL < DE)
    0x20, 0x09, // jr nz, big        (H > D → HL >= DE)
    0x7D, 0xBB, // ld a,l ; cp e     (H == D: compare low)
    0x30, 0x05, // jr nc, big        (L >= E → HL >= DE)
    // less: quotient 0, remainder = dividend.
    0xEB, // ex de,hl                (DE = dividend = remainder, HL = divisor)
    0x21, 0x00, 0x00, // ld hl,0     (quotient)
    0xC9, // ret
    // big: restoring division.
    0x44, 0x4D, // ld b,h ; ld c,l   (BC = dividend)
    0x21, 0x00, 0x00, // ld hl,0     (remainder)
    0x3E, 0x10, // ld a,16
    0xCB, 0x21, 0xCB, 0x10, // sla c ; rl b   (BC <<= 1, MSB -> CF)
    0xED, 0x6A, // adc hl,hl   (rem = rem*2 + bit)
    0xED, 0x52, // sbc hl,de   (rem -= divisor)
    0x30, 0x03, // jr nc,+3 -> set
    0x19, // add hl,de   (restore)
    0x18, 0x01, // jr +1 -> cont
    0x0C, // set: inc c   (quotient bit)
    0x3D, 0x20, 0xEF, // cont: dec a ; jr nz
    0xEB, // ex de,hl    (DE = remainder)
    0x60, 0x69, // ld h,b ; ld l,c   (HL = quotient)
    0xC9, // ret
];

/// Host-trap ids (match `spectrum::host::math_traps`): `HL = BC * DE`, and `HL = BC / DE`
/// with `DE = BC % DE`.
pub(super) const TRAP_MUL16: u8 = 0x10;

pub(super) const TRAP_DIVMOD16: u8 = 0x11;

/// 32-bit host traps. Convention (shared with the software siblings below): the left
/// operand `l` is pushed on the stack (low word on top), the right operand `r` is in
/// `HL:DE` (low:high). MUL32 leaves `l*r` in `HL:DE`; DIVMOD32 leaves the quotient in
/// `HL:DE` and writes the remainder back into the two stack words. Either way the
/// caller pops the two words afterwards (dropping them, or popping the remainder).
pub(super) const TRAP_MUL32: u8 = 0x12;

pub(super) const TRAP_DIVMOD32: u8 = 0x13;

/// Emit a host trap: `LD A, id ; ED FE` (the reserved `TRAP_OP`).
pub(super) fn gen_trap(a: &mut Asm, id: u8) {
    a.fx(&[0x3E, id]); // LD A, id
    a.fx(&[0xED, 0xFE]); // ED FE  (host trap)
}

pub(super) const TRAP_FILL16: u8 = 0x20; // fill `BC` slots (2-byte words) at `HL` with `DE`

pub(super) const TRAP_HALT: u8 = 0x30; // stop the run with status code `HL`

// ── The 32-bit software siblings (Spectrum target) ─────────────────────────────────
//
// Emitted through the `Asm` (labels resolve the internal jumps and the few static
// scratch words placed after each routine's code), unlike the hand-counted 16-bit
// byte arrays above. Calling convention: see [`TRAP_MUL32`]. Both routines keep the
// caller's two stack words in place so the caller-side cleanup is identical for the
// trap and the software paths. Not re-entrant (static scratch) — fine: the compiler
// has no recursion, and interrupts are off (`DI`) / absent on the cell bus.

/// `__mul16w`: `DE:HL = BC * DE` — the full 32-bit product of two 16-bit values.
/// The classic shift-add: `DE:HL` doubles each step (multiplier bits exit the top of
/// `D` into carry, product bits fill from the bottom), adding `BC` when the bit is
/// set. Clobbers `AF`.
pub(super) fn emit_mul16w(a: &mut Asm) {
    a.define("__mul16w");
    a.ld_imm(R16::Hl, Imm::Abs(0)); // LD HL, 0
    a.fx(&[0x3E, 16]); // LD A, 16
    let top = a.label();
    let skip = a.label();
    a.place(top);
    a.add_hl(R16::Hl); // ADD HL,HL
    a.fx(&[0xCB, 0x13]); // RL E
    a.fx(&[0xCB, 0x12]); // RL D          (CF = multiplier MSB out)
    a.jump(0xD2, skip); // JP NC,skip
    a.add_hl(R16::Bc); // ADD HL,BC     (product += multiplicand)
    a.jump(0xD2, skip); // JP NC,skip
    a.fx(&[0x13]); // INC DE        (carry into the high word)
    a.place(skip);
    a.fx(&[0x3D]); // DEC A
    a.jump(0xC2, top); // JP NZ,top
    a.fx(&[0xC9]); // RET
}

/// `__mul32`: `HL:DE = l * r` (mod 2^32) — `l` in the two stack words under the
/// return address, `r` in `HL:DE`. Three 16-bit partial products:
/// `l.lo*r.lo` in full 32 (`__mul16w`), plus `l.lo*r.hi` and `l.hi*r.lo` low words
/// into the high half (`l.hi*r.hi` only feeds bits ≥ 32 — dropped by wrapping).
pub(super) fn emit_mul32(a: &mut Asm) {
    a.define("__mul32");
    let (llo, lhi, rlo, rhi, phi) = (a.label(), a.label(), a.label(), a.label(), a.label());
    // Spill r, then pull l from under the return address (restoring the stack shape
    // so the caller's cleanup matches the trap path).
    a.st_hl_mem(Imm::Label(rlo)); // LD (Rlo), HL
    a.ex_de_hl(); // EX DE,HL
    a.st_hl_mem(Imm::Label(rhi)); // LD (Rhi), HL
    a.pop(R16::Bc); // POP BC        (return address)
    a.pop(R16::Hl); // POP HL        (l.lo)
    a.st_hl_mem(Imm::Label(llo)); // LD (Llo), HL
    a.pop(R16::De); // POP DE        (l.hi)
    a.push(R16::De); // PUSH DE
    a.push(R16::Hl); // PUSH HL
    a.push(R16::Bc); // PUSH BC       (return address back)
    a.ex_de_hl(); // EX DE,HL      (HL = l.hi)
    a.st_hl_mem(Imm::Label(lhi)); // LD (Lhi), HL
                                  // p = l.lo * r.lo, full 32.
    a.ld_wide_mem(R16::Bc, Imm::Label(llo)); // LD BC,(Llo)
    a.ld_wide_mem(R16::De, Imm::Label(rlo)); // LD DE,(Rlo)
    a.call("__mul16w"); // DE:HL = BC*DE
    a.push(R16::Hl); // PUSH HL       (p.lo — safe across the __mul16 calls)
    a.ex_de_hl(); // EX DE,HL
    a.st_hl_mem(Imm::Label(phi)); // LD (Phi), HL
                                  // p.hi += l.lo * r.hi (low word).
    a.ld_hl_mem(Imm::Label(llo)); // LD HL,(Llo)
    a.ld_wide_mem(R16::De, Imm::Label(rhi)); // LD DE,(Rhi)
    a.call("__mul16"); // HL = HL*DE (low 16)
    a.ex_de_hl(); // EX DE,HL      (DE = t)
    a.ld_hl_mem(Imm::Label(phi)); // LD HL,(Phi)
    a.add_hl(R16::De); // ADD HL,DE
    a.st_hl_mem(Imm::Label(phi)); // LD (Phi), HL
                                  // p.hi += l.hi * r.lo (low word).
    a.ld_hl_mem(Imm::Label(lhi)); // LD HL,(Lhi)
    a.ld_wide_mem(R16::De, Imm::Label(rlo)); // LD DE,(Rlo)
    a.call("__mul16");
    a.ex_de_hl(); // EX DE,HL
    a.ld_hl_mem(Imm::Label(phi)); // LD HL,(Phi)
    a.add_hl(R16::De); // ADD HL,DE
    a.ex_de_hl(); // EX DE,HL      (DE = p.hi)
    a.pop(R16::Hl); // POP HL        (p.lo)
    a.fx(&[0xC9]); // RET
                   // The static scratch words (addressed by the label operands above).
    for l in [llo, lhi, rlo, rhi, phi] {
        a.place(l);
        a.data_word(Imm::Abs(0));
    }
}

/// `__divmod32`: restoring 32÷32 division — dividend `N` in the two stack words,
/// divisor `D` in `HL:DE`. Quotient returns in `HL:DE`; the remainder is written
/// back into the two stack words (the caller pops it for `%`, discards for `/`).
/// The `N/Q` register shifts left in the alternate set (safe: interrupts are off /
/// absent); the remainder lives in main `HL:DE` with `D` fetched from scratch. The
/// carry out of the remainder shift is the 33rd bit — it forces a commit, so a
/// divisor ≥ 2^31 still divides correctly. `D == 0` yields `q = 0xFFFF_FFFF`,
/// `rem = N` (bounded garbage — rustc-checked code can't reach it).
pub(super) fn emit_divmod32(a: &mut Asm) {
    a.define("__divmod32");
    let (dlo, dhi, dret) = (a.label(), a.label(), a.label());
    // Spill D and the return address; move N into the alternate BC:DE.
    a.st_hl_mem(Imm::Label(dlo)); // LD (Dlo), HL
    a.ex_de_hl(); // EX DE,HL
    a.st_hl_mem(Imm::Label(dhi)); // LD (Dhi), HL
    a.pop(R16::Bc); // POP BC        (return address)
    a.st_wide_mem(R16::Bc, Imm::Label(dret)); // LD (Dret), BC
    a.pop(R16::De); // POP DE        (N.lo)
    a.pop(R16::Bc); // POP BC        (N.hi)
    a.push(R16::Bc); // PUSH BC        ─┐ hand N across to the
    a.push(R16::De); // PUSH DE         │ alternate register set
    a.fx(&[0xD9]); // EXX             │
    a.pop(R16::De); // POP DE (N.lo)   │
    a.pop(R16::Bc); // POP BC (N.hi)  ─┘
    a.fx(&[0xD9]); // EXX
                   // R (remainder) = 0 in main HL:DE.
    a.ld_imm(R16::Hl, Imm::Abs(0)); // LD HL, 0
    a.ld_imm(R16::De, Imm::Abs(0)); // LD DE, 0
    a.fx(&[0x3E, 32]); // LD A, 32
    let (top, force, commit, next) = (a.label(), a.label(), a.label(), a.label());
    a.place(top);
    // Shift N/Q left one bit (alternate set); CF = the next dividend bit.
    a.fx(&[0xD9]); // EXX
    a.fx(&[0xCB, 0x23]); // SLA E
    a.fx(&[0xCB, 0x12]); // RL D
    a.fx(&[0xCB, 0x11]); // RL C
    a.fx(&[0xCB, 0x10]); // RL B          (CF = N msb out)
    a.fx(&[0xD9]); // EXX           (flags survive)
                   // R = R<<1 | bit — 33 bits: a carry out of the high word forces a commit.
    a.fx(&[0xED, 0x6A]); // ADC HL,HL     (R.lo)
    a.ex_de_hl(); // EX DE,HL
    a.fx(&[0xED, 0x6A]); // ADC HL,HL     (R.hi; CF = bit 32)
    a.jump(0xDA, force); // JP C,force
                         // Trial subtract T = R - D. (Entering: HL = R.hi, DE = R.lo.)
    a.ex_de_hl(); // EX DE,HL      (HL = R.lo, DE = R.hi)
    a.ld_wide_mem(R16::Bc, Imm::Label(dlo)); // LD BC,(Dlo)
    a.fx(&[0xB7]); // OR A
    a.fx(&[0xED, 0x42]); // SBC HL,BC     (T.lo)
    a.ex_de_hl(); // EX DE,HL      (HL = R.hi, DE = T.lo)
    a.ld_wide_mem(R16::Bc, Imm::Label(dhi)); // LD BC,(Dhi)
    a.fx(&[0xED, 0x42]); // SBC HL,BC     (T.hi; CF = R < D)
    a.jump(0xD2, commit); // JP NC,commit
                          // Restore: R = T + D.
    a.ex_de_hl(); // (HL = T.lo, DE = T.hi)
    a.ld_wide_mem(R16::Bc, Imm::Label(dlo)); // LD BC,(Dlo)
    a.add_hl(R16::Bc); // ADD HL,BC     (R.lo back)
    a.ex_de_hl(); // (HL = T.hi, DE = R.lo)
    a.ld_wide_mem(R16::Bc, Imm::Label(dhi)); // LD BC,(Dhi)
    a.fx(&[0xED, 0x4A]); // ADC HL,BC     (R.hi back)
    a.jump(0xC3, next);
    // Force: the shifted-out bit 32 makes R ≥ D whatever the 32-bit compare says.
    a.place(force);
    a.ex_de_hl();
    a.ld_wide_mem(R16::Bc, Imm::Label(dlo)); // LD BC,(Dlo)
    a.fx(&[0xB7]); // OR A
    a.fx(&[0xED, 0x42]); // SBC HL,BC
    a.ex_de_hl();
    a.ld_wide_mem(R16::Bc, Imm::Label(dhi)); // LD BC,(Dhi)
    a.fx(&[0xED, 0x42]); // SBC HL,BC     (the hidden bit 32 absorbs any borrow)
                         // Commit: keep T as the new R; the quotient bit is bit 0 just vacated by SLA E.
    a.place(commit);
    a.fx(&[0xD9]); // EXX
    a.fx(&[0x1C]); // INC E
    a.fx(&[0xD9]); // EXX
    a.place(next); // (all paths: HL = R.hi, DE = R.lo)
    a.ex_de_hl(); // EX DE,HL
    a.fx(&[0x3D]); // DEC A
    a.jump(0xC2, top); // JP NZ,top
                       // Done: remainder = HL:DE (lo:hi), quotient = alternate BC:DE (hi:lo).
    a.push(R16::De); // PUSH DE       (rem.hi — stays for the caller)
    a.push(R16::Hl); // PUSH HL       (rem.lo — stays for the caller)
    a.fx(&[0xD9]); // EXX
    a.push(R16::Bc); // PUSH BC        ─┐ hand Q back across
    a.push(R16::De); // PUSH DE         │
    a.fx(&[0xD9]); // EXX             │
    a.pop(R16::Hl); // POP HL (Q.lo)   │
    a.pop(R16::De); // POP DE (Q.hi)  ─┘
    a.ld_wide_mem(R16::Bc, Imm::Label(dret)); // LD BC,(Dret)
    a.push(R16::Bc); // PUSH BC       (return address)
    a.fx(&[0xC9]); // RET           (the remainder words remain on the stack)
    for l in [dlo, dhi, dret] {
        a.place(l);
        a.data_word(Imm::Abs(0));
    }
}

/// `__sdivmod16`: signed 16-bit divide — `HL = HL / DE`, `DE = HL % DE`, two's
/// complement, truncating toward zero, remainder taking the dividend's sign (rustc
/// semantics). Strips the signs, runs the unsigned core (the software `__divmod16` on
/// Spectrum, the `ED FE` DIVMOD16 trap on Cell — so a `/ 0` still honours the
/// divide-by-zero policy), and reapplies them. Clobbers AF/BC.
pub(super) fn emit_sdivmod16(a: &mut Asm) {
    a.define("__sdivmod16");
    let (abs_l, abs_r, fix_rem, fix_q) = (a.label(), a.label(), a.label(), a.label());
    // Stash the result signs: quotient = sign(l) ^ sign(r), remainder = sign(l).
    a.fx(&[0x7C]); // LD A,H
    a.fx(&[0xAA]); // XOR D
    a.push(R16::Af); // PUSH AF       (bit 7 = negate quotient)
    a.fx(&[0x7C]); // LD A,H
    a.push(R16::Af); // PUSH AF       (bit 7 = negate remainder)
                     // |l|: negate HL if negative.
    a.fx(&[0xCB, 0x7C]); // BIT 7,H
    a.jump(0xCA, abs_l); // JP Z
    a.fx(&[0x7D]); // LD A,L
    a.fx(&[0x2F]); // CPL
    a.fx(&[0x6F]); // LD L,A
    a.fx(&[0x7C]); // LD A,H
    a.fx(&[0x2F]); // CPL
    a.fx(&[0x67]); // LD H,A
    a.fx(&[0x23]); // INC HL
    a.place(abs_l);
    // |r|: negate DE if negative.
    a.fx(&[0xCB, 0x7A]); // BIT 7,D
    a.jump(0xCA, abs_r); // JP Z
    a.fx(&[0x7B]); // LD A,E
    a.fx(&[0x2F]); // CPL
    a.fx(&[0x5F]); // LD E,A
    a.fx(&[0x7A]); // LD A,D
    a.fx(&[0x2F]); // CPL
    a.fx(&[0x57]); // LD D,A
    a.fx(&[0x13]); // INC DE
    a.place(abs_r);
    // The unsigned core: HL/DE -> HL = q, DE = rem.
    match a.target {
        Target::Spectrum48 => a.call("__divmod16"),
        Target::Cell => {
            a.fx(&[0x44]); // LD B,H
            a.fx(&[0x4D]); // LD C,L        (BC = |dividend|)
            gen_trap(a, TRAP_DIVMOD16); // HL = BC/DE, DE = BC%DE
        }
    }
    // Reapply the signs (remainder first — its flag was pushed last).
    a.pop(R16::Af); // POP AF        (bit 7 = negate remainder)
    a.fx(&[0x07]); // RLCA          (bit 7 -> carry)
    a.jump(0xD2, fix_rem); // JP NC
    a.fx(&[0x7B]); // LD A,E
    a.fx(&[0x2F]); // CPL
    a.fx(&[0x5F]); // LD E,A
    a.fx(&[0x7A]); // LD A,D
    a.fx(&[0x2F]); // CPL
    a.fx(&[0x57]); // LD D,A
    a.fx(&[0x13]); // INC DE
    a.place(fix_rem);
    a.pop(R16::Af); // POP AF        (bit 7 = negate quotient)
    a.fx(&[0x07]); // RLCA
    a.jump(0xD2, fix_q); // JP NC
    a.fx(&[0x7D]); // LD A,L
    a.fx(&[0x2F]); // CPL
    a.fx(&[0x6F]); // LD L,A
    a.fx(&[0x7C]); // LD A,H
    a.fx(&[0x2F]); // CPL
    a.fx(&[0x67]); // LD H,A
    a.fx(&[0x23]); // INC HL
    a.place(fix_q);
    a.fx(&[0xC9]); // RET
}
