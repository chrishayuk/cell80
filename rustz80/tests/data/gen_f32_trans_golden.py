#!/usr/bin/env python3
"""Golden-table generator for the F2 transcendental kernels (fexp/fln/fpow).

Writes `f32_trans_golden.json` next to itself. Two independent references per case:

- `true_bits` — the CORRECTLY-ROUNDED f32 result, computed with mpmath at 96-bit
  precision (MPFR-class ground truth) and rounded once, ties-to-even, by explicit
  candidate selection. This is what the declared ULP bound (`//! accuracy:`) is
  measured against — libm is involved nowhere.
- `sim_bits` — the kernel algorithm SIMULATED host-side in numpy float32. Every
  kernel step is a correctly-rounded IEEE f32 op (the F0 kernels), and numpy f32
  scalar arithmetic is the same correctly-rounded op — so the simulation is
  bit-exact to the kernel by construction. The Rust harness asserts kernel ==
  sim_bits EXACTLY (catches any transcription drift between this file and
  softfloat.rs), and ulp(kernel, true_bits) <= bound (the accuracy contract).

The per-function `bound` written into the JSON is the MEASURED max ULP over the
bank plus 1 headroom — set from measurement, never hope (H-F2 discipline). Rerun:

    python3 gen_f32_trans_golden.py

Deterministic (fixed seeds, fixed mpmath precision 96); the JSON is diff-clean on
rerun. Requires numpy + mpmath.
"""

import json
import math
import pathlib

import numpy as np
from mpmath import mp, mpf, exp as mexp, log as mlog

mp.prec = 96
RNG = np.random.RandomState(0x5EED_F2)

F32 = np.float32
POS_INF, NEG_INF, QNAN = 0x7F800000, 0xFF800000, 0x7FC00000
ONE = 0x3F800000

# The Cephes-constant bit patterns the kernels carry (softfloat.rs must match).
LOG2EF, EXP_C1, EXP_C2 = 0x3FB8AA3B, 0x3F318000, 0xB95E8083
EXP_P = [0x39506967, 0x3AB743CE, 0x3C088908, 0x3D2AA9C1, 0x3E2AAAAA, 0x3F000000]
SQRTHF = 0x3F3504F3
LOG_P = [0x3D9021BB, 0xBDEBD1B8, 0x3DEF251A, 0xBDFE5D4F, 0x3E11E9BF,
         0xBE2AAE50, 0x3E4CCEAC, 0xBE7FFFFC, 0x3EAAAAAA]
LOG_Q1, LOG_Q2 = 0xB95E8083, 0x3F318000
EXP_HI, EXP_LO = 0x42B17218, 0x42CFF1B5  # |x| magnitudes of the clamp thresholds


def bits(x: np.float32) -> int:
    return int(np.array(x, dtype=np.float32).view(np.uint32))


def f32(b: int) -> np.float32:
    return np.array(np.uint32(b), dtype=np.uint32).view(np.float32)[()]


def is_nan(b: int) -> bool:
    return b & 0x7FFFFFFF > 0x7F800000


def round_f32(v) -> int:
    """Correctly round an mpf to f32 bits, ties-to-even, by candidate selection."""
    if v == 0:
        return 0
    neg = v < 0
    a = -v if neg else v
    maxf = (mpf(2) ** 128) - (mpf(2) ** 103)
    if a >= maxf + mpf(2) ** 102:  # the round-to-infinity boundary (tie -> inf)
        return NEG_INF if neg else POS_INF
    if a < mpf(2) ** -150:  # below half the min subnormal (tie -> 0, even)
        return 0x80000000 if neg else 0
    if a == mpf(2) ** -150:
        return 0x80000000 if neg else 0
    with np.errstate(all="ignore"):
        c = F32(float(a))  # double is correctly rounded by mpmath; refine below
        cands = {c}
        cands.add(np.nextafter(c, F32(np.inf)))
        cands.add(np.nextafter(c, F32(-np.inf)))
    best, best_err = None, None
    for cd in sorted(cands):
        if not np.isfinite(cd) or cd < 0:
            continue
        err = abs(mpf(float(cd)) - a)
        if best is None or err < best_err or (err == best_err and bits(cd) & 1 == 0):
            best, best_err = cd, err
    b = bits(best)
    return (b | 0x80000000) if neg else b


# ── the kernel algorithms, simulated in correctly-rounded f32 (numpy) ─────────


def rust_round(x: np.float32) -> np.float32:
    """Rust f32::round — half away from zero. Exact in double for f32 inputs."""
    d = float(x)
    a = abs(d)
    fl = math.floor(a)
    r = fl + 1 if a - fl >= 0.5 else fl
    return F32(-r if d < 0 else r)


