//! Host-oracle tests for the raw-arith pack (`cell80/cells/raw-arith/*.rs`) — the raw
//! wrapping/unchecked arithmetic primitives (no prior pack had a plain, always-wrapping
//! two-argument add/sub, or a general-purpose variable-amount shift): see
//! `cell80/tests/library/common.rs` for the shared `cell_src`/`run_cell` helpers.

use crate::common::run_cell;

#[test]
fn raw_arith_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("add", &[100, 50], 150),
        ("add", &[65535, 1], 0), // wraps, unlike add_sat's clamp to 65535
        ("add", &[60000, 6000], 464), // 66000 mod 65536
        ("sub", &[100, 30], 70),
        ("sub", &[30, 100], 65466), // wraps, unlike sub_sat's floor at 0
        ("sub", &[0, 1], 65535),
        ("shl", &[1, 0], 1),
        ("shl", &[1, 3], 8),
        ("shl", &[1, 15], 32768),
        ("shl", &[1, 16], 0), // shift >= 16 saturates a u16 to 0
        ("shl", &[0xFFFF, 4], 0xFFF0),
        ("shr", &[0xFFFF, 0], 0xFFFF),
        ("shr", &[0xFFFF, 15], 1),
        ("shr", &[0xFFFF, 16], 0), // shift >= 16 saturates a u16 to 0
        ("shr", &[0x8000, 15], 1), // logical, not arithmetic: no sign extension
    ];
    for (id, args, exp) in cases {
        assert_eq!(run_cell(id, args), *exp, "{id}{args:?}");
    }
}
