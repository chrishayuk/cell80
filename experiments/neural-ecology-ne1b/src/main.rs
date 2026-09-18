use ne0c::{
    backend::{Cells, Mode},
    digest,
    model::*,
    runner, Result,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Deserialize)]
struct System {
    id: String,
    weights: Vec<u16>,
}
#[derive(Deserialize)]
struct Input {
    seed: u64,
    topology: Topology,
    systems: Vec<System>,
}

fn seal(path: &Path, seed: u64, split: &str) -> Result<serde_json::Value> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = base.join("../..");
    let s: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    assert_eq!(s["binary_sha256"], digest(&fs::read(env::current_exe()?)?));
    for (name, expected) in s["files"].as_object().ok_or("missing seal files")? {
        if expected.as_str() != Some(&digest(&fs::read(repo.join(name))?)) {
            return Err(format!("changed sealed file: {name}").into());
        }
    }
    let stage = if seed == 3001 {
        "development"
    } else {
        "evaluation"
    };
    assert!(seed == 3001 || (4001..=4010).contains(&seed));
    assert_eq!(s["stage"], stage);
    if split == "test" {
        let p = base.join(format!("artifacts/{stage}_fit_manifest.json"));
        let m: serde_json::Value = serde_json::from_slice(&fs::read(p)?)?;
        assert_eq!(m["all_fits_complete"], true);
        assert_eq!(m["code_seal_sha256"], digest(&fs::read(path)?));
    }
    Ok(s)
}

// Same frozen-weight operation as NE-0c runner::tick(..., false), generalized
// from three networks to arbitrary independent systems. No plasticity path.
fn step(
    cells: &Cells,
    mode: Mode,
    networks: &mut [Network],
    top: &Topology,
    symbol: u8,
) -> Result<(Vec<i64>, Vec<Node>)> {
    let old: Vec<_> = networks
        .iter()
        .flat_map(|n| n.nodes.iter().copied())
        .collect();
    let currents: Vec<_> = networks
        .iter()
        .flat_map(|n| n.currents(top, symbol))
        .collect();
    let next = cells.nodes(mode, &old, &currents)?;
    for (a, network) in networks.iter_mut().enumerate() {
        network.nodes.copy_from_slice(&next[a * N..(a + 1) * N]);
    }
    Ok((currents, next))
}