def sim_fexp(xb: int) -> int:
    mag, sgn = xb & 0x7FFFFFFF, xb >> 31
    if mag > 0x7F800000:
        return QNAN
    if mag == 0x7F800000:
        return POS_INF if sgn == 0 else 0
    if sgn == 0 and mag >= EXP_HI:
        return POS_INF
    if sgn == 1 and mag >= EXP_LO:
        return 0
    if mag == 0:
        return ONE
    with np.errstate(all="ignore"):
        x = f32(xb)
        nf = rust_round(F32(x * f32(LOG2EF)))
        nmag = bits(nf) & 0x7FFFFFFF
        nv = 0
        if nmag != 0:
            nv = ((nmag & 0x7FFFFF) | 0x800000) >> (150 - (nmag >> 23))
        r = F32(F32(x - F32(nf * f32(EXP_C1))) - F32(nf * f32(EXP_C2)))
        p = f32(EXP_P[0])
        for c in EXP_P[1:]:
            p = F32(F32(p * r) + f32(c))
        er = F32(F32(F32(F32(r * r) * p) + r) + f32(ONE))
        k1, k2 = nv >> 1, nv - (nv >> 1)
        if bits(nf) >> 31 == 1:
            s1, s2 = (127 - k1) << 23, (127 - k2) << 23
        else:
            s1, s2 = (127 + k1) << 23, (127 + k2) << 23
        return bits(F32(F32(er * f32(s1)) * f32(s2)))


def sim_fln(xb: int) -> int:
    mag, sgn = xb & 0x7FFFFFFF, xb >> 31
    if mag > 0x7F800000:
        return QNAN
    if mag == 0:
        return NEG_INF
    if sgn == 1:
        return QNAN
    if mag == 0x7F800000:
        return POS_INF
    with np.errstate(all="ignore"):
        b, esub = mag, 0
        if b < 0x800000:
            b = bits(F32(f32(b) * f32(0x4C000000)))
            esub = 25
        eb = b >> 23
        xh = (b & 0x7FFFFF) | 0x3F000000
        e = eb - (126 + esub)
        if xh < SQRTHF:
            e -= 1
            x1 = F32(F32(f32(xh) + f32(xh)) - f32(ONE))
        else:
            x1 = F32(f32(xh) - f32(ONE))
        z = F32(x1 * x1)
        p = f32(LOG_P[0])
        for c in LOG_P[1:]:
            p = F32(F32(p * x1) + f32(c))
        y = F32(F32(x1 * z) * p)
        fe = F32(abs(e))
        if e < 0:
            fe = F32(-fe)
        y = F32(y + F32(fe * f32(LOG_Q1)))
        y = F32(y - F32(z * f32(0x3F000000)))
        return bits(F32(F32(x1 + y) + F32(fe * f32(LOG_Q2))))


def sim_fpow(ab: int, bb: int) -> int:
    amag, bmag = ab & 0x7FFFFFFF, bb & 0x7FFFFFFF
    if bmag == 0:
        return ONE
    if ab == ONE:
        return ONE
    if amag > 0x7F800000 or bmag > 0x7F800000:
        return QNAN
    if (ab >> 31) == 1 and amag != 0:
        return QNAN
    if amag == 0:
        return POS_INF if (bb >> 31) == 1 else 0
    with np.errstate(all="ignore"):
        return sim_fexp(bits(F32(f32(bb) * f32(sim_fln(ab)))))


# ── ground truth per case ─────────────────────────────────────────────────────


def true_fexp(xb: int) -> int:
    if is_nan(xb):
        return QNAN
    if xb == POS_INF:
        return POS_INF
    if xb == NEG_INF:
        return 0
    return round_f32(mexp(mpf(float(f32(xb)))))


def true_fln(xb: int) -> int:
    if is_nan(xb):
        return QNAN
    mag, sgn = xb & 0x7FFFFFFF, xb >> 31
    if mag == 0:
        return NEG_INF
    if sgn == 1:
        return QNAN
    if xb == POS_INF:
        return POS_INF
    return round_f32(mlog(mpf(float(f32(xb)))))


def true_fpow(ab: int, bb: int) -> int:
    # Only sampled over finite a > 0, finite b != 0 — specials are pinned in the
    # Rust specials table, not the golden bank.
    a, b = mpf(float(f32(ab))), mpf(float(f32(bb)))
    return round_f32(mexp(b * mlog(a)))


# ── the sampled banks ─────────────────────────────────────────────────────────


def f32_from_value(v: float) -> int:
    with np.errstate(all="ignore"):
        return bits(F32(v))


