use anyhow::{Context, Result, ensure};
use noether::{
    numthy::FiniteField,
    topology::{SimplicialCollapseSequence, StrongCoreReduction},
};
use serde::{Deserialize, Serialize};

use crate::{
    CONTRACT, CohomologyCensus, IdentityCensus, TopologyCensus,
    enrich::{cohomology_record, component_sizes, degree_count},
    enumerate::{MAX_REFERENCE_VERTICES, reference_representatives},
    records::{
        SimplicialComplexIdentityRecord, SimplicialComplexKey, SimplicialComplexTopologyRecord,
    },
};

const ACCEPTED_COUNTS: &str =
    include_str!("../../../collections/finite-simplicial-complexes/validation/counts.json");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CountSources {
    pub unlabeled: String,
    pub labeled: String,
}

#[derive(Debug, Deserialize)]
struct CountTable {
    sources: CountSources,
    orders: Vec<ExpectedCounts>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCounts {
    vertices: usize,
    unlabeled: String,
    labeled: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedCounts {
    pub unlabeled: u128,
    pub labeled: u128,
}

pub fn accepted_counts(vertices: usize) -> Result<AcceptedCounts> {
    let table: CountTable = serde_json::from_str(ACCEPTED_COUNTS)?;
    let expected = table
        .orders
        .iter()
        .find(|entry| entry.vertices == vertices)
        .with_context(|| format!("no accepted counts for {vertices} vertices"))?;
    Ok(AcceptedCounts {
        unlabeled: expected.unlabeled.parse()?,
        labeled: expected.labeled.parse()?,
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CountCheck {
    pub expected: String,
    pub observed: String,
    pub passed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationChecks {
    pub unlabeled: CountCheck,
    pub labeled_from_automorphisms: CountCheck,
    pub keys_unique_and_sorted: bool,
    pub record_shapes: bool,
    pub canonical_reconstruction: bool,
    pub automorphism_recomputation: bool,
    pub reference_key_set_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidationReport {
    pub contract: &'static str,
    pub vertices: usize,
    pub status: &'static str,
    pub generator: &'static str,
    pub accepted_count_sources: CountSources,
    pub checks: ValidationChecks,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TopologyValidationChecks {
    pub keys_unique_and_sorted: bool,
    pub f_vector_and_euler: bool,
    pub components: bool,
    pub strong_core_replay: bool,
    pub core_homology_invariance: bool,
    pub elementary_collapse_replay: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TopologyValidationReport {
    pub contract: &'static str,
    pub vertices: usize,
    pub status: &'static str,
    pub records: usize,
    pub checks: TopologyValidationChecks,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CohomologyValidationChecks {
    pub keys_unique_and_sorted: bool,
    pub topology_keys_and_cores: bool,
    pub canonical_cocycles: bool,
    pub multiplication_table: bool,
    pub presentation_or_complete_table: bool,
    pub operation_values: bool,
    pub extent_certificates: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CohomologyValidationReport {
    pub contract: &'static str,
    pub vertices: usize,
    pub coefficient: String,
    pub status: &'static str,
    pub records: usize,
    pub checks: CohomologyValidationChecks,
}

pub fn validate_identity_census(census: &IdentityCensus) -> Result<ValidationReport> {
    let table: CountTable = serde_json::from_str(ACCEPTED_COUNTS)?;
    let expected = accepted_counts(census.vertices)?;
    ensure!(
        census
            .records
            .windows(2)
            .all(|rows| rows[0].key < rows[1].key),
        "identity keys are not uniquely sorted"
    );

    let factorial = factorial(census.vertices)?;
    let mut labeled = 0_u128;
    for record in &census.records {
        record.check_shape()?;
        ensure!(record.vertices == census.vertices, "mixed vertex shell");
        let complex = record.key.reconstruct();
        ensure!(
            SimplicialComplexKey::from_canonical(&complex) == record.key,
            "canonical key did not reconstruct"
        );
        let automorphisms = complex.automorphism_group()?;
        let observed_order: u128 = automorphisms.order().to_decimal().parse()?;
        let stored_order: u128 = record.automorphism_group.order.parse()?;
        ensure!(observed_order == stored_order, "automorphism order changed");
        ensure!(
            factorial % observed_order == 0,
            "orbit-stabilizer division failed"
        );
        labeled = labeled
            .checked_add(factorial / observed_order)
            .context("labeled count overflow")?;
    }

    let reference_key_set_complete = if census.vertices <= MAX_REFERENCE_VERTICES {
        let (_, reference) = reference_representatives(census.vertices, 100_000)?;
        let observed = census
            .records
            .iter()
            .map(|record| &record.key)
            .collect::<Vec<_>>();
        reference.keys().eq(observed)
    } else {
        true
    };
    ensure!(reference_key_set_complete, "reference key-set mismatch");
    ensure!(
        census.records.len() as u128 == expected.unlabeled,
        "unlabeled count mismatch"
    );
    ensure!(labeled == expected.labeled, "labeled count mismatch");

    Ok(ValidationReport {
        contract: CONTRACT,
        vertices: census.vertices,
        status: "complete",
        generator: census.generator.id(),
        accepted_count_sources: table.sources,
        checks: ValidationChecks {
            unlabeled: CountCheck {
                expected: expected.unlabeled.to_string(),
                observed: census.records.len().to_string(),
                passed: true,
            },
            labeled_from_automorphisms: CountCheck {
                expected: expected.labeled.to_string(),
                observed: labeled.to_string(),
                passed: true,
            },
            keys_unique_and_sorted: true,
            record_shapes: true,
            canonical_reconstruction: true,
            automorphism_recomputation: true,
            reference_key_set_complete,
        },
    })
}

pub fn validate_topology_census(census: &TopologyCensus) -> Result<TopologyValidationReport> {
    ensure!(
        census
            .records
            .windows(2)
            .all(|rows| rows[0].complex < rows[1].complex),
        "topology keys are not uniquely sorted"
    );
    for record in &census.records {
        ensure!(
            record.record_version == crate::RECORD_VERSION,
            "wrong version"
        );
        let complex = record.complex.reconstruct();
        ensure!(complex.count(0) == census.vertices, "mixed topology shell");
        let expected_f = (0..degree_count(&complex))
            .map(|dimension| complex.count(dimension))
            .collect::<Vec<_>>();
        ensure!(record.f_vector == expected_f, "f-vector mismatch");
        let alternating = record
            .f_vector
            .iter()
            .enumerate()
            .map(|(degree, count)| {
                if degree % 2 == 0 {
                    *count as i64
                } else {
                    -(*count as i64)
                }
            })
            .sum::<i64>();
        ensure!(
            record.euler_characteristic == complex.euler_characteristic()
                && alternating == record.euler_characteristic,
            "Euler-characteristic mismatch"
        );
        ensure!(
            record.component_sizes == component_sizes(&complex),
            "component sizes mismatch"
        );

        let strong = StrongCoreReduction::from_json(&record.strong_core.reduction_json)?;
        strong.verify()?;
        ensure!(
            record.strong_core.verified,
            "strong reduction is unverified"
        );
        ensure!(
            strong.source() == &complex,
            "strong reduction source mismatch"
        );
        ensure!(
            record.strong_core.source_vertices == strong.source().count(0)
                && record.strong_core.target_vertices == strong.core().count(0),
            "strong reduction vertex counts mismatch"
        );
        let strong_steps = strong
            .steps()
            .iter()
            .map(|step| (step.vertex(), step.dominator()))
            .collect::<Vec<_>>();
        let stored_strong_steps = record
            .strong_core
            .steps
            .iter()
            .map(|step| (step.vertex, step.dominator))
            .collect::<Vec<_>>();
        ensure!(
            stored_strong_steps == strong_steps,
            "strong reduction steps mismatch"
        );
        let canonical_core = strong.core().canonical_form()?;
        ensure!(
            SimplicialComplexKey::from_canonical(&canonical_core) == record.strong_core.target,
            "strong core target mismatch"
        );
        ensure!(
            record.integral_homology.len() == record.f_vector.len(),
            "stored homology range mismatch"
        );
        for stored in &record.integral_homology {
            let source = complex.homology_invariants(stored.degree)?;
            let core = strong.core().homology_invariants(stored.degree)?;
            ensure!(source == core, "strong core changed homology");
            ensure!(stored.free_rank == core.free_rank(), "free-rank mismatch");
            ensure!(
                stored.torsion_coefficients
                    == core
                        .torsion_coefficients()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>(),
                "torsion mismatch"
            );
        }

        let collapse =
            SimplicialCollapseSequence::from_json(&record.greedy_collapse.sequence_json)?;
        collapse.verify()?;
        ensure!(
            record.greedy_collapse.verified,
            "collapse sequence is unverified"
        );
        ensure!(collapse.source() == &complex, "collapse source mismatch");
        let collapse_steps = collapse
            .steps()
            .iter()
            .map(|step| (step.free_face(), step.coface()))
            .collect::<Vec<_>>();
        let stored_collapse_steps = record
            .greedy_collapse
            .steps
            .iter()
            .map(|step| (step.free_face.as_slice(), step.coface.as_slice()))
            .collect::<Vec<_>>();
        ensure!(
            stored_collapse_steps == collapse_steps,
            "collapse steps mismatch"
        );
        ensure!(
            collapse.is_terminal() == record.greedy_collapse.terminal,
            "collapse terminal flag mismatch"
        );
        let target = collapse.target().canonical_form()?;
        ensure!(
            SimplicialComplexKey::from_canonical(&target) == record.greedy_collapse.target,
            "collapse target mismatch"
        );
    }
    Ok(TopologyValidationReport {
        contract: CONTRACT,
        vertices: census.vertices,
        status: "complete",
        records: census.records.len(),
        checks: TopologyValidationChecks {
            keys_unique_and_sorted: true,
            f_vector_and_euler: true,
            components: true,
            strong_core_replay: true,
            core_homology_invariance: true,
            elementary_collapse_replay: true,
        },
    })
}

pub fn validate_cohomology_census(
    census: &CohomologyCensus,
    identities: &[SimplicialComplexIdentityRecord],
    topology: &[SimplicialComplexTopologyRecord],
) -> Result<CohomologyValidationReport> {
    ensure!(
        census
            .records
            .windows(2)
            .all(|rows| rows[0].complex < rows[1].complex),
        "cohomology keys are not uniquely sorted"
    );
    ensure!(
        census.records.len() == identities.len() && identities.len() == topology.len(),
        "cohomology source row mismatch"
    );
    let field = FiniteField::prime(census.prime)?;
    for ((stored, identity), topology) in census.records.iter().zip(identities).zip(topology) {
        ensure!(
            stored.complex == identity.key && stored.complex == topology.complex,
            "cohomology source key mismatch"
        );
        ensure!(
            stored.computed_on_strong_core == topology.strong_core.target,
            "cohomology strong-core target mismatch"
        );
        let expected = cohomology_record(identity, topology, &field)?;
        ensure!(*stored == expected, "cohomology recomputation mismatch");
    }
    Ok(CohomologyValidationReport {
        contract: CONTRACT,
        vertices: census.vertices,
        coefficient: format!("F{}", census.prime),
        status: "complete",
        records: census.records.len(),
        checks: CohomologyValidationChecks {
            keys_unique_and_sorted: true,
            topology_keys_and_cores: true,
            canonical_cocycles: true,
            multiplication_table: true,
            presentation_or_complete_table: true,
            operation_values: true,
            extent_certificates: true,
        },
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
    use crate::enumerate::orderly_representatives;
    use crate::{Generator, identity_census};

    #[test]
    fn accepted_count_table_covers_the_first_edition() {
        assert_eq!(accepted_counts(6).unwrap().unlabeled, 16_143);
        assert_eq!(accepted_counts(6).unwrap().labeled, 7_785_062);
    }

    #[test]
    fn fourth_shell_passes_count_and_orbit_stabilizer_gates() {
        let (stats, representatives) = orderly_representatives(4, 100_000).unwrap();
        let census = identity_census(
            Generator::CanonicalVertexLink,
            4,
            100_000,
            stats,
            representatives,
        )
        .unwrap();
        let report = validate_identity_census(&census).unwrap();
        assert_eq!(report.status, "complete");
        assert_eq!(report.checks.unlabeled.observed, "20");
        assert_eq!(report.checks.labeled_from_automorphisms.observed, "114");
    }
}
