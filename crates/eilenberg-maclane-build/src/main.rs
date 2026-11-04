use std::{path::PathBuf, time::Instant};

use anyhow::Result;
use clap::{Parser, Subcommand};
use eilenberg_maclane_build::{
    enrich::build_census,
    jsonl::{emit_census, load_census},
    source::{load_source, validate_source},
    validate::validate_census,
};

#[derive(Debug, Parser)]
#[command(name = "eilenberg-maclane-build")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate the complete finite-abelian source shell through order 32.
    ValidateSource,
    /// Build, revalidate, and atomically install one development artifact.
    Build {
        #[arg(long, default_value_t = 32)]
        maximum_finite_order: usize,
        #[arg(long, default_value_t = 4)]
        maximum_delooping_degree: usize,
        #[arg(long, default_value_t = 16)]
        exact_through_degree: usize,
        #[arg(long, env = "EILENBERG_MACLANE_DATA_ROOT")]
        data_root: PathBuf,
    },
    /// Reload, hash-check, and mathematically recompute an installed artifact.
    Validate {
        #[arg(long, default_value_t = 32)]
        maximum_finite_order: usize,
        #[arg(long, default_value_t = 4)]
        maximum_delooping_degree: usize,
        #[arg(long, default_value_t = 16)]
        exact_through_degree: usize,
        #[arg(long, env = "EILENBERG_MACLANE_DATA_ROOT")]
        data_root: PathBuf,
    },
}

fn main() -> Result<()> {
    let start = Instant::now();
    let source = load_source()?;
    match Cli::parse().command {
        Command::ValidateSource => {
            println!(
                "{}",
                serde_json::to_string_pretty(&validate_source(&source)?)?
            );
        }
        Command::Build {
            maximum_finite_order,
            maximum_delooping_degree,
            exact_through_degree,
            data_root,
        } => {
            let source_validation = validate_source(&source)?;
            let census = build_census(
                &source,
                maximum_finite_order,
                maximum_delooping_degree,
                exact_through_degree,
            )?;
            let validation = validate_census(&source, &census)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&emit_census(
                    &data_root,
                    &census,
                    source_validation,
                    validation,
                )?)?
            );
        }
        Command::Validate {
            maximum_finite_order,
            maximum_delooping_degree,
            exact_through_degree,
            data_root,
        } => {
            validate_source(&source)?;
            let (_, census) = load_census(
                &data_root,
                maximum_finite_order,
                maximum_delooping_degree,
                exact_through_degree,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&validate_census(&source, &census)?)?
            );
        }
    }
    eprintln!("elapsed_seconds={:.3}", start.elapsed().as_secs_f64());
    Ok(())
}
