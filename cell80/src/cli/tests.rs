//! CLI tests — the verb surface end-to-end (parsers, dispatch, serve loop, and
//! `run_cli` over temp libraries), exactly as they lived in the monolithic cli.rs.
use super::meta::{parse_meta, parse_scale};
use super::serve::{dispatch, serve_loop};
use super::*;
use crate::{Cartridge, CartridgeOpts, CellConfig};

#[test]
fn parse_meta_reads_finite_result() {
    // The F0.4 header key: `off` opts an IEEE-plumbing cell out of the
    // non-finite-return escalation; absent means the default (on).
    let (.., finite, _) = parse_meta("//! s\n//! finite_result: off\nfn run() -> u16 { 1u16 }");
    assert_eq!(finite, Some(false));
    let (.., finite, _) = parse_meta("//! s\n//! finite_result: on\nfn run() -> u16 { 1u16 }");
    assert_eq!(finite, Some(true));
    let (.., finite, _) = parse_meta("//! s\nfn run() -> u16 { 1u16 }");
    assert_eq!(finite, None);
}

#[test]
fn parse_meta_reads_accuracy() {
    // The F2 header key: a free-form declared ULP bound (harness-verified, not
    // parser-validated); absent (or empty) means None — exact semantics.
    let (_, _, _, _, _, acc, _, _) =
        parse_meta("//! s\n//! accuracy: <= 4 ulp over [-87.34, 88.72]\nfn run() -> u16 { 1u16 }");
    assert_eq!(acc.as_deref(), Some("<= 4 ulp over [-87.34, 88.72]"));
    let (_, _, _, _, _, acc, _, _) = parse_meta("//! s\nfn run() -> u16 { 1u16 }");
    assert_eq!(acc, None);
    let (_, _, _, _, _, acc, _, _) =
        parse_meta("//! s\n//! accuracy:   \nfn run() -> u16 { 1u16 }");
    assert_eq!(acc, None);
    // The accuracy line never leaks into the summary.
    let (summary, ..) =
        parse_meta("//! accuracy: <= 2 ulp\n//! the real summary\nfn run() -> u16 { 1u16 }");
    assert_eq!(summary, "the real summary");
}

