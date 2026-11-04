use std::{path::PathBuf, time::Instant};

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use finite_group_cohomology_census::{
    enrich::{complete_modular_census, modular_record},
    integral::{
        DEFAULT_CENSUS_BAR_ENTRY_LIMIT, integral_census, integral_record, validate_integral_census,
    },
    integral_jsonl::{emit_integral_census, load_integral_census},
    jsonl::{emit_modular_census, load_modular_census},
    source::{load_source, prime_divisors, validate_source},
    validate::validate_modular_census,
};

#[derive(Debug, Parser)]
#[command(name = "finite-group-cohomology-census")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Reconstruct all 144 pinned source groups and check accepted counts.
    ValidateSource,
    /// Compute and print one modular record without installing an artifact.
    ModularRecord {
        #[arg(long)]
        group_label: String,
        #[arg(long)]
        prime: u64,
        #[arg(long)]
        exact_through_degree: usize,
    },
    /// Build, revalidate, and atomically install one modular development shell.
    Modular {
        #[arg(long)]
        maximum_order: usize,
        #[arg(long)]
        initial_proof_degree: usize,
        #[arg(long)]
        maximum_proof_degree: usize,
        #[arg(long, env = "FINITE_GROUP_COHOMOLOGY_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Reload, hash-check, and mathematically recompute an installed shell.
    ValidateModular {
        #[arg(long)]
        maximum_order: usize,
        #[arg(long)]
        initial_proof_degree: usize,
        #[arg(long)]
        maximum_proof_degree: usize,
        #[arg(long, env = "FINITE_GROUP_COHOMOLOGY_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Recompute every modular coefficient row of one pinned source group.
    ValidateModularGroup {
        #[arg(long)]
        group_label: String,
        /// Directory of <group-label>-F<prime>.json records.
        #[arg(long)]
        records_root: PathBuf,
        #[arg(long, default_value_t = 8)]
        initial_proof_degree: usize,
        #[arg(long, default_value_t = 12)]
        maximum_proof_degree: usize,
    },
    /// Compute and print one exact integral record.
    IntegralRecord {
        #[arg(long)]
        group_label: String,
        #[arg(long, default_value_t = 16)]
        additive_through_degree: usize,
        #[arg(long, default_value_t = DEFAULT_CENSUS_BAR_ENTRY_LIMIT)]
        bar_entry_limit: usize,
    },
    /// Build complete finite-abelian rings and budgeted sparse-bar truncations.
    Integral {
        #[arg(long, default_value_t = 32)]
        maximum_order: usize,
        #[arg(long, default_value_t = 16)]
        additive_through_degree: usize,
        #[arg(long, default_value_t = DEFAULT_CENSUS_BAR_ENTRY_LIMIT)]
        bar_entry_limit: usize,
        #[arg(long, env = "FINITE_GROUP_COHOMOLOGY_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Reload, hash-check, and recompute an installed integral family artifact.
    ValidateIntegral {
        #[arg(long, default_value_t = 32)]
        maximum_order: usize,
        #[arg(long, default_value_t = 16)]
        additive_through_degree: usize,
        #[arg(long, default_value_t = DEFAULT_CENSUS_BAR_ENTRY_LIMIT)]
        bar_entry_limit: usize,
        #[arg(long, env = "FINITE_GROUP_COHOMOLOGY_DATA_ROOT")]
        data_root: PathBuf,
    },
}

fn main() -> Result<()> {
    let start = Instant::now();
    let source = load_source()?;
    match Cli::parse().command {
        Command::ValidateSource => {
            let report = validate_source(&source)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::ModularRecord {
            group_label,
            prime,
            exact_through_degree,
        } => {
            validate_source(&source)?;
            let group = source
                .groups
                .iter()
                .find(|group| group.lmfdb_label == group_label)
                .ok_or_else(|| anyhow::anyhow!("unknown source group {group_label}"))?;
            let record = modular_record(group, prime, exact_through_degree)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        Command::Modular {
            maximum_order,
            initial_proof_degree,
            maximum_proof_degree,
            data_root,
        } => {
            validate_source(&source)?;
            let census = complete_modular_census(
                &source,
                maximum_order,
                initial_proof_degree,
                maximum_proof_degree,
            )?;
            let validation = validate_modular_census(&source, &census)?;
            let manifest = emit_modular_census(&data_root, &census, validation)?;
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        }
        Command::ValidateModular {
            maximum_order,
            initial_proof_degree,
            maximum_proof_degree,
            data_root,
        } => {
            validate_source(&source)?;
            let (manifest, census) = load_modular_census(
                &data_root,
                maximum_order,
                initial_proof_degree,
                maximum_proof_degree,
            )?;
            let report = validate_modular_census(&source, &census)?;
            ensure!(
                report == manifest.validation,
                "stored validation report disagrees with independent replay"
            );
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::ValidateModularGroup {
            group_label,
            records_root,
            initial_proof_degree,
            maximum_proof_degree,
        } => {
            validate_source(&source)?;
            let mut selected_source = source;
            selected_source
                .groups
                .retain(|group| group.lmfdb_label == group_label);
            ensure!(
                selected_source.groups.len() == 1,
                "unknown source group {group_label}"
            );
            let group = &selected_source.groups[0];
            let records = prime_divisors(group.small_groups_order)
                .into_iter()
                .map(|prime| {
                    let path = records_root.join(format!("{group_label}-F{prime}.json"));
                    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
                })
                .collect::<Result<Vec<_>>>()?;
            let census = finite_group_cohomology_census::ModularCensus {
                maximum_order: selected_source.maximum_order,
                initial_proof_degree,
                maximum_proof_degree,
                records,
            };
            let validation = validate_modular_census(&selected_source, &census)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "group_label": group_label,
                    "noether_version": noether::VERSION,
                    "noether_revision": finite_group_cohomology_census::NOETHER_REVISION,
                    "validation": validation,
                }))?
            );
        }
        Command::IntegralRecord {
            group_label,
            additive_through_degree,
            bar_entry_limit,
        } => {
            validate_source(&source)?;
            let group = source
                .groups
                .iter()
                .find(|group| group.lmfdb_label == group_label)
                .ok_or_else(|| anyhow::anyhow!("unknown source group {group_label}"))?;
            let record = integral_record(group, additive_through_degree, bar_entry_limit)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        Command::Integral {
            maximum_order,
            additive_through_degree,
            bar_entry_limit,
            data_root,
        } => {
            validate_source(&source)?;
            let census = integral_census(
                &source,
                maximum_order,
                additive_through_degree,
                bar_entry_limit,
            )?;
            let validation = validate_integral_census(&source, &census)?;
            let manifest = emit_integral_census(&data_root, &census, validation)?;
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        }
        Command::ValidateIntegral {
            maximum_order,
            additive_through_degree,
            bar_entry_limit,
            data_root,
        } => {
            validate_source(&source)?;
            let (manifest, census) = load_integral_census(
                &data_root,
                maximum_order,
                additive_through_degree,
                bar_entry_limit,
            )?;
            let report = validate_integral_census(&source, &census)?;
            ensure!(
                report == manifest.validation,
                "stored integral validation report disagrees with independent replay"
            );
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    eprintln!("elapsed_seconds={:.3}", start.elapsed().as_secs_f64());
    Ok(())
}
