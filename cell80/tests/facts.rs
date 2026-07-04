//! The fact file's DoD (docs/12 §7): round-trip, tamper (result/cost/halt each
//! caught, line named), contradiction decided by execution, sampling
//! unpredictability, budget-halt rejection, and the config wall — every
//! outcome-affecting knob lives inside the hash.

use cell80::{
    Cartridge, CartridgeOpts, CellConfig, CellHost, CellProgram, DivByZero, Fact, ImportPolicy,
    Runner, DEFAULT_CYCLES,
};

fn mul_cart() -> Cartridge {
    Cartridge::compile(
        "fn run(a: u16, b: u16) -> u16 { a * b }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("mul.v1".into()),
            ..Default::default()
        },
    )
    .unwrap()
}

fn score_cart() -> Cartridge {
    Cartridge::compile(
        "struct Score { wx: u16, x: u16, total: u32 }
         impl Score {
             fn run(&mut self) -> u16 {
                 self.total = (self.wx as u32) * (self.x as u32);
                 (self.total >> 8) as u16
             }
         }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("score.v1".into()),
            entry: Some("Score::run".into()),
            ..Default::default()
        },
    )
    .unwrap()
}

/// Run a small workload on a caching host and export it: value facts + state facts.
fn export_workload() -> (Vec<u8>, u64) {
    let mut host = CellHost::new();
    host.set_cache(true);
    host.add(mul_cart());
    host.add(score_cart());
    let hm = host.load("mul.v1").unwrap();
    for (a, b) in [(3u16, 7u16), (200, 300), (0, 9), (7, 0)] {
        host.run_fast(hm, &[a, b], DEFAULT_CYCLES).unwrap();
    }
    let hs = host.load("score.v1").unwrap();
    for (wx, x) in [(3u64, 17u64), (500, 400), (1, 1)] {
        host.run_state_fast(hs, &[("wx".into(), wx), ("x".into(), x)], DEFAULT_CYCLES)
            .unwrap();
    }
    let mut buf = Vec::new();
    let n = host.export_facts(&mut buf, "test@dod").unwrap();
    (buf, n)
}

#[test]
fn fact_lines_round_trip() {
    // Every exported line parses back to an equal Fact, and the file is stable:
    // header + sorted canonical lines (so `sort -u` merges and re-exports agree).
    let (buf, n) = export_workload();
    assert_eq!(n, 7);
    let text = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("{\"facts\":1,\"lib\":\"cell80\","));
    assert_eq!(lines.len() as u64, n + 1);
    let mut sorted = lines[1..].to_vec();
    sorted.sort();
    assert_eq!(&lines[1..], &sorted[..], "fact lines are sorted");
    for l in &lines[1..] {
        let f = Fact::from_line(l).expect("parses");
        assert_eq!(&f.to_line(), l, "canonical re-emit is identical");
    }
}

