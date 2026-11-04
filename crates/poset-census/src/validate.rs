use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use noether::{combin::Poset, topology::SimplicialComplex};
use serde::{Deserialize, Serialize};

use crate::{
    Census,
    enrich::{complex_key, complex_record, enrich_poset},
    records::PosetKey,
};

const ACCEPTED_COUNTS: &str =
    include_str!("../../../collections/finite-posets/validation/counts.json");

#[derive(Debug, Deserialize)]
struct CountTable {
    sources: CountSources,
    orders: Vec<ExpectedCounts>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CountSources {
    pub unlabeled_posets: String,
    pub connected_unlabeled_posets: String,
    pub labeled_posets: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ExpectedCounts {
    order: usize,
    unlabeled: String,
    connected_unlabeled: String,
    labeled: String,
}

/// Accepted enumeration totals used by complete-order validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedOrderCounts {
    pub unlabeled: u128,
    pub connected_unlabeled: u128,
    pub labeled: u128,
}

/// Return the accepted totals for one order from the checked-in count table.
pub fn accepted_order_counts(order: usize) -> Result<AcceptedOrderCounts> {
    let table: CountTable = serde_json::from_str(ACCEPTED_COUNTS)?;
    let expected = table
        .orders
        .iter()
        .find(|counts| counts.order == order)
        .with_context(|| format!("no accepted count table entry for order {order}"))?;
    Ok(AcceptedOrderCounts {
        unlabeled: parse_count(&expected.unlabeled)?,
        connected_unlabeled: parse_count(&expected.connected_unlabeled)?,
        labeled: parse_count(&expected.labeled)?,
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScopeReport {
    pub order: usize,
    pub status: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HomotopyScopeReport {
    pub source_orders_through: usize,
    pub status: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CountCheck {
    pub expected: String,
    pub observed: String,
    pub passed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationChecks {
    pub unlabeled_posets: CountCheck,
    pub connected_unlabeled_posets: CountCheck,
    pub connected_component_multiset_reconstruction: CountCheck,
    pub labeled_posets_from_automorphisms: CountCheck,
    pub poset_keys_unique_and_sorted: bool,
    pub complex_keys_unique_and_sorted: bool,
    pub reference_key_set_complete: bool,
    pub record_shapes: bool,
    pub canonical_keys_and_covers: bool,
    pub automorphism_generators_and_orbits: bool,
    pub core_replay_and_idempotence: bool,
    pub opposite_relations: bool,
    pub order_complex_reconstruction: bool,
    pub euler_and_integral_homology: bool,
    pub core_homology_invariance: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OracleReport {
    pub name: &'static str,
    pub status: &'static str,
    pub reason: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationReport {
    pub contract: &'static str,
    pub order: usize,
    pub status: &'static str,
    pub generator: &'static str,
    pub raw_candidate_count: u64,
    pub accepted_count_sources: CountSources,
    pub isomorphism_scope: ScopeReport,
    pub homotopy_scope: HomotopyScopeReport,
    pub checks: ValidationChecks,
    pub external_oracles: Vec<OracleReport>,
}

/// Recompute every local v1 gate available to the reference builder.
pub fn validate_census(census: &Census) -> Result<ValidationReport> {
    let table: CountTable = serde_json::from_str(ACCEPTED_COUNTS)?;
    let expected = table
        .orders
        .iter()
        .find(|counts| counts.order == census.order)
        .with_context(|| format!("no accepted count table entry for order {}", census.order))?;

    ensure!(
        census
            .posets
            .windows(2)
            .all(|rows| rows[0].key < rows[1].key),
        "Poset records are not uniquely sorted"
    );
    ensure!(
        census
            .complexes
            .windows(2)
            .all(|rows| rows[0].key < rows[1].key),
        "complex records are not uniquely sorted"
    );
    let complex_by_key = census
        .complexes
        .iter()
        .map(|record| (&record.key, record))
        .collect::<BTreeMap<_, _>>();

    let mut connected = 0_u128;
    let mut labeled = 0_u128;
    let factorial = factorial(census.order)?;
    for record in &census.posets {
        record.check_shape()?;
        ensure!(record.order == census.order, "mixed order in Poset output");
        connected += u128::from(record.invariants.is_connected);

        let poset = Poset::new(
            record.order,
            &record
                .covers
                .iter()
                .map(|edge| (edge[0], edge[1]))
                .collect::<Vec<_>>(),
        )?;
        let canonical = poset.canonical_label()?;
        ensure!(
            canonical.verify(&poset),
            "canonical-label evidence did not verify"
        );
        ensure!(
            PosetKey::from_canonical_label(&canonical) == record.key,
            "canonical key did not reconstruct"
        );
        let reconstructed_covers = canonical
            .poset()
            .covers()
            .edges()
            .iter()
            .map(|edge| [edge.source, edge.target])
            .collect::<Vec<_>>();
        ensure!(
            reconstructed_covers == record.covers,
            "canonical covers did not reconstruct"
        );
        let (recomputed_record, _) = enrich_poset(&poset)?;
        ensure!(
            recomputed_record == *record,
            "Poset record fields did not reconstruct"
        );

        let group = canonical.automorphism_group();
        ensure!(
            group
                .generators()
                .iter()
                .all(|generator| group.contains(generator).unwrap_or(false)),
            "automorphism generator failed group membership"
        );
        ensure!(
            group.order().to_decimal() == record.automorphism_group.order,
            "group order changed"
        );
        let group_order = record
            .automorphism_group
            .order
            .parse::<u128>()
            .context("reference validation group order exceeds u128")?;
        ensure!(
            factorial % group_order == 0,
            "automorphism order does not divide n!"
        );
        labeled += factorial / group_order;

        let core = poset.core()?;
        ensure!(core.verify(&poset), "core reduction did not replay");
        let core_key = PosetKey::from_canonical_label(&core.core().canonical_label()?);
        ensure!(
            core_key == record.relations.has_core.target,
            "core target changed"
        );
        ensure!(core.core().is_core(), "core has a beat point");
        ensure!(
            core.core().core()?.core() == core.core(),
            "core reduction is not idempotent"
        );

        let opposite_key = PosetKey::from_canonical_label(&poset.opposite()?.canonical_label()?);
        ensure!(
            opposite_key == record.relations.opposite.target,
            "opposite target changed"
        );
        ensure!(
            record.relations.opposite.is_self_dual == (opposite_key == record.key),
            "self-duality flag changed"
        );

        let complex = complex_by_key
            .get(&record.relations.order_complex.target)
            .context("order-complex target does not resolve")?;
        let expected_facets = poset
            .order_complex()?
            .facets()
            .into_iter()
            .map(|facet| facet.vertices().to_vec())
            .collect::<Vec<_>>();
        ensure!(
            expected_facets == complex.facets,
            "order-complex facets changed"
        );
        check_complex(complex)?;

        let source_homology = &complex.integral_homology;
        let core_complex = complex_record(core.core().order(), &core.core().order_complex()?)?;
        ensure!(
            homology_signature(source_homology)
                == homology_signature(&core_complex.integral_homology),
            "beat-point reduction changed integral homology for {:?}: source {:?}, core {:?}",
            record.key,
            source_homology,
            core_complex.integral_homology
        );
    }

    let observed_unlabeled = census.posets.len() as u128;
    let reconstructed = reconstruct_from_connected_counts(census.order, &table, connected)?;
    let expected_unlabeled = parse_count(&expected.unlabeled)?;
    let expected_connected = parse_count(&expected.connected_unlabeled)?;
    let expected_labeled = parse_count(&expected.labeled)?;
    ensure!(
        observed_unlabeled == expected_unlabeled,
        "unlabeled count mismatch"
    );
    ensure!(connected == expected_connected, "connected count mismatch");
    ensure!(
        reconstructed == observed_unlabeled,
        "component reconstruction count mismatch"
    );
    ensure!(
        labeled == expected_labeled,
        "labeled/unlabeled identity mismatch"
    );

    let checks = ValidationChecks {
        unlabeled_posets: count_check(expected_unlabeled, observed_unlabeled),
        connected_unlabeled_posets: count_check(expected_connected, connected),
        connected_component_multiset_reconstruction: count_check(observed_unlabeled, reconstructed),
        labeled_posets_from_automorphisms: count_check(expected_labeled, labeled),
        poset_keys_unique_and_sorted: true,
        complex_keys_unique_and_sorted: true,
        reference_key_set_complete: true,
        record_shapes: true,
        canonical_keys_and_covers: true,
        automorphism_generators_and_orbits: true,
        core_replay_and_idempotence: true,
        opposite_relations: true,
        order_complex_reconstruction: true,
        euler_and_integral_homology: true,
        core_homology_invariance: true,
    };
    Ok(ValidationReport {
        contract: crate::CONTRACT,
        order: census.order,
        status: "passed",
        generator: census.generator.id(),
        raw_candidate_count: census.raw_candidate_count,
        accepted_count_sources: table.sources,
        isomorphism_scope: ScopeReport {
            order: census.order,
            status: "complete",
        },
        homotopy_scope: HomotopyScopeReport {
            source_orders_through: census.order,
            status: "complete",
        },
        checks,
        external_oracles: vec![OracleReport {
            name: "GAP",
            status: "skipped",
            reason: "not required for the reference pilot",
        }],
    })
}

fn homology_signature(
    groups: &[crate::records::IntegralHomologyRecord],
) -> BTreeMap<usize, (usize, Vec<String>)> {
    groups
        .iter()
        .filter(|group| group.free_rank != 0 || !group.invariant_factors.is_empty())
        .map(|group| {
            (
                group.degree,
                (group.free_rank, group.invariant_factors.clone()),
            )
        })
        .collect()
}

fn check_complex(record: &crate::records::ComplexRecord) -> Result<()> {
    ensure!(
        complex_key(record.vertex_count, &record.facets)? == record.key,
        "complex key did not reconstruct"
    );
    let complex = SimplicialComplex::from_facets(&record.facets);
    let reconstructed = complex_record(record.vertex_count, &complex)?;
    ensure!(
        reconstructed == *record,
        "complex invariants did not reconstruct"
    );
    let betti_euler = record
        .integral_homology
        .iter()
        .map(|group| {
            let rank = group.free_rank as i128;
            if group.degree % 2 == 0 { rank } else { -rank }
        })
        .sum::<i128>();
    ensure!(
        betti_euler == i128::from(record.euler_characteristic),
        "Euler/Betti mismatch"
    );
    Ok(())
}

fn reconstruct_from_connected_counts(
    order: usize,
    table: &CountTable,
    observed_current_connected: u128,
) -> Result<u128> {
    if order == 0 {
        return Ok(1);
    }
    let mut connected = BTreeMap::new();
    for entry in &table.orders {
        if entry.order > 0 && entry.order < order {
            connected.insert(entry.order, parse_count(&entry.connected_unlabeled)?);
        }
    }
    connected.insert(order, observed_current_connected);

    let mut coefficients = vec![0_u128; order + 1];
    coefficients[0] = 1;
    for (size, kinds) in connected {
        let mut updated = vec![0_u128; order + 1];
        for total in 0..=order {
            if coefficients[total] == 0 {
                continue;
            }
            for multiplicity in 0..=((order - total) / size) {
                let choices = multiset_count(kinds, multiplicity)?;
                updated[total + multiplicity * size] = updated[total + multiplicity * size]
                    .checked_add(
                        coefficients[total]
                            .checked_mul(choices)
                            .context("count overflow")?,
                    )
                    .context("count overflow")?;
            }
        }
        coefficients = updated;
    }
    Ok(coefficients[order])
}

fn multiset_count(kinds: u128, multiplicity: usize) -> Result<u128> {
    if multiplicity == 0 {
        return Ok(1);
    }
    if kinds == 0 {
        return Ok(0);
    }
    let mut value = 1_u128;
    for step in 1..=multiplicity as u128 {
        value = value
            .checked_mul(kinds + step - 1)
            .context("multiset count overflow")?
            / step;
    }
    Ok(value)
}

fn factorial(order: usize) -> Result<u128> {
    (1..=order).try_fold(1_u128, |value, factor| {
        value
            .checked_mul(factor as u128)
            .context("factorial exceeds u128")
    })
}

fn parse_count(value: &str) -> Result<u128> {
    value
        .parse()
        .with_context(|| format!("accepted count {value:?} exceeds u128"))
}

fn count_check(expected: u128, observed: u128) -> CountCheck {
    CountCheck {
        expected: expected.to_string(),
        observed: observed.to_string(),
        passed: expected == observed,
    }
}