def exp_inputs() -> list[int]:
    xs = set()
    # specials + the F0 edge families
    xs |= {0, 0x80000000, POS_INF, NEG_INF, QNAN, 0x7F800001, 0xFFC00000,
           0x00000001, 0x80000001, 0x007FFFFF, 0x00800000, ONE, ONE | 0x80000000,
           0x3F800001, 0x33800000, 0xB3800000}
    # the clamp thresholds and their neighbours
    for t in (EXP_HI, EXP_LO):
        for d in (-2, -1, 0):  # at/below the magnitude threshold on both signs
            xs.add((t + d))
            xs.add((t + d) | 0x80000000)
    # reduction-boundary clusters: x near (k + 0.5) * ln2 (fround tie pre-images)
    ln2 = math.log(2)
    for k in (-140, -100, -60, -10, -1, 0, 1, 10, 60, 120, 127):
        base = f32_from_value((k + 0.5) * ln2)
        for d in (-2, -1, 0, 1, 2):
            xs.add(base + d)
    # dense small |x| (the exp(x) ~ 1+x regime) and the finance band
    for v in np.concatenate([
        RNG.uniform(-103.9, 88.7, 250),
        RNG.uniform(-1.0, 1.0, 200),
        np.exp(RNG.uniform(np.log(1e-30), 0, 100)),
        -np.exp(RNG.uniform(np.log(1e-30), 0, 100)),
        RNG.uniform(-0.05, 0.05, 100),  # ln(1+r)-scale finance arguments
    ]):
        xs.add(f32_from_value(float(v)))
    return sorted(xs)


def ln_inputs() -> list[int]:
    xs = set()
    xs |= {0, 0x80000000, POS_INF, NEG_INF, QNAN, ONE | 0x80000000,
           0x00000001, 0x00000002, 0x007FFFFF, 0x00800000, 0x00800001,
           ONE, 0x7F7FFFFF}
    # the near-1 band, where ln is hardest (catastrophic cancellation regime)
    for d in range(-40, 41):
        xs.add(ONE + d)
    # the sqrt(2)/sqrt(1/2) branch boundary in the reduced mantissa
    for base in (SQRTHF, 0x3FB504F3):  # sqrt(1/2), sqrt(2)
        for d in (-2, -1, 0, 1, 2):
            xs.add(base + d)
    # powers of two (exact-mantissa inputs, e-term dominated)
    for k in range(-149, 128, 9):
        xs.add(f32_from_value(math.ldexp(1.0, k)))
    # log-uniform sweep + subnormals + the finance band 1+r
    for v in np.concatenate([
        np.exp(RNG.uniform(np.log(1e-38), np.log(3e38), 350)),
        np.exp(RNG.uniform(np.log(1e-45), np.log(1e-38), 50)),
        1.0 + RNG.uniform(1e-4, 0.5, 150),
    ]):
        xs.add(f32_from_value(float(v)))
    return sorted(xs)


def pow_inputs() -> list[tuple[int, int]]:
    ps = []
    # the finance shapes: (1+r)^n and general moderate-|b ln a| pairs
    for _ in range(150):
        r = float(np.exp(RNG.uniform(np.log(1e-4), np.log(0.5))))
        n = float(RNG.uniform(1, 360))
        ps.append((f32_from_value(1.0 + r), f32_from_value(n)))
        ps.append((f32_from_value(1.0 + r), f32_from_value(-n)))
    for _ in range(400):
        a = float(np.exp(RNG.uniform(np.log(1e-6), np.log(1e6))))
        b = float(RNG.uniform(-30, 30))
        if abs(b * math.log(a)) <= 60 and b != 0:
            ps.append((f32_from_value(a), f32_from_value(b)))
    # fractional roots (the nth_root_f32 displacement case)
    for n in (2, 3, 5, 7, 12):
        for _ in range(10):
            a = float(np.exp(RNG.uniform(np.log(1e-3), np.log(1e3))))
            ps.append((f32_from_value(a), f32_from_value(1.0 / n)))
    return sorted(set(ps))


def ulp_dist(a: int, b: int) -> int:
    def ord32(x: int) -> int:
        return (x | 0x80000000) if x >> 31 == 0 else 0x80000000 - (x & 0x7FFFFFFF)

    if is_nan(a) and is_nan(b):
        return 0
    return abs(ord32(a) - ord32(b))


def build():
    out = {}
    for name, inputs, sim, true in (
        ("fexp", [(x,) for x in exp_inputs()], sim_fexp, true_fexp),
        ("fln", [(x,) for x in ln_inputs()], sim_fln, true_fln),
        ("fpow", pow_inputs(), sim_fpow, true_fpow),
    ):
        cases, max_ulp = [], 0
        for args in inputs:
            s, t = sim(*args), true(*args)
            d = ulp_dist(s, t)
            max_ulp = max(max_ulp, d)
            cases.append(list(args) + [s, t])
        out[name] = {"bound": max_ulp + 1, "measured_max_ulp": max_ulp, "cases": cases}
        print(f"{name}: {len(cases)} cases, measured max ulp {max_ulp} -> bound {max_ulp + 1}")
    path = pathlib.Path(__file__).with_name("f32_trans_golden.json")
    path.write_text(json.dumps(out, separators=(",", ":")) + "\n")
    print(f"wrote {path}")


if __name__ == "__main__":
    build()
