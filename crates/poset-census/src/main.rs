use std::{
    path::PathBuf,
    time::{Instant, SystemTime},
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use poset_census::{
    Generator, enrich_census_with_generator,
    enumerate::{orderly_representatives, reference_representatives},
    jsonl::emit_completed_order,
    shard::{
        CertificationOptions, MergeOptions, PublishOptions, ShardOptions, certify_orderly_shard,
        merge_orderly_shards, publish_certified_merge, run_orderly_shard,
    },
    validate::validate_census,
};

#[derive(Debug, Parser)]
#[command(
    name = "poset-census",
    about = "Build validated finite-poset census records"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the complete redundant generator used as a small-order oracle.
    Reference {
        /// Poset order to enumerate.
        #[arg(long)]
        order: usize,
        /// Explicit root for generated work and census data.
        #[arg(long, env = "POSET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        /// Maximum number of relation subsets the reference generator may visit.
        #[arg(long, default_value_t = 1_000_000)]
        candidate_cap: u64,
    },
    /// Run the complete hereditary orderly generator.
    Orderly {
        /// Poset order to enumerate.
        #[arg(long)]
        order: usize,
        /// Explicit root for generated work and census data.
        #[arg(long, env = "POSET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        /// Maximum number of downsets the orderly generator may visit.
        #[arg(long, default_value_t = 10_000_000)]
        candidate_cap: u64,
    },
    /// Run or resume one deterministic canonical-parent range.
    OrderlyShard {
        /// Child order produced by extending the completed order immediately below it.
        #[arg(long)]
        order: usize,
        /// Explicit root containing the parent census and durable work state.
        #[arg(long, env = "POSET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        /// Stable identifier shared by every shard and the merge.
        #[arg(long)]
        run_id: String,
        /// Zero-based shard id.
        #[arg(long)]
        shard_id: usize,
        /// Total number of contiguous canonical-parent ranges.
        #[arg(long)]
        shard_count: usize,
        /// Maximum downsets this shard may visit across all resumptions.
        #[arg(long, default_value_t = 10_000_000)]
        candidate_cap: u64,
        /// Number of complete parent rows committed in each immutable part.
        #[arg(long, default_value_t = 1_000)]
        checkpoint_every: usize,
        /// Optional parent lease for this invocation; resume with the same command.
        #[arg(long)]
        parent_budget: Option<usize>,
    },
    /// Merge completed canonical-parent shards into validated candidate files.
    MergeOrderlyShards {
        /// Child order produced by the shards.
        #[arg(long)]
        order: usize,
        /// Explicit root containing durable shard state.
        #[arg(long, env = "POSET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        /// Stable run identifier used by the shards.
        #[arg(long)]
        run_id: String,
        /// Total number of completed shards.
        #[arg(long)]
        shard_count: usize,
    },
    /// Independently reconstruct every mathematical row in one completed shard.
    CertifyOrderlyShard {
        /// Child order contained in the completed shard.
        #[arg(long)]
        order: usize,
        /// Explicit root containing durable shard state.
        #[arg(long, env = "POSET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        /// Stable run identifier used by generation and merge.
        #[arg(long)]
        run_id: String,
        /// Zero-based shard id.
        #[arg(long)]
        shard_id: usize,
        /// Total number of shards in the run.
        #[arg(long)]
        shard_count: usize,
    },
    /// Atomically install a merge after every shard has passed certification.
    PublishCertifiedMerge {
        /// Completed child order to install.
        #[arg(long)]
        order: usize,
        /// Explicit root containing the merge and immutable census.
        #[arg(long, env = "POSET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        /// Stable run identifier used by generation and merge.
        #[arg(long)]
        run_id: String,
        /// Total number of certified shards.
        #[arg(long)]
        shard_count: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Reference {
            order,
            data_root,
            candidate_cap,
        } => {
            let started = SystemTime::now();
            let timer = Instant::now();
            let (raw_candidate_count, representatives) =
                reference_representatives(order, candidate_cap)?;
            let census = enrich_census_with_generator(
                Generator::Reference,
                order,
                candidate_cap,
                raw_candidate_count,
                representatives,
            )?;
            let validation = validate_census(&census)?;
            let output =
                emit_completed_order(&data_root, &census, &validation, started, timer.elapsed())?;
            println!(
                "wrote {} posets and {} linked complexes to {}",
                census.posets.len(),
                census.complexes.len(),
                output.display()
            );
        }
        Command::Orderly {
            order,
            data_root,
            candidate_cap,
        } => {
            let started = SystemTime::now();
            let timer = Instant::now();
            let (stats, representatives) = orderly_representatives(order, candidate_cap)?;
            let census = enrich_census_with_generator(
                Generator::HereditaryOrderly,
                order,
                candidate_cap,
                stats.downset_count,
                representatives,
            )?;
            let validation = validate_census(&census)?;
            let output =
                emit_completed_order(&data_root, &census, &validation, started, timer.elapsed())?;
            println!(
                "wrote {} posets and {} linked complexes to {}; visited {} downsets and ran {} canonical tests",
                census.posets.len(),
                census.complexes.len(),
                output.display(),
                stats.downset_count,
                stats.canonical_test_count
            );
        }
        Command::OrderlyShard {
            order,
            data_root,
            run_id,
            shard_id,
            shard_count,
            candidate_cap,
            checkpoint_every,
            parent_budget,
        } => {
            let outcome = run_orderly_shard(&ShardOptions {
                data_root,
                run_id,
                target_order: order,
                shard_id,
                shard_count,
                candidate_cap,
                checkpoint_every,
                parent_budget,
            })?;
            println!(
                "shard {shard_id}/{shard_count} {} at parent {}/{} with {} accepted children in {}",
                if outcome.complete {
                    "complete"
                } else {
                    "checkpointed"
                },
                outcome.next_parent_index,
                outcome.parent_end,
                outcome.accepted_count,
                outcome.directory.display()
            );
        }
        Command::MergeOrderlyShards {
            order,
            data_root,
            run_id,
            shard_count,
        } => {
            let outcome = merge_orderly_shards(&MergeOptions {
                data_root,
                run_id,
                target_order: order,
                shard_count,
            })?;
            println!(
                "merged {} posets and {} linked complexes into {}",
                outcome.poset_count,
                outcome.complex_count,
                outcome.directory.display()
            );
        }
        Command::CertifyOrderlyShard {
            order,
            data_root,
            run_id,
            shard_id,
            shard_count,
        } => {
            let outcome = certify_orderly_shard(&CertificationOptions {
                data_root,
                run_id,
                target_order: order,
                shard_id,
                shard_count,
            })?;
            println!(
                "certified shard {shard_id}/{shard_count}: {} posets and {} linked complexes in {}",
                outcome.poset_count,
                outcome.complex_count,
                outcome.directory.display()
            );
        }
        Command::PublishCertifiedMerge {
            order,
            data_root,
            run_id,
            shard_count,
        } => {
            let outcome = publish_certified_merge(&PublishOptions {
                data_root,
                run_id,
                target_order: order,
                shard_count,
            })?;
            println!(
                "published {} certified posets and {} linked complexes to {}",
                outcome.poset_count,
                outcome.complex_count,
                outcome.directory.display()
            );
        }
    }
    Ok(())
}
