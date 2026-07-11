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
from mpmath import mp, mpf, exp as mexp, log as mlog, sin as msin, cos as mcos, atan2 as matan2

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
TWO_OPI, SIN_DOM = 0x3F22F983, 0x46000000  # 2/pi; |x| <= 8192, the declared trig domain
PIO2_1, PIO2_2, PIO2_3 = 0x3FC90000, 0x39FDA000, 0x33A22169
SIN_C = [0xB94CA1F9, 0x3C08839E, 0xBE2AAAA3]
COS_C = [0x37CCF5CE, 0xBAB6061A, 0x3D2AAAA5]
ATAN_C = [0x3DA4F0D1, 0xBE0E1B85, 0x3E4C925F, 0xBEAAAA2A]
TAN_PI8, PIO4, PIO2, PI_B, PI34 = 0x3ED413CD, 0x3F490FDB, 0x3FC90FDB, 0x40490FDB, 0x4016CBE4


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


def _sincos_core(xb: int, want_sin: bool) -> int:
    with np.errstate(all="ignore"):
        x = f32(xb)
        nf = rust_round(F32(x * f32(TWO_OPI)))
        nmag = bits(nf) & 0x7FFFFFFF
        nv = 0
        if nmag != 0:
            nv = ((nmag & 0x7FFFFF) | 0x800000) >> (150 - (nmag >> 23))
        q = nv & 3
        if bits(nf) >> 31 == 1 and q != 0:
            q = 4 - q
        r = F32(x - F32(nf * f32(PIO2_1)))
        r = F32(r - F32(nf * f32(PIO2_2)))
        r = F32(r - F32(nf * f32(PIO2_3)))
        z = F32(r * r)

        def sin_poly():
            p = f32(SIN_C[0])
            for c in SIN_C[1:]:
                p = F32(F32(p * z) + f32(c))
            return F32(F32(F32(r * z) * p) + r)

        def cos_poly():
            p = f32(COS_C[0])
            for c in COS_C[1:]:
                p = F32(F32(p * z) + f32(c))
            t = F32(F32(F32(z * z) * p) + F32(z * f32(0xBF000000)))
            return F32(t + f32(0x3F800000))

        if want_sin:
            v = sin_poly() if q in (0, 2) else cos_poly()
            vb = bits(v)
            if q >= 2:
                vb ^= 0x80000000
        else:
            v = cos_poly() if q in (0, 2) else sin_poly()
            vb = bits(v)
            if q in (1, 2):
                vb ^= 0x80000000
        return vb


def sim_fsin(xb: int) -> int:
    mag = xb & 0x7FFFFFFF
    if mag >= 0x7F800000 or mag > SIN_DOM:
        return QNAN
    if mag == 0:
        return xb
    return _sincos_core(xb, True)


def sim_fcos(xb: int) -> int:
    mag = xb & 0x7FFFFFFF
    if mag >= 0x7F800000 or mag > SIN_DOM:
        return QNAN
    if mag == 0:
        return ONE
    return _sincos_core(xb, False)


def sim_fatan2(yb: int, xb: int) -> int:
    ymag, xmag = yb & 0x7FFFFFFF, xb & 0x7FFFFFFF
    ys, xs = yb >> 31, xb >> 31
    if ymag > 0x7F800000 or xmag > 0x7F800000:
        return QNAN
    if ymag == 0:
        return (PI_B | (ys << 31)) if xs == 1 else ys << 31
    if xmag == 0:
        return PIO2 | (ys << 31)
    if ymag == 0x7F800000 and xmag == 0x7F800000:
        return (PI34 if xs == 1 else PIO4) | (ys << 31)
    if ymag == 0x7F800000:
        return PIO2 | (ys << 31)
    if xmag == 0x7F800000:
        return (PI_B | (ys << 31)) if xs == 1 else ys << 31
    with np.errstate(all="ignore"):
        num, den, inv = ymag, xmag, False
        if ymag > xmag:
            num, den, inv = xmag, ymag, True
        t = F32(f32(num) / f32(den))
        w, bias = t, False
        if bits(t) > TAN_PI8:
            w = F32(F32(t - f32(ONE)) / F32(t + f32(ONE)))
            bias = True
        z = F32(w * w)
        p = f32(ATAN_C[0])
        for c in ATAN_C[1:]:
            p = F32(F32(p * z) + f32(c))
        base = F32(F32(F32(w * z) * p) + w)
        if bias:
            base = F32(base + f32(PIO4))
        if inv:
            base = F32(f32(PIO2) - base)
        if xs == 1:
            base = F32(f32(PI_B) - base)
        return bits(base) | (ys << 31)


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


def true_fsin(xb: int) -> int:
    # Outside the declared |x| <= 8192 domain the CONTRACT is NaN (a deliberate
    # wall, pinned in the Rust specials table too) — the mathematical sin is not
    # the reference there, the declared behaviour is.
    if (xb & 0x7FFFFFFF) > SIN_DOM:
        return QNAN
    return round_f32(msin(mpf(float(f32(xb)))))


def true_fcos(xb: int) -> int:
    if (xb & 0x7FFFFFFF) > SIN_DOM:
        return QNAN
    return round_f32(mcos(mpf(float(f32(xb)))))


def true_fatan2(yb: int, xb: int) -> int:
    return round_f32(matan2(mpf(float(f32(yb))), mpf(float(f32(xb)))))


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


