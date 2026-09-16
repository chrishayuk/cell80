// Fork of sealed NE-0 runner: cadence eligibility and explicit reset branch only.
use crate::{
    backend::{Cells, Mode},
    model::*,
    Result,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    time::Instant,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Activity {
    pub spikes: u64,
    pub ticks: u64,
    pub seen: Vec<bool>,
}
impl Default for Activity {
    fn default() -> Self {
        Self {
            spikes: 0,
            ticks: 0,
            seen: vec![false; N],
        }
    }
}
impl Activity {
    fn add(&mut self, nodes: &[Node]) {
        self.ticks += 1;
        for (i, n) in nodes.iter().enumerate() {
            self.spikes += u64::from(n.s);
            self.seen[i] |= n.s != 0;
        }
    }
    pub fn report(&self) -> serde_json::Value {
        let rate = self.spikes as f64 / (self.ticks * N as u64).max(1) as f64;
        let fraction = self.seen.iter().filter(|&&x| x).count() as f64 / N as f64;
        json!({"ticks":self.ticks,"spike_fraction":rate,"active_unit_fraction":fraction,
            "pass":self.ticks>0 && (0.01..=0.35).contains(&rate) && fraction>=0.5})
    }
}
pub struct Frame {
    pub currents: Vec<i64>,
    pub new_nodes: Vec<Node>,
    pub deltas: Vec<Vec<i16>>,
    pub movement: Vec<[Movement; 2]>,
}

pub fn tick(
    cells: &Cells,
    mode: Mode,
    networks: &mut [Network],
    top: &Topology,
    symbol: u8,
    adapt: bool,
) -> Result<Frame> {
    let old: Vec<_> = networks
        .iter()
        .flat_map(|n| n.nodes.iter().copied())
        .collect();
    let currents: Vec<_> = networks
        .iter()
        .flat_map(|n| n.currents(top, symbol))
        .collect();
    let new_nodes = cells.nodes(mode, &old, &currents)?;
    let plastic = if adapt {
        cells.deltas(mode, &old[N..2 * N], &new_nodes[N..2 * N], &top.sources)?
    } else {
        vec![0; E]
    };
    let yoked = top.permutation.iter().map(|&p| plastic[p]).collect();
    let deltas = vec![vec![0; E], plastic, yoked];
    let mut movement = Vec::with_capacity(3);
    for a in 0..3 {
        movement.push(apply(&mut networks[a], top, &deltas[a]));
        networks[a]
            .nodes
            .copy_from_slice(&new_nodes[a * N..(a + 1) * N]);
    }
    for sign in 0..2 {
        assert_eq!(
            movement[1][sign].requested_hist,
            movement[2][sign].requested_hist
        );
    }
    Ok(Frame {
        currents,
        new_nodes,
        deltas,
        movement,
    })
}
fn same_frame(a: &Frame, b: &Frame) -> bool {
    a.currents == b.currents && a.new_nodes == b.new_nodes && a.deltas == b.deltas
}
fn state_hash(
    top_bytes: &[u8],
    network: &Network,
    phase: &str,
    t: usize,
    symbol: u8,
    deltas: &[i16],
) -> String {
    let mut bytes = Vec::with_capacity(top_bytes.len() + E * 4 + N * 6 + 32);
    bytes.extend_from_slice(b"NE0-state-v1\0");
    bytes.extend_from_slice(top_bytes);
    bytes.extend_from_slice(phase.as_bytes());
    bytes.extend_from_slice(&(t as u64).to_le_bytes());
    bytes.push(symbol);
    for w in &network.weights {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    for n in &network.nodes {
        bytes.extend_from_slice(&n.bytes());
    }
    for d in deltas {
        bytes.extend_from_slice(&d.to_le_bytes());
    }
    crate::digest(&bytes)
}
pub fn save_json(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}
fn peak_rss_bytes() -> Result<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage fills the correctly sized output; read only on success.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let rss = unsafe { usage.assume_init() }.ru_maxrss as u64;
    Ok(if cfg!(target_os = "macos") {
        rss
    } else {
        rss * 1024
    })
}
pub fn load_checkpoint(path: &Path) -> Result<Checkpoint> {
    Ok(serde_json::from_reader(File::open(path)?)?)
}
pub fn load_persistent(path: &Path) -> Result<Checkpoint> {
    let checkpoint: PersistentCheckpoint = serde_json::from_reader(File::open(path)?)?;
    Ok(checkpoint.reconstruct())
}

pub fn run_phase(
    cells: &Cells,
    checkpoint: Checkpoint,
    phase: &str,
    count: usize,
    out: &Path,
    dev: bool,
) -> Result<serde_json::Value> {
    run_with_modes(
        cells,
        checkpoint,
        phase,
        count,
        out,
        dev,
        &[Mode::Reference, Mode::Cpu, Mode::Metal, Mode::Metal],
        true,
        true,
    )
}
fn run_with_modes(
    cells: &Cells,
    checkpoint: Checkpoint,
    phase: &str,
    count: usize,
    out: &Path,
    dev: bool,
    modes: &[Mode],
    learning: bool,
    reset_transients: bool,
) -> Result<serde_json::Value> {
    if !["A", "B", "C"].contains(&phase) {
        return Err("unknown phase".into());
    }
    let seed = checkpoint.seed;
    let binary_sha256 = crate::digest(&fs::read(std::env::current_exe()?)?);
    fs::create_dir_all(out.join("logs"))?;
    fs::create_dir_all(out.join("checkpoints"))?;
    fs::create_dir_all(out.join("datasets"))?;
    let logpath = out.join("logs").join(format!("{phase}-ticks.jsonl"));
    if logpath.exists() {
        return Err(format!("refusing to overwrite phase log {}", logpath.display()).into());
    }
    let mut log = BufWriter::new(File::create(&logpath)?);
    let started = Instant::now();
    let adapt = phase == "A";
    let stream = grammar(seed, phase, count + usize::from(!adapt));
    fs::write(
        out.join("datasets").join(format!("{phase}-symbols.u8")),
        &stream,
    )?;
    let top = &checkpoint.topology;
    let top_bytes = top.bytes();
    let initial = prepare_initial(&checkpoint.networks, reset_transients);
    let initial_node_hashes: Vec<_> = initial
        .iter()
        .map(|n| crate::digest(&n.nodes.iter().flat_map(|n| n.bytes()).collect::<Vec<_>>()))
        .collect();
    let mut worlds = vec![initial.clone(); modes.len()];
    let mut ir_world = initial.clone();
    let mut totals: Vec<[Movement; 2]> = (0..3).map(|_| Default::default()).collect();
    let mut activities: Vec<Activity> = (0..3).map(|_| Default::default()).collect();
    let mut feature_writers = Vec::new();
    if !adapt {
        for arm in ARMS {
            feature_writers.push(BufWriter::new(File::create(
                out.join("datasets").join(format!("{phase}-{arm}-q.u16")),
            )?));
        }
    }
    let mut final_hashes = vec![];
    for t in 0..count {
        let reference = tick(
            cells,
            modes[0],
            &mut worlds[0],
            top,
            stream[t],
            adapt && learning && eligible(t),
        )?;
        let hashes: Vec<_> = (0..3)
            .map(|a| {
                state_hash(
                    &top_bytes,
                    &worlds[0][a],
                    phase,
                    t,
                    stream[t],
                    &reference.deltas[a],
                )
            })
            .collect();
        for k in 1..modes.len() {
            let frame = tick(
                cells,
                modes[k],
                &mut worlds[k],
                top,
                stream[t],
                adapt && learning && eligible(t),
            )?;
            if !same_frame(&reference, &frame) || worlds[0] != worlds[k] {
                return Err(format!(
                    "G0 mismatch seed={seed} phase={phase} tick={t} backend={:?}/{k}",
                    modes[k]
                )
                .into());
            }
            let other: Vec<_> = (0..3)
                .map(|a| {
                    state_hash(
                        &top_bytes,
                        &worlds[k][a],
                        phase,
                        t,
                        stream[t],
                        &frame.deltas[a],
                    )
                })
                .collect();
            if hashes != other {
                return Err("canonical state hash mismatch".into());
            }
        }
        if dev && t < 256 {
            let ir = tick(
                cells,
                Mode::Ir,
                &mut ir_world,
                top,
                stream[t],
                adapt && learning && eligible(t),
            )?;
            if !same_frame(&reference, &ir) || ir_world != worlds[0] {
                return Err(format!("IR phase mismatch at {phase}/{t}").into());
            }
        }
        if !adapt {
            for a in 0..3 {
                assert_eq!(worlds[0][a].weights, initial[a].weights);
                assert!(reference.deltas[a].iter().all(|&d| d == 0));
            }
        }
        for a in 0..3 {
            for s in 0..2 {
                totals[a][s].add(&reference.movement[a][s]);
            }
            if (adapt && t >= count.saturating_sub(10_000)) || (!adapt && t >= WARM) {
                activities[a].add(&worlds[0][a].nodes);
            }
            if !adapt && t >= WARM {
                for node in &worlds[0][a].nodes {
                    feature_writers[a].write_all(&node.q.to_le_bytes())?;
                }
            }
        }
        // Hashes of all phases, plus exact equality above, localize ordering mistakes.
        let current_bytes: Vec<_> = reference
            .currents
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let node_bytes: Vec<_> = reference.new_nodes.iter().flat_map(|n| n.bytes()).collect();
        serde_json::to_writer(
            &mut log,
            &json!({"tick":t,"hashes":hashes,
            "current_hash":crate::digest(&current_bytes),"node_hash":crate::digest(&node_bytes),
            "movement":if adapt{Some(&reference.movement)}else{None}}),
        )?;
        log.write_all(b"\n")?;
        final_hashes = hashes;
        if (t + 1) % 1000 == 0 || t + 1 == count {
            save_json(
                &out.join("checkpoints")
                    .join(format!("{phase}-{:06}.json", t + 1)),
                &Checkpoint {
                    seed,
                    topology: top.clone(),
                    networks: worlds[0].clone(),
                },
            )?;
            log.flush()?;
            eprintln!(
                "seed={seed} phase={phase} ticks={}/{} seconds={:.1}",
                t + 1,
                count,
                started.elapsed().as_secs_f64()
            );
        }
    }
    for w in &mut feature_writers {
        w.flush()?;
    }
    log.flush()?;
    let mut arm_reports = serde_json::Map::new();
    for a in 0..3 {
        let weights = &worlds[0][a].weights;
        let changed = weights
            .iter()
            .zip(&initial[a].weights)
            .filter(|(x, y)| x != y)
            .count();
        let absolute: u64 = weights
            .iter()
            .zip(&initial[a].weights)
            .map(|(&x, &y)| x.abs_diff(y) as u64)
            .sum();
        let endpoints = weights.iter().filter(|&&w| w == 0 || w == 32).count() as f64 / E as f64;
        arm_reports.insert(ARMS[a].into(),json!({"activity":activities[a].report(),"movement":totals[a],
            "endpoint_fraction":endpoints,"endpoint_pass":endpoints<=0.25,
            "changed_fraction":changed as f64/E as f64,"mean_absolute_change":absolute as f64/E as f64,
            "adaptation_pass":changed as f64/E as f64>=0.01&&absolute as f64/E as f64>=0.10,
            "persistent_hash":crate::digest(&worlds[0][a].persistent_bytes())}));
    }
    let summary = json!({"seed":seed,"phase":phase,"ticks":count,"seconds":started.elapsed().as_secs_f64(),
        "development":dev,"backend_agreement":true,"metal_replay":modes.iter().filter(|m|matches!(m,Mode::Metal)).count()>=2,"reset_identity":reset_transients,"transient_reset":reset_transients,"cadence":4096,"update_opportunities":if adapt&&learning{count/4096}else{0},
        "ir_phase_checks":if dev{count.min(256)}else{0},"topology_hash":crate::digest(&top_bytes),
        "stream_hash":crate::digest(&stream),"final_hashes":final_hashes,"arms":arm_reports,
        "initial_node_hashes":initial_node_hashes,
        "peak_process_rss_bytes":peak_rss_bytes()?,
        "binary_sha256":binary_sha256});
    save_json(
        &out.join("logs").join(format!("{phase}-summary.json")),
        &summary,
    )?;
    if adapt {
        let checkpoint = Checkpoint {
            seed,
            topology: top.clone(),
            networks: worlds[0].clone(),
        };
        save_json(
            &out.join("checkpoints/persistent-A.json"),
            &PersistentCheckpoint::capture(&checkpoint),
        )?;
        save_json(
            &out.join("checkpoints/final-A.json"),
            &Checkpoint {
                seed,
                topology: top.clone(),
                networks: worlds[0].clone(),
            },
        )?;
    }
    Ok(summary)
}

/// Fresh-process diagnostic entry. Original phase logs must exist; diagnostics
/// cannot open previously unseen evaluation data or overwrite primary outputs.
pub fn replay(cells: &Cells, root: &Path, phase: &str, variant: &str) -> Result<serde_json::Value> {
    let primary: serde_json::Value = serde_json::from_reader(File::open(
        root.join("logs").join(format!("{phase}-summary.json")),
    )?)?;
    if !["persistent", "restored", "disabled"].contains(&variant) {
        return Err("unknown replay variant".into());
    }
    if (variant == "disabled") != (phase == "A") {
        return Err("disabled replay is A only; others B/C only".into());
    }
    let mut checkpoint = load_persistent(&root.join("checkpoints/persistent-A.json"))?;
    if variant == "restored" {
        checkpoint.networks[1].weights = Network::new(checkpoint.seed).weights;
    }
    if variant == "disabled" {
        checkpoint.networks = vec![Network::new(checkpoint.seed); 3];
    }
    let output = root.join("diagnostics").join(format!("{variant}-{phase}"));
    let count = primary["ticks"].as_u64().ok_or("missing count")? as usize;
    run_with_modes(
        cells,
        checkpoint,
        phase,
        count,
        &output,
        false,
        &[Mode::Reference, Mode::Cpu],
        variant != "disabled",
        true,
    )?;
    let expected = BufReader::new(File::open(
        root.join("logs").join(format!("{phase}-ticks.jsonl")),
    )?);
    let actual = BufReader::new(File::open(
        output.join("logs").join(format!("{phase}-ticks.jsonl")),
    )?);
    let mut checked = 0usize;
    let mut actual = actual.lines();
    for line in expected.lines() {
        let a: serde_json::Value = serde_json::from_str(&line?)?;
        let b: serde_json::Value =
            serde_json::from_str(&actual.next().ok_or("short diagnostic history")??)?;
        if variant == "persistent" {
            assert_eq!(a["hashes"], b["hashes"]);
        } else {
            assert_eq!(a["hashes"][0], b["hashes"][1]);
        }
        checked += 1;
    }
    assert!(actual.next().is_none());
    assert_eq!(checked, count);
    if phase != "A" {
        let pairs: Vec<_> = if variant == "persistent" {
            ARMS.iter().map(|&a| (a, a)).collect()
        } else {
            vec![("Frozen", "Plastic")]
        };
        for (a, b) in pairs {
            let lhs = fs::read(root.join("datasets").join(format!("{phase}-{a}-q.u16")))?;
            let rhs = fs::read(output.join("datasets").join(format!("{phase}-{b}-q.u16")))?;
            assert_eq!(lhs, rhs, "replay feature bytes");
        }
    }
    let result = json!({"phase":phase,"variant":variant,"pass":true,"compared_ticks":checked,"fresh_process":true});
    save_json(
        &root
            .join("logs")
            .join(format!("replay-{variant}-{phase}.json")),
        &result,
    )?;
    let names = [
        "persistent-B",
        "persistent-C",
        "restored-B",
        "restored-C",
        "disabled-A",
    ];
    let complete = names.iter().all(|name| {
        root.join("logs")
            .join(format!("replay-{name}.json"))
            .exists()
    });
    if complete {
        save_json(
            &root.join("logs/persistence.json"),
            &json!({"pass":true,"checks":names}),
        )?;
    }
    Ok(result)
}

pub fn fixtures(cells: &Cells) -> Result<serde_json::Value> {
    let mut cases = Vec::new();
    for v in [0, 1, 63, 64, 127, 255] {
        for q in [0, 1, 63, 64, 254, 255] {
            for r in [0, 1] {
                for current in [-512, -1, 0, 1, 48, 64, 560] {
                    cases.push((Node { v, q, r, s: 0 }, current));
                }
            }
        }
    }
    let nodes: Vec<_> = cases.iter().map(|(n, _)| *n).collect();
    let currents: Vec<_> = cases.iter().map(|(_, i)| *i).collect();
    let inputs: Vec<_> = currents.iter().map(|&i| [i as i16 as u16, 0, 0]).collect();
    let bytes: Vec<_> = nodes.iter().flat_map(|n| n.abi_bytes()).collect();
    let ir = cells.node.run(Mode::Ir, &inputs, &bytes)?;
    for mode in [Mode::Cpu, Mode::Metal, Mode::Metal] {
        let got = cells.node.run(mode, &inputs, &bytes)?;
        if got != ir {
            return Err(format!("node fixture sextet/state/step mismatch {mode:?}").into());
        }
    }
    for (i, b) in ir.1.chunks_exact(8).enumerate() {
        assert_eq!(Node::from_abi(b), reference_node(nodes[i], currents[i]));
    }
    // Hand-calculated cases independent of the exhaustive corner enumeration.
    assert_eq!(
        reference_node(
            Node {
                v: 0,
                q: 0,
                r: 0,
                s: 0
            },
            64
        ),
        Node {
            v: 0,
            q: 64,
            r: 1,
            s: 1
        }
    );
    assert_eq!(
        reference_node(
            Node {
                v: 255,
                q: 255,
                r: 1,
                s: 1
            },
            560
        ),
        Node {
            v: 0,
            q: 239,
            r: 0,
            s: 0
        }
    );
    assert_eq!(
        reference_node(
            Node {
                v: 0,
                q: 255,
                r: 0,
                s: 0
            },
            64
        )
        .q,
        255
    );
    assert_eq!(
        reference_node(
            Node {
                v: 1,
                q: 0,
                r: 0,
                s: 0
            },
            -1
        )
        .v,
        0
    );
    let mut edge_inputs = Vec::new();
    for pre in [0, 1, 63, 64, 65, 127, 255] {
        for post in [0, 1, 63, 64, 65, 127, 255] {
            for flags in 0..4 {
                edge_inputs.push([pre, post, flags]);
            }
        }
    }
    let edge_ir = cells.edge.run(Mode::Ir, &edge_inputs, &[])?;
    for mode in [Mode::Cpu, Mode::Metal, Mode::Metal] {
        assert_eq!(cells.edge.run(mode, &edge_inputs, &[])?, edge_ir);
    }
    for (input, row) in edge_inputs.iter().zip(&edge_ir.0) {
        let pre = Node {
            q: input[0],
            s: (input[2] & 1) as u8,
            ..Default::default()
        };
        let post = Node {
            q: input[1],
            ..Default::default()
        };
        let new = Node {
            s: ((input[2] >> 1) & 1) as u8,
            ..Default::default()
        };
        assert_eq!(
            row[0] as i16,
            reference_delta(pre, post, new),
            "edge input {input:?}"
        );
    }
    let hashbytes: Vec<_> =
        ir.0.iter()
            .chain(&edge_ir.0)
            .flat_map(|row| row.iter().flat_map(|x| x.to_le_bytes()))
            .collect();
    Ok(
        json!({"node_cases":cases.len(),"edge_cases":edge_inputs.len(),"sextets_and_steps_equal":true,
        "node_state_equal":true,"metal_repeat_equal":true,"fixture_hash":crate::digest(&hashbytes),
        "node_source_hash":crate::digest(crate::backend::NODE_SOURCE.as_bytes()),
        "plasticity_source_hash":crate::digest(crate::backend::EDGE_SOURCE.as_bytes())}),
    )
}

pub fn eligible(t: usize) -> bool {
    (t + 1) % 4096 == 0
}

pub fn prepare_initial(networks: &[Network], reset: bool) -> Vec<Network> {
    let mut initial = networks.to_vec();
    if reset {
        for net in &mut initial {
            let weights = net.persistent_bytes();
            net.reset();
            assert_eq!(weights, net.persistent_bytes());
            let once = net.clone();
            net.reset();
            assert_eq!(once, *net);
            assert!(net.nodes.iter().all(|&n| n == Node::default()));
        }
    }
    initial
}

pub fn run_continuation(
    cells: &Cells,
    checkpoint: Checkpoint,
    phase: &str,
    out: &Path,
    dev: bool,
) -> Result<serde_json::Value> {
    if !["B", "C"].contains(&phase) {
        return Err("continuation is B/C only".into());
    }
    run_with_modes(
        cells,
        checkpoint,
        phase,
        WARM + SAMPLES,
        out,
        dev,
        &[Mode::Reference, Mode::Cpu, Mode::Metal, Mode::Metal],
        false,
        false,
    )
}

#[cfg(test)]
mod ne0c_tests {
    use super::*;
    #[test]
    fn cadence_has_twelve_eligible_ticks() {
        assert_eq!(
            (0..ADAPT).filter(|&t| eligible(t)).collect::<Vec<_>>(),
            (1..=12).map(|i| i * 4096 - 1).collect::<Vec<_>>()
        );
        assert!(!eligible(0));
        assert!(!eligible(4096));
    }
    #[test]
    fn primary_resets_secondary_preserves_and_both_preserve_weights() {
        let mut n = Network {
            weights: vec![17; E],
            nodes: vec![
                Node {
                    v: 31,
                    r: 1,
                    s: 1,
                    q: 255
                };
                N
            ],
        };
        n.weights[0] = 0;
        let inputs = vec![n.clone(); 3];
        let primary = prepare_initial(&inputs, true);
        let continued = prepare_initial(&inputs, false);
        assert_eq!(continued, inputs);
        for net in primary {
            assert_eq!(net.weights, n.weights);
            assert!(net.nodes.iter().all(|&x| x == Node::default()));
        }
        assert_eq!(inputs[0], n);
    }
}
