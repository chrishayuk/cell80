use ne0c::{backend::Cells, model::*, runner, Result};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn verify_seal(path: &Path) -> Result<()> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = fs::canonicalize(base.join("../.."))?;
    let bytes = fs::read(path)?;
    let s: serde_json::Value = serde_json::from_slice(&bytes)?;
    let full = fs::canonicalize(path)?;
    let relative = full.strip_prefix(&repo)?;
    let committed = Command::new("git")
        .current_dir(&repo)
        .args(["show", &format!("HEAD:{}", relative.display())])
        .output()?;
    if !committed.status.success() || committed.stdout != bytes {
        return Err("seal must match committed bytes".into());
    }
    if s["evaluation_seeds"] != serde_json::json!((4001..=4010).collect::<Vec<_>>()) {
        return Err("wrong evaluation cohort".into());
    }
    if s["binary_sha256"] != ne0c::digest(&fs::read(env::current_exe()?)?) {
        return Err("binary differs from seal".into());
    }
    for (p, h) in s["files"].as_object().ok_or("missing sealed files")? {
        if h.as_str() != Some(ne0c::digest(&fs::read(repo.join(p))?).as_str()) {
            return Err(format!("sealed file changed: {p}").into());
        }
    }
    Ok(())
}

fn main_result() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if args.is_empty() {
        return Err("usage: ne0c fixtures | phase SEED A|B|C ROOT dev|eval [SEAL] | continuation SEED B|C ROOT dev|eval [SEAL] | replay ROOT PHASE VARIANT".into());
    }
    if args[0] == "replay" {
        let cells = Cells::new(&base.join("artifacts/build"))?;
        let r = runner::replay(
            &cells,
            Path::new(args.get(1).ok_or("root")?),
            args.get(2).ok_or("phase")?,
            args.get(3).ok_or("variant")?,
        )?;
        println!("{r}");
        return Ok(());
    }
    if args[0] == "fixtures" {
        let cells = Cells::new(&base.join("artifacts/build"))?;
        let f = runner::fixtures(&cells)?;
        runner::save_json(&base.join("artifacts/fixtures.json"), &f)?;
        println!("{f}");
        return Ok(());
    }
    if !["phase", "continuation"].contains(&args[0].as_str()) {
        return Err("unknown command".into());
    }
    let seed: u64 = args.get(1).ok_or("seed")?.parse()?;
    let phase = args.get(2).ok_or("phase")?;
    let root = PathBuf::from(args.get(3).ok_or("root")?);
    let mode = args.get(4).ok_or("dev/eval")?;
    let dev = match mode.as_str() {
        "dev" => true,
        "eval" => false,
        _ => return Err("dev/eval required".into()),
    };
    if dev {
        if seed != 3001 {
            return Err("engineering validation is seed 3001 only".into());
        }
    } else {
        if !(4001..=4010).contains(&seed) {
            return Err("evaluation seeds are 4001-4010".into());
        }
        verify_seal(Path::new(args.get(5).ok_or("evaluation seal required")?))?;
    }
    let continued = args[0] == "continuation";
    let out = if continued {
        root.join("diagnostics/continuation")
    } else {
        root.clone()
    };
    if phase == "C" && !out.join("checkpoints/probes.json").exists() {
        return Err("C requires frozen B probes".into());
    }
    let checkpoint = if continued {
        if !root.join("logs/result.json").exists() || !root.join("logs/persistence.json").exists() {
            return Err("continuation follows primary C and persistence checks".into());
        }
        runner::load_checkpoint(&root.join("checkpoints/final-A.json"))?
    } else if phase == "A" {
        Checkpoint {
            seed,
            topology: Topology::new(seed),
            networks: vec![Network::new(seed); 3],
        }
    } else {
        runner::load_persistent(&root.join("checkpoints/persistent-A.json"))?
    };
    if checkpoint.seed != seed {
        return Err("checkpoint seed mismatch".into());
    }
    let cells = Cells::new(&base.join("artifacts/build"))?;
    let summary = if continued {
        runner::run_continuation(&cells, checkpoint, phase, &out, dev)?
    } else {
        runner::run_phase(
            &cells,
            checkpoint,
            phase,
            if phase == "A" { ADAPT } else { WARM + SAMPLES },
            &out,
            dev,
        )?
    };
    if !dev {
        verify_seal(Path::new(&args[5]))?;
    }
    println!("{summary}");
    Ok(())
}

fn main() {
    if let Err(e) = main_result() {
        eprintln!("NE0c ERROR: {e}");
        std::process::exit(1);
    }
}
