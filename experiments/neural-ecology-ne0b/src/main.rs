use ne0::{
    backend::{Cells, Mode},
    model::*,
    runner::{self, Frame},
    Result,
};
use serde_json::{json, Value};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};

const INTERVALS: [usize; 4] = [1, 16, 256, 4096];

fn eligible(t: usize, k: usize) -> bool {
    (t + 1) % k == 0
}

#[derive(Default)]
struct Activity {
    ticks: u64,
    spikes: u64,
    seen: Vec<bool>,
}
impl Activity {
    fn add(&mut self, n: &Network) {
        if self.seen.is_empty() {
            self.seen = vec![false; N];
        }
        self.ticks += 1;
        for (i, node) in n.nodes.iter().enumerate() {
            self.spikes += u64::from(node.s);
            self.seen[i] |= node.s != 0;
        }
    }
    fn report(&self) -> Value {
        let rate = self.spikes as f64 / (self.ticks * N as u64) as f64;
        let active = self.seen.iter().filter(|&&x| x).count() as f64 / N as f64;
        json!({"ticks":self.ticks, "spike_fraction":rate, "active_unit_fraction":active,
            "pass": self.ticks == 10_000 && (0.01..=0.35).contains(&rate) && active >= 0.5})
    }
}

fn same_frame(a: &Frame, b: &Frame) -> bool {
    a.currents == b.currents && a.new_nodes == b.new_nodes && a.deltas == b.deltas
}

fn state_hash(n: &Network, delta: &[i16], t: usize, symbol: u8, phase: &str) -> String {
    let mut bytes = Vec::with_capacity(E * 4 + N * 6 + 32);
    bytes.extend_from_slice(b"NE0b-dev-state-v1\0");
    bytes.extend_from_slice(phase.as_bytes());
    bytes.extend_from_slice(&(t as u64).to_le_bytes());
    bytes.push(symbol);
    bytes.extend_from_slice(&n.persistent_bytes());
    for node in &n.nodes {
        bytes.extend_from_slice(&node.bytes());
    }
    for d in delta {
        bytes.extend_from_slice(&d.to_le_bytes());
    }
    ne0::digest(&bytes)
}

fn reset(checkpoint: Checkpoint) -> Result<Checkpoint> {
    let persistent = PersistentCheckpoint::capture(&checkpoint);
    let bytes = serde_json::to_vec(&persistent)?;
    let decoded: PersistentCheckpoint = serde_json::from_slice(&bytes)?;
    let mut reset = decoded.reconstruct();
    assert_eq!(checkpoint.topology, reset.topology);
    for (old, new) in checkpoint.networks.iter().zip(&mut reset.networks) {
        assert_eq!(old.weights, new.weights);
        assert!(new.nodes.iter().all(|&n| n == Node::default()));
        let before = new.clone();
        new.reset();
        new.reset();
        assert_eq!(*new, before);
    }
    Ok(reset)
}

