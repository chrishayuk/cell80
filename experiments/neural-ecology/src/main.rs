use ne0::{backend::Cells, model::*, runner, Result};
use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    if let Err(e) = main_result() {
        eprintln!("NE0 ERROR: {e}");
        std::process::exit(1);
    }
}
fn main_result() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err("usage: ne0 fixtures OUT | phase SEED A|B|C OUT dev|eval [SEAL] | smoke SEED OUT | replay INPUT OUT".into());
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if args[0] == "experiment" {
        let status = Command::new("python3")
            .arg(base.join("run.py"))
            .args(&args[1..])
            .status()?;
        if !status.success() {
            return Err(format!("experiment driver exited {status}").into());
        }
        return Ok(());
    }
    let cells = Cells::new(&base.join("artifacts/build"))?;
    match args[0].as_str() {
        "fixtures" => {
            let out = PathBuf::from(args.get(1).ok_or("missing output")?);
            let report = runner::fixtures(&cells)?;
            runner::save_json(&out, &report)?;
            println!("{report}");
        }
        "replay" => {
            let root = PathBuf::from(args.get(1).ok_or("root required")?);
            let result = runner::replay(
                &cells,
                &root,
                args.get(2).ok_or("phase required")?,
                args.get(3).ok_or("variant required")?,
            )?;
            println!("{result}");
        }
        "extract-persistent" => {
            let root = PathBuf::from(args.get(1).ok_or("root required")?);
            let checkpoint = runner::load_checkpoint(&root.join("checkpoints/final-A.json"))?;
            if !(1001..=1003).contains(&checkpoint.seed) {
                return Err("legacy conversion is development-only".into());
            }
            let out = root.join("checkpoints/persistent-A.json");
            if out.exists() {
                return Err("persistent checkpoint already exists".into());
            }
            runner::save_json(&out, &PersistentCheckpoint::capture(&checkpoint))?;
        }
        "phase" | "smoke" => {
            let seed: u64 = args.get(1).ok_or("seed required")?.parse()?;
            let smoke = args[0] == "smoke";
            let phase = if smoke {
                "A"
            } else {
                args.get(2).ok_or("phase required")?.as_str()
            };
            let out = PathBuf::from(
                args.get(if smoke { 2 } else { 3 })
                    .ok_or("output required")?,
            );
            let dev = smoke || args.get(4).ok_or("dev/eval required")? == "dev";
            if dev {
                if !(1001..=1003).contains(&seed) {
                    return Err("development may only use seeds 1001-1003".into());
                }
            } else {
                if !(2001..=2010).contains(&seed) {
                    return Err("evaluation may only use seeds 2001-2010".into());
                }
                let seal: serde_json::Value = serde_json::from_slice(&fs::read(
                    args.get(5)
                        .ok_or("evaluation requires implementation seal")?,
                )?)?;
                let executable = env::current_exe()?;
                if seal["binary_sha256"].as_str()
                    != Some(ne0::digest(&fs::read(executable)?).as_str())
                {
                    return Err("evaluation binary does not match seal".into());
                }
                if seal["evaluation_seeds"] != serde_json::json!((2001..=2010).collect::<Vec<_>>())
                {
                    return Err("seal seed mismatch".into());
                }
                let repo = fs::canonicalize(base.join("../.."))?;
                let seal_path = fs::canonicalize(args.get(5).unwrap())?;
                let relative = seal_path
                    .strip_prefix(&repo)?
                    .to_str()
                    .ok_or("invalid seal path")?;
                let committed = Command::new("git")
                    .current_dir(&repo)
                    .args(["show", &format!("HEAD:{relative}")])
                    .output()?;
                if !committed.status.success() || committed.stdout != fs::read(&seal_path)? {
                    return Err(
                        "implementation seal must be committed, with unchanged contents".into(),
                    );
                }
                for (name, hash) in seal["files"].as_object().ok_or("seal lacks file hashes")? {
                    if hash.as_str() != Some(ne0::digest(&fs::read(repo.join(name))?).as_str()) {
                        return Err(format!("sealed source/dependency changed: {name}").into());
                    }
                }
            }
            if phase == "C" && !out.join("checkpoints/probes.json").exists() {
                return Err("C requires B probes saved first".into());
            }
            let checkpoint = if phase == "A" {
                let net = Network::new(seed);
                Checkpoint {
                    seed,
                    topology: Topology::new(seed),
                    networks: vec![net; 3],
                }
            } else {
                let c = runner::load_persistent(&out.join("checkpoints/persistent-A.json"))?;
                if c.seed != seed {
                    return Err("checkpoint seed mismatch".into());
                }
                c
            };
            let count = if smoke {
                256
            } else if phase == "A" {
                ADAPT
            } else {
                WARM + SAMPLES
            };
            let summary = runner::run_phase(&cells, checkpoint, phase, count, &out, dev)?;
            println!("{summary}");
        }
        _ => return Err("unknown command".into()),
    }
    Ok(())
}
