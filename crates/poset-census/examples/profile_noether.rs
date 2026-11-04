use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use noether::combin::Poset;
use poset_census::{
    Generator,
    enrich::enrich_poset,
    enrich_census, enrich_census_with_generator,
    enumerate::{orderly_representatives, reference_representatives},
    validate::validate_census,
};

#[derive(Debug, Parser)]
#[command(about = "Reproducible finite-poset profiling harness")]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Time individual Noether operations on representative poset families.
    Operations {
        #[arg(long, default_value_t = 5)]
        repetitions: usize,
        #[arg(long, default_value_t = 12)]
        maximum_topology_order: usize,
    },
    /// Time the redundant generator and downstream enrichment by complete order.
    Pipeline {
        #[arg(long, default_value_t = 6)]
        maximum_order: usize,
        #[arg(long, default_value_t = 100_000_000)]
        candidate_cap: u64,
    },
    /// Time hereditary orderly generation and downstream enrichment.
    OrderlyPipeline {
        #[arg(long, default_value_t = 7)]
        maximum_order: usize,
        #[arg(long, default_value_t = 10_000_000)]
        candidate_cap: u64,
    },
    /// Time orderly generation alone at one target order.
    OrderlyGeneration {
        #[arg(long)]
        order: usize,
        #[arg(long, default_value_t = 100_000_000)]
        candidate_cap: u64,
    },
}

fn main() -> Result<()> {
    match Arguments::parse().command {
        Command::Operations {
            repetitions,
            maximum_topology_order,
        } => profile_operations(repetitions, maximum_topology_order),
        Command::Pipeline {
            maximum_order,
            candidate_cap,
        } => profile_pipeline(maximum_order, candidate_cap),
        Command::OrderlyPipeline {
            maximum_order,
            candidate_cap,
        } => profile_orderly_pipeline(maximum_order, candidate_cap),
        Command::OrderlyGeneration {
            order,
            candidate_cap,
        } => profile_orderly_generation(order, candidate_cap),
    }
}

fn profile_orderly_generation(order: usize, candidate_cap: u64) -> Result<()> {
    let started = Instant::now();
    let (stats, representatives) = orderly_representatives(order, candidate_cap)
        .with_context(|| format!("orderly enumeration at order {order}"))?;
    println!(
        "order,parents,downsets,orbit_representatives,canonical_tests,isomorphism_classes,enumerate_milliseconds"
    );
    let level = stats.levels.last();
    println!(
        "{order},{},{},{},{},{},{:.3}",
        level.map_or(0, |value| value.parent_count),
        level.map_or(0, |value| value.downset_count),
        level.map_or(0, |value| value.orbit_representative_count),
        level.map_or(0, |value| value.canonical_test_count),
        representatives.len(),
        milliseconds(started.elapsed()),
    );
    Ok(())
}

fn profile_operations(repetitions: usize, maximum_topology_order: usize) -> Result<()> {
    anyhow::ensure!(repetitions > 0, "repetitions must be positive");
    println!("case,order,covers,operation,median_milliseconds,repetitions");
    for case in cases()? {
        let poset = Poset::new(case.order, &case.relations)?;
        emit(
            &case,
            "construct",
            measure(repetitions, || Ok(Poset::new(case.order, &case.relations)?))?,
            repetitions,
        );
        emit(
            &case,
            "canonical_and_automorphisms",
            measure(repetitions, || Ok(poset.canonical_label()?))?,
            repetitions,
        );
        emit(
            &case,
            "structural_invariants",
            measure(repetitions, || {
                Ok::<_, anyhow::Error>((
                    poset.strict_lower_counts(),
                    poset.strict_upper_counts(),
                    poset.connected_components(),
                    poset.height(),
                    poset.width(),
                    poset.is_lattice(),
                ))
            })?,
            repetitions,
        );
        emit(
            &case,
            "core",
            measure(repetitions, || Ok(poset.core()?))?,
            repetitions,
        );
        emit(
            &case,
            "opposite_and_canonical",
            measure(repetitions, || Ok(poset.opposite()?.canonical_label()?))?,
            repetitions,
        );

        if case.order <= maximum_topology_order {
            emit(
                &case,
                "order_complex",
                measure(repetitions, || Ok(poset.order_complex()?))?,
                repetitions,
            );
            let complex = poset.order_complex()?;
            emit(
                &case,
                "integral_homology_all_degrees",
                measure(repetitions, || {
                    if complex.dimension() < 0 {
                        return Ok(Vec::new());
                    }
                    (0..=complex.dimension() as usize)
                        .map(|degree| complex.homology_invariants(degree))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(Into::into)
                })?,
                repetitions,
            );
            emit(
                &case,
                "integral_homology_reused_boundaries",
                measure(repetitions, || {
                    complex.graded_homology_invariants().map_err(Into::into)
                })?,
                repetitions,
            );
            emit(
                &case,
                "integral_homology_via_core",
                measure(repetitions, || {
                    poset.graded_homology_invariants().map_err(Into::into)
                })?,
                repetitions,
            );
            emit(
                &case,
                "full_enrichment",
                measure(repetitions, || enrich_poset(&poset))?,
                repetitions,
            );
        }
    }
    Ok(())
}

