"""Shared fixtures: a tiny synthetic cell library + retrieval dataset, small enough
that generator/eval tests run in milliseconds while covering every cell class the
real library has — value twins (behaviour separates, text can't), status-flag state
twins (only post-run fields separate), an f32 cell (non-scalar skip), and an
always-escalating cell (no clean runs)."""

import json

import pytest

CELL_SOURCES = {
    "pick_lo.rs": "//! pick between two numbers\n//! tags: compare, pair\n"
    "fn run(a: u16, b: u16) -> u16 { if a < b { a } else { b } }",
    "pick_hi.rs": "//! pick between two numbers\n//! tags: compare, pair\n"
    "fn run(a: u16, b: u16) -> u16 { if a < b { b } else { a } }",
    "f_add.rs": "//! combine two fields into out\n//! tags: state, combine\n"
    "//! entry: F::run\n"
    "struct F { a: u16, b: u16, out: u16 }\n"
    "impl F { fn run(&mut self) -> u16 { self.out = self.a + self.b; 1u16 } }",
    "f_sub.rs": "//! combine two fields into out, subtracting\n//! tags: state, combine\n"
    "//! entry: F::run\n"
    "struct F { a: u16, b: u16, out: u16 }\n"
    "impl F { fn run(&mut self) -> u16 { self.out = self.a - self.b; 1u16 } }",
    "fp_ident.rs": "//! float identity\n//! tags: float\n"
    "fn run(x: f32) -> f32 { x }",
    "always_out.rs": "//! always escalates\n//! tags: guard\n"
    "//! limits: escalates always\n"
    "fn run(a: u16) -> u16 { halt(0xFF06u16); 0u16 }",
}

DATASET_ROWS = [
    {"id": "lo-1", "query": "pick the smaller of two numbers", "expected": "pick_lo", "category": "direct"},
    {"id": "lo-2", "query": "the lesser of a pair", "expected": "pick_lo", "category": "paraphrase"},
    {"id": "hi-1", "query": "pick the bigger of two numbers", "expected": "pick_hi", "category": "direct"},
    {"id": "add-1", "query": "combine two fields into out", "expected": "f_add", "category": "direct"},
    {"id": "add-2", "query": "sum a pair of state fields", "expected": "f_add", "category": "paraphrase"},
    {"id": "fp-1", "query": "float identity", "expected": "fp_ident", "category": "direct"},
    {"id": "halt-1", "query": "always escalates", "expected": "always_out", "category": "direct"},
    {"id": "ghost-1", "query": "does not exist", "expected": "no_such_cell", "category": "direct"},
]


@pytest.fixture(scope="session")
def tiny_setup(tmp_path_factory):
    """(library_dir, dataset_path) for the synthetic library above."""
    root = tmp_path_factory.mktemp("tiny-lib")
    cells = root / "cells"
    cells.mkdir()
    for name, src in CELL_SOURCES.items():
        (cells / name).write_text(src)
    dataset = root / "tiny-retrieval.jsonl"
    dataset.write_text("\n".join(json.dumps(r) for r in DATASET_ROWS) + "\n")
    return str(cells), str(dataset)
