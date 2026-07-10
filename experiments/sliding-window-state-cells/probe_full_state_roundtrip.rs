//! Not wired into the cargo workspace (see `../sliding-window-state-cells-findings.md` for
//! why) — drop this into `cell80/tests/` next to a copy of `simple_moving_average.rs` under
//! `cell80/cells/running-stats/` and run with `cargo test -p cell80 --test <this file>` to
//! reproduce. Bypasses `StateCell` (which only round-trips scalar fields) and manually
//! re-feeds the *whole* state — the `window` array included — through `Runner`'s raw
//! `(addr, Ty, value)` input triples, to verify the cell's own window/head/count/sum logic
//! and the compiler's array-field layout independently of the (missing) named array
//! round-trip surface.
use cell80::{Runner, Ty, DEFAULT_CYCLES};

const STATE_BASE: u16 = 0xB000;
const VALUE: u16 = STATE_BASE;
const WINDOW0: u16 = STATE_BASE + 2;
const HEAD: u16 = STATE_BASE + 18;
const COUNT: u16 = STATE_BASE + 20;
const SUM: u16 = STATE_BASE + 22;
const AVG: u16 = STATE_BASE + 26;

#[test]
fn probe_sma_full_state_roundtrip() {
    let src = std::fs::read_to_string("cells/running-stats/simple_moving_average.rs").unwrap();
    let mut r = Runner::compile(&src).unwrap();

    let mut window = [0u16; 8];
    let mut head: u16 = 0;
    let mut count: u16 = 0;
    let mut sum: u32 = 0;

    let mut step = |r: &mut Runner, value: u16| -> u16 {
        let mut inputs: Vec<(u16, Ty, u64)> = vec![
            (VALUE, Ty::U16, value as u64),
            (HEAD, Ty::U16, head as u64),
            (COUNT, Ty::U16, count as u64),
            (SUM, Ty::U32, sum as u64),
        ];
        for (i, w) in window.iter().enumerate() {
            inputs.push((WINDOW0 + (i as u16) * 2, Ty::U16, *w as u64));
        }
        let report = r
            .run_with_inputs(
                Some("SimpleMovingAverage::run"),
                &[STATE_BASE],
                &inputs,
                DEFAULT_CYCLES,
            )
            .unwrap();
        // Read every field back so the next call's inputs reflect this run's writes —
        // the whole-state round trip a real named-array surface would do automatically.
        head = r.peek_u16(HEAD);
        count = r.peek_u16(COUNT);
        sum = r.peek_u32(SUM);
        for (i, w) in window.iter_mut().enumerate() {
            *w = r.peek_u16(WINDOW0 + (i as u16) * 2);
        }
        let avg = r.peek_u16(AVG);
        assert_eq!(report.result, avg, "HL result should equal the avg field");
        avg
    };

    // Filling the window: avg is over however many samples have arrived so far.
    // Past the fill point: the oldest sample leaves the window as each new one arrives.
    let expect = [
        (10, 10), (20, 15), (30, 20), (40, 25),
        (50, 30), (60, 35), (70, 40), (80, 45),
        (90, 55), (100, 65),
    ];
    for (value, want) in expect {
        assert_eq!(step(&mut r, value), want, "value={value}");
    }
}
