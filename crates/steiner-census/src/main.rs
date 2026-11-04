use std::{
    path::PathBuf,
    time::{Instant, SystemTime},
};

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use noether::category::{SteinerCanonicalLabel, SteinerCategory};
use steiner_census::{
    DEFAULT_STREET_DEGREE, Generator, RICH_ENRICHMENT_CONTRACT, assemble_census, enrich_census,
    enumerate::{orderly_representatives, reference_representatives},
    identity_census,
    jsonl::{
        emit_completed_shell, emit_gray_table, emit_identity_shell, emit_light_shell,
        emit_mapping_shell, emit_rich_shell, load_identity_shell, load_light_shell,
    },
    light_census, mapping_census, rich_census,
    validate::{
        validate_census, validate_identity_census, validate_light_census, validate_mapping_census,
    },
};

#[derive(Debug, Parser)]
#[command(
    name = "steiner-census",
    about = "Build validated finite strong-Steiner-category census records"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Enumerate and certify only canonical category identities.
    Identity(IdentityOptions),
    /// Compute the uniform inexpensive enrichment of a certified identity shell.
    Light(LightOptions),
    /// Compute exact cells and every ordered object-pair mapping category.
    Mappings(LightOptions),
    /// Compute expensive cells, mappings, basepoints, and categorical sidecars.
    Rich(RichOptions),
    /// Run the complete redundant signed-boundary-matrix oracle.
    Reference(ShellOptions),
    /// Run the hereditary canonical cell-attachment generator.
    Orderly(ShellOptions),
    /// Compare both complete generators without enrichment.
    Compare {
        #[arg(long, default_value_t = 5)]
        maximum_atomic_size: usize,
        #[arg(long, default_value_t = 2_000_000)]
        candidate_cap: u64,
    },
    /// Compare the unpruned recursive oracle with the production generator.
    RecursiveCompare {
        #[arg(long, default_value_t = 9)]
        maximum_atomic_size: usize,
        #[arg(long, default_value_t = 20_000_000)]
        candidate_cap: u64,
    },
    /// Print generation counts and work counters through a shell.
    Counts {
        #[arg(long, default_value_t = 7)]
        maximum_atomic_size: usize,
        #[arg(long, default_value_t = 20_000_000)]
        candidate_cap: u64,
    },
    /// Emit the fixed ordered 5-by-5 Gray tensor table.
    GrayTable {
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Clone, Debug, clap::Args)]
struct ShellOptions {
    #[arg(long)]
    atomic_size: usize,
    #[arg(long, env = "STEINER_CENSUS_DATA_ROOT")]
    data_root: PathBuf,
    #[arg(long, default_value_t = 20_000_000)]
    candidate_cap: u64,
    #[arg(long, default_value_t = DEFAULT_STREET_DEGREE)]
    street_degree: usize,
}

#[derive(Clone, Debug, clap::Args)]
struct IdentityOptions {
    #[arg(long)]
    atomic_size: usize,
    #[arg(long, env = "STEINER_CENSUS_DATA_ROOT")]
    data_root: PathBuf,
    #[arg(long, default_value_t = 20_000_000)]
    candidate_cap: u64,
}

#[derive(Clone, Debug, clap::Args)]
struct LightOptions {
    #[arg(long)]
    atomic_size: usize,
    #[arg(long, env = "STEINER_CENSUS_DATA_ROOT")]
    data_root: PathBuf,
}

