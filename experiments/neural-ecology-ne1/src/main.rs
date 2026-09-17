use ne0c::{
    backend::{Cells, Mode},
    digest,
    model::*,
    runner, Result,
};
use serde_json::json;
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};

fn check_seal(path: &Path, seed: u64, split: &str) -> Result<serde_json::Value> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let seal: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    if seal["binary_sha256"].as_str() != Some(&digest(&fs::read(env::current_exe()?)?)) {
        return Err("binary changed since code seal".into());
    }
    for (file, hash) in seal["files"].as_object().ok_or("files missing")? {
        if hash.as_str() != Some(&digest(&fs::read(repo.join(file))?)) {
            return Err(format!("sealed file changed: {file}").into());
        }
    }
    if seed != 3001 && (!(4001..=4010).contains(&seed) || seal["stage"] != "evaluation") {
        return Err("evaluation requires validated execution seal".into());
    }
    if split == "test" {
        let key = if seed == 3001 {
            "development_fit_manifest"
        } else {
            "evaluation_fit_manifest"
        };
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("artifacts")
            .join(format!("{key}.json"));
        let manifest: serde_json::Value = serde_json::from_slice(&fs::read(manifest_path)?)?;
        if manifest["all_fits_complete"] != true
            || manifest["code_seal_sha256"] != digest(&fs::read(path)?)
        {
            return Err("missing or incompatible pre-test fit manifest".into());
        }
    }
    Ok(seal)
}

fn run() -> Result<()> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("fixtures") {
        let cells = Cells::new(&base.join("../neural-ecology-ne0c/artifacts/build"))?;
        let fixtures = runner::fixtures(&cells)?;
        runner::save_json(&base.join("artifacts/fixtures.json"), &fixtures)?;
        println!("{fixtures}");
        return Ok(());
    }
    if args.len() != 5 || args[0] != "measure" {
        return Err(
            "usage: ne1 fixtures | measure PERSISTENT.json fit|validation|test OUTPUT SEAL.json"
                .into(),
        );
    }
    let persistent: PersistentCheckpoint = serde_json::from_slice(&fs::read(&args[1])?)?;
    let checkpoint = persistent.reconstruct();
    let split = &args[2];
    if !["fit", "validation", "test"].contains(&split.as_str()) {
        return Err("bad split".into());
    }
    let seal_path = Path::new(&args[4]);
    check_seal(seal_path, checkpoint.seed, split)?;
    let out = Path::new(&args[3]);
    if out.exists() {
        return Err("refusing to overwrite measurement attempt".into());
    }
    fs::create_dir_all(out)?;
    let initial = checkpoint.networks;
    assert!(initial
        .iter()
        .all(|net| net.nodes.iter().all(|node| *node == Node::default())));
    let weights: Vec<_> = initial
        .iter()
        .map(|n| digest(&n.persistent_bytes()))
        .collect();
    let symbols = grammar(
        checkpoint.seed,
        &format!("NE1-v1/{split}"),
        WARM + SAMPLES + 1,
    );
    fs::write(out.join("symbols.u8"), &symbols)?;
    let cells = Cells::new(&base.join("../neural-ecology-ne0c/artifacts/build"))?;
    let modes = [Mode::Reference, Mode::Cpu, Mode::Metal, Mode::Metal];
    let mut worlds = vec![initial.clone(); 4];
    let mut files: Vec<_> = ARMS
        .iter()
        .map(|arm| File::create(out.join(format!("{arm}-full.u16"))).map(BufWriter::new))
        .collect::<std::io::Result<_>>()?;
    let mut logs = BufWriter::new(File::create(out.join("ticks.jsonl"))?);
    let mut activity: Vec<runner::Activity> = (0..3).map(|_| Default::default()).collect();
    let started = Instant::now();
    for t in 0..WARM + SAMPLES {
        let reference = runner::tick(
            &cells,
            modes[0],
            &mut worlds[0],
            &checkpoint.topology,
            symbols[t],
            false,
        )?;
        assert!(reference.deltas.iter().flatten().all(|delta| *delta == 0));
        for k in 1..4 {
            let other = runner::tick(
                &cells,
                modes[k],
                &mut worlds[k],
                &checkpoint.topology,
                symbols[t],
                false,
            )?;
            if worlds[0] != worlds[k]
                || reference.currents != other.currents
                || reference.new_nodes != other.new_nodes
                || reference.deltas != other.deltas
            {
                return Err(format!("backend mismatch at {t}/{k}").into());
            }
        }
        let hashes: Vec<_> = worlds[0]
            .iter()
            .map(|net| digest(&net.nodes.iter().flat_map(|n| n.bytes()).collect::<Vec<_>>()))
            .collect();
        serde_json::to_writer(&mut logs, &json!({"tick":t,"node_hashes":hashes}))?;
        logs.write_all(b"\n")?;
        for a in 0..3 {
            assert_eq!(worlds[0][a].weights, initial[a].weights);
            if t >= WARM {
                activity[a].ticks += 1;
                for (i, node) in worlds[0][a].nodes.iter().enumerate() {
                    activity[a].spikes += u64::from(node.s);
                    activity[a].seen[i] |= node.s != 0;
                }
                for field in 0..4 {
                    for node in &worlds[0][a].nodes {
                        let value = match field {
                            0 => node.v,
                            1 => node.r as u16,
                            2 => node.s as u16,
                            _ => node.q,
                        };
                        files[a].write_all(&value.to_le_bytes())?;
                    }
                }
            }
        }
        if (t + 1) % 1000 == 0 {
            eprintln!(
                "seed={} split={split} ticks={} seconds={:.1}",
                checkpoint.seed,
                t + 1,
                started.elapsed().as_secs_f64()
            );
        }
    }
    for file in &mut files {
        file.flush()?;
    }
    logs.flush()?;
    check_seal(seal_path, checkpoint.seed, split)?;
    let mut artifacts = serde_json::Map::new();
    for arm in ARMS {
        let name = format!("{arm}-full.u16");
        artifacts.insert(name.clone(), json!(digest(&fs::read(out.join(name))?)));
    }
    let summary = json!({"seed":checkpoint.seed,"split":split,"ticks":WARM+SAMPLES,"rows":SAMPLES,
        "reset":true,"backend_agreement":true,"metal_repeat":true,"weights_unchanged":true,
        "weight_hashes":weights,"stream_sha256":digest(&symbols),"feature_sha256":artifacts,
        "activity":activity.iter().map(|x|x.report()).collect::<Vec<_>>(),
        "binary_sha256":digest(&fs::read(env::current_exe()?)?),"seconds":started.elapsed().as_secs_f64()});
    runner::save_json(&out.join("summary.json"), &summary)?;
    println!("{summary}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("NE1 ERROR: {error}");
        std::process::exit(1);
    }
}
