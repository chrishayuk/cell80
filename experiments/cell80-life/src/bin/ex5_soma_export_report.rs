//! EX-5: SOMA hand-off (`experiments/deterministic-ecology.md`) — does the population
//! substrate serve SOMA's creature-raiser? Not a biology claim: scoping/interface only. Runs
//! a real EX-3 flagship simulation, picks one surviving organism, and hash-attests + proves
//! behavioral identity for every cell its resolved genome touches — Z80 vs RV32 (the robot's
//! target ISA, per `docs/13-multi-target-spec.md`) vs the CPU-reference interpreter already
//! proven throughout EX-0–EX-4.
//!
//! **Not macOS-gated**: `Rv32Runner`/`rustrv32::run_cell` is a pure-Rust RISC-V executor, not
//! Metal-backed, so the core Z80/RV32/CPU-reference proof runs on any platform. The GPU
//! cross-check is a macOS-only bonus, reported separately and never gating the pass/fail
//! verdict — that verdict is specifically "behaviorally identical on the robot's ISA."
//!
//! **Scope, stated plainly (per the design doc's own "scoping/interface only, not a biology
//! claim" framing)**: an organism's genome is a tuple of 6 gene-cell choices (3 fixed:
//! decay/eat/split; 3 evolved: hungry_promoter/repro_promoter/sense_move), not literally one
//! `.cell` file. This attests each of the 6 independently, tied together by the organism's
//! `GenomeFields::hash()` (EX-4's lineage content-address) as a single genome digest — not a
//! single composed whole-organism RV32 program (that would require folding the tick
//! engine's host-orchestrated control flow into one cell, a real redesign, not a prototype).
//! Cycle counts are not reported here — the RV32 cycle table stays provisional until the
//! RP2350 `mcycle` co-sign (B4), per `rv32.rs`'s own module doc.
use std::fs;
use std::path::{Path, PathBuf};

use cell80::{
    Cartridge, CartridgeOpts, CellConfig, Runner, Rv32Runner, DEFAULT_PROBES, RV32_TARGET,
    Z80_CELL_TARGET,
};
use cell80_life::ex2::GenePools;
use cell80_life::ex3::{self, RunConfig3, StartingGenome3};
// The qualified `genes::` path is only used inside the macOS GPU block below.
#[cfg(target_os = "macos")]
use cell80_life::genes;
use cell80_life::genes::{CompiledGene, EngineKind};
use cell80_life::history::Species;
use cell80_life::lineage::GenomeFields;
use cell80_life::load_starting_genome;
use cell80_life::pools::{discover_pools, Pools};

fn cells_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
}

fn genome_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("genomes/{name}.json"))
}

fn load_starting3(name: &str, species: Species, role_pools: &Pools) -> StartingGenome3 {
    let starting = load_starting_genome(&genome_path(name));
    StartingGenome3 {
        species,
        initial_energy: starting.initial_energy,
        decay_amount: starting.decay_amount,
        repro_threshold: starting.repro_threshold,
        repro_give_pct: starting.repro_give_pct,
        hungry_promoter: role_pools.promoter_index(&starting.genes.hungry_promoter),
        repro_promoter: role_pools.promoter_index(&starting.genes.repro_promoter),
        sense_move: role_pools.movement_index(&starting.genes.sense_move),
    }
}

/// Compile both machine bodies from `name`'s real disk source, hash-attest them exactly
/// `cell80/tests/cartridge_v10.rs`'s proven pattern, and confirm Z80/CPU-reference == RV32
/// over `DEFAULT_PROBES` (sliced to `arity`). `compiled` is the same `CompiledGene` the
/// ecology engine itself runs (for the macOS-only GPU bonus cross-check).
fn attest_cell(role: &str, name: &str, arity: usize, compiled: &CompiledGene) -> bool {
    let path = cell80::find_cell_file(&cells_dir(), name).unwrap_or_else(|e| panic!("{e}"));
    let src =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let id = format!("ex5.{role}.{name}");

    let z80 = Cartridge::compile(
        &src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(id.clone()),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("z80 compile of `{name}`: {e}"));
    let rv32 = Cartridge::compile_rv32(
        &src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(id),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("rv32 compile of `{name}`: {e}"));

    assert_eq!(z80.manifest.target, Z80_CELL_TARGET);
    assert_eq!(rv32.manifest.target, RV32_TARGET);
    let family_match = z80.manifest.family_hash == rv32.manifest.family_hash;
    let artifact_differs = z80.artifact_hash() != rv32.artifact_hash();

    let back =
        Cartridge::from_bytes(&rv32.to_bytes()).unwrap_or_else(|e| panic!("rv32 round-trip: {e}"));
    let roundtrip_ok = back.manifest.target == RV32_TARGET
        && back.manifest.family_hash == rv32.manifest.family_hash;

    let mut zr = Runner::new(z80.z80().expect("z80 body"));
    let rr = Rv32Runner::load(&back).expect("rv32 runner");

    let mut agree = true;
    let mut n = 0u32;
    for probe in DEFAULT_PROBES {
        let args16 = &probe[..arity];
        let z_rep = zr
            .run(None, args16, 1_000_000)
            .unwrap_or_else(|e| panic!("z80 run: {e}"));
        let args32: Vec<u32> = args16.iter().map(|&v| v as u32).collect();
        let r_rep = rr
            .run(&args32, &[], 1_000_000)
            .unwrap_or_else(|e| panic!("rv32 run: {e}"));
        n += 1;
        if z_rep.result as u32 != r_rep.result {
            agree = false;
            println!(
                "      MISMATCH on {args16:?}: z80={} rv32={}",
                z_rep.result, r_rep.result
            );
        }
    }

    let ok = family_match && artifact_differs && roundtrip_ok && agree;
    println!(
        "  {role:<16} {name:<20} family_hash_match={family_match:<5} artifact_hash_differs={artifact_differs:<5} \
         roundtrip_ok={roundtrip_ok:<5} agree({n} probes)={agree:<5}  [{}]",
        if ok { "OK" } else { "FAIL" }
    );

    #[cfg(target_os = "macos")]
    {
        let probes: Vec<[u16; 3]> = DEFAULT_PROBES.to_vec();
        let gpu_out = genes::batch_run(EngineKind::Gpu, compiled, &probes);
        let cpu_out = genes::batch_run(EngineKind::CpuReference, compiled, &probes);
        let gpu_agrees = gpu_out.iter().zip(&cpu_out).all(|(g, c)| g.0 == c.0);
        println!(
            "      [macOS bonus] GPU == CPU-reference interpreter over {} probes: {gpu_agrees}",
            probes.len()
        );
    }
    #[cfg(not(target_os = "macos"))]
    let _ = compiled;

    ok
}

