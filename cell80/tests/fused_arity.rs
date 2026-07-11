//! The fused search's arity tie-break (checkpoint 22): register probing zero-fills
//! missing args, so a 3-ary cell can be observationally identical to a 2-ary one on
//! every 2-ary example (`median3(a,b,0) ≡ min(a,b)`). That collision is an artifact
//! of the ABI projection — the manifest knows the arity — so among behavioural equals
//! the declared-arity match ranks first, ahead of the text tie-break. The zero-hit
//! tail is untouched: garbage examples still degrade to the plain text order.

use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost};

fn cell(id: &str, src: &str, summary: &str) -> Cartridge {
    Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(id.into()),
            summary: summary.into(),
            tags: vec!["pick".into(), "pair".into()],
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn arity_match_outranks_zero_fill_equivalents() {
    let mut host = CellHost::new();
    // mid3(a,b,0) == lo2(a,b) for all a,b — observationally identical under 2-ary
    // probes. Text surface deliberately favours the WRONG one (summary word order +
    // id order both put mid3 first on ties).
    host.add(cell(
        "a_mid3",
        "fn run(a: u16, b: u16, c: u16) -> u16 {
             let hi = if a > b { a } else { b };
             let hi2 = if hi > c { hi } else { c };
             let lo = if a < b { a } else { b };
             let lo2 = if lo < c { lo } else { c };
             a + b + c - hi2 - lo2
         }",
        "pick between values of a pair or triple",
    ));
    host.add(cell(
        "z_lo2",
        "fn run(a: u16, b: u16) -> u16 { if a < b { a } else { b } }",
        "pick between values of a pair or triple",
    ));

    // The query names the 3-ary cell, so text unambiguously favours the WRONG
    // candidate for 2-ary examples — the tie-break has to do the work.
    let q = "a_mid3 pick between values";
    // Both reproduce every 2-ary example (mid3 through zero-fill)…
    let routed: Vec<String> = host
        .route_by_examples(&[(vec![3, 7], 3), (vec![9, 4], 4)], 5)
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert!(routed.contains(&"a_mid3".to_string()) && routed.contains(&"z_lo2".to_string()));
    let plain: Vec<String> = host.search(q, 5).iter().map(|m| m.id.clone()).collect();
    assert_eq!(plain[0], "a_mid3");

    // The fused ranking dissolves the collision: 2-ary examples → the 2-ary cell.
    let fused = host
        .search_with_examples(q, &[(vec![3, 7], 3), (vec![9, 4], 4)], 5)
        .unwrap();
    assert_eq!(fused[0].id, "z_lo2", "declared arity must beat zero-fill");
    // 3-ary examples put the 3-ary cell first instead.
    let fused = host
        .search_with_examples(q, &[(vec![3, 7, 5], 5), (vec![9, 2, 4], 4)], 5)
        .unwrap();
    assert_eq!(fused[0].id, "a_mid3");

    // Zero-hit tail keeps pure text order: garbage examples degrade to plain search,
    // arity notwithstanding.
    let ids = |ms: Vec<&cell80::Manifest>| ms.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
    assert_eq!(
        ids(host
            .search_with_examples(q, &[(vec![2, 2], 9999)], 5)
            .unwrap()),
        plain
    );
}