#[derive(Clone, Debug, clap::Args)]
struct RichOptions {
    #[arg(long)]
    atomic_size: usize,
    #[arg(long, env = "STEINER_CENSUS_DATA_ROOT")]
    data_root: PathBuf,
    #[arg(long, default_value_t = 20_000_000)]
    candidate_cap: u64,
    #[arg(long, default_value_t = DEFAULT_STREET_DEGREE)]
    street_degree: usize,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Identity(options) => build_identity_shell(options),
        Command::Light(options) => build_light_shell(options),
        Command::Mappings(options) => build_mapping_shell(options),
        Command::Rich(options) => build_rich_shell(options),
        Command::Reference(options) => build_shell(Generator::Reference, options),
        Command::Orderly(options) => build_shell(Generator::CellAttachment, options),
        Command::Compare {
            maximum_atomic_size,
            candidate_cap,
        } => {
            for size in 0..=maximum_atomic_size {
                let (reference_stats, reference) = reference_representatives(size, candidate_cap)?;
                let (orderly_stats, orderly) = orderly_representatives(size, candidate_cap)?;
                ensure!(
                    reference.keys().eq(orderly.keys()),
                    "generator key mismatch at atomic size {size}"
                );
                println!(
                    "size={size} count={} reference_candidates={} orderly_candidates={} orbit_representatives={} orderly_canonical_tests={}",
                    reference.len(),
                    reference_stats.raw_candidates,
                    orderly_stats.raw_candidates,
                    orderly_stats.orbit_representatives,
                    orderly_stats.canonical_tests
                );
            }
            Ok(())
        }
        Command::Counts {
            maximum_atomic_size,
            candidate_cap,
        } => {
            for size in 0..=maximum_atomic_size {
                let timer = Instant::now();
                let (stats, categories) = orderly_representatives(size, candidate_cap)?;
                println!(
                    "size={size} count={} candidates={} orbit_representatives={} canonical_tests={} elapsed_ms={}",
                    categories.len(),
                    stats.raw_candidates,
                    stats.orbit_representatives,
                    stats.canonical_tests,
                    timer.elapsed().as_millis()
                );
            }
            Ok(())
        }
        Command::RecursiveCompare {
            maximum_atomic_size,
            candidate_cap,
        } => {
            for size in 0..=maximum_atomic_size {
                let (reference_stats, reference) =
                    steiner_census::enumerate::recursive_reference_representatives(
                        size,
                        candidate_cap,
                    )?;
                let (orderly_stats, orderly) = orderly_representatives(size, candidate_cap)?;
                ensure!(
                    reference.keys().eq(orderly.keys()),
                    "recursive-reference key mismatch at atomic size {size}"
                );
                println!(
                    "size={size} count={} recursive_candidates={} orderly_candidates={} orbit_representatives={} orderly_canonical_tests={}",
                    reference.len(),
                    reference_stats.raw_candidates,
                    orderly_stats.raw_candidates,
                    orderly_stats.orbit_representatives,
                    orderly_stats.canonical_tests
                );
            }
            Ok(())
        }
        Command::GrayTable { output } => {
            let named = named_gray_factors()?;
            #[derive(serde::Serialize)]
            struct Row<'a> {
                left_name: &'a str,
                right_name: &'a str,
                #[serde(flatten)]
                product: steiner_census::records::GrayProductRecord,
            }
            let mut rows = Vec::new();
            for (left_name, left) in &named {
                for (right_name, right) in &named {
                    rows.push(Row {
                        left_name,
                        right_name,
                        product: steiner_census::enrich::gray_product_record(left, right)?,
                    });
                }
            }
            emit_gray_table(&output, &rows)?;
            println!(
                "wrote {} ordered Gray products to {}",
                rows.len(),
                output.display()
            );
            Ok(())
        }
    }
}

fn build_identity_shell(options: IdentityOptions) -> Result<()> {
    let started = SystemTime::now();
    let timer = Instant::now();
    let (stats, representatives) =
        orderly_representatives(options.atomic_size, options.candidate_cap)?;
    let census = identity_census(
        Generator::CellAttachment,
        options.atomic_size,
        options.candidate_cap,
        stats,
        representatives,
    )?;
    let validation = validate_identity_census(&census)?;
    let output = emit_identity_shell(
        &options.data_root,
        &census,
        &validation,
        started,
        timer.elapsed(),
    )?;
    println!(
        "wrote {} certified identities to {}",
        census.identities.len(),
        output.display()
    );
    Ok(())
}

