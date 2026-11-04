use anyhow::{Result, ensure};
use noether::{SemanticType, Z, numthy::FiniteField};
use sha2::{Digest, Sha256};

use crate::{
    CohomologyCensus, HomotopyCensus, RECORD_VERSION, SteenrodOperationsCensus, TopologyCensus,
    records::{
        ChainCancellationRecord, ChainModuleRecord, CohomologyBasisRecord,
        CohomologyOperationRecord, CohomologyProductRecord, CohomologyProductTermRecord,
        ElementaryHornCollapseRecord, ElementaryHornRelationRecord, FiniteFieldContextRecord,
        FiniteMultiplicationTableCertificate, GreedyHornCollapseRecord, IntegralHomologyRecord,
        NormalizedBoundaryRecord, NormalizedChainReductionRecord, ReducedDifferentialRecord,
        RingExtentRecord, SimplicialSetCohomologyRecord, SimplicialSetHomotopyRecord,
        SimplicialSetIdentityRecord, SimplicialSetKey, SimplicialSetSteenrodOperationsRecord,
        SimplicialSetTopologyRecord,
    },
};

pub fn enrich_steenrod_operations(
    total_nondegenerate: usize,
    maximum_dimension: usize,
    prime: u64,
    identities: &[SimplicialSetIdentityRecord],
    cohomology: &[SimplicialSetCohomologyRecord],
) -> Result<SteenrodOperationsCensus> {
    ensure!(
        prime % 2 == 1,
        "odd-primary operations require an odd prime"
    );
    ensure!(
        identities.len() == cohomology.len(),
        "Steenrod source row mismatch"
    );
    let field = FiniteField::prime(prime)?;
    let records = identities
        .iter()
        .zip(cohomology)
        .map(|(identity, algebra)| {
            ensure!(
                identity.total_nondegenerate == total_nondegenerate,
                "mixed identity shell"
            );
            ensure!(
                identity.dimension <= maximum_dimension as i64,
                "dimension cap exceeded"
            );
            steenrod_operations_record(identity, algebra, &field)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(SteenrodOperationsCensus {
        total_nondegenerate,
        maximum_dimension,
        prime,
        records,
    })
}

pub fn steenrod_operations_record(
    identity: &SimplicialSetIdentityRecord,
    cohomology: &SimplicialSetCohomologyRecord,
    field: &FiniteField,
) -> Result<SimplicialSetSteenrodOperationsRecord> {
    ensure!(
        field.characteristic().to_string() != "2" && field.degree() == 1,
        "odd-primary operations require an odd prime field"
    );
    identity.check_shape()?;
    ensure!(
        cohomology.simplicial_set == identity.key,
        "cohomology source key mismatch"
    );
    let coefficient_context = FiniteFieldContextRecord {
        characteristic: field.characteristic().to_string(),
        extension_degree: field.degree(),
        defining_modulus: field.defining_modulus().map(<[u64]>::to_vec),
    };
    ensure!(
        cohomology.coefficient_context == coefficient_context,
        "cohomology coefficient context mismatch"
    );

    let value = identity.key.reconstruct()?;
    let algebra = value.cohomology_ring(field.clone())?;
    algebra.verify()?;
    let algebra_sha256 = hex::encode(Sha256::digest(algebra.canonical_bytes()?));
    let RingExtentRecord::CompletePresentation { certificate } = &cohomology.extent;
    ensure!(
        certificate.algebra_sha256 == algebra_sha256,
        "cohomology algebra hash mismatch"
    );
    ensure!(
        algebra.basis().len() == cohomology.basis.len()
            && algebra
                .basis()
                .iter()
                .zip(&cohomology.basis)
                .all(|(actual, stored)| {
                    actual.id() == stored.id && actual.degree() == stored.degree
                }),
        "cohomology basis mismatch"
    );

    let terms = |values: &[noether::topology::SimplicialCohomologyProductTerm]| {
        values
            .iter()
            .map(|term| CohomologyProductTermRecord {
                basis_id: cohomology.basis[term.basis()].id.clone(),
                coefficient: term
                    .coefficient()
                    .coefficients()
                    .into_iter()
                    .map(|coefficient| coefficient.to_string())
                    .collect(),
            })
            .collect::<Vec<_>>()
    };
    let mut operations = Vec::new();
    for (source, element) in algebra.basis().iter().enumerate() {
        let degree = usize::try_from(element.degree())?;
        for power in 0..=degree / 2 {
            operations.push(CohomologyOperationRecord {
                operation: format!("P^{power}"),
                source_id: element.id().to_owned(),
                target_terms: terms(&algebra.steenrod_reduced_power(power, source)?),
                evidence: if power == 0 {
                    "steenrod-reduced-power-zero-identity/v1"
                } else if power.checked_mul(2) == Some(degree) {
                    "unstable-top-reduced-power-via-p-fold-cup-product/v1"
                } else {
                    "kaufmann-medina-mardones-may-steenrod/v1"
                }
                .to_owned(),
            });
        }
        if degree > 0 {
            for power in 0..=(degree - 1) / 2 {
                operations.push(CohomologyOperationRecord {
                    operation: format!("beta P^{power}"),
                    source_id: element.id().to_owned(),
                    target_terms: terms(&algebra.bockstein_reduced_power(power, source)?),
                    evidence: "kaufmann-medina-mardones-may-steenrod-bockstein/v1".to_owned(),
                });
            }
        }
    }

    Ok(SimplicialSetSteenrodOperationsRecord {
        record_version: RECORD_VERSION,
        simplicial_set: identity.key.clone(),
        coefficient_context,
        cohomology_record_sha256: hex::encode(Sha256::digest(serde_json::to_vec(cohomology)?)),
        operations,
    })
}

pub fn enrich_homotopy(
    total_nondegenerate: usize,
    maximum_dimension: usize,
    identities: &[SimplicialSetIdentityRecord],
) -> Result<HomotopyCensus> {
    let records = identities
        .iter()
        .map(|identity| {
            ensure!(
                identity.total_nondegenerate == total_nondegenerate,
                "mixed identity shell"
            );
            ensure!(
                identity.dimension <= maximum_dimension as i64,
                "dimension cap exceeded"
            );
            homotopy_record(identity)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(HomotopyCensus {
        total_nondegenerate,
        maximum_dimension,
        records,
    })
}

pub fn homotopy_record(
    identity: &SimplicialSetIdentityRecord,
) -> Result<SimplicialSetHomotopyRecord> {
    identity.check_shape()?;
    let value = identity.key.reconstruct()?;
    let elementary_horn_relations = value
        .elementary_horn_collapses()
        .into_iter()
        .map(|step| {
            let target = value.elementary_horn_collapse(&step)?.canonical_form()?;
            Ok(ElementaryHornRelationRecord {
                step: ElementaryHornCollapseRecord::from_noether(&step),
                target: SimplicialSetKey::from_value(&target),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let greedy = value.greedy_horn_collapse_reduction()?;
    greedy.verify()?;
    let canonical_target = greedy.target().canonical_form()?;
    Ok(SimplicialSetHomotopyRecord {
        record_version: RECORD_VERSION,
        simplicial_set: identity.key.clone(),
        method: "elementary-horn-collapse/anodyne-expansion-v1".to_owned(),
        elementary_horn_relations,
        greedy_horn_collapse: GreedyHornCollapseRecord {
            target: SimplicialSetKey::from_value(&canonical_target),
            source_cells: greedy.source().total_cells(),
            target_cells: greedy.target().total_cells(),
            steps: greedy
                .steps()
                .iter()
                .map(ElementaryHornCollapseRecord::from_noether)
                .collect(),
            terminal: greedy.is_terminal(),
            verified: true,
        },
    })
}

pub fn enrich_cohomology(
    total_nondegenerate: usize,
    maximum_dimension: usize,
    prime: u64,
    identities: &[SimplicialSetIdentityRecord],
) -> Result<CohomologyCensus> {
    let field = FiniteField::prime(prime)?;
    let records = identities
        .iter()
        .map(|identity| {
            ensure!(
                identity.total_nondegenerate == total_nondegenerate,
                "mixed identity shell"
            );
            ensure!(
                identity.dimension <= maximum_dimension as i64,
                "dimension cap exceeded"
            );
            cohomology_record(identity, &field)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(CohomologyCensus {
        total_nondegenerate,
        maximum_dimension,
        prime,
        records,
    })
}

pub fn cohomology_record(
    identity: &SimplicialSetIdentityRecord,
    field: &FiniteField,
) -> Result<SimplicialSetCohomologyRecord> {
    identity.check_shape()?;
    let value = identity.key.reconstruct()?;
    let algebra = value.cohomology_ring(field.clone())?;
    let algebra_sha256 = hex::encode(Sha256::digest(algebra.canonical_bytes()?));
    let basis = algebra
        .basis()
        .iter()
        .map(|element| CohomologyBasisRecord {
            id: element.id().to_owned(),
            degree: element.degree(),
            cocycle_coefficients: element
                .cocycle()
                .values()
                .map(|value| {
                    value
                        .coefficients()
                        .into_iter()
                        .map(|coefficient| coefficient.to_string())
                        .collect()
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let product_terms = |terms: &[noether::topology::SimplicialCohomologyProductTerm]| {
        terms
            .iter()
            .map(|term| CohomologyProductTermRecord {
                basis_id: basis[term.basis()].id.clone(),
                coefficient: term
                    .coefficient()
                    .coefficients()
                    .into_iter()
                    .map(|coefficient| coefficient.to_string())
                    .collect(),
            })
            .collect::<Vec<_>>()
    };
    let products = algebra
        .products()
        .iter()
        .map(|product| CohomologyProductRecord {
            left_id: basis[product.left()].id.clone(),
            right_id: basis[product.right()].id.clone(),
            terms: product_terms(product.terms()),
        })
        .collect::<Vec<_>>();
    let operations = if field.characteristic().to_string() == "2" {
        algebra
            .basis()
            .iter()
            .enumerate()
            .flat_map(|(index, element)| {
                (0..=element.degree()).map(move |square| (index, element, square))
            })
            .map(|(index, element, square)| {
                let square = usize::try_from(square)?;
                Ok(CohomologyOperationRecord {
                    operation: format!("Sq^{square}"),
                    source_id: element.id().to_owned(),
                    target_terms: product_terms(&algebra.steenrod_square(square, index)?),
                    evidence: if square == 0 {
                        "steenrod-square-zero-identity/v1"
                    } else if u32::try_from(square)? == element.degree() {
                        "unstable-top-square-via-alexander-whitney-cup-product/v1"
                    } else {
                        "medina-mardones-subset-cup-i/v1"
                    }
                    .to_owned(),
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let connected_presentation_json = algebra
        .presentation()
        .map(|presentation| presentation.to_json())
        .transpose()?;
    let presentation_sha256 = algebra
        .presentation()
        .map(|presentation| {
            presentation
                .canonical_bytes()
                .map(|bytes| hex::encode(Sha256::digest(bytes)))
        })
        .transpose()?;
    let maximum_nonzero_degree = basis.iter().map(|basis| basis.degree).max().unwrap_or(0);
    let additive_dimensions = (0..=maximum_nonzero_degree)
        .map(|degree| basis.iter().filter(|basis| basis.degree == degree).count())
        .collect();

    Ok(SimplicialSetCohomologyRecord {
        record_version: RECORD_VERSION,
        simplicial_set: identity.key.clone(),
        coefficient_context: FiniteFieldContextRecord {
            characteristic: field.characteristic().to_string(),
            extension_degree: field.degree(),
            defining_modulus: field.defining_modulus().map(<[u64]>::to_vec),
        },
        basis,
        products,
        operations,
        connected_presentation_json,
        extent: RingExtentRecord::CompletePresentation {
            certificate: FiniteMultiplicationTableCertificate {
                method: "finite-normalized-cochain-multiplication-table/v1".to_owned(),
                maximum_nonzero_degree,
                additive_dimensions,
                algebra_sha256,
                presentation_sha256,
                verified: true,
            },
        },
    })
}

pub fn enrich_topology(
    total_nondegenerate: usize,
    maximum_dimension: usize,
    identities: &[SimplicialSetIdentityRecord],
) -> Result<TopologyCensus> {
    let records = identities
        .iter()
        .map(|identity| {
            ensure!(
                identity.total_nondegenerate == total_nondegenerate,
                "mixed identity shell"
            );
            ensure!(
                identity.dimension <= maximum_dimension as i64,
                "dimension cap exceeded"
            );
            topology_record(identity)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(TopologyCensus {
        total_nondegenerate,
        maximum_dimension,
        records,
    })
}

pub fn topology_record(
    identity: &SimplicialSetIdentityRecord,
) -> Result<SimplicialSetTopologyRecord> {
    identity.check_shape()?;
    let value = identity.key.reconstruct()?;
    let degree_count = usize::try_from(value.dimension().saturating_add(1)).unwrap_or(0);
    let normalized_boundaries = (0..degree_count)
        .map(|degree| {
            let boundary = value.boundary_matrix(Z, degree)?;
            let (rows, columns) = boundary.shape();
            Ok(NormalizedBoundaryRecord {
                degree,
                rows,
                columns,
                canonical_bytes_sha256: hex::encode(Sha256::digest(boundary.canonical_bytes()?)),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let integral_homology = (0..degree_count)
        .map(|degree| {
            let group = value.homology_invariants(degree)?;
            Ok(IntegralHomologyRecord {
                degree,
                free_rank: group.free_rank(),
                torsion_coefficients: group
                    .torsion_coefficients()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let reduction = value.normalized_chain_reduction(Z)?;
    reduction.verify()?;
    let normalized_chain_reduction = reduction_record(&reduction, degree_count)?;
    Ok(SimplicialSetTopologyRecord {
        record_version: RECORD_VERSION,
        simplicial_set: identity.key.clone(),
        euler_characteristic: value.euler_characteristic(),
        normalized_boundaries,
        normalized_chain_reduction,
        integral_homology,
    })
}

fn reduction_record(
    reduction: &noether::homalg::ChainReduction<noether::IntegerRing>,
    degree_count: usize,
) -> Result<NormalizedChainReductionRecord> {
    let reduced_modules = (0..degree_count)
        .map(|degree| ChainModuleRecord {
            degree: degree as i32,
            rank: reduction.reduced().module_dimension(degree as i32),
        })
        .collect::<Vec<_>>();
    let reduced_differentials = (0..degree_count)
        .filter_map(|degree| {
            reduction
                .reduced()
                .differential(degree as i32)
                .map(|differential| (degree, differential))
        })
        .map(|(degree, differential)| {
            let (rows, columns) = differential.shape();
            Ok(ReducedDifferentialRecord {
                degree: degree as i32,
                rows,
                columns,
                canonical_bytes_sha256: hex::encode(Sha256::digest(
                    differential.canonical_bytes()?,
                )),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(NormalizedChainReductionRecord {
        method: "universal-unit-algebraic-morse".to_owned(),
        coefficient_context: "Z".to_owned(),
        source_cells: reduction
            .source()
            .degrees()
            .iter()
            .map(|degree| reduction.source().module_dimension(*degree))
            .sum(),
        reduced_cells: reduced_modules.iter().map(|module| module.rank).sum(),
        cancellations: reduction
            .cancellations()
            .iter()
            .map(ChainCancellationRecord::from_noether)
            .collect(),
        reduced_modules,
        reduced_differentials,
        witness_verified: true,
    })
}

pub fn verify_reduction_record(
    value: &noether::topology::SimplicialSet,
    stored: &NormalizedChainReductionRecord,
) -> Result<()> {
    ensure!(
        stored.method == "universal-unit-algebraic-morse",
        "unknown normalized-chain reduction method"
    );
    ensure!(stored.coefficient_context == "Z", "wrong reduction ring");
    ensure!(stored.witness_verified, "reduction witness is not verified");
    let cancellations = stored
        .cancellations
        .iter()
        .map(ChainCancellationRecord::to_noether)
        .collect::<Result<Vec<_>>>()?;
    let source = value.chain_complex(Z)?;
    let replay = source.reduction_from_cancellations(&cancellations)?;
    replay.verify()?;
    let degree_count = usize::try_from(value.dimension().saturating_add(1)).unwrap_or(0);
    ensure!(
        reduction_record(&replay, degree_count)? == *stored,
        "replayed normalized-chain reduction differs from the stored target"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use noether::topology::SimplicialSet;

    use super::*;

    #[test]
    fn minimal_two_sphere_uses_normalized_cells() {
        let sphere = SimplicialSet::sphere(2).canonical_form().unwrap();
        let identity = SimplicialSetIdentityRecord::from_canonical(&sphere).unwrap();
        let record = topology_record(&identity).unwrap();
        assert_eq!(record.euler_characteristic, 2);
        assert_eq!(record.integral_homology[0].free_rank, 1);
        assert_eq!(record.integral_homology[1].free_rank, 0);
        assert_eq!(record.integral_homology[2].free_rank, 1);
        assert_eq!(record.normalized_boundaries[2].columns, 1);
        assert_eq!(record.normalized_chain_reduction.source_cells, 2);
        assert_eq!(record.normalized_chain_reduction.reduced_cells, 2);
        verify_reduction_record(&sphere, &record.normalized_chain_reduction).unwrap();
    }

    #[test]
    fn interval_reduction_records_and_replays_its_pivot() {
        let interval = SimplicialSet::new(vec![
            vec![vec![], vec![]],
            vec![vec![
                noether::topology::SimplicialFace::Cell(1),
                noether::topology::SimplicialFace::Cell(0),
            ]],
        ])
        .unwrap()
        .canonical_form()
        .unwrap();
        let identity = SimplicialSetIdentityRecord::from_canonical(&interval).unwrap();
        let record = topology_record(&identity).unwrap();
        assert_eq!(record.normalized_chain_reduction.source_cells, 3);
        assert_eq!(record.normalized_chain_reduction.reduced_cells, 1);
        assert_eq!(record.normalized_chain_reduction.cancellations.len(), 1);
        verify_reduction_record(&interval, &record.normalized_chain_reduction).unwrap();

        let mut false_witness = record.normalized_chain_reduction;
        false_witness.cancellations[0].row = 99;
        assert!(verify_reduction_record(&interval, &false_witness).is_err());
    }

    #[test]
    fn minimal_sphere_cohomology_records_complete_squares() {
        let sphere = SimplicialSet::sphere(2).canonical_form().unwrap();
        let identity = SimplicialSetIdentityRecord::from_canonical(&sphere).unwrap();
        let field = FiniteField::prime(2_u8).unwrap();
        let record = cohomology_record(&identity, &field).unwrap();
        assert_eq!(
            record
                .basis
                .iter()
                .map(|element| element.degree)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
        assert_eq!(record.operations.len(), 4);
        assert!(record.operations.iter().any(|operation| {
            operation.operation == "Sq^0"
                && operation.source_id == "h2_0"
                && operation.target_terms[0].basis_id == "h2_0"
        }));
        assert!(matches!(
            record.extent,
            RingExtentRecord::CompletePresentation { .. }
        ));
    }

    #[test]
    fn moore_space_steenrod_sidecar_records_nonzero_mod_three_bockstein() {
        let moore = SimplicialSet::new(vec![
            vec![vec![]],
            vec![
                vec![
                    noether::topology::SimplicialFace::Cell(0),
                    noether::topology::SimplicialFace::Cell(0),
                ],
                vec![
                    noether::topology::SimplicialFace::Cell(0),
                    noether::topology::SimplicialFace::Cell(0),
                ],
            ],
            vec![
                vec![
                    noether::topology::SimplicialFace::Cell(0),
                    noether::topology::SimplicialFace::Cell(1),
                    noether::topology::SimplicialFace::Cell(0),
                ],
                vec![
                    noether::topology::SimplicialFace::Cell(0),
                    noether::topology::SimplicialFace::degenerate(0, 0, vec![0, 0]),
                    noether::topology::SimplicialFace::Cell(1),
                ],
            ],
        ])
        .unwrap()
        .canonical_form()
        .unwrap();
        let identity = SimplicialSetIdentityRecord::from_canonical(&moore).unwrap();
        let field = FiniteField::prime(3_u8).unwrap();
        let cohomology = cohomology_record(&identity, &field).unwrap();
        let record = steenrod_operations_record(&identity, &cohomology, &field).unwrap();
        let degree_one = cohomology
            .basis
            .iter()
            .find(|basis| basis.degree == 1)
            .unwrap();
        let degree_two = cohomology
            .basis
            .iter()
            .find(|basis| basis.degree == 2)
            .unwrap();
        let bockstein = record
            .operations
            .iter()
            .find(|operation| {
                operation.operation == "beta P^0" && operation.source_id == degree_one.id
            })
            .unwrap();
        assert_eq!(bockstein.target_terms.len(), 1);
        assert_eq!(bockstein.target_terms[0].basis_id, degree_two.id);
        assert_ne!(bockstein.target_terms[0].coefficient, vec!["0"]);
        assert_eq!(
            record.operations.len(),
            cohomology
                .basis
                .iter()
                .map(|basis| basis.degree as usize + 1)
                .sum::<usize>()
        );
    }

    #[test]
    fn interval_homotopy_record_links_both_collapses_to_a_point() {
        let interval = SimplicialSet::new(vec![
            vec![vec![], vec![]],
            vec![vec![
                noether::topology::SimplicialFace::Cell(1),
                noether::topology::SimplicialFace::Cell(0),
            ]],
        ])
        .unwrap()
        .canonical_form()
        .unwrap();
        let identity = SimplicialSetIdentityRecord::from_canonical(&interval).unwrap();
        let record = homotopy_record(&identity).unwrap();
        assert_eq!(record.elementary_horn_relations.len(), 2);
        assert!(
            record
                .elementary_horn_relations
                .iter()
                .all(|relation| relation.target.reconstruct().unwrap().total_cells() == 1)
        );
        assert_eq!(record.greedy_horn_collapse.target_cells, 1);
        assert!(record.greedy_horn_collapse.terminal);
    }
}