fn phase(
    cells: &Cells,
    mut worlds: Vec<Vec<Network>>,
    top: &Topology,
    seed: u64,
    k: usize,
    name: &str,
    out: &Path,
    initial: &Network,
) -> Result<(Vec<Vec<Network>>, Value)> {
    let started = Instant::now();
    let count = if name == "A" { ADAPT } else { WARM + SAMPLES };
    let symbols = grammar(seed, if name == "A" { "A" } else { "NE0B-R" }, count);
    fs::write(out.join(format!("{name}-symbols.u8")), &symbols)?;
    let mut log = BufWriter::new(File::create(out.join(format!("{name}-ticks.jsonl")))?);
    let mut frozen_history = String::with_capacity(count * 65);
    let mut activity: [Activity; 3] = Default::default();
    let mut movement: Vec<[Movement; 2]> = vec![Default::default(); 3];
    let weights_before: Vec<_> = worlds[0].iter().map(|n| n.weights.clone()).collect();
    let mut opportunities = 0;
    for (t, &symbol) in symbols.iter().enumerate() {
        let adapt = name == "A" && eligible(t, k);
        opportunities += usize::from(adapt);
        let expected = runner::tick(cells, Mode::Reference, &mut worlds[0], top, symbol, adapt)?;
        for (i, mode) in [Mode::Cpu, Mode::Metal, Mode::Metal]
            .into_iter()
            .enumerate()
        {
            let actual = runner::tick(cells, mode, &mut worlds[i + 1], top, symbol, adapt)?;
            assert!(
                same_frame(&expected, &actual),
                "frame mismatch seed={seed} K={k} phase={name} tick={t} mode={mode:?}"
            );
            assert_eq!(
                worlds[0],
                worlds[i + 1],
                "committed network mismatch at tick {t}"
            );
        }
        for a in 0..3 {
            for sign in 0..2 {
                movement[a][sign].add(&expected.movement[a][sign]);
            }
            if (name == "A" && t >= ADAPT - SAMPLES) || (name == "R" && t >= WARM) {
                activity[a].add(&worlds[0][a]);
            }
            if name == "R" {
                assert_eq!(worlds[0][a].weights, weights_before[a]);
            }
        }
        let hashes: Vec<_> = worlds[0]
            .iter()
            .zip(&expected.deltas)
            .map(|(n, d)| state_hash(n, d, t, symbol, name))
            .collect();
        frozen_history.push_str(&hashes[0]);
        frozen_history.push('\n');
        serde_json::to_writer(&mut log, &json!({"tick":t,"hashes":hashes}))?;
        log.write_all(b"\n")?;
        if (t + 1) % 10_000 == 0 || t + 1 == count {
            runner::save_json(
                &out.join(format!("{name}-{:06}.json", t + 1)),
                &Checkpoint {
                    seed,
                    topology: top.clone(),
                    networks: worlds[0].clone(),
                },
            )?;
            eprintln!(
                "seed={seed} K={k} phase={name} ticks={}/{} seconds={:.1}",
                t + 1,
                count,
                started.elapsed().as_secs_f64()
            );
        }
    }
    log.flush()?;
    let mut arms = serde_json::Map::new();
    for a in 0..3 {
        let n = &worlds[0][a];
        let endpoints = n.weights.iter().filter(|&&w| w == 0 || w == 32).count() as f64 / E as f64;
        let changed = n
            .weights
            .iter()
            .zip(&initial.weights)
            .filter(|(w, i)| w != i)
            .count() as f64
            / E as f64;
        let mad = n
            .weights
            .iter()
            .zip(&initial.weights)
            .map(|(&w, &i)| w.abs_diff(i) as u64)
            .sum::<u64>() as f64
            / E as f64;
        arms.insert(
            ARMS[a].into(),
            json!({"activity":activity[a].report(),
            "endpoint_fraction":endpoints,"endpoint_pass":endpoints<=0.25,
            "changed_fraction":changed,"mean_absolute_change":mad,
            "adaptation_pass":changed>=0.01 && mad>=0.1,"movement":movement[a],
            "persistent_hash":ne0::digest(&n.persistent_bytes())}),
        );
    }
    let report = json!({"seed":seed,"interval":k,"phase":name,"ticks":count,
        "update_opportunities":opportunities,"backend_agreement":true,"reset_identity":true,
        "binary_sha256":ne0::digest(&fs::read(env::current_exe()?)?),
        "topology_hash":ne0::digest(&top.bytes()),"stream_hash":ne0::digest(&symbols),
        "frozen_history_hash":ne0::digest(frozen_history.as_bytes()),
        "seconds":started.elapsed().as_secs_f64(),"arms":arms});
    runner::save_json(&out.join(format!("{name}-summary.json")), &report)?;
    Ok((worlds, report))
}

fn verify_seal(path: &Path) -> Result<()> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = fs::canonicalize(base.join("../.."))?;
    let bytes = fs::read(path)?;
    let seal: Value = serde_json::from_slice(&bytes)?;
    let relative = fs::canonicalize(path)?.strip_prefix(&repo)?.to_path_buf();
    let committed = std::process::Command::new("git")
        .current_dir(&repo)
        .args(["show", &format!("HEAD:{}", relative.display())])
        .output()?;
    if !committed.status.success() || committed.stdout != bytes {
        return Err("seal must match committed bytes".into());
    }
    for (p, h) in seal["files"].as_object().ok_or("missing sealed files")? {
        if Some(ne0::digest(&fs::read(repo.join(p))?).as_str()) != h.as_str() {
            return Err(format!("sealed file changed: {p}").into());
        }
    }
    if seal["binary_sha256"] != ne0::digest(&fs::read(env::current_exe()?)?) {
        return Err("binary differs from seal".into());
    }
    Ok(())
}

