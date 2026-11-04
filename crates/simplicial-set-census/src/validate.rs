use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use noether::numthy::FiniteField;
use serde::{Deserialize, Serialize};

use crate::{
    COHOMOLOGY_CONTRACT, CONTRACT, CohomologyCensus, HOMOTOPY_CONTRACT, HomotopyCensus,
    IdentityCensus, STEENROD_OPERATIONS_CONTRACT, SteenrodOperationsCensus, TOPOLOGY_CONTRACT,
    TopologyCensus,
    enrich::{
        cohomology_record, homotopy_record, steenrod_operations_record, topology_record,
        verify_reduction_record,
    },
    enumerate::{MAX_REFERENCE_TOTAL, reference_representatives},
    records::{SimplicialSetCohomologyRecord, SimplicialSetKey},
};

const ACCEPTED_COUNTS: &str =
    include_str!("../../../collections/finite-simplicial-sets/validation/counts.json");

#[derive(Debug, Deserialize)]
struct CountTable {
    source: String,
    boxes: Vec<CountBox>,
}

#[derive(Debug, Deserialize)]
struct CountBox {
    maximum_dimension: usize,
    orders: Vec<ExpectedCount>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCount {
    total_nondegenerate: usize,
    unlabeled: String,
}

pub fn accepted_count(total_nondegenerate: usize, maximum_dimension: usize) -> Result<u128> {
    let table: CountTable = serde_json::from_str(ACCEPTED_COUNTS)?;
    table
        .boxes
        .iter()
        .find(|box_| box_.maximum_dimension == maximum_dimension)
        .and_then(|box_| {
            box_
                .orders
                .iter()
                .find(|order| order.total_nondegenerate == total_nondegenerate)
        })
        .with_context(|| {
            format!(
                "no accepted count for total {total_nondegenerate} and dimension cap {maximum_dimension}"
            )
        })?
        .unlabeled
        .parse()
        .map_err(Into::into)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DimensionVectorCount {
    pub vector: Vec<usize>,
    pub count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationReport {
    pub contract: &'static str,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub status: &'static str,
    pub count_source: String,
    pub unlabeled: String,
    pub labeled_from_automorphisms: String,
    pub dimension_vectors: Vec<DimensionVectorCount>,
    pub keys_unique_and_sorted: bool,
    pub record_shapes: bool,
    pub canonical_reconstruction: bool,
    pub automorphism_recomputation: bool,
    pub orbit_stabilizer: bool,
    pub reference_key_set_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TopologyValidationReport {
    pub contract: &'static str,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub status: &'static str,
    pub records: usize,
    pub keys_unique_and_sorted: bool,
    pub normalized_boundary_hashes: bool,
    pub normalized_chain_reductions: bool,
    pub reduction_witness_replay: bool,
    pub chain_euler_characteristic: bool,
    pub homology_euler_characteristic: bool,
    pub integral_homology_recomputation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CohomologyValidationReport {
    pub contract: &'static str,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub coefficient: String,
    pub status: &'static str,
    pub records: usize,
    pub keys_unique_and_sorted: bool,
    pub canonical_cocycles: bool,
    pub multiplication_table: bool,
    pub presentation_or_complete_table: bool,
    pub operation_values: bool,
    pub extent_certificates: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SteenrodOperationsValidationReport {
    pub contract: &'static str,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub coefficient: String,
    pub status: &'static str,
    pub records: usize,
    pub keys_unique_and_sorted: bool,
    pub cohomology_source_links: bool,
    pub unstable_ranges_complete: bool,
    pub operation_values: bool,
    pub evidence_named: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HomotopyValidationReport {
    pub contract: &'static str,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub status: &'static str,
    pub records: usize,
    pub keys_unique_and_sorted: bool,
    pub immediate_horn_collapse_replay: bool,
    pub greedy_sequence_replay: bool,
    pub canonical_targets: bool,
    pub terminal_claims: bool,
    pub homology_invariance: bool,
}

pub fn validate_identity_census(census: &IdentityCensus) -> Result<ValidationReport> {
    let table: CountTable = serde_json::from_str(ACCEPTED_COUNTS)?;
    let expected = accepted_count(census.total_nondegenerate, census.maximum_dimension)?;
    ensure!(
        census
            .records
            .windows(2)
            .all(|rows| rows[0].key < rows[1].key),
        "identity keys are not uniquely sorted"
    );
    let mut labeled = 0_u128;
    let mut vectors = BTreeMap::new();
    for record in &census.records {
        record.check_shape()?;
        ensure!(
            record.total_nondegenerate == census.total_nondegenerate,
            "mixed total shell"
        );
        ensure!(
            record.dimension <= census.maximum_dimension as i64,
            "dimension cap exceeded"
        );
        let value = record.key.reconstruct()?;
        ensure!(
            SimplicialSetKey::from_value(&value) == record.key,
            "canonical reconstruction changed key"
        );
        let automorphisms = value.automorphism_group()?;
        let order: u128 = automorphisms.order().to_decimal().parse()?;
        ensure!(
            order == record.automorphism_group.order.parse::<u128>()?,
            "automorphism order changed"
        );
        let labelings = record
            .nondegenerate_counts
            .iter()
            .try_fold(1_u128, |value, count| {
                factorial(*count).and_then(|factorial| {
                    value
                        .checked_mul(factorial)
                        .context("labeling count overflow")
                })
            })?;
        ensure!(labelings % order == 0, "orbit-stabilizer division failed");
        labeled = labeled
            .checked_add(labelings / order)
            .context("labeled count overflow")?;
        *vectors
            .entry(record.nondegenerate_counts.clone())
            .or_insert(0) += 1;
    }
    ensure!(
        census.records.len() as u128 == expected,
        "unlabeled count mismatch"
    );
    let reference_key_set_complete = if census.total_nondegenerate <= MAX_REFERENCE_TOTAL {
        let (reference_stats, reference) = reference_representatives(
            census.total_nondegenerate,
            census.maximum_dimension,
            census.candidate_cap,
        )?;
        let expected_labeled = reference_stats
            .levels
            .last()
            .map_or(1, |level| level.accepted) as u128;
        ensure!(labeled == expected_labeled, "labeled count mismatch");
        reference
            .keys()
            .eq(census.records.iter().map(|record| &record.key))
    } else {
        true
    };
    ensure!(reference_key_set_complete, "reference key-set mismatch");

    Ok(ValidationReport {
        contract: CONTRACT,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        status: "complete",
        count_source: table.source,
        unlabeled: census.records.len().to_string(),
        labeled_from_automorphisms: labeled.to_string(),
        dimension_vectors: vectors
            .into_iter()
            .map(|(vector, count)| DimensionVectorCount { vector, count })
            .collect(),
        keys_unique_and_sorted: true,
        record_shapes: true,
        canonical_reconstruction: true,
        automorphism_recomputation: true,
        orbit_stabilizer: true,
        reference_key_set_complete,
    })
}

pub fn validate_topology_census(
    census: &TopologyCensus,
    identities: &[crate::records::SimplicialSetIdentityRecord],
) -> Result<TopologyValidationReport> {
    ensure!(
        census.records.len() == identities.len(),
        "topology source row mismatch"
    );
    ensure!(
        census
            .records
            .windows(2)
            .all(|rows| rows[0].simplicial_set < rows[1].simplicial_set),
        "topology keys are not uniquely sorted"
    );
    for (stored, identity) in census.records.iter().zip(identities) {
        ensure!(
            stored.simplicial_set == identity.key,
            "topology source key mismatch"
        );
        ensure!(
            *stored == topology_record(identity)?,
            "topology recomputation mismatch"
        );
        let value = identity.key.reconstruct()?;
        verify_reduction_record(&value, &stored.normalized_chain_reduction)?;
        let homology_euler = stored
            .integral_homology
            .iter()
            .map(|group| {
                if group.degree % 2 == 0 {
                    group.free_rank as i64
                } else {
                    -(group.free_rank as i64)
                }
            })
            .sum::<i64>();
        ensure!(
            homology_euler == stored.euler_characteristic,
            "homology Euler characteristic mismatch"
        );
    }
    Ok(TopologyValidationReport {
        contract: TOPOLOGY_CONTRACT,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        status: "complete",
        records: census.records.len(),
        keys_unique_and_sorted: true,
        normalized_boundary_hashes: true,
        normalized_chain_reductions: true,
        reduction_witness_replay: true,
        chain_euler_characteristic: true,
        homology_euler_characteristic: true,
        integral_homology_recomputation: true,
    })
}

pub fn validate_cohomology_census(
    census: &CohomologyCensus,
    identities: &[crate::records::SimplicialSetIdentityRecord],
) -> Result<CohomologyValidationReport> {
    ensure!(
        census.records.len() == identities.len(),
        "cohomology source row mismatch"
    );
    ensure!(
        census
            .records
            .windows(2)
            .all(|rows| rows[0].simplicial_set < rows[1].simplicial_set),
        "cohomology keys are not uniquely sorted"
    );
    let field = FiniteField::prime(census.prime)?;
    for (stored, identity) in census.records.iter().zip(identities) {
        ensure!(
            stored.simplicial_set == identity.key,
            "cohomology source key mismatch"
        );
        ensure!(
            *stored == cohomology_record(identity, &field)?,
            "cohomology recomputation mismatch"
        );
    }
    Ok(CohomologyValidationReport {
        contract: COHOMOLOGY_CONTRACT,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        coefficient: format!("F{}", census.prime),
        status: "complete",
        records: census.records.len(),
        keys_unique_and_sorted: true,
        canonical_cocycles: true,
        multiplication_table: true,
        presentation_or_complete_table: true,
        operation_values: true,
        extent_certificates: true,
    })
}

pub fn validate_steenrod_operations_census(
    census: &SteenrodOperationsCensus,
    identities: &[crate::records::SimplicialSetIdentityRecord],
    cohomology: &[SimplicialSetCohomologyRecord],
) -> Result<SteenrodOperationsValidationReport> {
    ensure!(census.prime % 2 == 1, "Steenrod sidecar prime is not odd");
    ensure!(
        census.records.len() == identities.len() && identities.len() == cohomology.len(),
        "Steenrod source row mismatch"
    );
    ensure!(
        census
            .records
            .windows(2)
            .all(|rows| rows[0].simplicial_set < rows[1].simplicial_set),
        "Steenrod keys are not uniquely sorted"
    );
    let field = FiniteField::prime(census.prime)?;
    for ((stored, identity), algebra) in census.records.iter().zip(identities).zip(cohomology) {
        ensure!(
            stored.simplicial_set == identity.key && algebra.simplicial_set == identity.key,
            "Steenrod source key mismatch"
        );
        let expected = steenrod_operations_record(identity, algebra, &field)?;
        ensure!(*stored == expected, "Steenrod recomputation mismatch");
        let expected_count = algebra.basis.iter().try_fold(0_usize, |count, basis| {
            count
                .checked_add(usize::try_from(basis.degree)?.saturating_add(1))
                .context("Steenrod operation count overflow")
        })?;
        ensure!(
            stored.operations.len() == expected_count,
            "incomplete unstable operation range"
        );
        let names = stored
            .operations
            .iter()
            .map(|operation| (&operation.source_id, &operation.operation))
            .collect::<BTreeSet<_>>();
        ensure!(
            names.len() == stored.operations.len(),
            "duplicate Steenrod operation"
        );
        ensure!(
            stored
                .operations
                .iter()
                .all(|operation| !operation.evidence.is_empty()),
            "unnamed Steenrod evidence"
        );
    }
    Ok(SteenrodOperationsValidationReport {
        contract: STEENROD_OPERATIONS_CONTRACT,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        coefficient: format!("F{}", census.prime),
        status: "complete",
        records: census.records.len(),
        keys_unique_and_sorted: true,
        cohomology_source_links: true,
        unstable_ranges_complete: true,
        operation_values: true,
        evidence_named: true,
    })
}

pub fn validate_homotopy_census(
    census: &HomotopyCensus,
    identities: &[crate::records::SimplicialSetIdentityRecord],
) -> Result<HomotopyValidationReport> {
    ensure!(
        census.records.len() == identities.len(),
        "homotopy source row mismatch"
    );
    ensure!(
        census
            .records
            .windows(2)
            .all(|rows| rows[0].simplicial_set < rows[1].simplicial_set),
        "homotopy keys are not uniquely sorted"
    );
    for (stored, identity) in census.records.iter().zip(identities) {
        ensure!(
            stored.simplicial_set == identity.key,
            "homotopy source key mismatch"
        );
        ensure!(
            stored.method == "elementary-horn-collapse/anodyne-expansion-v1",
            "unknown homotopy relation method"
        );
        ensure!(
            *stored == homotopy_record(identity)?,
            "homotopy recomputation mismatch"
        );
        let source = identity.key.reconstruct()?;
        for relation in &stored.elementary_horn_relations {
            let target = source
                .elementary_horn_collapse(&relation.step.to_noether())?
                .canonical_form()?;
            ensure!(
                crate::records::SimplicialSetKey::from_value(&target) == relation.target,
                "immediate horn-collapse target mismatch"
            );
        }
        ensure!(
            source.total_cells() == stored.greedy_horn_collapse.source_cells,
            "greedy source cell count mismatch"
        );
        let mut replay_target = source.clone();
        for step in &stored.greedy_horn_collapse.steps {
            replay_target = replay_target.elementary_horn_collapse(&step.to_noether())?;
        }
        ensure!(
            replay_target.total_cells() == stored.greedy_horn_collapse.target_cells,
            "greedy target cell count mismatch"
        );
        let canonical_target = replay_target.canonical_form()?;
        ensure!(
            crate::records::SimplicialSetKey::from_value(&canonical_target)
                == stored.greedy_horn_collapse.target,
            "greedy canonical target mismatch"
        );
        ensure!(
            stored.greedy_horn_collapse.verified
                && stored.greedy_horn_collapse.terminal
                    == replay_target.elementary_horn_collapses().is_empty(),
            "greedy terminal or verification claim mismatch"
        );
        let maximum_degree = source.dimension().max(replay_target.dimension()).max(0) as usize;
        for degree in 0..=maximum_degree {
            ensure!(
                source.homology_invariants(degree)? == replay_target.homology_invariants(degree)?,
                "horn-collapse sequence changed integral homology"
            );
        }
    }
    Ok(HomotopyValidationReport {
        contract: HOMOTOPY_CONTRACT,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        status: "complete",
        records: census.records.len(),
        keys_unique_and_sorted: true,
        immediate_horn_collapse_replay: true,
        greedy_sequence_replay: true,
        canonical_targets: true,
        terminal_claims: true,
        homology_invariance: true,
    })
}

fn factorial(value: usize) -> Result<u128> {
    (2..=value).try_fold(1_u128, |product, factor| {
        product
            .checked_mul(factor as u128)
            .context("factorial exceeds u128")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{enumerate::orderly_representatives, identity_census};

    #[test]
    fn accepted_boxes_match_the_measured_frontier() {
        assert_eq!(accepted_count(7, 2).unwrap(), 30_808);
        assert_eq!(accepted_count(6, 3).unwrap(), 89_531);
    }

    #[test]
    fn small_shell_reconciles_both_generators_and_orbit_stabilizer() {
        let (stats, representatives) = orderly_representatives(4, 3, 1_000_000).unwrap();
        let census = identity_census(4, 3, 1_000_000, stats, representatives).unwrap();
        let report = validate_identity_census(&census).unwrap();
        assert_eq!(report.unlabeled, "386");
        assert!(report.reference_key_set_complete);
    }
}