fn build_light_shell(options: LightOptions) -> Result<()> {
    let started = SystemTime::now();
    let timer = Instant::now();
    let identity = load_identity_shell(&options.data_root, options.atomic_size)?;
    let census = light_census(options.atomic_size, &identity.records)?;
    let validation = validate_light_census(&census, &identity.records)?;
    let output = emit_light_shell(
        &options.data_root,
        &census,
        &validation,
        &identity,
        started,
        timer.elapsed(),
    )?;
    println!(
        "wrote {} complete light enrichments to {}",
        census.records.len(),
        output.display()
    );
    Ok(())
}

fn build_mapping_shell(options: LightOptions) -> Result<()> {
    let started = SystemTime::now();
    let timer = Instant::now();
    let identity = load_identity_shell(&options.data_root, options.atomic_size)?;
    let census = mapping_census(options.atomic_size, &identity.records)?;
    let validation = validate_mapping_census(&census, &identity.records)?;
    let output = emit_mapping_shell(
        &options.data_root,
        &census,
        &validation,
        &identity,
        started,
        timer.elapsed(),
    )?;
    println!(
        "wrote {} complete mapping enrichments and {} mapping categories to {}",
        census.records.len(),
        census.mappings.len(),
        output.display()
    );
    Ok(())
}

fn build_rich_shell(options: RichOptions) -> Result<()> {
    let started = SystemTime::now();
    let timer = Instant::now();
    let identity = load_identity_shell(&options.data_root, options.atomic_size)?;
    let light = load_light_shell(&options.data_root, options.atomic_size)?;
    let rich = rich_census(
        options.atomic_size,
        &identity.records,
        &light.records,
        options.street_degree,
    )?;
    let assembled = assemble_census(
        Generator::CellAttachment,
        options.atomic_size,
        options.candidate_cap,
        0,
        &identity.records,
        &light.records,
        &rich,
    )?;
    let mut validation = validate_census(&assembled)?;
    validation.contract = RICH_ENRICHMENT_CONTRACT.to_owned();
    let output = emit_rich_shell(
        &options.data_root,
        &rich,
        &validation,
        &identity,
        &light,
        options.street_degree,
        started,
        timer.elapsed(),
    )?;
    println!(
        "wrote rich enrichments for {} categories, {} mappings, and {} basepoint orbits to {}",
        rich.records.len(),
        rich.mappings.len(),
        rich.basepoints.len(),
        output.display()
    );
    Ok(())
}

fn build_shell(generator: Generator, options: ShellOptions) -> Result<()> {
    let started = SystemTime::now();
    let timer = Instant::now();
    let (stats, representatives) = match generator {
        Generator::Reference => {
            reference_representatives(options.atomic_size, options.candidate_cap)?
        }
        Generator::CellAttachment => {
            orderly_representatives(options.atomic_size, options.candidate_cap)?
        }
    };
    let census = enrich_census(
        generator,
        options.atomic_size,
        options.candidate_cap,
        stats.raw_candidates,
        representatives,
        options.street_degree,
    )?;
    let validation = validate_census(&census)?;
    let output = emit_completed_shell(
        &options.data_root,
        &census,
        &validation,
        options.street_degree,
        started,
        timer.elapsed(),
    )?;
    println!(
        "wrote {} categories, {} mappings, {} basepoint orbits, and {} homotopy Posets to {}",
        census.categories.len(),
        census.mappings.len(),
        census.basepoints.len(),
        census.homotopy_posets.len(),
        output.display()
    );
    Ok(())
}

fn named_gray_factors() -> Result<Vec<(&'static str, SteinerCanonicalLabel)>> {
    fn canonical(category: SteinerCategory) -> Result<SteinerCanonicalLabel> {
        Ok(category.canonical_form()?.canonical_label()?)
    }
    let discrete = |count| {
        SteinerCategory::from_boundaries(vec![count], vec![Vec::new()])
            .expect("every finite discrete category is strong Steiner")
    };
    Ok(vec![
        ("empty", canonical(SteinerCategory::empty())?),
        ("point", canonical(SteinerCategory::point())?),
        ("discrete-2", canonical(discrete(2))?),
        ("discrete-3", canonical(discrete(3))?),
        ("arrow", canonical(SteinerCategory::globe(1)?)?),
    ])
}