fn run() -> Result<()> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args: Vec<_> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("fixtures") {
        let cells = Cells::new(&base.join("../neural-ecology-ne0c/artifacts/build"))?;
        let old = runner::fixtures(&cells)?;
        // Explicitly compare batching to the unchanged three-arm evaluator.
        let top = Topology::new(3001);
        let mut baseline = vec![Network::new(3001); 3];
        let mut batch = baseline.clone();
        for t in 0..32 {
            let frame = runner::tick(
                &cells,
                Mode::Reference,
                &mut baseline,
                &top,
                (t % 4) as u8,
                false,
            )?;
            let (currents, nodes) = step(&cells, Mode::Reference, &mut batch, &top, (t % 4) as u8)?;
            assert_eq!(baseline, batch);
            assert_eq!(currents, frame.currents);
            assert_eq!(nodes, frame.new_nodes);
            assert!(frame.deltas.iter().flatten().all(|&d| d == 0));
        }
        runner::save_json(
            &base.join("artifacts/fixtures.json"),
            &json!({"inherited":old,"generalized_tick_identity":true,"ticks":32}),
        )?;
        return Ok(());
    }
    if args.len() != 5 || args[0] != "measure" {
        return Err(
            "usage: ne1b fixtures | measure INPUT.json fit|validation|test OUTDIR SEAL.json".into(),
        );
    }
    let input: Input = serde_json::from_slice(&fs::read(&args[1])?)?;
    let split = &args[2];
    assert!(["fit", "validation", "test"].contains(&split.as_str()));
    let sealpath = Path::new(&args[4]);
    seal(sealpath, input.seed, split)?;
    let out = Path::new(&args[3]);
    if out.exists() {
        return Err("refusing to overwrite trajectory attempt".into());
    }
    fs::create_dir_all(out)?;
    let initial: Vec<_> = input
        .systems
        .iter()
        .map(|s| {
            assert_eq!(s.weights.len(), E);
            assert!(s.weights.iter().all(|&w| w <= 32));
            Network {
                weights: s.weights.clone(),
                nodes: vec![Node::default(); N],
            }
        })
        .collect();
    assert_eq!(initial.len(), 71);
    let symbols = grammar(input.seed, &format!("NE1-v1/{split}"), WARM + SAMPLES + 1);
    fs::write(out.join("symbols.u8"), &symbols)?;
    let cells = Cells::new(&base.join("../neural-ecology-ne0c/artifacts/build"))?;
    let modes = [Mode::Reference, Mode::Cpu, Mode::Metal, Mode::Metal];
    let mut worlds = vec![initial.clone(); 4];
    let mut files: Vec<_> = input
        .systems
        .iter()
        .map(|s| File::create(out.join(format!("{}-q.u16", s.id))).map(BufWriter::new))
        .collect::<std::io::Result<_>>()?;
    let mut full_hashes = vec![Sha256::new(); initial.len()];
    let mut replay_hashes = vec![Sha256::new(); initial.len()];
    let mut logs = BufWriter::new(File::create(out.join("ticks.jsonl"))?);
    let mut activity: Vec<runner::Activity> =
        (0..initial.len()).map(|_| Default::default()).collect();
    let started = Instant::now();
    for t in 0..WARM + SAMPLES {
        let reference = step(
            &cells,
            modes[0],
            &mut worlds[0],
            &input.topology,
            symbols[t],
        )?;
        for k in 1..4 {
            let other = step(
                &cells,
                modes[k],
                &mut worlds[k],
                &input.topology,
                symbols[t],
            )?;
            if reference != other || worlds[0] != worlds[k] {
                return Err(format!("backend mismatch tick={t} backend={k}").into());
            }
        }
        let hashes: Vec<_> = worlds[0]
            .iter()
            .map(|n| digest(&n.nodes.iter().flat_map(|v| v.bytes()).collect::<Vec<_>>()))
            .collect();
        serde_json::to_writer(&mut logs, &json!({"tick":t,"node_hashes":hashes}))?;
        logs.write_all(b"\n")?;
        for a in 0..initial.len() {
            assert_eq!(worlds[0][a].weights, initial[a].weights);
            replay_hashes[a].update(hashes[a].as_bytes());
            if t >= WARM {
                activity[a].ticks += 1;
                let mut full = Vec::with_capacity(N * 8);
                for field in 0..4 {
                    for (i, node) in worlds[0][a].nodes.iter().enumerate() {
                        let value = match field {
                            0 => node.v,
                            1 => node.r as u16,
                            2 => node.s as u16,
                            _ => node.q,
                        };
                        full.extend_from_slice(&value.to_le_bytes());
                        if field == 3 {
                            files[a].write_all(&value.to_le_bytes())?;
                            activity[a].spikes += u64::from(node.s);
                            activity[a].seen[i] |= node.s != 0;
                        }
                    }
                }
                full_hashes[a].update(&full);
            }
        }
        if (t + 1) % 1000 == 0 {
            eprintln!(
                "seed={} split={split} ticks={} seconds={:.1}",
                input.seed,
                t + 1,
                started.elapsed().as_secs_f64()
            );
        }
    }
    for f in &mut files {
        f.flush()?;
    }
    logs.flush()?;
    seal(sealpath, input.seed, split)?;
    let systems: Vec<_> = input
        .systems
        .iter()
        .enumerate()
        .map(|(i, s)| {
            json!({
        "id":s.id,"weight_sha256":digest(&initial[i].persistent_bytes()),
        "q_sha256":digest(&fs::read(out.join(format!("{}-q.u16",s.id))).unwrap()),
        "full_sha256":format!("{:x}",full_hashes[i].clone().finalize()),
        "replay_sha256":format!("{:x}",replay_hashes[i].clone().finalize()),
        "activity":activity[i].report()})
        })
        .collect();
    runner::save_json(
        &out.join("summary.json"),
        &json!({"seed":input.seed,"split":split,
        "ticks":WARM+SAMPLES,"rows":SAMPLES,"reset":true,"backend_agreement":true,
        "metal_repeat":true,"weights_unchanged":true,"plasticity_disabled":true,
        "requested_delta_nonzero":0,"stream_sha256":digest(&symbols),"systems":systems,
        "ticks_sha256":digest(&fs::read(out.join("ticks.jsonl"))?),
        "input_sha256":digest(&fs::read(&args[1])?),"binary_sha256":digest(&fs::read(env::current_exe()?)?),
        "seconds":started.elapsed().as_secs_f64()}),
    )?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("NE1b ERROR: {e}");
        std::process::exit(1);
    }
}