fn main_result() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    if args.first().map(String::as_str) == Some("fixtures") {
        let cells = Cells::new(&base.join("artifacts/build"))?;
        let f = runner::fixtures(&cells)?;
        runner::save_json(&base.join("artifacts/fixtures.json"), &f)?;
        println!("{f}");
        return Ok(());
    }
    if args.len() != 5 || args[0] != "run" {
        return Err("usage: ne0b fixtures | run SEED INTERVAL OUTPUT SEAL".into());
    }
    let seed: u64 = args[1].parse()?;
    let k: usize = args[2].parse()?;
    if !(3001..=3003).contains(&seed) || !INTERVALS.contains(&k) {
        return Err("only the fixed development grid is allowed".into());
    }
    let seal = Path::new(&args[4]);
    verify_seal(seal)?;
    let out = Path::new(&args[3]);
    fs::create_dir(out)?; // Refuse to overwrite any existing attempt.
    let mut marker = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(out.join("started.json"))?;
    writeln!(
        marker,
        "{}",
        json!({"seed":seed,"interval":k,"seal_sha256":ne0::digest(&fs::read(seal)?)})
    )?;
    let cells = Cells::new(&base.join("artifacts/build"))?;
    let top = Topology::new(seed);
    let initial = Network::new(seed);
    let initial_checkpoint = Checkpoint {
        seed,
        topology: top.clone(),
        networks: vec![initial.clone(); 3],
    };
    runner::save_json(&out.join("initial.json"), &initial_checkpoint)?;
    let worlds = vec![initial_checkpoint.networks.clone(); 4];
    let (worlds, a) = phase(&cells, worlds, &top, seed, k, "A", out, &initial)?;
    runner::save_json(
        &out.join("persistent-A.json"),
        &PersistentCheckpoint::capture(&Checkpoint {
            seed,
            topology: top.clone(),
            networks: worlds[0].clone(),
        }),
    )?;
    let mut fresh = Vec::new();
    for networks in worlds {
        fresh.push(
            reset(Checkpoint {
                seed,
                topology: top.clone(),
                networks,
            })?
            .networks,
        );
    }
    let (_, r) = phase(&cells, fresh, &top, seed, k, "R", out, &initial)?;
    verify_seal(seal)?;
    runner::save_json(
        &out.join("result.json"),
        &json!({"seed":seed,"interval":k,
        "development_only":true,"A":a,"R":r}),
    )?;
    Ok(())
}

fn main() {
    if let Err(e) = main_result() {
        eprintln!("NE0b ERROR: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cadence_boundaries_and_budgets() {
        for (k, n) in [(1, 50_000), (16, 3_125), (256, 195), (4096, 12)] {
            assert_eq!((0..ADAPT).filter(|&t| eligible(t, k)).count(), n);
            assert!(eligible(k - 1, k));
            if k > 1 {
                assert!(!eligible(k - 2, k));
                assert!(!eligible(k, k));
            }
        }
    }
    #[test]
    fn persistent_reconstruction_discards_transients() {
        let mut network = Network::new(3001);
        network.weights[0] = 32;
        network.nodes[0] = Node {
            v: 55,
            r: 1,
            s: 1,
            q: 255,
        };
        let topology = Topology {
            sources: vec![0; E],
            permutation: (0..E).collect(),
        };
        let c = Checkpoint {
            seed: 3001,
            topology,
            networks: vec![network.clone(); 3],
        };
        let r = reset(c).unwrap();
        assert_eq!(r.networks[1].weights, network.weights);
        assert!(r.networks[1].nodes.iter().all(|&n| n == Node::default()));
    }
    #[test]
    fn equality_checks_include_currents_and_deltas() {
        let frame = || Frame {
            currents: vec![1],
            new_nodes: vec![Node::default()],
            deltas: vec![vec![1]],
            movement: vec![],
        };
        let a = frame();
        let mut b = frame();
        assert!(same_frame(&a, &b));
        b.currents[0] = 2;
        assert!(!same_frame(&a, &b));
        b = frame();
        b.deltas[0][0] = -1;
        assert!(!same_frame(&a, &b));
    }
}
