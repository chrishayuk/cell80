//! M2 of the math campaign (docs/math-campaign-spec.md): the plan IR, the
//! renderer's determinism (the precipitation story's load-bearing detail), the
//! render-time unit algebra, the kill classes, and the counterfactual battery.

use cell80::plan::{Plan, Quantity};
use cell80::{CellHost, DEFAULT_CYCLES};

fn plan(json: &str) -> Plan {
    Plan::from_json(json).expect("plan parses")
}

/// The spec's lego example, §"The architecture".
const LEGO: &str = r#"{
    "quantities": [ {"id":"lego_sets","value":13,"unit":"count"},
                    {"id":"lego_price","value":1500,"unit":"cents_per_count"} ],
    "ops":        [ ["mul","lego_sets","lego_price","lego_money"] ],
    "target":     "lego_money" }"#;

#[test]
fn renderer_is_deterministic_and_canonical() {
    // Same plan, permuted quantity order → byte-identical source → identical
    // artifact hash. (Op order is semantic and stays; quantities canonicalize.)
    let a = plan(LEGO);
    let mut b = a.clone();
    b.quantities.reverse();
    let (sa, sb) = (a.render().unwrap(), b.render().unwrap());
    assert_eq!(sa, sb, "quantity order must not leak into the source");
    let hash = |src: &str| {
        cell80::Cartridge::compile(
            src,
            cell80::CellConfig::sandboxed(),
            cell80::CartridgeOpts {
                entry: Some("P::run".into()),
                ..Default::default()
            },
        )
        .unwrap()
        .artifact_hash()
    };
    assert_eq!(hash(&sa), hash(&sb));

    // And a different-schema plan renders differently (add vs mul).
    let c = plan(
        &LEGO
            .replace("mul", "add")
            .replace("cents_per_count", "count"),
    );
    assert_ne!(c.render().unwrap(), sa);
}

#[test]
fn solve_answers_the_lego_problem() {
    let mut host = CellHost::new();
    host.set_cache(true);
    let rep = host.solve(&[plan(LEGO)], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.answer, Some(13 * 1500));
    assert!(!rep.battery_ran);
    assert_eq!(rep.outcomes.len(), 1);
    assert!(rep.outcomes[0].kill.is_none());
    assert!(!rep.outcomes[0].retrieved, "first sighting compiles");

    // The same schema with different numbers is *retrieved*, not recompiled —
    // the H-M3 precipitation counter. (Same source modulo state values.)
    let mut again = plan(LEGO);
    again.quantities[0].value = 7;
    let rep = host.solve(&[again], DEFAULT_CYCLES).unwrap();
    assert_eq!(rep.answer, Some(7 * 1500));
    assert!(
        rep.outcomes[0].retrieved,
        "same schema ⇒ same hash ⇒ retrieved"
    );
}

#[test]
fn render_time_unit_algebra() {
    // Adding money to time dies before compilation; rate algebra flows.
    let bad = plan(
        r#"{"quantities":[{"id":"a","value":1,"unit":"cents"},{"id":"b","value":2,"unit":"hours"}],
            "ops":[["add","a","b","c"]], "target":"c"}"#,
    );
    let err = bad.render().unwrap_err();
    assert!(err.contains("unit mismatch"), "{err}");

    // cents_per_time × time → cents: the wage-rate flow the units pack exists for.
    let wage = plan(
        r#"{"quantities":[{"id":"rate","value":1200,"unit":"cents_per_time"},
                          {"id":"worked","value":5,"unit":"time"}],
            "ops":[["mul","rate","worked","pay"]], "target":"pay"}"#,
    );
    let mut host = CellHost::new();
    host.set_cache(true);
    assert_eq!(
        host.solve(&[wage], DEFAULT_CYCLES).unwrap().answer,
        Some(6000)
    );

    // Unknown units and unknown ops are render rejects (repair-row material).
    assert!(plan(
        r#"{"quantities":[{"id":"a","value":1,"unit":"furlongs"}],"ops":[],"target":"a"}"#
    )
    .render()
    .is_err());
    assert!(plan(
        r#"{"quantities":[{"id":"a","value":1,"unit":"count"}],"ops":[["pow","a","a","b"]],"target":"b"}"#
    )
    .render()
    .is_err());
}

#[test]
fn kill_classes() {
    // A negative intermediate, an overflow, a violated exact_div, and a division
    // by zero each kill the plan with the reason named — never a wrong answer.
    let mut host = CellHost::new();
    host.set_cache(true);
    let negative = plan(
        r#"{"quantities":[{"id":"have","value":3,"unit":"count"},{"id":"spend","value":5,"unit":"count"}],
            "ops":[["sub","have","spend","left"]], "target":"left"}"#,
    );
    let overflow = plan(
        r#"{"quantities":[{"id":"a","value":4000000000,"unit":"count"},{"id":"b","value":2,"unit":"scalar"}],
            "ops":[["mul","a","b","c"]], "target":"c"}"#,
    );
    let inexact = plan(
        r#"{"quantities":[{"id":"money","value":10,"unit":"cents"},{"id":"price","value":3,"unit":"cents"}],
            "ops":[["div","money","price","n"]], "target":"n",
            "constraints":[["exact_div","money","price"]]}"#,
    );
    let div0 = plan(
        r#"{"quantities":[{"id":"a","value":5,"unit":"count"},{"id":"z","value":0,"unit":"scalar"}],
            "ops":[["div","a","z","c"]], "target":"c"}"#,
    );
    let rep = host
        .solve(&[negative, overflow, inexact, div0], DEFAULT_CYCLES)
        .unwrap();
    assert_eq!(rep.answer, None, "every plan dies — escalate");
    let kills: Vec<&str> = rep
        .outcomes
        .iter()
        .map(|o| o.kill.as_deref().unwrap())
        .collect();
    assert!(kills[0].contains("needs_wider_math"), "{kills:?}");
    assert!(kills[1].contains("needs_wider_math"), "{kills:?}");
    assert!(kills[2].contains("out_of_domain"), "{kills:?}");
    assert!(kills[3].contains("div_by_zero"), "{kills:?}");
}

