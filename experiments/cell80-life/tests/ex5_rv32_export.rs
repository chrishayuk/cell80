//! EX-5's mechanism proof (`experiments/deterministic-ecology.md`): a real, disk-loaded
//! gene cell — not a hand-picked inline literal — compiled to both machine bodies and
//! hash-attested, exactly `cell80/tests/cartridge_v10.rs`'s `one_cell_two_bodies_one_family`
//! pattern. Pinned here independent of a full EX-3 run (the report binary,
//! `ex5_soma_export_report`, does that part) — this test is fast and fully deterministic.
use std::fs;
use std::path::{Path, PathBuf};

use cell80::{
    Cartridge, CartridgeOpts, CellConfig, Runner, Rv32Runner, RV32_TARGET, Z80_CELL_TARGET,
};

fn cells_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
}

fn load_source(name: &str) -> String {
    let path = cell80::find_cell_file(&cells_dir(), name).unwrap_or_else(|e| panic!("{e}"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn opts(id: &str) -> CartridgeOpts {
    CartridgeOpts {
        id: Some(id.to_string()),
        ..Default::default()
    }
}

/// One of EX-3's fixed, curated gene cells (`hungry_promoter`/`predator`'s attack-promoter
/// role) — a real cell from the ecology's own genome, not a synthetic example.
#[test]
fn is_gt_hash_attests_and_agrees_across_bodies() {
    let src = load_source("is_gt");

    let z80 = Cartridge::compile(&src, CellConfig::sandboxed(), opts("ex5.is_gt")).unwrap();
    let rv32 = Cartridge::compile_rv32(&src, CellConfig::sandboxed(), opts("ex5.is_gt")).unwrap();

    assert_eq!(z80.manifest.target, Z80_CELL_TARGET);
    assert_eq!(rv32.manifest.target, RV32_TARGET);
    assert_eq!(
        z80.manifest.family_hash, rv32.manifest.family_hash,
        "one source, one family"
    );
    assert_ne!(
        z80.artifact_hash(),
        rv32.artifact_hash(),
        "distinct per-body artifacts"
    );

    let back = Cartridge::from_bytes(&rv32.to_bytes()).unwrap();
    assert_eq!(back.manifest.target, RV32_TARGET);
    assert_eq!(back.manifest.family_hash, rv32.manifest.family_hash);

    let mut zr = Runner::new(z80.z80().unwrap());
    let rr = Rv32Runner::load(&back).unwrap();

    for &(a, b, want) in &[
        (7u16, 3u16, 1u16),
        (3, 7, 0),
        (5, 5, 0),
        (0, 0, 0),
        (65535, 0, 1),
    ] {
        let z_rep = zr.run(None, &[a, b], 100_000).unwrap();
        let r_rep = rr.run(&[a as u32, b as u32], &[], 100_000).unwrap();
        assert_eq!(z_rep.result, want, "z80 is_gt({a},{b})");
        assert_eq!(r_rep.result, want as u32, "rv32 is_gt({a},{b})");
        assert_eq!(
            z_rep.result as u32, r_rep.result,
            "z80/rv32 disagree on is_gt({a},{b})"
        );
    }
}

/// A 3-arg gene cell (EX-3's `sense_move` role) — confirms the arity-3 shape (used by
/// `sense_move`) round-trips through the same pipeline as the arity-2 promoters.
#[test]
fn argmax3_hash_attests_and_agrees_across_bodies() {
    let src = load_source("argmax3");

    let z80 = Cartridge::compile(&src, CellConfig::sandboxed(), opts("ex5.argmax3")).unwrap();
    let rv32 = Cartridge::compile_rv32(&src, CellConfig::sandboxed(), opts("ex5.argmax3")).unwrap();
    assert_eq!(z80.manifest.family_hash, rv32.manifest.family_hash);
    assert_ne!(z80.artifact_hash(), rv32.artifact_hash());

    let mut zr = Runner::new(z80.z80().unwrap());
    let rr = Rv32Runner::load(&rv32).unwrap();

    for &(a, b, c, want) in &[
        (3u16, 7u16, 12u16, 2u16),
        (10, 3, 7, 0),
        (5, 9, 5, 1),
        (0, 0, 0, 0),
    ] {
        let z_rep = zr.run(None, &[a, b, c], 100_000).unwrap();
        let r_rep = rr
            .run(&[a as u32, b as u32, c as u32], &[], 100_000)
            .unwrap();
        assert_eq!(z_rep.result, want, "z80 argmax3({a},{b},{c})");
        assert_eq!(
            z_rep.result as u32, r_rep.result,
            "z80/rv32 disagree on argmax3({a},{b},{c})"
        );
    }
}