#[test]
fn import_round_trip_serves_from_facts() {
    // Export from host A, import into host B: the workload re-runs as hits served
    // from imported facts — zero re-execution beyond the verification sample.
    let (buf, n) = export_workload();
    let mut b = CellHost::new();
    b.set_cache(true);
    b.add(mul_cart());
    b.add(score_cart());
    let rep = b
        .import_facts(
            &buf[..],
            &ImportPolicy {
                seed: Some(42),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(rep.read, n);
    assert_eq!(rep.accepted, n);
    assert!(rep.failures.is_empty() && !rep.file_failed);
    assert!(rep.verified >= 1, "min-1 sampling");

    let hm = b.load("mul.v1").unwrap();
    let f = b.run_fast(hm, &[200, 300], DEFAULT_CYCLES).unwrap();
    assert_eq!(f.result, 200u16.wrapping_mul(300));
    let hs = b.load("score.v1").unwrap();
    let (_, state) = b
        .run_state_fast(hs, &[("wx".into(), 500), ("x".into(), 400)], DEFAULT_CYCLES)
        .unwrap();
    assert_eq!(
        state.iter().find(|(k, _)| k == "total").unwrap().1,
        500 * 400
    );
    // Both hits came from imported facts (the provenance split).
    assert_eq!(b.cache_split(hm).unwrap(), Some((0, 1)));
    assert_eq!(b.cache_split(hs).unwrap(), Some((0, 1)));
}

#[test]
fn tamper_is_caught_result_cost_and_halt() {
    // Mutate one digit of the result, the cost, and the halt — each import fails
    // the file (FailFile) and names the line. Cost matters even when the result
    // matches: a fact that runs long is a lie.
    let (buf, _) = export_workload();
    let text = String::from_utf8(buf).unwrap();
    let mutate = |find: &str, replace: &str| -> String {
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        let i = lines
            .iter()
            .position(|l| l.contains("\"args\":[200,300]"))
            .expect("workload line present");
        lines[i] = lines[i].replacen(find, replace, 1);
        lines.join("\n")
    };
    let cases = [
        mutate("\"r\":[60000", "\"r\":[60001"), // result digit (200*300)
        mutate("\"cy\":", "\"cy\":9"),          // cost claim
        mutate("\"h\":\"ok\"", "\"h\":\"halt:7\""), // halt kind
    ];
    for (which, tampered) in cases.iter().enumerate() {
        let mut b = CellHost::new();
        b.set_cache(true);
        b.add(mul_cart());
        b.add(score_cart());
        let rep = b
            .import_facts(
                tampered.as_bytes(),
                &ImportPolicy {
                    verify_fraction: 1.0, // deterministic catch for the test
                    seed: Some(1),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(rep.file_failed, "case {which}: file must fail");
        assert_eq!(rep.failures.len(), 1, "case {which}");
        assert!(rep.failures[0].key.contains("run(200,300)"), "case {which}");
        assert!(rep.failures[0].line > 1, "case {which}: line named");
        // Nothing imported: a fresh load has no imported hits to serve.
        let hm = b.load("mul.v1").unwrap();
        b.run_fast(hm, &[3, 7], DEFAULT_CYCLES).unwrap();
        assert_eq!(b.cache_split(hm).unwrap(), Some((0, 0)));
    }
}

#[test]
fn contradiction_is_decided_by_execution() {
    // Two lines, same key, different outcome: the importer executes the key and
    // keeps the truth — the lie is reported, the truth imports (quarantine mode).
    let (buf, _) = export_workload();
    let text = String::from_utf8(buf).unwrap();
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let i = lines
        .iter()
        .position(|l| l.contains("\"args\":[3,7]"))
        .unwrap();
    let lie = lines[i].replace("\"r\":[21,", "\"r\":[22,");
    lines.push(lie);
    let file = lines.join("\n");

    let mut b = CellHost::new();
    b.set_cache(true);
    b.add(mul_cart());
    b.add(score_cart());
    let rep = b
        .import_facts(
            file.as_bytes(),
            &ImportPolicy {
                verify_fraction: 0.0, // only the contradiction pass runs
                quarantine: true,
                seed: Some(7),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(rep.failures.len(), 1, "exactly the lie fails");
    assert!(rep.failures[0].got.contains("r=[21,"), "truth re-executed");
    assert!(!rep.file_failed, "quarantine salvages the remainder");
    // The truth imported and serves.
    let hm = b.load("mul.v1").unwrap();
    let f = b.run_fast(hm, &[3, 7], DEFAULT_CYCLES).unwrap();
    assert_eq!(f.result, 21);
    assert_eq!(b.cache_split(hm).unwrap(), Some((0, 1)));
}

#[test]
fn sampling_is_unpredictable_but_effective() {
    // The predictability test (docs/12 §7): a producer holding the importer's code
    // but not its seed cannot place a tamper that survives — across seeds at 1%
    // sampling the tamper is caught (and it *is* a sample: not caught every time).
    let (buf, _) = export_workload();
    let text = String::from_utf8(buf).unwrap();
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let i = lines
        .iter()
        .position(|l| l.contains("\"args\":[0,9]"))
        .unwrap();
    lines[i] = lines[i].replace("\"r\":[0,", "\"r\":[1,");
    let file = lines.join("\n");

    let mut caught = 0u32;
    for seed in 0..1000u64 {
        let mut b = CellHost::new();
        b.set_cache(true);
        b.add(mul_cart());
        b.add(score_cart());
        let rep = b
            .import_facts(
                file.as_bytes(),
                &ImportPolicy {
                    verify_fraction: 0.01,
                    seed: Some(seed),
                    ..Default::default()
                },
            )
            .unwrap();
        if rep.file_failed {
            caught += 1;
        }
    }
    // 7 lines, 1% + min-1: the tamper is sampled ~1/7 of trials — must be caught
    // sometimes (an adversary can't survive) and not always (it is a sample).
    assert!(caught > 30, "caught {caught}/1000 — sampling never fires?");
    assert!(caught < 700, "caught {caught}/1000 — not a sample");
}

#[test]
fn budget_and_unknown_lines_reject() {
    let (buf, n) = export_workload();
    let mut text = String::from_utf8(buf).unwrap();
    // A budget halt is not a fact; an artifact we don't hold is unfalsifiable.
    text.push_str("\n{\"a\":\"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"e\":\"run\",\"args\":[1],\"r\":[1,0,0],\"cy\":5,\"tr\":0,\"h\":\"ok\"}");
    text.push_str("\n{\"a\":\"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"e\":\"run\",\"args\":[2],\"r\":[1,0,0],\"cy\":5,\"tr\":0,\"h\":\"cycle_budget\"}");
    text.push_str("\nnot json at all");
    let mut b = CellHost::new();
    b.set_cache(true);
    b.add(mul_cart());
    b.add(score_cart());
    let rep = b
        .import_facts(
            text.as_bytes(),
            &ImportPolicy {
                seed: Some(3),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(rep.accepted, n);
    assert_eq!(rep.rejected_unknown_artifact, 1);
    assert_eq!(rep.rejected_budget_halt, 1);
    assert_eq!(rep.rejected_malformed, 1);
}

#[test]
fn the_config_wall_holds() {
    // The invariant (docs/12 §"What a fact is"): every outcome-affecting knob lives
    // inside the hash. (1) Exhaustive destructure — adding a `CellConfig` field
    // breaks this test until the author decides its hashing story. (2) Each field,
    // varied alone, changes the image bytes and so the content address. (3) Facts
    // exported under one policy don't even *speak about* the other's artifact.
    let base = CellConfig::sandboxed();
    let CellConfig {
        allow_raw_memory: _,
        allow_ports: _,
        max_code_bytes: _,
        max_touched: _,
        div_by_zero: _,
    } = base.clone();

    let src = "fn run(a: u16, b: u16) -> u16 { a / b }";
    let image = |cfg: CellConfig| {
        CellProgram::compile_with_config(src, cfg)
            .unwrap()
            .to_bytes()
    };
    let baseline = image(base.clone());
    let variants = [
        CellConfig {
            allow_raw_memory: true,
            ..base.clone()
        },
        CellConfig {
            allow_ports: true,
            ..base.clone()
        },
        CellConfig {
            max_code_bytes: Some(2048),
            ..base.clone()
        },
        CellConfig {
            max_touched: Some(9),
            ..base.clone()
        },
        CellConfig {
            div_by_zero: DivByZero::Saturate,
            ..base.clone()
        },
    ];
    for (i, v) in variants.into_iter().enumerate() {
        assert_ne!(
            image(v),
            baseline,
            "config field {i} escapes the image bytes"
        );
    }

    // And end-to-end: the same source under a different halt policy is a different
    // artifact — its facts are rejected as unknown, never misapplied.
    let cart_halt = Cartridge::compile(src, base.clone(), CartridgeOpts::default()).unwrap();
    let cart_sat = Cartridge::compile(
        src,
        CellConfig {
            div_by_zero: DivByZero::Saturate,
            ..base
        },
        CartridgeOpts {
            id: Some("div.sat".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_ne!(cart_halt.artifact_hash(), cart_sat.artifact_hash());

    let mut a = CellHost::new();
    a.set_cache(true);
    a.add(cart_halt);
    let h = a.load("run").unwrap();
    a.run_fast(h, &[7, 0], DEFAULT_CYCLES).unwrap(); // div-by-zero fact (halt policy)
    let mut buf = Vec::new();
    a.export_facts(&mut buf, "wall").unwrap();

    let mut b = CellHost::new();
    b.set_cache(true);
    b.add(cart_sat); // holds only the *other* policy's artifact
    let rep = b.import_facts(&buf[..], &ImportPolicy::default()).unwrap();
    assert_eq!(rep.accepted, 0);
    assert_eq!(rep.rejected_unknown_artifact, 1);
}

#[test]
fn verify_facts_audits_every_line() {
    // The CI verb: every line re-executed, nothing imported.
    let (buf, n) = export_workload();
    let mut b = CellHost::new();
    b.set_cache(true);
    b.add(mul_cart());
    b.add(score_cart());
    let rep = b.verify_facts(&buf[..]).unwrap();
    assert_eq!(rep.verified, n);
    assert!(rep.failures.is_empty());
    // Nothing imported (dry run): a load serves no imported facts.
    let hm = b.load("mul.v1").unwrap();
    b.run_fast(hm, &[3, 7], DEFAULT_CYCLES).unwrap();
    assert_eq!(b.cache_split(hm).unwrap(), Some((0, 0)));
}

#[test]
fn cli_facts_verbs_end_to_end() {
    // The Act-3 beats through the actual CLI verbs: export from a calls file,
    // import cleanly, tamper one digit with `sed`-equivalent, watch it fail.
    use cell80::run_cli;
    let dir = std::env::temp_dir().join(format!("cell80-facts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("mul.rs"),
        "fn run(a: u16, b: u16) -> u16 { a * b }",
    )
    .unwrap();
    let calls = dir.join("calls.txt");
    std::fs::write(&calls, "mul 3 7\nmul 200 300\n").unwrap();
    let d = dir.to_str().unwrap();

    let facts_text = run_cli(&[
        "facts".into(),
        "export".into(),
        d.into(),
        "--calls".into(),
        calls.to_str().unwrap().into(),
        "--producer".into(),
        "cli-test".into(),
    ])
    .expect("export works");
    assert!(facts_text.contains("\"args\":[3,7]"));
    let facts_file = dir.join("lib.facts");
    std::fs::write(&facts_file, &facts_text).unwrap();

    // Clean verify passes…
    let ok = run_cli(&[
        "facts".into(),
        "verify".into(),
        facts_file.to_str().unwrap().into(),
        d.into(),
    ])
    .expect("verify passes");
    assert!(ok.contains("no lies caught"), "{ok}");

    // …the tamper beat: one digit, re-import, file rejected, line named.
    let tampered = facts_text.replace("\"r\":[21,", "\"r\":[27,");
    std::fs::write(&facts_file, tampered).unwrap();
    let err = run_cli(&[
        "facts".into(),
        "import".into(),
        facts_file.to_str().unwrap().into(),
        d.into(),
        "--verify-fraction".into(),
        "1".into(),
    ])
    .expect_err("tamper must fail the import");
    assert!(err.contains("FILE REJECTED"), "{err}");
    assert!(err.contains("run(3,7)"), "{err}");
    std::fs::remove_dir_all(&dir).ok();
}

// ── routing over the fact library ───────────────────────────────────────────────

fn min_cart() -> Cartridge {
    Cartridge::compile(
        "fn run(a: u16, b: u16) -> u16 { let mut m = a; if b < a { m = b; } m }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("min".into()),
            ..Default::default()
        },
    )
    .unwrap()
}

fn max_cart() -> Cartridge {
    Cartridge::compile(
        "fn run(a: u16, b: u16) -> u16 { let mut m = a; if b > a { m = b; } m }",
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some("max".into()),
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn route_rides_imported_facts() {
    // Export claims about min from host A; import into host B; route by the same
    // examples — min's probes are answered from the imported facts (no execution),
    // max's are executed, and the ranking picks min by behaviour.
    let mut producer = CellHost::new();
    producer.set_cache(true);
    producer.add(min_cart());
    let h = producer.load("min").unwrap();
    for (a, b) in [(3u16, 7u16), (10, 4)] {
        producer.run_fast(h, &[a, b], DEFAULT_CYCLES).unwrap();
    }
    let mut file = Vec::new();
    producer.export_facts(&mut file, "route@dod").unwrap();

    let mut consumer = CellHost::new();
    consumer.set_cache(true);
    consumer.add(min_cart());
    consumer.add(max_cart());
    let rep = consumer
        .import_facts(
            &file[..],
            &ImportPolicy {
                seed: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(rep.failures.is_empty() && !rep.file_failed);

    let examples = vec![(vec![3u16, 7], 3u16), (vec![10, 4], 4)];
    let route = consumer.route_by_examples_facts(&examples, 10).unwrap();
    assert_eq!(route.ranked, vec![(2, "min".to_string())]); // max reproduces neither
    assert_eq!(route.probe_runs, 4); // 2 cells × 2 examples
    assert_eq!(route.from_facts, 2); // min: both answered from claims
    assert_eq!(route.local, 2); // max: both executed
}

#[test]
fn route_without_facts_executes_everything() {
    // No import: the fact-aware route degrades to the execute-everything path —
    // same ranking contract, zero probes answered from facts.
    let mut host = CellHost::new();
    host.set_cache(true);
    host.add(min_cart());
    host.add(max_cart());
    let examples = vec![(vec![3u16, 7], 7u16), (vec![10, 4], 10)];
    let route = host.route_by_examples_facts(&examples, 10).unwrap();
    assert_eq!(route.ranked, vec![(2, "max".to_string())]);
    assert_eq!((route.from_facts, route.local), (0, 4));
}

#[test]
fn cli_route_end_to_end_with_facts() {
    // The card-catalogue beat through the actual CLI: export min's claims, then
    // `route <dir> 3,7=3 10,4=4 --facts` finds min *from the fact library* — and
    // the behavioural flip (same inputs, opposite outputs) finds max by execution.
    use cell80::run_cli;
    let dir = std::env::temp_dir().join(format!("cell80-route-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("min.rs"),
        "//! smaller of two\nfn run(a: u16, b: u16) -> u16 { let mut m = a; if b < a { m = b; } m }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("max.rs"),
        "//! larger of two\nfn run(a: u16, b: u16) -> u16 { let mut m = a; if b > a { m = b; } m }\n",
    )
    .unwrap();
    let calls = dir.join("calls.txt");
    std::fs::write(&calls, "min 3 7\nmin 10 4\n").unwrap();
    let d = dir.to_str().unwrap();
    let s = |v: &[&str]| -> Vec<String> { v.iter().map(|t| t.to_string()).collect() };

    let facts_text = run_cli(&s(&[
        "facts",
        "export",
        d,
        "--calls",
        calls.to_str().unwrap(),
    ]))
    .unwrap();
    let facts_file = dir.join("min.facts");
    std::fs::write(&facts_file, &facts_text).unwrap();

    let out = run_cli(&s(&[
        "route",
        d,
        "3,7=3",
        "10,4=4",
        "--facts",
        facts_file.to_str().unwrap(),
    ]))
    .unwrap();
    assert!(out.contains("min —"), "min must match:\n{out}");
    assert!(
        !out.lines().any(|l| l.contains("max —")),
        "max must not match:\n{out}"
    );
    assert!(
        out.contains("2 answered from imported facts"),
        "provenance split:\n{out}"
    );

    let flipped = run_cli(&s(&["route", d, "3,7=7", "10,4=10"])).unwrap();
    assert!(flipped.contains("max —"), "flip finds max:\n{flipped}");
    assert!(
        flipped.contains("0 answered from imported facts"),
        "no facts imported:\n{flipped}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn fact_line_parse_rejections() {
    // Every malformed shape is a per-line reject with a reason (never a panic).
    let good = "{\"a\":\"sha256:0000000000000000000000000000000000000000000000000000000000000000\",\"e\":\"run\",\"args\":[1],\"r\":[1,0,0],\"cy\":5,\"tr\":0,\"h\":\"ok\"}";
    assert!(Fact::from_line(good).is_ok());
    let bad = [
        "not json",
        "[1,2,3]",                                               // not an object
        &good.replace("\"a\":\"sha256:", "\"a\":\"md5:"),        // wrong hash scheme
        &good.replace("000000", "zzzzzz"),                       // bad hex
        &good.replacen("0000", "00", 1),                         // short hex
        &good.replace(",\"e\":\"run\"", ""),                     // missing entry
        &good.replace(",\"args\":[1]", ""),                      // missing input
        &good.replace("\"args\":[1]", "\"args\":[99999]"),       // arg > u16
        &good.replace("\"args\":[1]", "\"args\":\"x\""),         // args not an array
        &good.replace("\"r\":[1,0,0]", "\"r\":[1,0]"),           // short regs
        &good.replace("\"r\":[1,0,0]", "\"r\":[1,0,99999]"),     // reg > u16
        &good.replace(",\"cy\":5", ""),                          // missing cost
        &good.replace(",\"tr\":0", ""),                          // missing trapped_ops
        &good.replace("\"h\":\"ok\"", "\"h\":\"weird\""),        // unknown halt
        &good.replace("\"h\":\"ok\"", "\"h\":\"halt:x\""),       // bad halt code
        &good.replace("\"h\":\"ok\"", "\"h\":\"memory_limit\""), // budget-relative
    ];
    for (i, b) in bad.iter().enumerate() {
        assert!(Fact::from_line(b).is_err(), "case {i} should reject: {b}");
    }
    // Halt encodings round-trip through the line, including state facts with `f`/`out`.
    for h in ["div_by_zero", "halt:7", "escalate:65281"] {
        let line = good.replace("\"h\":\"ok\"", &format!("\"h\":\"{h}\""));
        let f = Fact::from_line(&line).unwrap();
        assert_eq!(f.to_line(), line);
    }
    let state = good
        .replace("\"args\":[1]", "\"f\":{\"a\":1,\"b\":2}")
        .replace("\"h\":\"ok\"", "\"h\":\"ok\",\"out\":{\"y\":9}");
    let f = Fact::from_line(&state).unwrap();
    assert_eq!(f.to_line(), state);
}

#[test]
fn import_conflicts_with_live_entries_lose_by_execution() {
    // A warm runner's own cache entry *is* an execution result — an imported claim
    // that contradicts it loses (both the value and the state shape).
    let (buf, _) = export_workload();
    let text = String::from_utf8(buf).unwrap();
    let mut b = CellHost::new();
    b.set_cache(true);
    b.add(mul_cart());
    b.add(score_cart());
    // Compute local truths first (the entries the file will collide with).
    let hm = b.load("mul.v1").unwrap();
    b.run_fast(hm, &[3, 7], DEFAULT_CYCLES).unwrap();
    let hs = b.load("score.v1").unwrap();
    b.run_state_fast(hs, &[("wx".into(), 3), ("x".into(), 17)], DEFAULT_CYCLES)
        .unwrap();
    // Tamper both corresponding lines, then import with no sampling: the vs-live
    // insert path catches both.
    let tampered = text
        .replace("\"args\":[3,7],\"r\":[21,", "\"args\":[3,7],\"r\":[22,")
        .replace("\"out\":{\"total\":51,", "\"out\":{\"total\":52,");
    let rep = b
        .import_facts(
            tampered.as_bytes(),
            &ImportPolicy {
                verify_fraction: 0.0,
                quarantine: true,
                seed: Some(9),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(rep.failures.len(), 2, "{:?}", rep.failures);
    // The local truths still serve.
    let f = b.run_fast(hm, &[3, 7], DEFAULT_CYCLES).unwrap();
    assert_eq!(f.result, 21);
    // And re-importing the *clean* file dedupes against live entries silently.
    let rep = b
        .import_facts(
            text.as_bytes(),
            &ImportPolicy {
                verify_fraction: 0.0,
                quarantine: true,
                seed: Some(9),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(rep.failures.is_empty(), "{:?}", rep.failures);
}

#[test]
fn import_report_json_and_cli_json_flag() {
    // The report's JSON rendering carries the failures; the CLI --json path
    // renders it on both success and (as the error payload) failure.
    use cell80::run_cli;
    let dir = std::env::temp_dir().join(format!("cell80-factsjson-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("mul.rs"),
        "fn run(a: u16, b: u16) -> u16 { a * b }",
    )
    .unwrap();
    let calls = dir.join("calls.txt");
    std::fs::write(&calls, "# comment line\nmul 3 7\n").unwrap();
    let d = dir.to_str().unwrap();
    let facts_text = run_cli(&[
        "facts".into(),
        "export".into(),
        d.into(),
        "--calls".into(),
        calls.to_str().unwrap().into(),
    ])
    .unwrap();
    let ff = dir.join("lib.facts");
    std::fs::write(&ff, &facts_text).unwrap();
    let ok = run_cli(&[
        "facts".into(),
        "import".into(),
        ff.to_str().unwrap().into(),
        d.into(),
        "--json".into(),
        "--quarantine".into(),
    ])
    .unwrap();
    assert!(ok.contains("\"file_failed\":false"), "{ok}");
    std::fs::write(&ff, facts_text.replace("\"r\":[21,", "\"r\":[23,")).unwrap();
    let err = run_cli(&[
        "facts".into(),
        "verify".into(),
        ff.to_str().unwrap().into(),
        d.into(),
        "--json".into(),
    ])
    .unwrap_err();
    assert!(err.contains("\"failures\":[{\"line\":"), "{err}");
    // Flag/verb error paths.
    assert!(run_cli(&["facts".into()]).is_err());
    assert!(run_cli(&["facts".into(), "unknown".into()]).is_err());
    assert!(run_cli(&["facts".into(), "export".into(), d.into()]).is_err()); // no --calls
    assert!(run_cli(&["facts".into(), "export".into(), d.into(), "--bogus".into()]).is_err());
    assert!(run_cli(&[
        "facts".into(),
        "import".into(),
        ff.to_str().unwrap().into(),
        d.into(),
        "--bogus".into()
    ])
    .is_err());
    assert!(run_cli(&[
        "facts".into(),
        "import".into(),
        "/nonexistent.facts".into(),
        d.into()
    ])
    .is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cli_facts_export_state_calls() {
    // The calls file drives a state cell by named fields (and --producer stamps).
    use cell80::run_cli;
    let dir = std::env::temp_dir().join(format!("cell80-factsst-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("acc.rs"),
        "//! running accumulator\n//! entry: Acc::run\nstruct Acc { x: u16, total: u32 }
         impl Acc { fn run(&mut self) -> u16 { self.total = self.x as u32 * 2u32; (self.total & 0xFFFFu32) as u16 } }",
    )
    .unwrap();
    let calls = dir.join("calls.txt");
    std::fs::write(&calls, "acc x=21\nacc x=1000\n").unwrap();
    let out = run_cli(&[
        "facts".into(),
        "export".into(),
        dir.to_str().unwrap().into(),
        "--calls".into(),
        calls.to_str().unwrap().into(),
        "--producer".into(),
        "state@cli".into(),
    ])
    .unwrap();
    assert!(out.contains("\"producer\":\"state@cli\""), "{out}");
    assert!(out.contains("\"f\":{\"x\":21}"), "{out}");
    assert!(out.contains("\"out\":{"), "{out}");
    // Bad calls lines error with the line number.
    std::fs::write(&calls, "acc x=notanum\n").unwrap();
    let err = run_cli(&[
        "facts".into(),
        "export".into(),
        dir.to_str().unwrap().into(),
        "--calls".into(),
        calls.to_str().unwrap().into(),
    ])
    .unwrap_err();
    assert!(err.contains("calls line 1"), "{err}");
    std::fs::write(&calls, "acc 5 notanum\n").unwrap();
    assert!(run_cli(&[
        "facts".into(),
        "export".into(),
        dir.to_str().unwrap().into(),
        "--calls".into(),
        calls.to_str().unwrap().into(),
    ])
    .is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn runner_fact_edges() {
    // The uncached run_state_fast tail, the buffer-input rejection, resolve errors,
    // the branchy run_many_fast fallback, and an entropy-seeded Rng import.
    let src = "
        struct S { a: u16, out: u16 }
        impl S { fn run(&mut self) -> u16 { self.out = self.a + 1u16; self.out } }
    ";
    let mut r = Runner::compile(src).unwrap(); // cache NOT enabled
    let layout = rustz80::struct_layout(src, "S").unwrap();
    let addr =
        |n: &str| cell80::STATE_BASE + layout.iter().find(|f| f.name == n).unwrap().offset * 2;
    let reads = vec![("out".to_string(), addr("out"), cell80::Ty::U16)];
    let (f, s) = r
        .run_state_fast(
            Some("S::run"),
            &[(addr("a"), cell80::Ty::U16, 41)],
            &reads,
            DEFAULT_CYCLES,
        )
        .unwrap();
    assert_eq!((f.result, s[0].1), (42, 42));
    // A buffer-typed input can't ride the scalar triple.
    let err = r
        .run_state_fast(
            Some("S::run"),
            &[(addr("a"), cell80::Ty::Bytes(4), 1)],
            &reads,
            DEFAULT_CYCLES,
        )
        .unwrap_err();
    assert!(err.contains("bytes[4]"), "{err}");
    // Resolve errors: no run/main; unknown entry names the available ones.
    let mut none = Runner::compile("fn helper(a: u16) -> u16 { a }").unwrap();
    assert!(none.run_fast(None, &[], DEFAULT_CYCLES).is_err());
    assert!(none.run(None, &[], DEFAULT_CYCLES).is_err());
    let err = none
        .run_fast(Some("nope"), &[], DEFAULT_CYCLES)
        .unwrap_err();
    assert!(err.contains("available"), "{err}");
    let err = none.run(Some("nope"), &[], DEFAULT_CYCLES).unwrap_err();
    assert!(err.contains("available"), "{err}");
    // A branchy entry can't decode for the straight-line replayer — the authentic
    // fallback answers per input.
    let mut br =
        Runner::compile("fn run(a: u16) -> u16 { if a > 5u16 { a * 2u16 } else { a } }").unwrap();
    let out = br
        .run_many_fast(None, &[&[3u16][..], &[9u16][..]], DEFAULT_CYCLES)
        .unwrap();
    assert_eq!((out[0].result, out[1].result), (3, 18));
    // An entropy-seeded import (policy.seed = None) exercises the local-entropy Rng.
    let (buf, _) = export_workload();
    let mut b = CellHost::new();
    b.set_cache(true);
    b.add(mul_cart());
    b.add(score_cart());
    let rep = b.import_facts(&buf[..], &ImportPolicy::default()).unwrap();
    assert!(rep.verified >= 1 && rep.failures.is_empty());
}

#[test]
fn search_scored_exposes_the_margin() {
    let mut host = CellHost::new();
    host.add(mul_cart());
    host.add(score_cart());
    host.add(
        Cartridge::compile(
            "fn run(a: u16, b: u16) -> u16 { a + b }",
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some("sum.v1".into()),
                summary: "the sum of two numbers".into(),
                tags: vec!["math".into()],
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let hits = host.search_scored("sum of two numbers", 3);
    assert!(!hits.is_empty());
    assert!(hits.windows(2).all(|w| w[0].0 >= w[1].0), "sorted by score");
}
