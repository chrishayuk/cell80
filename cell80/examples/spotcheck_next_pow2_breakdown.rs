//! Scratch follow-up to `spotcheck_next_pow2_z80.rs`: isolate which piece of the
//! discovered composition actually drives its ~1,313 T-state fixed cost. Confirms
//! or refutes the hypothesis that `snap_up`'s div/mul (software routines on a chip
//! with no hardware MUL/DIV) dominate — vs. `cell80::Runner`'s known flat 4-T-state
//! host trap for mul/div (`cell80/src/runner.rs:127`, the same mechanism
//! `cost-discovery` found underpriced by ~36x), which would make the div/mul a
//! minor contributor and the real cost live elsewhere (the shift-heavy
//! smear-then-subtract in `highest_set_bit`, or general call/kernel-bank overhead).

use cell80::{CartridgeOpts, CellConfig, Halt, Runner};

fn compile(id: &str, src: &str) -> cell80::Cartridge {
    cell80::Cartridge::compile(
        src,
        CellConfig::permissive(),
        CartridgeOpts {
            id: Some(id.into()),
            kernel_bank: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("{id} failed to compile: {e}"))
}

fn cost(cart: &cell80::Cartridge, args: &[u16]) -> (u16, u64, u64) {
    let mut r = Runner::new(cart.z80().unwrap());
    let entry = cart.manifest.entry.clone();
    let f = r
        .run_fast(Some(&entry), args, cell80::DEFAULT_CYCLES)
        .unwrap();
    assert!(
        matches!(f.halt, Halt::Returned),
        "non-Returned: {:?}",
        f.halt
    );
    (f.result, f.cycles, f.trapped_ops)
}

// piece A: is_zero(x) then mask_xor(x, iz) — the edge-case patch only.
const PIECE_A: &str = "fn run(x: u16) -> u16 { let iz = (x == 0u16) as u16; x ^ iz }";

// piece B: highest_set_bit(x) alone — the smear-then-subtract.
const PIECE_B: &str = "fn run(x: u16) -> u16 { \
    let mut v = x; \
    v = v | (v >> 1u16); \
    v = v | (v >> 2u16); \
    v = v | (v >> 4u16); \
    v = v | (v >> 8u16); \
    v - (v >> 1u16) \
}";

// piece C: snap_up(mx, hsb) alone — the div/mul stage, two-arg so it can be fed
// representative (mx, hsb) pairs directly without re-deriving them.
const PIECE_C: &str = "fn run(mx: u16, hsb: u16) -> u16 { \
    if hsb != 0u16 && mx != 0u16 { ((mx - 1u16) / hsb + 1u16) * hsb } else { mx } \
}";

// the full composed win, for reference.
const FULL: &str = "fn run(x: u16) -> u16 { \
    let iz = (x == 0u16) as u16; \
    let mx = x ^ iz; \
    let mut v = x; \
    v = v | (v >> 1u16); \
    v = v | (v >> 2u16); \
    v = v | (v >> 4u16); \
    v = v | (v >> 8u16); \
    let hsb = v - (v >> 1u16); \
    if hsb != 0u16 && mx != 0u16 { ((mx - 1u16) / hsb + 1u16) * hsb } else { mx } \
}";

fn main() {
    let a = compile("piece_a", PIECE_A);
    let b = compile("piece_b", PIECE_B);
    let c = compile("piece_c", PIECE_C);
    let full = compile("full", FULL);

    // Representative points spanning the domain, matching the earlier full-domain
    // sweep's own probes (1, 255, 32769, 65535) plus 0.
    for &x in &[0u16, 1, 255, 32769, 65535] {
        let (_, ca, ta) = cost(&a, &[x]);
        let (hsb_result, cb, tb) = cost(&b, &[x]);
        // Feed piece C the actual (mx, hsb) this x would produce, matching FULL's
        // real internal data flow rather than an arbitrary pair.
        let iz = (x == 0u16) as u16;
        let mx = x ^ iz;
        let (_, cc, tc) = cost(&c, &[mx, hsb_result]);
        let (rf, cf, tf) = cost(&full, &[x]);
        println!(
            "x={x:>6}  A(edge-patch)={ca:>5} B(highest_set_bit)={cb:>5} C(snap_up)={cc:>5}  \
             sum={:>5}  FULL={cf:>5} (trapped: a={ta} b={tb} c={tc} full={tf})  full_result={rf}",
            ca + cb + cc
        );
    }
}
