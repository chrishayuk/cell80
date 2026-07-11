//! The `.cell` v11 additions: array state-field types (wire code 6 — element
//! sub-code + element count, the named-array round-trip surface) and the optional
//! accuracy contract (`//! accuracy:`, F-wave §F2). Pre-v11 cartridges never
//! contain either, so back-compat reads hold — the v6 buffer-code posture.

use cell80::{ArrayElem, Cartridge, CartridgeOpts, CellConfig, Ty};

const SMA_SRC: &str = "
struct Sma { value: u16, window: [u16; 8], head: u16, count: u16, sum: u32, avg: u16 }
impl Sma {
    fn run(&mut self) -> u16 {
        let full = self.count == 8u16;
        let evict = if full { self.window[self.head as usize] as u32 } else { 0u32 };
        self.window[self.head as usize] = self.value;
        self.sum = self.sum - evict + (self.value as u32);
        if !full { self.count = self.count + 1u16; }
        self.head = (self.head + 1u16) % 8u16;
        self.avg = (self.sum / (self.count as u32)) as u16;
        self.avg
    }
}";

#[test]
fn v11_array_state_field_roundtrips() {
    let cart = Cartridge::compile(
        SMA_SRC,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("sma.v11".into()),
            entry: Some("Sma::run".into()),
            ..Default::default()
        },
    )
    .unwrap();
    // The manifest carries the array field typed — not dropped, not misread as
    // eight scalars.
    let window = cart
        .manifest
        .state_addrs
        .iter()
        .find(|(n, _, _)| n == "window")
        .expect("array field is name-addressed");
    assert_eq!(window.2, Ty::Array(ArrayElem::U16, 8));
    assert_eq!(cart.manifest.state_addrs.len(), 6); // every field addressable

    // Wire round-trip through the verifying loader (the array entry is inside
    // the hash-covered prefix, so this is also an identity check).
    let back = Cartridge::from_bytes(&cart.to_bytes()).unwrap();
    assert_eq!(back.manifest.state_addrs, cart.manifest.state_addrs);
    assert_eq!(back.artifact_hash(), cart.artifact_hash());
}

#[test]
fn v11_wide_array_field_is_distinct_from_word_pairs() {
    // [u32; 2] must round-trip as u32[2], never as u16[4] or a scalar u32 — the
    // element sub-code is load-bearing.
    let src = "struct W { xs: [u32; 2], out: u32 }
               impl W { fn run(&mut self) -> u16 { self.out = self.xs[0] + self.xs[1]; 0u16 } }";
    let cart = Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("wide.v11".into()),
            entry: Some("W::run".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let back = Cartridge::from_bytes(&cart.to_bytes()).unwrap();
    let xs = back
        .manifest
        .state_addrs
        .iter()
        .find(|(n, _, _)| n == "xs")
        .unwrap();
    assert_eq!(xs.2, Ty::Array(ArrayElem::U32, 2));
}

#[test]
fn v11_accuracy_contract_roundtrips() {
    let bound = "<= 4 ulp over [-87.34, 88.72]";
    let cart = Cartridge::compile(
        "fn run(a: u16) -> u16 { a }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("acc.v11".into()),
            accuracy: Some(bound.into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(cart.manifest.accuracy.as_deref(), Some(bound));
    let back = Cartridge::from_bytes(&cart.to_bytes()).unwrap();
    assert_eq!(back.manifest.accuracy.as_deref(), Some(bound));

    // The default is None — exact/correctly-rounded semantics, every pre-F2 cell.
    // Same id/source, no accuracy: the ONLY manifest difference is the contract.
    let plain = Cartridge::compile(
        "fn run(a: u16) -> u16 { a }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("acc.v11".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(plain.manifest.accuracy, None);
    // And the accuracy contract is hash-covered: declaring one is a different
    // artifact, never a silent relabel.
    assert_ne!(cart.artifact_hash(), plain.artifact_hash());
}