fn profile_orderly_pipeline(maximum_order: usize, candidate_cap: u64) -> Result<()> {
    println!(
        "order,parents,downsets,orbit_representatives,canonical_tests,isomorphism_classes,enumerate_milliseconds,enrich_milliseconds,validate_milliseconds"
    );
    for order in 0..=maximum_order {
        let started = Instant::now();
        let (stats, representatives) = orderly_representatives(order, candidate_cap)
            .with_context(|| format!("orderly enumeration at order {order}"))?;
        let enumerate = started.elapsed();
        let level = stats.levels.last();

        let started = Instant::now();
        let census = enrich_census_with_generator(
            Generator::HereditaryOrderly,
            order,
            candidate_cap,
            stats.downset_count,
            representatives,
        )?;
        let enrich = started.elapsed();

        let started = Instant::now();
        validate_census(&census)?;
        let validate = started.elapsed();
        println!(
            "{order},{},{},{},{},{},{:.3},{:.3},{:.3}",
            level.map_or(0, |value| value.parent_count),
            level.map_or(0, |value| value.downset_count),
            level.map_or(0, |value| value.orbit_representative_count),
            level.map_or(0, |value| value.canonical_test_count),
            census.posets.len(),
            milliseconds(enumerate),
            milliseconds(enrich),
            milliseconds(validate)
        );
    }
    Ok(())
}

fn profile_pipeline(maximum_order: usize, candidate_cap: u64) -> Result<()> {
    println!(
        "order,raw_candidates,isomorphism_classes,complexes,enumerate_milliseconds,enrich_milliseconds,validate_milliseconds"
    );
    for order in 0..=maximum_order {
        let started = Instant::now();
        let (raw_candidates, representatives) = reference_representatives(order, candidate_cap)
            .with_context(|| format!("reference enumeration at order {order}"))?;
        let enumerate = started.elapsed();

        let started = Instant::now();
        let census = enrich_census(order, candidate_cap, raw_candidates, representatives)?;
        let enrich = started.elapsed();

        let started = Instant::now();
        validate_census(&census)?;
        let validate = started.elapsed();
        println!(
            "{order},{raw_candidates},{},{},{:.3},{:.3},{:.3}",
            census.posets.len(),
            census.complexes.len(),
            milliseconds(enumerate),
            milliseconds(enrich),
            milliseconds(validate)
        );
    }
    Ok(())
}

struct Case {
    name: String,
    order: usize,
    relations: Vec<(u32, u32)>,
}

fn cases() -> Result<Vec<Case>> {
    let mut cases = Vec::new();
    for order in [8, 10, 12, 16] {
        cases.push(Case {
            name: format!("antichain_{order}"),
            order,
            relations: Vec::new(),
        });
        cases.push(Case {
            name: format!("chain_{order}"),
            order,
            relations: (0..order.saturating_sub(1) as u32)
                .map(|lower| (lower, lower + 1))
                .collect(),
        });
        cases.push(Case {
            name: format!("complete_height_two_{order}"),
            order,
            relations: complete_height_two(order)?,
        });
        cases.push(Case {
            name: format!("crown_{order}"),
            order,
            relations: crown(order)?,
        });
        cases.push(Case {
            name: format!("generic_layered_{order}"),
            order,
            relations: generic_layered(order)?,
        });
    }
    cases.push(Case {
        name: "boolean_lattice_4".to_owned(),
        order: 16,
        relations: boolean_lattice(4),
    });
    Ok(cases)
}

fn complete_height_two(order: usize) -> Result<Vec<(u32, u32)>> {
    anyhow::ensure!(order.is_multiple_of(2), "height-two case needs even order");
    let middle = u32::try_from(order / 2)?;
    let order = u32::try_from(order)?;
    Ok((0..middle)
        .flat_map(|lower| (middle..order).map(move |upper| (lower, upper)))
        .collect())
}

fn crown(order: usize) -> Result<Vec<(u32, u32)>> {
    anyhow::ensure!(order.is_multiple_of(2), "crown case needs even order");
    let middle = u32::try_from(order / 2)?;
    Ok((0..middle)
        .flat_map(|lower| {
            (0..middle)
                .filter(move |&upper_index| upper_index != lower)
                .map(move |upper_index| (lower, middle + upper_index))
        })
        .collect())
}

fn generic_layered(order: usize) -> Result<Vec<(u32, u32)>> {
    let first = order / 3;
    let second = 2 * order / 3;
    let order_u32 = u32::try_from(order)?;
    let mut relations = Vec::new();
    for lower in 0..u32::try_from(first)? {
        for upper in u32::try_from(first)?..u32::try_from(second)? {
            if mix(lower, upper).is_multiple_of(3) {
                relations.push((lower, upper));
            }
        }
    }
    for lower in u32::try_from(first)?..u32::try_from(second)? {
        for upper in u32::try_from(second)?..order_u32 {
            if mix(lower, upper).is_multiple_of(4) {
                relations.push((lower, upper));
            }
        }
    }
    Ok(relations)
}

fn boolean_lattice(rank: u32) -> Vec<(u32, u32)> {
    let order = 1_u32 << rank;
    (0..order)
        .flat_map(|subset| {
            (0..rank).filter_map(move |bit| {
                let member = 1_u32 << bit;
                (subset & member == 0).then_some((subset, subset | member))
            })
        })
        .collect()
}

fn mix(lower: u32, upper: u32) -> u32 {
    lower
        .wrapping_mul(0x9e37_79b9)
        .rotate_left(7)
        .wrapping_add(upper.wrapping_mul(0x85eb_ca6b))
}

fn measure<T, F>(repetitions: usize, mut operation: F) -> Result<Duration>
where
    F: FnMut() -> Result<T>,
{
    black_box(operation()?);
    let mut samples = Vec::with_capacity(repetitions);
    for _ in 0..repetitions {
        let started = Instant::now();
        black_box(operation()?);
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    Ok(samples[samples.len() / 2])
}

fn emit(case: &Case, operation: &str, duration: Duration, repetitions: usize) {
    println!(
        "{},{},{},{operation},{:.3},{repetitions}",
        case.name,
        case.order,
        case.relations.len(),
        milliseconds(duration)
    );
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
