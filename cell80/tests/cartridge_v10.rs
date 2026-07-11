//! The `.cell` v10 cell-family identity (docs 13 §2.6 / WS-E1): the target id
//! names the machine body, the family hash names the *cell* — SHA-256 over the
//! canonical source, shared by sibling-target bodies, invariant under everything
//! that isn't the source (id, summary, tags).

use cell80::{
    Cartridge, CartridgeOpts, CellConfig, Runner, Rv32Runner, RV32_TARGET, Z80_CELL_TARGET,
};

fn compile(src: &str, id: &str) -> Cartridge {
    Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(id.into()),
            ..Default::default()
        },
    )
    .unwrap()
}

const SRC: &str = "fn run(a: u16, b: u16) -> u16 { a * b + 7 }";

#[test]
fn v10_roundtrips_target_and_family_hash() {
    let cart = compile(SRC, "fam.v1");
    assert_eq!(cart.manifest.target, Z80_CELL_TARGET);
    let family = cart
        .manifest
        .family_hash
        .expect("compile stamps the family");
    let bytes = cart.to_bytes();
    // from_bytes verifies the artifact hash by default — the v10 fields are
    // inside the covered prefix, so the roundtrip is also an identity check.
    let back = Cartridge::from_bytes(&bytes).unwrap();
    assert_eq!(back.manifest.target, Z80_CELL_TARGET);
    assert_eq!(back.manifest.family_hash, Some(family));
}

#[test]
fn family_hash_is_the_source_not_the_manifest() {
    // Same source under different ids/summaries: one family.
    let a = compile(SRC, "fam.a");
    let mut opts = CartridgeOpts {
        id: Some("fam.b".into()),
        ..Default::default()
    };
    opts.summary = "an entirely different description".into();
    opts.tags = vec!["other".into()];
    let b = Cartridge::compile(SRC, CellConfig::sandboxed(), opts).unwrap();
    assert_eq!(a.manifest.family_hash, b.manifest.family_hash);
    // Different ids ⇒ different artifact hashes — the per-target body identity
    // still covers the whole manifest.
    assert_ne!(a.artifact_hash(), b.artifact_hash());
    // A different source is a different family.
    let c = compile("fn run(a: u16, b: u16) -> u16 { a * b + 8 }", "fam.c");
    assert_ne!(a.manifest.family_hash, c.manifest.family_hash);
}

#[test]
fn an_unknown_machine_body_is_refused_up_front() {
    let mut cart = compile(SRC, "fam.rv");
    cart.manifest.target = "avr8-mega-nonesuch".into();
    let bytes = cart.to_bytes();
    let Err(err) = Cartridge::from_bytes(&bytes) else {
        panic!("an unknown body must be refused");
    };
    assert!(
        err.contains("avr8-mega-nonesuch") && err.contains(Z80_CELL_TARGET),
        "refusal must name the body and the hosts: {err}"
    );
}

#[test]
fn one_cell_two_bodies_one_family() {
    // WS-E3: sibling cartridges from one source — the family hash ties them,
    // each body keeps its own artifact hash, and each runner refuses the other.
    let z80 = compile(SRC, "fam.pair");
    let rv32 = Cartridge::compile_rv32(
        SRC,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("fam.pair".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(z80.manifest.target, Z80_CELL_TARGET);
    assert_eq!(rv32.manifest.target, RV32_TARGET);
    assert_eq!(z80.manifest.family_hash, rv32.manifest.family_hash);
    assert_ne!(z80.artifact_hash(), rv32.artifact_hash());

    // The rv32 body round-trips through the verifying loader.
    let back = Cartridge::from_bytes(&rv32.to_bytes()).unwrap();
    assert_eq!(back.manifest.target, RV32_TARGET);
    assert_eq!(back.manifest.family_hash, rv32.manifest.family_hash);

    // Both bodies run — and agree. (a=7, b=6: 7*6+7 = 49.)
    let mut zr = Runner::new(z80.z80().unwrap());
    let z_rep = zr.run(None, &[7, 6], 100_000).unwrap();
    let rr = Rv32Runner::load(&back).unwrap();
    let r_rep = rr.run(&[7, 6], &[], 100_000).unwrap();
    assert_eq!(z_rep.result as u32, r_rep.result);
    assert_eq!(r_rep.result, 49);

    // Cross-runner refusals name both sides.
    let Err(e) = Rv32Runner::load(&z80) else {
        panic!("Rv32Runner must refuse a z80 body");
    };
    assert!(
        e.contains(Z80_CELL_TARGET) && e.contains(RV32_TARGET),
        "{e}"
    );
    let Err(e) = back.z80() else {
        panic!("the z80 boundary must refuse an rv32 body");
    };
    assert!(e.contains(RV32_TARGET) || e.contains("rv32"), "{e}");
}

#[test]
fn rv32_body_uses_the_shared_kernel_prelude() {
    // A cell calling a prelude kernel compiles for both bodies — the same
    // prelude source rides both pipelines.
    let src = "fn run(a: u16, b: u16) -> u16 { gcd(a, b) }";
    let z80 = compile(src, "fam.gcd");
    let rv32 = Cartridge::compile_rv32(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("fam.gcd".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(z80.manifest.family_hash, rv32.manifest.family_hash);
    let rr = Rv32Runner::load(&rv32).unwrap();
    assert_eq!(rr.run(&[270, 192], &[], 1_000_000).unwrap().result, 6);
}