#[test]
fn finite_result_header_flows_to_the_manifest() {
    // A library `.rs` with the opt-out header compiles to a manifest with the
    // contract off — the whole `library_cartridge` wiring, not just the parser.
    let dir = std::env::temp_dir().join(format!("cell80-finres-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("nan_probe.rs");
    std::fs::write(
        &p,
        "//! Returns NaN deliberately (IEEE plumbing).\n//! finite_result: off\nfn run() -> f32 { 0.0f32 / 0.0f32 }\n",
    )
    .unwrap();
    let cart = library_cartridge(&p).unwrap().unwrap();
    assert!(!cart.manifest.finite_result);
    assert_eq!(cart.manifest.signature.ret, "f32");
    std::fs::remove_dir_all(&dir).ok();
}

fn host() -> CellHost {
    let mut h = CellHost::new();
    h.add(
        Cartridge::compile(
            "fn run(a: u16, b: u16) -> u16 { a * b }",
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some("mul".into()),
                summary: "multiply two".into(),
                tags: vec!["math".into(), "product".into()],
                entry: None,
                limits: Vec::new(),
                scale: None,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    h
}

#[test]
fn scale_header_parses() {
    // `//! scale: N` — a plain count or a Q-format (part after the point).
    assert_eq!(parse_scale("8"), Some(8));
    assert_eq!(parse_scale("q8.8"), Some(8));
    assert_eq!(parse_scale("Q16.16"), Some(16));
    assert_eq!(parse_scale("  12 "), Some(12));
    assert_eq!(parse_scale("nonsense"), None);
    let (summary, _, _, _, scale, _, _, _) =
        parse_meta("//! Q8.8 multiply\n//! tags: math\n//! scale: 8\nfn run() -> u16 { 0u16 }");
    assert_eq!(scale, Some(8));
    // The scale line never leaks into the summary; absent → None.
    assert_eq!(summary, "Q8.8 multiply");
    let (_, _, _, _, none, _, _, _) = parse_meta("//! plain\nfn run() -> u16 { 0u16 }");
    assert_eq!(none, None);
    // End-to-end: the library path picks up `q_mul`'s `//! scale: 8` (skip if the
    // sibling cells corpus isn't present, e.g. a packaged crates.io build).
    let cells_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cells");
    if let Ok(p) = crate::discover::find_cell_file(&cells_dir, "q_mul") {
        if let Some(Ok(cart)) = library_cartridge(&p) {
            assert_eq!(cart.manifest.scale, Some(8), "q_mul should declare scale 8");
        }
    }
}

#[test]
fn dispatch_covers_every_verb() {
    let mut h = host();
    // discovery.
    assert!(dispatch(&mut h, "search multiply").contains("mul"));
    assert!(dispatch(&mut h, "search zzznotfound").starts_with("no matches"));
    assert!(dispatch(&mut h, "inspect mul").contains("run(a: u16, b: u16)"));
    assert!(dispatch(&mut h, "inspect ghost").contains("no cell"));
    assert!(dispatch(&mut h, "inspect").starts_with("usage"));
    // load → run warm (incl. no-args) → run again → unload.
    assert!(dispatch(&mut h, "load mul").contains("handle 0"));
    assert!(dispatch(&mut h, "load nope").contains("no cell"));
    assert!(dispatch(&mut h, "load").starts_with("usage"));
    assert!(dispatch(&mut h, "run 0 6,7").contains("result 42"));
    assert!(dispatch(&mut h, "run 0 3,3").contains("result 9")); // reused warm
    assert!(dispatch(&mut h, "run 0").contains("result 0")); // no args
    assert!(dispatch(&mut h, "run 0 99999999").contains("bad arg")); // parse error
    assert!(dispatch(&mut h, "run notanum").starts_with("usage"));
    assert!(dispatch(&mut h, "unload 0").contains("unloaded"));
    assert!(dispatch(&mut h, "run 0 1,1").contains("invalid cell handle")); // freed
    assert!(dispatch(&mut h, "unload 0").contains("invalid cell handle"));
    assert!(dispatch(&mut h, "unload x").starts_with("usage"));
    // misc.
    assert!(dispatch(&mut h, "help").contains("search"));
    assert!(dispatch(&mut h, "").is_empty());
    assert!(dispatch(&mut h, "bogus").contains("unknown command"));
}

#[test]
fn serve_loop_runs_a_warm_session() {
    let mut h = host();
    let input = std::io::Cursor::new("load mul\nrun 0 6,7\nunload 0\nquit\nignored after quit\n");
    let mut out: Vec<u8> = Vec::new();
    let summary = serve_loop(&mut h, "test", input, &mut out).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("session:") && s.contains("handle 0"));
    assert!(s.contains("result 42") && s.contains("unloaded"));
    assert!(!s.contains("ignored after quit")); // loop stopped at `quit`
    assert!(summary.contains("session ended"));
}

#[test]
fn host_from_dir_loads_the_seed_library() {
    let dir = format!("{}/cells", env!("CARGO_MANIFEST_DIR"));
    let h = host_from_dir(&dir).unwrap();
    // The real invariant: every .rs cell under the directory (cells live in pack
    // subdirectories, discovered recursively) loads. (An exact count pin goes stale
    // mid-wave — the library grows while suites run.)
    let n_sources = crate::discover::discover_cell_files(&dir)
        .unwrap()
        .iter()
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .count();
    assert_eq!(h.len(), n_sources);

    // The library now holds a *distance family* (manhattan/chebyshev/euclid_sq), so a
    // bare "grid distance" is ambiguous; the cell-specific name still resolves.
    assert_eq!(h.search("manhattan distance", 3)[0].id, "manhattan");
    assert!(host_from_dir("/no/such/dir").is_err());
}

#[test]
fn route_verb_and_example_parsing() {
    let mut h = host();
    // Behavioural routing through the serve dispatch: (3,7)=21 surfaces `mul`.
    assert!(dispatch(&mut h, "route 3,7=21 2,5=10").contains("mul"));
    assert!(dispatch(&mut h, "route 3,7=9999").contains("no cell"));
    assert!(dispatch(&mut h, "route").starts_with("usage:"));
    assert!(dispatch(&mut h, "route nonsense").contains("bad example"));
    assert!(dispatch(&mut h, "route 3,7=1,2").contains("one output"));
    // parse_examples directly: happy + both error shapes.
    assert_eq!(parse_examples(&["3,7=21"]).unwrap(), vec![(vec![3, 7], 21)]);
    assert!(parse_examples(&["3;7"]).is_err());
    assert!(parse_examples(&["3=x"]).is_err());
}

#[test]
fn field_example_parsing_covers_all_expectation_forms() {
    use crate::FieldExample;
    // Expected return only — the classic route form.
    assert_eq!(
        parse_field_examples(&["x:3,y:4=7"]).unwrap(),
        vec![FieldExample {
            fields: vec![("x".into(), 3), ("y".into(), 4)],
            want_result: Some(7),
            want_fields: vec![],
        }]
    );
    // Return + expected post-run fields (the status-flag sibling separator),
    // and the fields-only form.
    assert_eq!(
        parse_field_examples(&["a:9,b:3=1,out:12"]).unwrap(),
        vec![FieldExample {
            fields: vec![("a".into(), 9), ("b".into(), 3)],
            want_result: Some(1),
            want_fields: vec![("out".into(), 12)],
        }]
    );
    assert_eq!(
        parse_field_examples(&["a:9,b:3=out:12"]).unwrap()[0].want_result,
        None
    );
    // Error shapes: no `=`, bare LHS field, two bare returns, empty RHS.
    assert!(parse_field_examples(&["x:3,y:4"]).is_err());
    assert!(parse_field_examples(&["x,y:4=7"]).is_err());
    assert!(parse_field_examples(&["x:3=1,2"]).is_err());
    assert!(parse_field_examples(&["x:3="]).is_err());
}

#[test]
fn search_verb_fuses_examples_into_the_ranking() {
    // Hermetic same-shape twins (identical text surface): trailing examples pick
    // the behavioural match where text alone falls back to id order.
    let dir = std::env::temp_dir().join(format!("cell80-fused-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("pick_lo.rs"),
        "//! pick between two numbers\n//! tags: compare, pair\n\
         fn run(a: u16, b: u16) -> u16 { if a < b { a } else { b } }",
    )
    .unwrap();
    std::fs::write(
        dir.join("pick_hi.rs"),
        "//! pick between two numbers\n//! tags: compare, pair\n\
         fn run(a: u16, b: u16) -> u16 { if a < b { b } else { a } }",
    )
    .unwrap();
    let dir = dir.to_str().unwrap().to_string();
    let q = "pick between two numbers".to_string();

    let top_of = |out: String| {
        out.lines()
            .nth(1)
            .unwrap_or_default()
            .trim_start()
            .to_string()
    };
    // Plain search: identical text, id order — hi first.
    let out = run_cli(&["search".into(), q.clone(), dir.clone()]).unwrap();
    assert!(top_of(out).starts_with("pick_hi"));
    // Examples flip it to the behavioural match.
    let out = run_cli(&["search".into(), q.clone(), dir.clone(), "3,7=3".into()]).unwrap();
    assert!(out.contains("+ 1 example(s)"));
    assert!(top_of(out).starts_with("pick_lo"));
    // Malformed example errors rather than silently falling back to text.
    assert!(run_cli(&["search".into(), q, dir, "3,7=1,2".into()]).is_err());
}

#[test]
fn route_and_search_field_form_through_the_cli() {
    // Status-flag state twins: both return 1 on every run; only post-run `out`
    // differs — the field-form surface end to end, including its error shapes.
    let dir = std::env::temp_dir().join(format!("cell80-fieldform-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let flag = |op: &str| {
        format!(
            "//! combine two fields into out\n//! tags: state, combine\n//! entry: F::run\n\
             struct F {{ a: u16, b: u16, out: u16 }}\n\
             impl F {{ fn run(&mut self) -> u16 {{ self.out = self.a {op} self.b; 1u16 }} }}"
        )
    };
    std::fs::write(dir.join("f_add.rs"), flag("+")).unwrap();
    std::fs::write(dir.join("f_sub.rs"), flag("-")).unwrap();
    let d = dir.to_str().unwrap().to_string();

    // route by named fields: the result-only form matches BOTH twins (status flag).
    let out = run_cli(&["route".into(), d.clone(), "a:9,b:3=1".into()]).unwrap();
    assert!(out.contains("f_add") && out.contains("f_sub"), "{out}");
    // No cell reproduces an impossible expectation.
    let out = run_cli(&["route".into(), d.clone(), "a:1,b:1=999".into()]).unwrap();
    assert!(out.contains("no cell"), "{out}");
    // Error shapes: mixing forms, and the expect-form belongs to `search`.
    assert!(run_cli(&[
        "route".into(),
        d.clone(),
        "a:9,b:3=1".into(),
        "3,7=3".into()
    ])
    .unwrap_err()
    .contains("pick one form"));
    assert!(
        run_cli(&["route".into(), d.clone(), "a:9,b:3=out:12".into()])
            .unwrap_err()
            .contains("`search` example form")
    );
    // A facts file caught lying fails the route rather than seeding bad provenance:
    // export a real claim for f_add(a=9,b=3)→1, tamper the result, import catches it
    // (one fact → the min-1 verification sample must check it).
    let calls = dir.join("calls.txt");
    std::fs::write(&calls, "f_add a=9 b=3\n").unwrap();
    let facts_text = run_cli(&[
        "facts".into(),
        "export".into(),
        d.clone(),
        "--calls".into(),
        calls.to_str().unwrap().into(),
    ])
    .unwrap();
    assert!(facts_text.contains("\"r\":[1,"), "{facts_text}");
    let bad_facts = dir.join("bad.facts");
    std::fs::write(&bad_facts, facts_text.replace("\"r\":[1,", "\"r\":[2,")).unwrap();
    assert!(run_cli(&[
        "route".into(),
        d.clone(),
        "3,7=3".into(),
        "--facts".into(),
        bad_facts.to_str().unwrap().into(),
    ])
    .is_err());

    // search with a field example: `expect` separates the twins where the
    // status-flag return can't — the fused path over named state, via the CLI.
    let out = run_cli(&[
        "search".into(),
        "combine two fields".into(),
        d.clone(),
        "a:9,b:3=1,out:12".into(),
    ])
    .unwrap();
    let top = out.lines().nth(1).unwrap_or_default().trim_start();
    assert!(top.starts_with("f_add"), "{out}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn keygen_and_sign_error_shapes() {
    // keygen to an unwritable path is a clean error (unix-only, like the verb).
    if cfg!(unix) {
        assert!(run_cli(&["keygen".into(), "/no/such/dir/x.key".into()]).is_err());
    }
    // sign: unknown flag, missing --key, and a wrong-size key are each named.
    let dir = std::env::temp_dir().join(format!("cell80-signerr-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("c.rs"),
        "//! add\nfn run(a: u16, b: u16) -> u16 { a + b }",
    )
    .unwrap();
    let cell = dir.join("c.cell");
    run_cli(&[
        "compile".into(),
        dir.join("c.rs").to_str().unwrap().into(),
        "-o".into(),
        cell.to_str().unwrap().into(),
    ])
    .unwrap();
    let cell_s: String = cell.to_str().unwrap().into();
    assert!(run_cli(&["sign".into(), cell_s.clone(), "--bogus".into()])
        .unwrap_err()
        .contains("unknown option"));
    assert!(run_cli(&["sign".into(), cell_s.clone()])
        .unwrap_err()
        .contains("--key"));
    let short_key = dir.join("short.key");
    std::fs::write(&short_key, [0u8; 5]).unwrap();
    assert!(run_cli(&[
        "sign".into(),
        cell_s,
        "--key".into(),
        short_key.to_str().unwrap().into(),
    ])
    .unwrap_err()
    .contains("32 bytes"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn run_and_exec_flags_end_to_end() {
    // The `run` flag surface: --set/--read/--cycles/--json + the safety flags,
    // then `exec` over a compiled cartridge with the same read-back.
    let dir = std::env::temp_dir().join(format!("cell80-cliflags-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src_path = dir.join("cell.rs");
    std::fs::write(
        &src_path,
        "struct S { x: u16, out: u16 }
         impl S { fn run(&mut self) -> u16 { self.out = self.x * 3u16; poke(0xC000u16, 1u8); self.out } }",
    )
    .unwrap();
    let layout_x = crate::STATE_BASE; // x is field 0
    let layout_out = crate::STATE_BASE + 2;
    let out = run_cli(&[
        "run".into(),
        src_path.to_str().unwrap().into(),
        "--entry".into(),
        "S::run".into(),
        "--args".into(),
        format!("{}", crate::STATE_BASE),
        "--set".into(),
        format!("{layout_x}:u16=14"),
        "--read".into(),
        format!("out@{layout_out}:u16"),
        "--cycles".into(),
        "100000".into(),
        "--allow-raw-memory".into(),
        "--allow-ports".into(),
        "--max-code-bytes".into(),
        "4096".into(),
        "--max-touched".into(),
        "512".into(),
        "--json".into(),
    ])
    .unwrap();
    assert!(out.contains("\"result\":42"), "{out}");
    assert!(out.contains("\"out\":42") || out.contains("42"), "{out}");
    // Unknown flag + missing file are clean errors.
    assert!(run_cli(&[
        "run".into(),
        src_path.to_str().unwrap().into(),
        "--bogus".into()
    ])
    .is_err());
    assert!(run_cli(&["run".into(), "/nope.rs".into()]).is_err());

    // Compile → exec with flags (the cartridge carries the policy).
    let cell_path = dir.join("cell.cell");
    run_cli(&[
        "compile".into(),
        src_path.to_str().unwrap().into(),
        "-o".into(),
        cell_path.to_str().unwrap().into(),
        "--entry".into(),
        "S::run".into(),
        "--allow-raw-memory".into(),
    ])
    .unwrap();
    let out = run_cli(&[
        "exec".into(),
        cell_path.to_str().unwrap().into(),
        "--args".into(),
        format!("{}", crate::STATE_BASE),
        "--set".into(),
        format!("{layout_x}:u16=10"),
        "--read".into(),
        format!("out@{layout_out}:u16"),
        "--cycles".into(),
        "100000".into(),
        "--json".into(),
    ])
    .unwrap();
    assert!(out.contains("\"result\":30"), "{out}");
    assert!(run_cli(&[
        "exec".into(),
        cell_path.to_str().unwrap().into(),
        "--bogus".into()
    ])
    .is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn keygen_sign_gate_graph_and_route_verbs() {
    let dir = std::env::temp_dir().join(format!("cell80-cliverbs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let d = dir.to_str().unwrap().to_string();
    std::fs::write(
        dir.join("mul.rs"),
        "//! Product of two values.\n//! tags: math\nfn run(a: u16, b: u16) -> u16 { a * b }",
    )
    .unwrap();

    // compile → (on unix: keygen → sign) → exec. `keygen` draws from
    // /dev/urandom, so the signing beats are unix-only — the same boundary
    // the verb itself has.
    let cell = dir.join("mul.cell");
    run_cli(&[
        "compile".into(),
        dir.join("mul.rs").to_str().unwrap().into(),
        "-o".into(),
        cell.to_str().unwrap().into(),
    ])
    .unwrap();
    if cfg!(unix) {
        let key = dir.join("signer.key");
        let pk = run_cli(&["keygen".into(), key.to_str().unwrap().into()]).unwrap();
        assert!(pk.contains("public"), "{pk}");
        assert!(run_cli(&["keygen".into()]).is_err());
        let signed = run_cli(&[
            "sign".into(),
            cell.to_str().unwrap().into(),
            "--key".into(),
            key.to_str().unwrap().into(),
        ])
        .unwrap();
        assert!(signed.contains("signed"), "{signed}");
        assert!(run_cli(&["sign".into(), cell.to_str().unwrap().into()]).is_err());
    }
    let out = run_cli(&[
        "exec".into(),
        cell.to_str().unwrap().into(),
        "--args".into(),
        "6,7".into(),
    ])
    .unwrap();
    assert!(out.contains("42"), "{out}");

    // index --gate over a retrieval file (admit through the CLI), + bad flag.
    let retrieval = dir.join("retrieval.jsonl");
    std::fs::write(
        &retrieval,
        "{\"id\": \"m-1\", \"query\": \"product of two values\", \"expected\": \"mul\", \"category\": \"direct\"}\n",
    )
    .unwrap();
    let gated = run_cli(&[
        "index".into(),
        d.clone(),
        "--gate".into(),
        retrieval.to_str().unwrap().into(),
        "--json".into(),
    ])
    .unwrap();
    assert!(gated.contains("\"admitted\""), "{gated}");
    assert!(run_cli(&["index".into(), d.clone(), "--bogus".into()]).is_err());

    // graph through the CLI: one node, an external input and a const wire.
    let graph = dir.join("g.json");
    std::fs::write(
        &graph,
        "{\"id\":\"g1\",\"nodes\":{\"m\":\"mul\"},\"wires\":[{\"to\":\"m.a\",\"input\":\"a\"},{\"to\":\"m.b\",\"const\":6}],\"outputs\":{\"out\":\"m.result\"}}",
    )
    .unwrap();
    let g = run_cli(&[
        "graph".into(),
        graph.to_str().unwrap().into(),
        d.clone(),
        "--input".into(),
        "a=7".into(),
        "--cycles".into(),
        "100000".into(),
        "--json".into(),
    ])
    .unwrap();
    assert!(g.contains("\"out\":42"), "{g}");
    assert!(run_cli(&[
        "graph".into(),
        graph.to_str().unwrap().into(),
        d.clone(),
        "--input".into(),
        "nonsense".into()
    ])
    .is_err());

    // route through the CLI — plain, then seeded with an exported fact file.
    let routed = run_cli(&["route".into(), d.clone(), "3,7=21".into(), "--json".into()]).unwrap();
    assert!(routed.contains("mul"), "{routed}");
    let calls = dir.join("calls.txt");
    std::fs::write(&calls, "mul 3 7\n").unwrap();
    let facts_text = run_cli(&[
        "facts".into(),
        "export".into(),
        d.clone(),
        "--calls".into(),
        calls.to_str().unwrap().into(),
    ])
    .unwrap();
    let facts = dir.join("lib.facts");
    std::fs::write(&facts, facts_text).unwrap();
    let routed = run_cli(&[
        "route".into(),
        d.clone(),
        "3,7=21".into(),
        "--facts".into(),
        facts.to_str().unwrap().into(),
    ])
    .unwrap();
    assert!(routed.contains("mul"), "{routed}");
    assert!(run_cli(&["route".into(), d.clone()]).is_err());
    assert!(run_cli(&["route".into(), d.clone(), "--bogus".into()]).is_err());
    assert!(run_cli(&["route".into(), d.clone(), "3,7=21".into(), "--facts".into()]).is_err());
    std::fs::remove_dir_all(&dir).ok();
}