fn main() {
    let role_pools = discover_pools(&cells_dir());
    let grazer = load_starting3("grazer", Species::Grazer, &role_pools);
    let predator = load_starting3("predator", Species::Predator, &role_pools);
    let grazer_disk = load_starting_genome(&genome_path("grazer"));
    let genes = GenePools::load(
        &cells_dir(),
        &grazer_disk.genes.decay,
        &grazer_disk.genes.eat,
        &grazer_disk.genes.split,
        &role_pools,
    )
    .expect("compiling gene pools");

    // The calibrated, satiation-mechanic-verified flagship config (ex3_predator_prey_report.rs
    // parts 1-3): 10/10 seeds sustained coexistence here, including this exact seed.
    let cfg = RunConfig3 {
        seed: 42,
        ticks: 3_000,
        initial_grazers: 60,
        initial_predators: 10,
        world_width: 48,
        world_height: 48,
        food_density: 0.3,
        food_value: 40,
        regrow_ticks: 8,
        mutation_enabled: true,
        predator_satiation_ticks: 20,
    };

    println!("== EX-5: running a real EX-3 flagship simulation to pick one evolved organism ==");
    let out = ex3::run(EngineKind::CpuReference, &cfg, &grazer, &predator, &genes);
    println!(
        "  {} ticks, final: grazers={} predators={}, total_predation_kills={}",
        out.ticks.len(),
        out.final_grazers,
        out.final_predators,
        out.total_predation_kills
    );

    let Some(final_tick) = out.ticks.last() else {
        println!("world went extinct before tick 0 — nothing to export.");
        return;
    };
    let Some(organism) = final_tick
        .organisms
        .iter()
        .find(|o| o.species == Species::Predator)
    else {
        println!("no surviving predator this seed/config — EX-5 needs one; try a different seed.");
        return;
    };

    println!(
        "\npicked organism id={} (species=Predator, energy={}) at tick {}",
        organism.id, organism.energy, final_tick.tick
    );
    let genome = GenomeFields::from_snapshot_eco(organism);
    println!(
        "genome digest (GenomeFields::hash, EX-4's lineage content-address): {}",
        genome.short_hash()
    );
    println!(
        "  decay_amount={} repro_threshold={} repro_give_pct={}",
        genome.decay_amount, genome.repro_threshold, genome.repro_give_pct
    );

    let hungry_name = role_pools.promoters[organism.hungry_promoter as usize].clone();
    let repro_name = role_pools.promoters[organism.repro_promoter as usize].clone();
    let sense_name = role_pools.movement[organism.sense_move as usize].clone();
    println!("  hungry_promoter -> {hungry_name}");
    println!("  repro_promoter  -> {repro_name}");
    println!("  sense_move      -> {sense_name}");

    println!(
        "\n== per-cell hash-attestation + behavioral identity (Z80 <-> RV32 <-> CPU-reference) =="
    );
    let roles: [(&str, &str, usize, &CompiledGene); 6] = [
        ("decay", &grazer_disk.genes.decay, 2, &genes.decay),
        ("eat", &grazer_disk.genes.eat, 2, &genes.eat),
        ("split", &grazer_disk.genes.split, 2, &genes.split),
        (
            "hungry_promoter",
            &hungry_name,
            2,
            &genes.hungry_pool[organism.hungry_promoter as usize],
        ),
        (
            "repro_promoter",
            &repro_name,
            2,
            &genes.repro_pool[organism.repro_promoter as usize],
        ),
        (
            "sense_move",
            &sense_name,
            3,
            &genes.sense_pool[organism.sense_move as usize],
        ),
    ];

    let mut all_pass = true;
    for (role, name, arity, compiled) in roles {
        all_pass &= attest_cell(role, name, arity, compiled);
    }

    println!(
        "\n== EX-5 gate (\"one organism evolved in EX-3, exported, shown behaviourally \
         identical on the robot's target ISA\"): {} ==",
        if all_pass { "PASS" } else { "FAIL" }
    );
    println!(
        "(scope: per-cell attestation, not a single composed whole-organism RV32 program; \
         cycle counts not reported — the RV32 cycle table stays provisional until B4.)"
    );
}
