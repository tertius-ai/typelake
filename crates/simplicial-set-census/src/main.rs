use std::time::Instant;

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use simplicial_set_census::{
    enrich::{enrich_cohomology, enrich_homotopy, enrich_steenrod_operations, enrich_topology},
    enumerate::{orderly_representatives, reference_representatives},
    identity_census,
    jsonl::{
        emit_cohomology_shell, emit_homotopy_shell, emit_identity_shell,
        emit_steenrod_operations_shell, emit_topology_shell, load_cohomology_shell,
        load_homotopy_shell, load_identity_shell, load_steenrod_operations_shell,
        load_topology_shell,
    },
    validate::{
        validate_cohomology_census, validate_homotopy_census, validate_identity_census,
        validate_steenrod_operations_census, validate_topology_census,
    },
};

#[derive(Debug, Parser)]
#[command(name = "simplicial-set-census")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build, validate, and atomically install one complete identity shell.
    Identity {
        #[arg(long)]
        total_nondegenerate: usize,
        #[arg(long)]
        maximum_dimension: usize,
        #[arg(long, env = "SIMPLICIAL_SET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        #[arg(long, default_value_t = 100_000_000)]
        candidate_cap: u64,
    },
    /// Build and install normalized integral topology for one identity shell.
    Topology {
        #[arg(long)]
        total_nondegenerate: usize,
        #[arg(long)]
        maximum_dimension: usize,
        #[arg(long, env = "SIMPLICIAL_SET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Reload and exactly revalidate one installed topology-v2 shell.
    ValidateTopology {
        #[arg(long)]
        total_nondegenerate: usize,
        #[arg(long)]
        maximum_dimension: usize,
        #[arg(long, env = "SIMPLICIAL_SET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Build and install a complete normalized finite-field cohomology sidecar.
    Cohomology {
        #[arg(long)]
        total_nondegenerate: usize,
        #[arg(long)]
        maximum_dimension: usize,
        #[arg(long)]
        prime: u64,
        #[arg(long, env = "SIMPLICIAL_SET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Reload and exactly revalidate one installed cohomology sidecar.
    ValidateCohomology {
        #[arg(long)]
        total_nondegenerate: usize,
        #[arg(long)]
        maximum_dimension: usize,
        #[arg(long)]
        prime: u64,
        #[arg(long, env = "SIMPLICIAL_SET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Build and install complete odd-primary Steenrod-operation data.
    SteenrodOperations {
        #[arg(long)]
        total_nondegenerate: usize,
        #[arg(long)]
        maximum_dimension: usize,
        #[arg(long)]
        prime: u64,
        #[arg(long, env = "SIMPLICIAL_SET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Reload and exactly revalidate one Steenrod-operation sidecar.
    ValidateSteenrodOperations {
        #[arg(long)]
        total_nondegenerate: usize,
        #[arg(long)]
        maximum_dimension: usize,
        #[arg(long)]
        prime: u64,
        #[arg(long, env = "SIMPLICIAL_SET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Build and install replayable elementary horn-collapse relations.
    Homotopy {
        #[arg(long)]
        total_nondegenerate: usize,
        #[arg(long)]
        maximum_dimension: usize,
        #[arg(long, env = "SIMPLICIAL_SET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Reload and exactly revalidate one installed homotopy-v1 sidecar.
    ValidateHomotopy {
        #[arg(long)]
        total_nondegenerate: usize,
        #[arg(long)]
        maximum_dimension: usize,
        #[arg(long, env = "SIMPLICIAL_SET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Print production counts and branching statistics through one total size.
    Counts {
        #[arg(long, default_value_t = 6)]
        maximum_total: usize,
        #[arg(long, default_value_t = 3)]
        maximum_dimension: usize,
        #[arg(long, default_value_t = 100_000_000)]
        candidate_cap: u64,
    },
    /// Compare production with the redundant labeled oracle.
    Compare {
        #[arg(long, default_value_t = 5)]
        maximum_total: usize,
        #[arg(long, default_value_t = 3)]
        maximum_dimension: usize,
        #[arg(long, default_value_t = 100_000_000)]
        candidate_cap: u64,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Identity {
            total_nondegenerate,
            maximum_dimension,
            data_root,
            candidate_cap,
        } => {
            let started = Instant::now();
            let (stats, representatives) =
                orderly_representatives(total_nondegenerate, maximum_dimension, candidate_cap)?;
            let census = identity_census(
                total_nondegenerate,
                maximum_dimension,
                candidate_cap,
                stats,
                representatives,
            )?;
            let validation = validate_identity_census(&census)?;
            let count = census.records.len();
            let output = emit_identity_shell(&data_root, &census, &validation)?;
            println!(
                "wrote {count} canonical simplicial sets to {} in {:.3}s",
                output.display(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::Topology {
            total_nondegenerate,
            maximum_dimension,
            data_root,
        } => {
            let started = Instant::now();
            let identities =
                load_identity_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let census = enrich_topology(total_nondegenerate, maximum_dimension, &identities)?;
            let validation = validate_topology_census(&census, &identities)?;
            let count = census.records.len();
            let output = emit_topology_shell(&data_root, &census, &validation)?;
            println!(
                "wrote {count} topology records to {} in {:.3}s",
                output.display(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::ValidateTopology {
            total_nondegenerate,
            maximum_dimension,
            data_root,
        } => {
            let started = Instant::now();
            let identities =
                load_identity_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let records = load_topology_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let census = simplicial_set_census::TopologyCensus {
                total_nondegenerate,
                maximum_dimension,
                records,
            };
            let report = validate_topology_census(&census, &identities)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            eprintln!(
                "revalidated {} topology records in {:.3}s",
                census.records.len(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::Cohomology {
            total_nondegenerate,
            maximum_dimension,
            prime,
            data_root,
        } => {
            let started = Instant::now();
            let identities =
                load_identity_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let census =
                enrich_cohomology(total_nondegenerate, maximum_dimension, prime, &identities)?;
            let validation = validate_cohomology_census(&census, &identities)?;
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
            total_nondegenerate,
            maximum_dimension,
            prime,
            data_root,
        } => {
            let started = Instant::now();
            let identities =
                load_identity_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let records =
                load_cohomology_shell(&data_root, total_nondegenerate, maximum_dimension, prime)?;
            let census = simplicial_set_census::CohomologyCensus {
                total_nondegenerate,
                maximum_dimension,
                prime,
                records,
            };
            let report = validate_cohomology_census(&census, &identities)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            eprintln!(
                "revalidated {} F{prime}-cohomology records in {:.3}s",
                census.records.len(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::SteenrodOperations {
            total_nondegenerate,
            maximum_dimension,
            prime,
            data_root,
        } => {
            let started = Instant::now();
            let identities =
                load_identity_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let cohomology =
                load_cohomology_shell(&data_root, total_nondegenerate, maximum_dimension, prime)?;
            let census = enrich_steenrod_operations(
                total_nondegenerate,
                maximum_dimension,
                prime,
                &identities,
                &cohomology,
            )?;
            let validation =
                validate_steenrod_operations_census(&census, &identities, &cohomology)?;
            let count = census.records.len();
            let output = emit_steenrod_operations_shell(&data_root, &census, &validation)?;
            println!(
                "wrote {count} F{prime} Steenrod-operation records to {} in {:.3}s",
                output.display(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::ValidateSteenrodOperations {
            total_nondegenerate,
            maximum_dimension,
            prime,
            data_root,
        } => {
            let started = Instant::now();
            let identities =
                load_identity_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let cohomology =
                load_cohomology_shell(&data_root, total_nondegenerate, maximum_dimension, prime)?;
            let records = load_steenrod_operations_shell(
                &data_root,
                total_nondegenerate,
                maximum_dimension,
                prime,
            )?;
            let census = simplicial_set_census::SteenrodOperationsCensus {
                total_nondegenerate,
                maximum_dimension,
                prime,
                records,
            };
            let report = validate_steenrod_operations_census(&census, &identities, &cohomology)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            eprintln!(
                "revalidated {} F{prime} Steenrod-operation records in {:.3}s",
                census.records.len(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::Homotopy {
            total_nondegenerate,
            maximum_dimension,
            data_root,
        } => {
            let started = Instant::now();
            let identities =
                load_identity_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let census = enrich_homotopy(total_nondegenerate, maximum_dimension, &identities)?;
            let validation = validate_homotopy_census(&census, &identities)?;
            let count = census.records.len();
            let output = emit_homotopy_shell(&data_root, &census, &validation)?;
            println!(
                "wrote {count} horn-collapse records to {} in {:.3}s",
                output.display(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::ValidateHomotopy {
            total_nondegenerate,
            maximum_dimension,
            data_root,
        } => {
            let started = Instant::now();
            let identities =
                load_identity_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let records = load_homotopy_shell(&data_root, total_nondegenerate, maximum_dimension)?;
            let census = simplicial_set_census::HomotopyCensus {
                total_nondegenerate,
                maximum_dimension,
                records,
            };
            let report = validate_homotopy_census(&census, &identities)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            eprintln!(
                "revalidated {} horn-collapse records in {:.3}s",
                census.records.len(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Command::Counts {
            maximum_total,
            maximum_dimension,
            candidate_cap,
        } => {
            for total in 0..=maximum_total {
                let started = Instant::now();
                let (stats, representatives) =
                    orderly_representatives(total, maximum_dimension, candidate_cap)?;
                let level = stats.levels.last();
                println!(
                    "total={total} dimension_cap={maximum_dimension} count={} raw={} boundary_orbits={} valid={} canonical={} duplicates={} elapsed={:.3}s",
                    representatives.len(),
                    level.map_or(0, |level| level.raw_boundaries),
                    level.map_or(0, |level| level.boundary_orbits),
                    level.map_or(0, |level| level.valid_augmentations),
                    level.map_or(0, |level| level.canonical_tests),
                    level.map_or(0, |level| level.duplicate_acceptances),
                    started.elapsed().as_secs_f64()
                );
            }
            Ok(())
        }
        Command::Compare {
            maximum_total,
            maximum_dimension,
            candidate_cap,
        } => {
            for total in 0..=maximum_total {
                let (_, production) =
                    orderly_representatives(total, maximum_dimension, candidate_cap)?;
                let (_, reference) =
                    reference_representatives(total, maximum_dimension, candidate_cap)?;
                ensure!(
                    production.keys().eq(reference.keys()),
                    "key mismatch at {total}"
                );
                println!("total={total} count={}", production.len());
            }
            Ok(())
        }
    }
}
