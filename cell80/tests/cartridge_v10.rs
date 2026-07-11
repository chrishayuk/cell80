//! The `.cell` v10 cell-family identity (docs 13 §2.6 / WS-E1): the target id
//! names the machine body, the family hash names the *cell* — SHA-256 over the
//! canonical source, shared by sibling-target bodies, invariant under everything
//! that isn't the source (id, summary, tags).

use cell80::{Cartridge, CartridgeOpts, CellConfig, Z80_CELL_TARGET};

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
fn a_foreign_machine_body_is_refused_up_front() {
    let mut cart = compile(SRC, "fam.rv");
    cart.manifest.target = "rv32im-hazard3-rp2350-sram".into();
    let bytes = cart.to_bytes();
    let Err(err) = Cartridge::from_bytes(&bytes) else {
        panic!("a foreign body must be refused");
    };
    assert!(
        err.contains("rv32im-hazard3-rp2350-sram") && err.contains(Z80_CELL_TARGET),
        "refusal must name both bodies: {err}"
    );
}
