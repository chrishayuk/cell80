//! The appended mul/div micro-runtime (Spectrum target) + the Cell80 `ED FE` trap ids.
use super::asm::Asm;

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
    a.byte(0x3E); // LD A, id
    a.byte(id);
    a.byte(0xED); // ED FE  (host trap)
    a.byte(0xFE);
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
    a.byte(0x21); // LD HL, 0
    a.word(0);
    a.byte(0x3E); // LD A, 16
    a.byte(16);
    let top = a.label();
    let skip = a.label();
    a.place(top);
    a.byte(0x29); // ADD HL,HL
    a.byte(0xCB);
    a.byte(0x13); // RL E
    a.byte(0xCB);
    a.byte(0x12); // RL D          (CF = multiplier MSB out)
    a.jump(0xD2, skip); // JP NC,skip
    a.byte(0x09); // ADD HL,BC     (product += multiplicand)
    a.jump(0xD2, skip); // JP NC,skip
    a.byte(0x13); // INC DE        (carry into the high word)
    a.place(skip);
    a.byte(0x3D); // DEC A
    a.jump(0xC2, top); // JP NZ,top
    a.byte(0xC9); // RET
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
    a.byte(0x22); // LD (Rlo), HL
    a.word_label(rlo);
    a.byte(0xEB); // EX DE,HL
    a.byte(0x22); // LD (Rhi), HL
    a.word_label(rhi);
    a.byte(0xC1); // POP BC        (return address)
    a.byte(0xE1); // POP HL        (l.lo)
    a.byte(0x22); // LD (Llo), HL
    a.word_label(llo);
    a.byte(0xD1); // POP DE        (l.hi)
    a.byte(0xD5); // PUSH DE
    a.byte(0xE5); // PUSH HL
    a.byte(0xC5); // PUSH BC       (return address back)
    a.byte(0xEB); // EX DE,HL      (HL = l.hi)
    a.byte(0x22); // LD (Lhi), HL
    a.word_label(lhi);
    // p = l.lo * r.lo, full 32.
    a.byte(0xED);
    a.byte(0x4B); // LD BC,(Llo)
    a.word_label(llo);
    a.byte(0xED);
    a.byte(0x5B); // LD DE,(Rlo)
    a.word_label(rlo);
    a.call("__mul16w"); // DE:HL = BC*DE
    a.byte(0xE5); // PUSH HL       (p.lo — safe across the __mul16 calls)
    a.byte(0xEB); // EX DE,HL
    a.byte(0x22); // LD (Phi), HL
    a.word_label(phi);
    // p.hi += l.lo * r.hi (low word).
    a.byte(0x2A); // LD HL,(Llo)
    a.word_label(llo);
    a.byte(0xED);
    a.byte(0x5B); // LD DE,(Rhi)
    a.word_label(rhi);
    a.call("__mul16"); // HL = HL*DE (low 16)
    a.byte(0xEB); // EX DE,HL      (DE = t)
    a.byte(0x2A); // LD HL,(Phi)
    a.word_label(phi);
    a.byte(0x19); // ADD HL,DE
    a.byte(0x22); // LD (Phi), HL
    a.word_label(phi);
    // p.hi += l.hi * r.lo (low word).
    a.byte(0x2A); // LD HL,(Lhi)
    a.word_label(lhi);
    a.byte(0xED);
    a.byte(0x5B); // LD DE,(Rlo)
    a.word_label(rlo);
    a.call("__mul16");
    a.byte(0xEB); // EX DE,HL
    a.byte(0x2A); // LD HL,(Phi)
    a.word_label(phi);
    a.byte(0x19); // ADD HL,DE
    a.byte(0xEB); // EX DE,HL      (DE = p.hi)
    a.byte(0xE1); // POP HL        (p.lo)
    a.byte(0xC9); // RET
                  // The static scratch words (addressed by the fixups above).
    for l in [llo, lhi, rlo, rhi, phi] {
        a.place(l);
        a.word(0);
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
    a.byte(0x22); // LD (Dlo), HL
    a.word_label(dlo);
    a.byte(0xEB); // EX DE,HL
    a.byte(0x22); // LD (Dhi), HL
    a.word_label(dhi);
    a.byte(0xC1); // POP BC        (return address)
    a.byte(0xED);
    a.byte(0x43); // LD (Dret), BC
    a.word_label(dret);
    a.byte(0xD1); // POP DE        (N.lo)
    a.byte(0xC1); // POP BC        (N.hi)
    a.byte(0xC5); // PUSH BC        ─┐ hand N across to the
    a.byte(0xD5); // PUSH DE         │ alternate register set
    a.byte(0xD9); // EXX             │
    a.byte(0xD1); // POP DE (N.lo)   │
    a.byte(0xC1); // POP BC (N.hi)  ─┘
    a.byte(0xD9); // EXX
                  // R (remainder) = 0 in main HL:DE.
    a.byte(0x21); // LD HL, 0
    a.word(0);
    a.byte(0x11); // LD DE, 0
    a.word(0);
    a.byte(0x3E); // LD A, 32
    a.byte(32);
    let (top, force, commit, next) = (a.label(), a.label(), a.label(), a.label());
    a.place(top);
    // Shift N/Q left one bit (alternate set); CF = the next dividend bit.
    a.byte(0xD9); // EXX
    a.byte(0xCB);
    a.byte(0x23); // SLA E
    a.byte(0xCB);
    a.byte(0x12); // RL D
    a.byte(0xCB);
    a.byte(0x11); // RL C
    a.byte(0xCB);
    a.byte(0x10); // RL B          (CF = N msb out)
    a.byte(0xD9); // EXX           (flags survive)
                  // R = R<<1 | bit — 33 bits: a carry out of the high word forces a commit.
    a.byte(0xED);
    a.byte(0x6A); // ADC HL,HL     (R.lo)
    a.byte(0xEB); // EX DE,HL
    a.byte(0xED);
    a.byte(0x6A); // ADC HL,HL     (R.hi; CF = bit 32)
    a.jump(0xDA, force); // JP C,force
                         // Trial subtract T = R - D. (Entering: HL = R.hi, DE = R.lo.)
    a.byte(0xEB); // EX DE,HL      (HL = R.lo, DE = R.hi)
    a.byte(0xED);
    a.byte(0x4B); // LD BC,(Dlo)
    a.word_label(dlo);
    a.byte(0xB7); // OR A
    a.byte(0xED);
    a.byte(0x42); // SBC HL,BC     (T.lo)
    a.byte(0xEB); // EX DE,HL      (HL = R.hi, DE = T.lo)
    a.byte(0xED);
    a.byte(0x4B); // LD BC,(Dhi)
    a.word_label(dhi);
    a.byte(0xED);
    a.byte(0x42); // SBC HL,BC     (T.hi; CF = R < D)
    a.jump(0xD2, commit); // JP NC,commit
                          // Restore: R = T + D.
    a.byte(0xEB); // (HL = T.lo, DE = T.hi)
    a.byte(0xED);
    a.byte(0x4B); // LD BC,(Dlo)
    a.word_label(dlo);
    a.byte(0x09); // ADD HL,BC     (R.lo back)
    a.byte(0xEB); // (HL = T.hi, DE = R.lo)
    a.byte(0xED);
    a.byte(0x4B); // LD BC,(Dhi)
    a.word_label(dhi);
    a.byte(0xED);
    a.byte(0x4A); // ADC HL,BC     (R.hi back)
    a.jump(0xC3, next);
    // Force: the shifted-out bit 32 makes R ≥ D whatever the 32-bit compare says.
    a.place(force);
    a.byte(0xEB);
    a.byte(0xED);
    a.byte(0x4B); // LD BC,(Dlo)
    a.word_label(dlo);
    a.byte(0xB7); // OR A
    a.byte(0xED);
    a.byte(0x42); // SBC HL,BC
    a.byte(0xEB);
    a.byte(0xED);
    a.byte(0x4B); // LD BC,(Dhi)
    a.word_label(dhi);
    a.byte(0xED);
    a.byte(0x42); // SBC HL,BC     (the hidden bit 32 absorbs any borrow)
                  // Commit: keep T as the new R; the quotient bit is bit 0 just vacated by SLA E.
    a.place(commit);
    a.byte(0xD9); // EXX
    a.byte(0x1C); // INC E
    a.byte(0xD9); // EXX
    a.place(next); // (all paths: HL = R.hi, DE = R.lo)
    a.byte(0xEB); // EX DE,HL
    a.byte(0x3D); // DEC A
    a.jump(0xC2, top); // JP NZ,top
                       // Done: remainder = HL:DE (lo:hi), quotient = alternate BC:DE (hi:lo).
    a.byte(0xD5); // PUSH DE       (rem.hi — stays for the caller)
    a.byte(0xE5); // PUSH HL       (rem.lo — stays for the caller)
    a.byte(0xD9); // EXX
    a.byte(0xC5); // PUSH BC        ─┐ hand Q back across
    a.byte(0xD5); // PUSH DE         │
    a.byte(0xD9); // EXX             │
    a.byte(0xE1); // POP HL (Q.lo)   │
    a.byte(0xD1); // POP DE (Q.hi)  ─┘
    a.byte(0xED);
    a.byte(0x4B); // LD BC,(Dret)
    a.word_label(dret);
    a.byte(0xC5); // PUSH BC       (return address)
    a.byte(0xC9); // RET           (the remainder words remain on the stack)
    for l in [dlo, dhi, dret] {
        a.place(l);
        a.word(0);
    }
}
