//! `CellGraph` — static, host-routed composition: a 3-node graph end-to-end, plus the
//! validation failures that must be caught *before* anything runs.

use std::collections::HashMap;

use cell80::{
    Cartridge, CartridgeOpts, CellConfig, CellGraph, CellHost, Feed, Port, DEFAULT_CYCLES,
};

/// A host loaded with the cells the graph wires together.
fn host() -> CellHost {
    let cell = |id: &str, entry: Option<&str>, src: &str| {
        Cartridge::compile(
            src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.into()),
                entry: entry.map(String::from),
                ..Default::default()
            },
        )
        .unwrap()
    };
    let mut h = CellHost::new();
    // The real seed cells (so they're guaranteed to compile): a state cell (manhattan,
    // x1,y1,x2,y2 -> dist) feeding two value cells.
    h.add(cell(
        "manhattan",
        Some("Pts::run"),
        include_str!("../cells/manhattan.rs"),
    ));
    h.add(cell(
        "weighted_sum",
        None,
        include_str!("../cells/weighted_sum.rs"),
    ));
    h.add(cell("clamp", None, include_str!("../cells/clamp.rs")));
    h
}

/// The move-ranker slice: manhattan -> weighted_sum -> clamp, host-routed.
fn move_ranker() -> CellGraph {
    CellGraph {
        id: "move_ranker.v1".into(),
        nodes: vec![
            ("dist".into(), "manhattan".into()),
            ("score".into(), "weighted_sum".into()),
            ("bounded".into(), "clamp".into()),
        ],
        wires: vec![
            // external grid points into the state cell
            (Port::new("dist", "x1"), Feed::Input("x1".into())),
            (Port::new("dist", "y1"), Feed::Input("y1".into())),
            (Port::new("dist", "x2"), Feed::Input("x2".into())),
            (Port::new("dist", "y2"), Feed::Input("y2".into())),
            // state-cell output -> value-cell input (the inter-cell edge)
            (
                Port::new("score", "a"),
                Feed::From(Port::new("dist", "dist")),
            ),
            (Port::new("score", "b"), Feed::Input("risk".into())),
            (Port::new("score", "c"), Feed::Input("cost".into())),
            // value-cell output -> value-cell input, with constant bounds
            (
                Port::new("bounded", "x"),
                Feed::From(Port::new("score", "result")),
            ),
            (Port::new("bounded", "lo"), Feed::Const(0)),
            (Port::new("bounded", "hi"), Feed::Const(10)),
        ],
        outputs: vec![("ranked".into(), Port::new("bounded", "result"))],
    }
}

#[test]
fn graph_runs_three_nodes_host_routed() {
    let mut h = host();
    let g = move_ranker();
    g.validate(&h).expect("graph should type-check");

    let inputs = HashMap::from([
        ("x1".into(), 3),
        ("y1".into(), 4),
        ("x2".into(), 10),
        ("y2".into(), 8),
        ("risk".into(), 2),
        ("cost".into(), 1),
    ]);
    let run = g.run(&mut h, &inputs, DEFAULT_CYCLES).unwrap();

    // dist = |3-10|+|4-8| = 11 ; score = 11 + 2*2 + 1*3 = 18 ; clamp(18, 0, 10) = 10.
    assert_eq!(run.outputs, vec![("ranked".into(), 10)]);
    assert_eq!(run.trace.len(), 3);
    assert_eq!(run.trace[0].node, "dist"); // topological order
    assert_eq!(run.trace[0].result, 11);
    assert_eq!(run.trace[1].result, 18);
    assert_eq!(run.trace[2].result, 10);
    assert!(run.cycles > 0);
    // The runners were returned to the pool (nothing left loaded).
    assert_eq!(h.live_count(), 0);
}

