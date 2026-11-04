use std::{path::PathBuf, time::Instant};

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use simplicial_complex_census::{
    Generator,
    enrich::{enrich_cohomology, enrich_topology},
    enumerate::{orderly_representatives, reference_representatives},
    identity_census,
    jsonl::{
        emit_cohomology_shell, emit_identity_shell, emit_topology_shell, load_cohomology_shell,
        load_identity_shell, load_topology_shell,
    },
    validate::{
        accepted_counts, validate_cohomology_census, validate_identity_census,
        validate_topology_census,
    },
};

#[derive(Debug, Parser)]
#[command(
    name = "simplicial-complex-census",
    about = "Build certified finite simplicial-complex collection shells"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build, validate, and atomically install one identity shell.
    Identity {
        #[arg(long)]
        vertices: usize,
        #[arg(long, env = "SIMPLICIAL_COMPLEX_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        #[arg(long, default_value_t = 100_000_000)]
        candidate_cap: u64,
    },
    /// Build and install the independently replaceable topology sidecar.
    Topology {
        #[arg(long)]
        vertices: usize,
        #[arg(long, env = "SIMPLICIAL_COMPLEX_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Build one complete finite-field cohomology sidecar from strong cores.
    Cohomology {
        #[arg(long)]
        vertices: usize,
        #[arg(long)]
        prime: u64,
        #[arg(long, env = "SIMPLICIAL_COMPLEX_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Reload, hash-check, and mathematically revalidate one cohomology sidecar.
    ValidateCohomology {
        #[arg(long)]
        vertices: usize,
        #[arg(long)]
        prime: u64,
        #[arg(long, env = "SIMPLICIAL_COMPLEX_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Compare the production generator with the independent labeled oracle.
    Compare {
        #[arg(long, default_value_t = 4)]
        maximum_vertices: usize,
        #[arg(long, default_value_t = 1_000_000)]
        candidate_cap: u64,
    },
    /// Print production counts and branching statistics through one shell.
    Counts {
        #[arg(long, default_value_t = 6)]
        maximum_vertices: usize,
        #[arg(long, default_value_t = 100_000_000)]
        candidate_cap: u64,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Identity {
            vertices,
            data_root,
            candidate_cap,
        } => {
            let started = Instant::now();
            let (stats, representatives) = orderly_representatives(vertices, candidate_cap)?;
            let census = identity_census(
                Generator::CanonicalVertexLink,
                vertices,
                candidate_cap,
                stats,
                representatives,
            )?;
            let validation = validate_identity_census(&census)?;
            let count = census.records.len();
            let output = emit_identity_shell(&data_root, &census, &validation)?;
            println!(
                "wrote {count} canonical complexes to {} in {:.3}s",
                output.display(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::Topology {
            vertices,
            data_root,
        } => {
            let started = Instant::now();
            let identities = load_identity_shell(&data_root, vertices)?;
            let census = enrich_topology(vertices, &identities)?;
            let validation = validate_topology_census(&census)?;
            let count = census.records.len();
            let output = emit_topology_shell(&data_root, &census, &validation)?;
            println!(
                "wrote {count} topology records to {} in {:.3}s",
                output.display(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::Cohomology {
            vertices,
            prime,
            data_root,
        } => {
            let started = Instant::now();
            let identities = load_identity_shell(&data_root, vertices)?;
            let topology = load_topology_shell(&data_root, vertices)?;
            let census = enrich_cohomology(vertices, prime, &identities, &topology)?;
            let validation = validate_cohomology_census(&census, &identities, &topology)?;
            let count = census.records.len();
            let output = emit_cohomology_shell(&data_root, &census, &validation)?;
            println!(
                "wrote {count} F{prime}-cohomology records to {} in {:.3}s",
                output.display(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::ValidateCohomology {
            vertices,
            prime,
            data_root,
        } => {
            let started = Instant::now();
            let identities = load_identity_shell(&data_root, vertices)?;
            let topology = load_topology_shell(&data_root, vertices)?;
            let records = load_cohomology_shell(&data_root, vertices, prime)?;
            let census = simplicial_complex_census::CohomologyCensus {
                vertices,
                prime,
                records,
            };
            let validation = validate_cohomology_census(&census, &identities, &topology)?;
            println!(
                "validated {} F{prime}-cohomology records in {:.3}s ({})",
                census.records.len(),
                started.elapsed().as_secs_f64(),
                validation.status
            );
            Ok(())
        }
        Command::Compare {
            maximum_vertices,
            candidate_cap,
        } => {
            for vertices in 0..=maximum_vertices {
                let (_, reference) = reference_representatives(vertices, candidate_cap)?;
                let (_, orderly) = orderly_representatives(vertices, candidate_cap)?;
                ensure!(
                    reference.keys().eq(orderly.keys()),
                    "key mismatch at {vertices}"
                );
                println!("vertices={vertices} count={}", orderly.len());
            }
            Ok(())
        }
        Command::Counts {
            maximum_vertices,
            candidate_cap,
        } => {
            for vertices in 0..=maximum_vertices {
                let started = Instant::now();
                let (stats, representatives) = orderly_representatives(vertices, candidate_cap)?;
                let expected = accepted_counts(vertices)?;
                ensure!(
                    representatives.len() as u128 == expected.unlabeled,
                    "accepted count mismatch at {vertices}"
                );
                let level = stats.levels.last();
                println!(
                    "vertices={vertices} count={} raw_links={} link_orbits={} canonical_tests={} duplicate_acceptances={} elapsed={:.3}s",
                    representatives.len(),
                    level.map_or(0, |value| value.raw_candidates),
                    level.map_or(0, |value| value.orbit_representatives),
                    level.map_or(0, |value| value.canonical_tests),
                    level.map_or(0, |value| value.duplicate_acceptances),
                    started.elapsed().as_secs_f64()
                );
            }
            Ok(())
        }
    }
}
