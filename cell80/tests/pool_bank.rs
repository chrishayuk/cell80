//! Pool recycling × the resident kernel bank: `Runner::new` stamps the bank for a
//! banked program, but a **recycled** bus (`CellPool::acquire` → `reset_for`) may have
//! been born under a non-bank cell — before the fix, its 0xC000 stayed zeroed and a
//! banked cell recycled onto it ran away into `cycle_budget` instead of returning.
//! Found live: a warm host cycling ordinary cells through the pool, then loading a
//! Finance80 `kernel_bank: on` cell (the `gen-examples` order-dependence, 2026-07-11).

use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost, Halt, DEFAULT_CYCLES};

#[test]
fn banked_cell_survives_pool_recycling() {
    let mut host = CellHost::new();
    host.add(
        Cartridge::compile(
            "fn run(a: u16, b: u16) -> u16 { a + b }",
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some("plain".into()),
                summary: "plain adder".into(),
                ..Default::default()
            },
        )
        .unwrap(),
    );
    host.add(
        Cartridge::compile(
            "struct B { x: f32, out: f32 }
             impl B { fn run(&mut self) -> u16 { self.out = self.x + 1.5f32; 1u16 } }",
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some("banked".into()),
                entry: Some("B::run".into()),
                summary: "banked f32 add".into(),
                kernel_bank: true,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let x = vec![("x".to_string(), 2.25f32.to_bits() as u64)];
    let want_out = (2.25f32 + 1.5f32).to_bits() as u64;
    let out_of = |state: &[(String, u64)]| {
        state
            .iter()
            .find(|(n, _)| n == "out")
            .map(|(_, v)| *v)
            .unwrap()
    };

    // Baseline: a fresh runner (empty pool) carries the bank.
    let h = host.load("banked").unwrap();
    let (rep, state) = host.run_state(h, &x, DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.halt, Halt::Returned);
    assert_eq!(out_of(&state), want_out);
    host.unload(h).unwrap();

    // Season the pool with a NON-bank cell, so its bus was born without the bank…
    let h = host.load("plain").unwrap();
    assert_eq!(host.run_fast(h, &[3, 4], DEFAULT_CYCLES).unwrap().result, 7);
    host.unload(h).unwrap();

    // …then recycle that bus for the banked cell: it must still return, and agree
    // with the fresh-runner baseline bit for bit.
    let h = host.load("banked").unwrap();
    let (rep, state) = host.run_state(h, &x, DEFAULT_CYCLES).unwrap();
    assert_eq!(
        rep.halt,
        Halt::Returned,
        "bank must be stamped on reset_for"
    );
    assert_eq!(out_of(&state), want_out);
    host.unload(h).unwrap();
}
