//! Host-oracle tests for the physics pack (`cell80/cells/physics/*.rs`) — the F3
//! wave: hand-authored typed-f32 cells whose every expected value is host rustc
//! f32 (the same golden reference the kernel banks use), plus the boundary story:
//! a non-finite result is a typed escalation, never an answer wearing Inf's
//! clothes. Values ride `Ty::F32` state fields as raw binary32 bits.

use crate::common::cell_src;
use cell80::{Halt, StateCell, DEFAULT_CYCLES};

fn run_f32(id: &str, state: &str, fields: &[(&str, f32)]) -> StateCell {
    let mut cell =
        StateCell::bind(&cell_src(id), state, None).unwrap_or_else(|e| panic!("{id}: {e}"));
    for (name, v) in fields {
        cell.set(name, v.to_bits() as u64)
            .unwrap_or_else(|e| panic!("{id}.{name}: {e}"));
    }
    let r = cell
        .run(DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    assert_eq!(r.halt, Halt::Returned, "{id}");
    cell
}

fn halt_of(id: &str, state: &str, fields: &[(&str, f32)]) -> Halt {
    let mut cell =
        StateCell::bind(&cell_src(id), state, None).unwrap_or_else(|e| panic!("{id}: {e}"));
    for (name, v) in fields {
        cell.set(name, v.to_bits() as u64).unwrap();
    }
    cell.run(DEFAULT_CYCLES).unwrap().halt
}

fn get(cell: &StateCell, field: &str) -> f32 {
    f32::from_bits(cell.get(field).expect(field) as u32)
}

#[test]
fn physics_pack_matches_rustc_bit_for_bit() {
    // kinetic_energy: 0.5*m*v*v
    for (m, v) in [
        (2.0f32, 3.0f32),
        (0.145, -40.2),
        (1.0e-3, 12.5),
        (80.0, 0.0),
    ] {
        let cell = run_f32("kinetic_energy_f32", "KineticEnergy", &[("m", m), ("v", v)]);
        let want = 0.5f32 * m * (v * v);
        assert_eq!(get(&cell, "e").to_bits(), want.to_bits(), "ke({m},{v})");
    }
    // drag_force: k*v*|v| keeps v's sign
    for (k, v) in [(0.47f32, 12.0f32), (0.47, -12.0), (1.5e-2, 0.0)] {
        let cell = run_f32("drag_force_f32", "DragForce", &[("k", k), ("v", v)]);
        let want = k * (v * v.abs());
        assert_eq!(get(&cell, "f").to_bits(), want.to_bits(), "drag({k},{v})");
    }
    // clamp: max-then-min form (NaN x resolves to lo — documented divergence
    // from f32::clamp's NaN propagation)
    for (x, lo, hi) in [
        (5.0f32, 0.0f32, 1.0f32),
        (-5.0, 0.0, 1.0),
        (0.5, 0.0, 1.0),
        (f32::NAN, 0.25, 1.0),
    ] {
        let cell = run_f32("clamp_f32", "ClampF32", &[("x", x), ("lo", lo), ("hi", hi)]);
        let want = x.max(lo).min(hi);
        assert_eq!(get(&cell, "out").to_bits(), want.to_bits(), "clamp({x})");
    }
    // verlet: x' = x + v*dt + 0.5*a*dt^2, v' = v + a*dt — the parenthesization
    // the cell declares, reproduced exactly host-side
    for (x, v, a, dt) in [
        (0.0f32, 5.0f32, -9.81f32, 1.0f32 / 60.0f32),
        (100.0, -3.5, 2.25, 0.016),
    ] {
        let cell = run_f32(
            "verlet_step_f32",
            "VerletStep",
            &[("x", x), ("v", v), ("a", a), ("dt", dt)],
        );
        let adt = a * dt;
        let want_x = x + v * dt + 0.5f32 * (adt * dt);
        let want_v = v + adt;
        assert_eq!(get(&cell, "x_out").to_bits(), want_x.to_bits());
        assert_eq!(get(&cell, "v_out").to_bits(), want_v.to_bits());
    }
    // spring-damper: a = -(k*x + c*v)*inv_m; v' = v + a*dt; x' = x + v'*dt
    let (x, v, k, c, inv_m, dt) = (0.1f32, 0.0f32, 50.0f32, 0.8f32, 1.0f32 / 2.5f32, 0.016f32);
    let cell = run_f32(
        "spring_damper_step_f32",
        "SpringDamperStep",
        &[
            ("x", x),
            ("v", v),
            ("k", k),
            ("c", c),
            ("inv_m", inv_m),
            ("dt", dt),
        ],
    );
    let a = -(k * x + c * v) * inv_m;
    let v1 = v + a * dt;
    let x1 = x + v1 * dt;
    assert_eq!(get(&cell, "x_out").to_bits(), x1.to_bits());
    assert_eq!(get(&cell, "v_out").to_bits(), v1.to_bits());
}

/// The boundary story: overflow is `float_overflow`, NaN is `float_domain` —
/// typed escalations, never silent non-finite answers.
#[test]
fn physics_pack_escalates_non_finite() {
    assert_eq!(
        halt_of(
            "kinetic_energy_f32",
            "KineticEnergy",
            &[("m", 3.0e38), ("v", 3.0e38)]
        ),
        Halt::Escalate(0xFF07),
        "overflowing energy must escalate"
    );
    assert_eq!(
        halt_of(
            "drag_force_f32",
            "DragForce",
            &[("k", f32::NAN), ("v", 1.0)]
        ),
        Halt::Escalate(0xFF08),
        "NaN input must surface as float_domain"
    );
    assert_eq!(
        halt_of(
            "verlet_step_f32",
            "VerletStep",
            &[("x", 1.0), ("v", 3.0e38), ("a", 0.0), ("dt", 3.0e38)],
        ),
        Halt::Escalate(0xFF07)
    );
}

/// The banked pair (`//! kernel_bank: on` — their images call into the resident
/// bank at `BANK_ORG` and carry only their own logic: 337 B and 650 B, down from
/// 8,197 B and 8,570 B inline). Same oracle discipline: bit-identical to host
/// rustc f32, typed escalation on non-finite results.
#[test]
fn banked_physics_cells_match_rustc_bit_for_bit() {
    // impulse: j = -(1+e)*(v1 - v2) / (inv_m1 + inv_m2), inverse masses in
    for (e, v1, v2, im1, im2) in [
        (0.8f32, 3.0f32, -1.0f32, 0.5f32, 1.0f32),
        (0.0, 10.0, 0.0, 0.1, 0.0), // second body static (inv_m = 0)
        (1.0, -2.5, 2.5, 2.0, 2.0),
    ] {
        let cell = run_f32(
            "impulse_1d_f32",
            "Impulse1d",
            &[
                ("e", e),
                ("v1", v1),
                ("v2", v2),
                ("inv_m1", im1),
                ("inv_m2", im2),
            ],
        );
        let want = -((1.0f32 + e) * (v1 - v2)) / (im1 + im2);
        assert_eq!(
            get(&cell, "j").to_bits(),
            want.to_bits(),
            "impulse({e},{v1},{v2})"
        );
    }
    // elastic: the cell's exact parenthesization, reproduced host-side
    for (m1, v1, m2, v2) in [
        (2.0f32, 3.0f32, 1.0f32, -1.5f32),
        (1.0, 5.0, 1.0, 0.0), // equal masses swap velocities
        (0.145, 40.0, 5.4, 0.0),
    ] {
        let cell = run_f32(
            "elastic_collision_1d_f32",
            "ElasticCollision1d",
            &[("m1", m1), ("v1", v1), ("m2", m2), ("v2", v2)],
        );
        let msum = m1 + m2;
        let d = m1 - m2;
        let w1 = (d * v1 + (2.0f32 * m2) * v2) / msum;
        let w2 = ((2.0f32 * m1) * v1 - d * v2) / msum;
        assert_eq!(get(&cell, "v1_out").to_bits(), w1.to_bits());
        assert_eq!(get(&cell, "v2_out").to_bits(), w2.to_bits());
    }
}

/// The banked boundary story: two static bodies (both inverse masses zero) make
/// the impulse denominator zero — ±Inf — and the cell escalates float_overflow
/// exactly as its doc line promises; zero total mass NaNs the elastic split.
#[test]
fn banked_physics_cells_escalate_non_finite() {
    assert_eq!(
        halt_of(
            "impulse_1d_f32",
            "Impulse1d",
            &[
                ("e", 0.5),
                ("v1", 1.0),
                ("v2", -1.0),
                ("inv_m1", 0.0),
                ("inv_m2", 0.0)
            ],
        ),
        Halt::Escalate(0xFF07),
        "static-static contact must escalate, not answer Inf"
    );
    assert_eq!(
        halt_of(
            "elastic_collision_1d_f32",
            "ElasticCollision1d",
            &[("m1", 0.0), ("v1", 1.0), ("m2", 0.0), ("v2", -1.0)],
        ),
        Halt::Escalate(0xFF08),
        "zero total mass is 0/0 — float_domain"
    );
}
