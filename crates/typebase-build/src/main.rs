use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use typebase_build::eilenberg_maclane::{
    EilenbergMacLaneBuildOptions, build_eilenberg_maclane_component,
};
use typebase_build::finite_group_cohomology::{
    FiniteGroupCohomologyBuildOptions, build_finite_group_cohomology_component,
};
use typebase_build::poset_groups::{GroupIdentificationOptions, identify_poset_groups};
use typebase_build::posets::{PosetBuildOptions, build_poset_component};
use typebase_build::simplicial_complexes::{
    SimplicialComplexBuildOptions, build_simplicial_complex_component,
};
use typebase_build::simplicial_sets::{SimplicialSetBuildOptions, build_simplicial_set_component};
use typebase_build::steiner::{SteinerBuildOptions, build_steiner_component};
use typebase_build::steiner_groups::{SteinerGroupIdentificationOptions, identify_steiner_groups};

#[derive(Debug, Parser)]
#[command(
    name = "typebase-build",
    about = "Build static TypeBase volume artifacts"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Identify abstract Poset automorphism groups using GAP's Small Groups library.
    IdentifyPosetGroups {
        /// Root containing certified finite-Poset census orders.
        #[arg(long, env = "POSET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        /// Empty output root for the immutable enrichment.
        #[arg(long)]
        output_root: PathBuf,
        /// Largest certified Poset order included in the enrichment.
        #[arg(long, default_value_t = 10)]
        maximum_order: usize,
        /// GAP executable supplying IdGroup and the Small Groups library.
        #[arg(long, default_value = "gap")]
        gap_executable: PathBuf,
    },
    /// Build the finite-Poset component of TypeBase Volume I.
    Posets {
        /// Root containing certified finite-Poset census orders.
        #[arg(long, env = "POSET_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        /// Empty output root for immutable browser artifacts.
        #[arg(long)]
        output_root: PathBuf,
        /// Largest certified Poset order selected into the component.
        #[arg(long, default_value_t = 10)]
        maximum_order: usize,
        /// Maximum consecutive labels stored in one detail Parquet file.
        #[arg(long, default_value_t = 10_000)]
        rows_per_detail_part: usize,
        /// Optional immutable abstract automorphism-group enrichment JSONL.
        #[arg(long)]
        group_identifications: Option<PathBuf>,
    },
    /// Build the strong-Steiner-category component of TypeBase Volume I.
    SteinerCategories {
        /// Root containing certified strong-Steiner-category census shells.
        #[arg(long, env = "STEINER_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        /// Empty output root for immutable browser artifacts.
        #[arg(long)]
        output_root: PathBuf,
        /// Largest complete atomic-size shell selected into the component.
        #[arg(long, default_value_t = 11)]
        maximum_atomic_size: usize,
        /// Root containing the certified finite-Poset census for homotopy links.
        #[arg(long, env = "POSET_CENSUS_DATA_ROOT")]
        poset_data_root: PathBuf,
        /// Optional immutable abstract automorphism-group enrichment JSONL.
        #[arg(long)]
        group_identifications: Option<PathBuf>,
    },
    /// Identify abstract Steiner-category automorphism groups with GAP.
    IdentifySteinerGroups {
        #[arg(long, env = "STEINER_CENSUS_DATA_ROOT")]
        data_root: PathBuf,
        #[arg(long)]
        output_root: PathBuf,
        #[arg(long, default_value_t = 11)]
        maximum_atomic_size: usize,
        #[arg(long, default_value = "gap")]
        gap_executable: PathBuf,
    },
    /// Build the finite-group cohomology component from a complete modular census.
    FiniteGroupCohomology {
        #[arg(long, env = "FINITE_GROUP_COHOMOLOGY_DATA_ROOT")]
        data_root: PathBuf,
        #[arg(long)]
        output_root: PathBuf,
        #[arg(long, default_value_t = 32)]
        maximum_order: usize,
        #[arg(long, default_value_t = 8)]
        initial_proof_degree: usize,
        #[arg(long, default_value_t = 12)]
        maximum_proof_degree: usize,
        #[arg(long, default_value_t = 16)]
        integral_additive_through_degree: usize,
        #[arg(long, default_value_t = 100_000)]
        integral_bar_entry_limit: usize,
    },
    /// Build the Eilenberg--Mac Lane component from one pinned invariant census.
    EilenbergMacLane {
        #[arg(long, env = "EILENBERG_MACLANE_DATA_ROOT")]
        data_root: PathBuf,
        #[arg(long)]
        output_root: PathBuf,
        #[arg(long, default_value_t = 32)]
        maximum_finite_order: usize,
        #[arg(long, default_value_t = 4)]
        maximum_delooping_degree: usize,
        #[arg(long, default_value_t = 16)]
        exact_through_degree: usize,
    },
    /// Build the finite-simplicial-complex component from complete shells.
    SimplicialComplexes {
        #[arg(long, env = "SIMPLICIAL_COMPLEX_DATA_ROOT")]
        data_root: PathBuf,
        #[arg(long)]
        output_root: PathBuf,
        #[arg(long, default_value_t = 6)]
        maximum_vertices: usize,
        #[arg(long, default_value_t = 256)]
        rows_per_detail_part: usize,
    },
    /// Build the finite-simplicial-set component at the first-edition frontier.
    SimplicialSets {
        #[arg(long, env = "SIMPLICIAL_SET_DATA_ROOT")]
        data_root: PathBuf,
        #[arg(long)]
        output_root: PathBuf,
        #[arg(long, default_value_t = 7)]
        maximum_total_nondegenerate: usize,
        #[arg(long, default_value_t = 3)]
        maximum_dimension: usize,
        #[arg(long, default_value_t = 2)]
        maximum_dimension_at_top_total: usize,
        #[arg(long, default_value_t = 128)]
        rows_per_detail_part: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::IdentifyPosetGroups {
            data_root,
            output_root,
            maximum_order,
            gap_executable,
        } => {
            let outcome = identify_poset_groups(&GroupIdentificationOptions {
                data_root,
                output_root,
                maximum_order,
                gap_executable,
            })?;
            println!(
                "identified {} of {} Poset automorphism groups in {}",
                outcome.identified_count,
                outcome.poset_count,
                outcome.directory.display()
            );
        }
        Command::Posets {
            data_root,
            output_root,
            maximum_order,
            rows_per_detail_part,
            group_identifications,
        } => {
            let outcome = build_poset_component(&PosetBuildOptions {
                data_root,
                output_root,
                maximum_order,
                rows_per_detail_part,
                group_identifications,
            })?;
            println!(
                "built {} TypeBase Poset rows in {}",
                outcome.poset_count,
                outcome.directory.display()
            );
        }
        Command::SteinerCategories {
            data_root,
            output_root,
            maximum_atomic_size,
            poset_data_root,
            group_identifications,
        } => {
            let outcome = build_steiner_component(&SteinerBuildOptions {
                data_root,
                output_root,
                maximum_atomic_size,
                poset_data_root,
                group_identifications,
            })?;
            println!(
                "built {} TypeBase Steiner-category rows in {}",
                outcome.category_count,
                outcome.directory.display()
            );
        }
        Command::IdentifySteinerGroups {
            data_root,
            output_root,
            maximum_atomic_size,
            gap_executable,
        } => {
            let outcome = identify_steiner_groups(&SteinerGroupIdentificationOptions {
                data_root,
                output_root,
                maximum_atomic_size,
                gap_executable,
            })?;
            println!(
                "identified {} of {} Steiner-category automorphism groups in {}",
                outcome.identified_count,
                outcome.category_count,
                outcome.directory.display()
            );
        }
        Command::FiniteGroupCohomology {
            data_root,
            output_root,
            maximum_order,
            initial_proof_degree,
            maximum_proof_degree,
            integral_additive_through_degree,
            integral_bar_entry_limit,
        } => {
            let outcome =
                build_finite_group_cohomology_component(&FiniteGroupCohomologyBuildOptions {
                    data_root,
                    output_root,
                    maximum_order,
                    initial_proof_degree,
                    maximum_proof_degree,
                    integral_additive_through_degree,
                    integral_bar_entry_limit,
                })?;
            println!(
                "built {} finite-group cohomology coefficient rows in {}",
                outcome.coefficient_rows,
                outcome.directory.display()
            );
        }
        Command::EilenbergMacLane {
            data_root,
            output_root,
            maximum_finite_order,
            maximum_delooping_degree,
            exact_through_degree,
        } => {
            let outcome = build_eilenberg_maclane_component(&EilenbergMacLaneBuildOptions {
                data_root,
                output_root,
                maximum_finite_order,
                maximum_delooping_degree,
                exact_through_degree,
            })?;
            println!(
                "built {} Eilenberg--Mac Lane spaces in {}",
                outcome.space_count,
                outcome.directory.display()
            );
        }
        Command::SimplicialComplexes {
            data_root,
            output_root,
            maximum_vertices,
            rows_per_detail_part,
        } => {
            let outcome = build_simplicial_complex_component(&SimplicialComplexBuildOptions {
                data_root,
                output_root,
                maximum_vertices,
                rows_per_detail_part,
            })?;
            println!(
                "built {} finite simplicial complexes in {}",
                outcome.complex_count,
                outcome.directory.display()
            );
        }
        Command::SimplicialSets {
            data_root,
            output_root,
            maximum_total_nondegenerate,
            maximum_dimension,
            maximum_dimension_at_top_total,
            rows_per_detail_part,
        } => {
            let outcome = build_simplicial_set_component(&SimplicialSetBuildOptions {
                data_root,
                output_root,
                maximum_total_nondegenerate,
                maximum_dimension,
                maximum_dimension_at_top_total,
                rows_per_detail_part,
            })?;
            println!(
                "built {} finite simplicial sets in {}",
                outcome.simplicial_set_count,
                outcome.directory.display()
            );
        }
    }
    Ok(())
}
