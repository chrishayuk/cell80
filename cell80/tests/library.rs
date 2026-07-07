//! Host-oracle tests for the cell library (`cell80/cells/`), split by pack (2026-07-07) to
//! mirror the cells' own pack-directory structure — one file per pack under
//! `cell80/tests/library/`, sharing `cell_src`/`run_cell` from `library/common.rs`. This
//! file is a thin root: `cargo test --test library` still runs every pack's tests as one
//! binary, just backed by many files instead of one 3,300-line one.

#[path = "library/common.rs"]
mod common;

#[path = "library/agentic-runtime.rs"]
mod agentic_runtime;
#[path = "library/bit-encoding.rs"]
mod bit_encoding;
#[path = "library/bit-mask.rs"]
mod bit_mask;
#[path = "library/bounds.rs"]
mod bounds;
#[path = "library/bucket-convert.rs"]
mod bucket_convert;
#[path = "library/calendrical-checksum.rs"]
mod calendrical_checksum;
#[path = "library/checked-arithmetic.rs"]
mod checked_arithmetic;
#[path = "library/combinatorics.rs"]
mod combinatorics;
#[path = "library/distance.rs"]
mod distance;
#[path = "library/fixed-point.rs"]
mod fixed_point;
#[path = "library/fractions.rs"]
mod fractions;
#[path = "library/geometry.rs"]
mod geometry;
#[path = "library/hashing.rs"]
mod hashing;
#[path = "library/money-bps.rs"]
mod money_bps;
#[path = "library/number-theory.rs"]
mod number_theory;
#[path = "library/packing-bcd.rs"]
mod packing_bcd;
#[path = "library/percent.rs"]
mod percent;
#[path = "library/predicates.rs"]
mod predicates;
#[path = "library/ranking-stats.rs"]
mod ranking_stats;
#[path = "library/running-stats.rs"]
mod running_stats;
#[path = "library/safe-arith.rs"]
mod safe_arith;
#[path = "library/scoring-choice.rs"]
mod scoring_choice;
#[path = "library/sequences.rs"]
mod sequences;
#[path = "library/signed-deltas.rs"]
mod signed_deltas;
#[path = "library/softfloat.rs"]
mod softfloat;
#[path = "library/spatial-grid.rs"]
mod spatial_grid;
#[path = "library/stateful-rng.rs"]
mod stateful_rng;
#[path = "library/units.rs"]
mod units;
#[path = "library/vector.rs"]
mod vector;
#[path = "library/verifier-ranker.rs"]
mod verifier_ranker;