def sincos_inputs() -> list[int]:
    xs = set()
    ln2 = math.log(2)
    _ = ln2
    # quadrant boundaries and their fround tie pre-images: x near (k + 0.5) * pi/2
    for k in (-5215, -1000, -100, -11, -3, -2, -1, 0, 1, 2, 3, 11, 100, 1000, 5215):
        base = f32_from_value((k + 0.5) * math.pi / 2)
        near = f32_from_value(k * math.pi / 2)
        for d in (-2, -1, 0, 1, 2):
            for b in (base, near):  # near: sin/cos zero/±1 crossings, cancellation zone
                v = b + d
                if 0 <= v <= 0xFFFFFFFF:
                    xs.add(v)
    # tiny |x| (sin x ~ x regime) and subnormals
    xs |= {0x00000001, 0x007FFFFF, 0x33800000, 0xB3800000, 0x3F800000}
    for v in np.concatenate([
        RNG.uniform(-8192, 8192, 300),
        RNG.uniform(-6.5, 6.5, 250),
        np.exp(RNG.uniform(np.log(1e-20), 0, 100)),
    ]):
        xs.add(f32_from_value(float(v)))
    # the domain wall itself
    xs.add(SIN_DOM)
    return sorted(xs)


def atan2_inputs() -> list[tuple[int, int]]:
    ps = set()
    # all four quadrants, axis-adjacent slivers, and the tan(pi/8) fold boundary
    for _ in range(400):
        y = float(RNG.uniform(-100, 100))
        x = float(RNG.uniform(-100, 100))
        if y != 0.0 and x != 0.0:
            ps.add((f32_from_value(y), f32_from_value(x)))
    for _ in range(100):
        m = float(np.exp(RNG.uniform(np.log(1e-6), np.log(1e6))))
        ps.add((f32_from_value(m * 0.4142135), f32_from_value(m)))  # near the fold
        ps.add((f32_from_value(m), f32_from_value(m)))  # exactly pi/4 rays
    for _ in range(60):
        big = float(np.exp(RNG.uniform(np.log(1e3), np.log(1e30))))
        tiny = float(np.exp(RNG.uniform(np.log(1e-30), np.log(1e-3))))
        ps.add((f32_from_value(tiny), f32_from_value(big)))
        ps.add((f32_from_value(big), f32_from_value(tiny)))
        ps.add((f32_from_value(-tiny), f32_from_value(-big)))
    return sorted(ps)


def ulp_dist(a: int, b: int) -> int:
    def ord32(x: int) -> int:
        return (x | 0x80000000) if x >> 31 == 0 else 0x80000000 - (x & 0x7FFFFFFF)

    if is_nan(a) and is_nan(b):
        return 0
    return abs(ord32(a) - ord32(b))


def abs_err(a: int, b: int) -> float:
    """|f32(a) − f32(b)| exactly, in double (both f32 values are double-exact)."""
    fa, fb = float(f32(a)), float(f32(b))
    if math.isnan(fa) and math.isnan(fb):
        return 0.0
    return abs(fa - fb)


def build():
    # sin/cos carry an ABSOLUTE error bound, not ULP: near their zeros at large
    # |x|, Cody–Waite reduction error is a fixed absolute quantity (~ulp of the
    # n·π/2 products), so relative ULP diverges there for EVERY non-Payne-Hanek
    # f32 trig — the honest contract is |err| ≤ bound over the declared domain.
    for name, inputs, sim, true, kind in (
        ("fexp", [(x,) for x in exp_inputs()], sim_fexp, true_fexp, "ulp"),
        ("fln", [(x,) for x in ln_inputs()], sim_fln, true_fln, "ulp"),
        ("fpow", pow_inputs(), sim_fpow, true_fpow, "ulp"),
        ("fsin", [(x,) for x in sincos_inputs()], sim_fsin, true_fsin, "abs"),
        ("fcos", [(x,) for x in sincos_inputs()], sim_fcos, true_fcos, "abs"),
        ("fatan2", atan2_inputs(), sim_fatan2, true_fatan2, "ulp"),
    ):
        cases = []
        max_ulp, max_abs = 0, 0.0
        for args in inputs:
            s, t = sim(*args), true(*args)
            if kind == "ulp":
                max_ulp = max(max_ulp, ulp_dist(s, t))
            else:
                max_abs = max(max_abs, abs_err(s, t))
            cases.append(list(args) + [s, t])
        if kind == "ulp":
            entry = {"bound_kind": "ulp", "bound": max_ulp + 1,
                     "measured_max_ulp": max_ulp, "cases": cases}
            print(f"{name}: {len(cases)} cases, measured max ulp {max_ulp} -> bound {max_ulp + 1}")
        else:
            bound = max_abs * 1.25
            entry = {"bound_kind": "abs", "bound": bound,
                     "measured_max_abs": max_abs, "cases": cases}
            print(f"{name}: {len(cases)} cases, measured max abs err {max_abs:.3e} -> bound {bound:.3e}")
        BANKS[name] = entry
    path = pathlib.Path(__file__).with_name("f32_trans_golden.json")
    path.write_text(json.dumps(BANKS, separators=(",", ":")) + "\n")
    print(f"wrote {path}")


BANKS: dict = {}


if __name__ == "__main__":
    build()