#[test]
fn graph_loads_from_json_and_runs() {
    let mut h = host();
    let json = r#"{
      "id": "move_ranker.v1",
      "nodes": { "dist": "manhattan", "score": "weighted_sum", "bounded": "clamp" },
      "wires": [
        {"to":"dist.x1","input":"x1"}, {"to":"dist.y1","input":"y1"},
        {"to":"dist.x2","input":"x2"}, {"to":"dist.y2","input":"y2"},
        {"to":"score.a","from":"dist.dist"},
        {"to":"score.b","input":"risk"}, {"to":"score.c","input":"cost"},
        {"to":"bounded.x","from":"score.result"},
        {"to":"bounded.lo","const":0}, {"to":"bounded.hi","const":10}
      ],
      "outputs": { "ranked": "bounded.result" }
    }"#;
    let g = CellGraph::from_json(json).unwrap();
    let inputs = HashMap::from([
        ("x1".into(), 3),
        ("y1".into(), 4),
        ("x2".into(), 10),
        ("y2".into(), 8),
        ("risk".into(), 2),
        ("cost".into(), 1),
    ]);
    let run = g.run(&mut h, &inputs, DEFAULT_CYCLES).unwrap();
    assert_eq!(run.outputs, vec![("ranked".into(), 10)]);
    assert!(run.to_json().contains("\"ranked\":10"));
    assert!(run.to_human().contains("outputs: {ranked=10}"));

    // Malformed JSON and a malformed port both error cleanly.
    assert!(CellGraph::from_json("not json").is_err());
    assert!(
        CellGraph::from_json(r#"{"nodes":{"a":"clamp"},"wires":[{"to":"noportsep"}]}"#).is_err()
    );
}

#[test]
fn validate_rejects_a_type_mismatch_before_running() {
    let mut h = host();
    // A cell with a u8 input — wiring a u16 source into it must be rejected.
    h.add(
        Cartridge::compile(
            "fn run(a: u8) -> u16 { a as u16 }",
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some("narrow".into()),
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let g = CellGraph {
        id: "bad.types".into(),
        nodes: vec![
            ("s".into(), "weighted_sum".into()),
            ("n".into(), "narrow".into()),
        ],
        wires: vec![
            (Port::new("s", "a"), Feed::Const(1)),
            (Port::new("s", "b"), Feed::Const(1)),
            (Port::new("s", "c"), Feed::Const(1)),
            // u16 result -> u8 port: mismatch.
            (Port::new("n", "a"), Feed::From(Port::new("s", "result"))),
        ],
        outputs: vec![("o".into(), Port::new("n", "result"))],
    };
    let err = g.validate(&h).unwrap_err();
    assert!(err.contains("type mismatch"), "got: {err}");
    // run() validates too — it must refuse rather than execute.
    assert!(g.run(&mut h, &HashMap::new(), DEFAULT_CYCLES).is_err());
}

#[test]
fn validate_catches_structural_errors() {
    let h = host();
    let base = |wires, nodes| CellGraph {
        id: "t".into(),
        nodes,
        wires,
        outputs: vec![],
    };

    // Unwired value-cell param.
    let g = base(
        vec![
            (Port::new("s", "a"), Feed::Const(1)),
            (Port::new("s", "b"), Feed::Const(1)),
        ],
        vec![("s".into(), "weighted_sum".into())],
    );
    assert!(g.validate(&h).unwrap_err().contains("not wired"));

    // Unknown input port.
    let g = base(
        vec![(Port::new("s", "z"), Feed::Const(1))],
        vec![("s".into(), "weighted_sum".into())],
    );
    assert!(g.validate(&h).unwrap_err().contains("no input port"));

    // Unknown cell.
    let g = base(vec![], vec![("x".into(), "ghost".into())]);
    assert!(g.validate(&h).unwrap_err().contains("no cell"));

    // A cycle: a.x <- b.result and b.x <- a.result (both clamp, all u16).
    let cyc = CellGraph {
        id: "cyc".into(),
        nodes: vec![("a".into(), "clamp".into()), ("b".into(), "clamp".into())],
        wires: vec![
            (Port::new("a", "x"), Feed::From(Port::new("b", "result"))),
            (Port::new("a", "lo"), Feed::Const(0)),
            (Port::new("a", "hi"), Feed::Const(9)),
            (Port::new("b", "x"), Feed::From(Port::new("a", "result"))),
            (Port::new("b", "lo"), Feed::Const(0)),
            (Port::new("b", "hi"), Feed::Const(9)),
        ],
        outputs: vec![],
    };
    assert!(cyc.validate(&h).unwrap_err().contains("cycle"));
}