#[test]
fn counterfactual_battery_separates_coincidental_agreement() {
    // a=2, b=3: `a+b` (5) and `a*b` (6) disagree — the battery perturbs and the
    // majority of consistent movers wins. Two adders vs one multiplier: the
    // adders' group survives with 5.
    let q = |a: u32, b: u32| {
        vec![
            Quantity {
                id: "a".into(),
                value: a,
                unit: "count".into(),
            },
            Quantity {
                id: "b".into(),
                value: b,
                unit: "count".into(),
            },
        ]
    };
    let mk = |op: &str, extra_op: Option<&str>| {
        let mut ops = vec![cell80::plan::PlanOp {
            op: op.into(),
            a: "a".into(),
            b: "b".into(),
            out: "c".into(),
        }];
        let mut target = "c".to_string();
        if let Some(e) = extra_op {
            // A second route to the same value: c2 = c <e> zero-ish trick keeps
            // schemas distinct while agreeing numerically.
            ops.push(cell80::plan::PlanOp {
                op: e.into(),
                a: "c".into(),
                b: "b".into(),
                out: "d".into(),
            });
            ops.push(cell80::plan::PlanOp {
                op: "sub".into(),
                a: "d".into(),
                b: "b".into(),
                out: "e".into(),
            });
            target = "e".into();
        }
        cell80::plan::Plan {
            quantities: q(2, 3),
            ops,
            target,
            constraints: vec![],
        }
    };
    let adder = mk("add", None);
    let adder2 = mk("add", Some("add")); // (a+b)+b-b — same function, different schema
    let muler = mk("mul", None);
    let mut host = CellHost::new();
    host.set_cache(true);
    let rep = host.solve(&[adder, muler, adder2], DEFAULT_CYCLES).unwrap();
    assert!(rep.battery_ran, "disagreement forces the battery");
    assert_eq!(rep.answer, Some(5), "the adders' group wins 2 v 1");
}

#[test]
fn solve_leaves_facts_behind() {
    // The campaign's residue: after solving, the fact file holds the runs.
    let mut host = CellHost::new();
    host.set_cache(true);
    host.solve(&[plan(LEGO)], DEFAULT_CYCLES).unwrap();
    let mut buf = Vec::new();
    let n = host.export_facts(&mut buf, "m2@test").unwrap();
    assert!(n >= 1);
    let text = String::from_utf8(buf).unwrap();
    assert!(
        text.contains("\"f\":{\"lego_price\":1500,\"lego_sets\":13}"),
        "{text}"
    );
}

#[test]
fn plan_parse_rejections() {
    for (i, bad) in [
        "not json",
        r#"{"ops":[],"target":"x"}"#,                                    // no quantities
        r#"{"quantities":[],"target":"x"}"#,                             // no ops
        r#"{"quantities":[],"ops":[],"target":"x"}"#,                    // undefined target
        r#"{"quantities":[{"id":"x"}],"ops":[],"target":"x"}"#,          // no value
        r#"{"quantities":[{"id":"x","value":99999999999}],"ops":[],"target":"x"}"#, // > u32
        r#"{"quantities":[{"id":"x","value":1}],"ops":[["add","x"]],"target":"x"}"#, // short op
        r#"{"quantities":[{"id":"x","value":1}],"ops":[],"target":"x","constraints":[["mystery","x"]]}"#,
    ]
    .iter()
    .enumerate()
    {
        let r = Plan::from_json(bad).and_then(|p| p.render());
        assert!(r.is_err(), "row {i} should reject: {bad}");
    }
    // Bad identifiers and reassignment die at render.
    assert!(plan(
        r#"{"quantities":[{"id":"Self","value":1,"unit":"count"}],"ops":[],"target":"Self"}"#
    )
    .render()
    .is_err());
    assert!(plan(
        r#"{"quantities":[{"id":"a","value":1,"unit":"count"}],
            "ops":[["add","a","a","a"]],"target":"a"}"#
    )
    .render()
    .is_err());
}

#[test]
fn cli_solve_verb() {
    use cell80::run_cli;
    let dir = std::env::temp_dir().join(format!("cell80-solve-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let plans = dir.join("plans.json");
    std::fs::write(&plans, LEGO).unwrap();
    let out = run_cli(&["solve".into(), plans.to_str().unwrap().into()]).unwrap();
    assert!(out.contains("answer: 19500"), "{out}");
    let out = run_cli(&[
        "solve".into(),
        plans.to_str().unwrap().into(),
        "--json".into(),
    ])
    .unwrap();
    assert!(out.contains("\"answer\":19500"), "{out}");
    // A dead plan set names the kills and says escalate.
    std::fs::write(
        &plans,
        r#"[{"quantities":[{"id":"a","value":1,"unit":"count"},{"id":"b","value":2,"unit":"count"}],
             "ops":[["sub","a","b","c"]],"target":"c"}]"#,
    )
    .unwrap();
    let out = run_cli(&["solve".into(), plans.to_str().unwrap().into()]).unwrap();
    assert!(out.contains("escalate"), "{out}");
    assert!(out.contains("needs_wider_math"), "{out}");
    assert!(run_cli(&[
        "solve".into(),
        plans.to_str().unwrap().into(),
        "--bogus".into()
    ])
    .is_err());
    assert!(run_cli(&["solve".into(), "/nope.json".into()]).is_err());
    std::fs::remove_dir_all(&dir).ok();
}
