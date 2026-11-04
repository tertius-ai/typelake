use anyhow::{Result, ensure};
use noether::{
    SemanticType,
    groups::{FiniteAbelianInvariantError, PermutationGroup},
    homalg::{
        FiniteAbelianGroupCohomology, FiniteAbelianGroupCohomologyError,
        FiniteAbelianGroupCohomologyPresentation, FiniteGroupCohomology,
        FiniteGroupCohomologyError, FiniteGroupResolutionStrategy,
    },
    numthy::FiniteField,
    topology::EilenbergMacLaneGeneratorOrigin,
};
use sha2::{Digest, Sha256};

use crate::{
    ModularCensus, RECORD_VERSION,
    records::{
        AdmissibleCohomologyOperationRecord, CohomologyAlgebraGeneratorRecord,
        CohomologyAlgebraRelationRecord, CohomologyAlgebraRelationTermRecord,
        CohomologyAlgebraTruncationRecord, CohomologyBasisRecord,
        CohomologyGeneratorOperationWitnessRecord, CohomologyProductRecord,
        CohomologyProductTermRecord, CompletionCertificateRecord,
        ElementaryAbelianRestrictionRecord, FiniteAbelianClassifyingSpaceRecord,
        FiniteFieldContextRecord, FiniteGroupModularCohomologyRecord, FiniteGroupSourceRecord,
        GroupSourceFile, ModularComputationRecord, ModularGeneratorOriginRecord,
        ModularGeneratorRecord, ModularOperationRecord, ResolutionEvidenceRecord, RingExtentRecord,
    },
    source::prime_divisors,
};

pub fn modular_record(
    source: &FiniteGroupSourceRecord,
    prime: u64,
    exact_through_degree: usize,
) -> Result<FiniteGroupModularCohomologyRecord> {
    ensure!(
        prime_divisors(source.small_groups_order).contains(&prime),
        "coefficient prime does not divide the group order"
    );
    let group = source.reconstruct()?;
    let field = FiniteField::prime(prime)?;
    let computation = match group.complete_abelian_modular_cohomology(field.clone()) {
        Ok(result) => finite_abelian_computation(&result, exact_through_degree)?,
        Err(FiniteAbelianGroupCohomologyError::AbelianInvariants(
            FiniteAbelianInvariantError::NonAbelian,
        )) => resolution_computation(&group, prime, exact_through_degree)?,
        Err(error) => return Err(error.into()),
    };
    let record = FiniteGroupModularCohomologyRecord {
        record_version: RECORD_VERSION,
        group_label: source.lmfdb_label.clone(),
        group_canonical_sha256: source.canonical_group_sha256()?,
        coefficient_context: FiniteFieldContextRecord {
            characteristic: field.characteristic().to_string(),
            extension_degree: field.degree(),
        },
        computation,
    };
    record.check_shape()?;
    Ok(record)
}

fn finite_abelian_computation(
    result: &FiniteAbelianGroupCohomologyPresentation,
    inspect_through_degree: usize,
) -> Result<ModularComputationRecord> {
    result.verify()?;
    let through_degree = u32::try_from(inspect_through_degree)?;
    Ok(ModularComputationRecord::FiniteAbelianClassifyingSpace {
        certificate: FiniteAbelianClassifyingSpaceRecord {
            method: "finite_abelian_classifying_space".to_owned(),
            invariant_factors: result
                .invariant_factors()
                .iter()
                .map(ToString::to_string)
                .collect(),
            presentation: serde_json::from_str(&result.presentation().to_json()?)?,
            generators: result
                .generators()
                .iter()
                .map(|generator| {
                    let origin = match generator.origin() {
                        EilenbergMacLaneGeneratorOrigin::FundamentalClass { component } => {
                            ModularGeneratorOriginRecord::FundamentalClass {
                                component: *component,
                            }
                        }
                        EilenbergMacLaneGeneratorOrigin::Bockstein { source_generator } => {
                            ModularGeneratorOriginRecord::Bockstein {
                                source_generator: *source_generator,
                            }
                        }
                        EilenbergMacLaneGeneratorOrigin::PeriodicCyclicClass {
                            component,
                            primary_exponent,
                        } => ModularGeneratorOriginRecord::PeriodicCyclicClass {
                            component: *component,
                            primary_exponent: *primary_exponent,
                        },
                        EilenbergMacLaneGeneratorOrigin::AdmissibleSteenrodSquare {
                            component,
                            operation,
                            fundamental_class_lifts_integrally,
                        } => ModularGeneratorOriginRecord::AdmissibleSteenrodSquare {
                            component: *component,
                            operation: AdmissibleCohomologyOperationRecord {
                                prime: operation.prime(),
                                display_name: operation.display_name(),
                                square_sequence: operation.square_sequence().map(ToOwned::to_owned),
                                bocksteins: operation.bocksteins().map(ToOwned::to_owned),
                                reduced_powers: operation.reduced_powers().map(ToOwned::to_owned),
                            },
                            fundamental_class_lifts_integrally: *fundamental_class_lifts_integrally,
                        },
                        other => {
                            return Err(anyhow::anyhow!(
                                "unexpected K(A,1) modular generator origin: {other:?}"
                            ));
                        }
                    };
                    Ok(ModularGeneratorRecord {
                        id: generator.id().to_owned(),
                        degree: generator.degree(),
                        display_name: generator.display_name().to_owned(),
                        origin,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            operations_scope: "primary_operations_on_presentation_generators".to_owned(),
            operations: result
                .operations()
                .iter()
                .map(|operation| {
                    Ok(ModularOperationRecord {
                        operation: AdmissibleCohomologyOperationRecord {
                            prime: operation.operation().prime(),
                            display_name: operation.operation().display_name(),
                            square_sequence: operation
                                .operation()
                                .square_sequence()
                                .map(ToOwned::to_owned),
                            bocksteins: operation.operation().bocksteins().map(ToOwned::to_owned),
                            reduced_powers: operation
                                .operation()
                                .reduced_powers()
                                .map(ToOwned::to_owned),
                        },
                        source_generator: operation.source_generator(),
                        target: serde_json::from_str(&operation.target().to_json()?)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            hilbert_function_through_degree: inspect_through_degree,
            hilbert_function: result.presentation().hilbert_function(through_degree)?,
            noether_result_sha256: hex::encode(Sha256::digest(result.canonical_bytes()?)),
            verified: true,
        },
    })
}

fn resolution_computation(
    group: &PermutationGroup,
    prime: u64,
    exact_through_degree: usize,
) -> Result<ModularComputationRecord> {
    let result = group.modular_cohomology(prime, exact_through_degree)?;
    result.verify()?;
    let mut basis = Vec::new();
    let mut degree_offsets = Vec::with_capacity(exact_through_degree + 2);
    for degree in 0..=exact_through_degree {
        degree_offsets.push(basis.len());
        for (index, cocycle) in result.basis_cocycles(degree)?.iter().enumerate() {
            basis.push(CohomologyBasisRecord {
                id: format!("h{degree}.{}", index + 1),
                degree,
                canonical_model_cocycle: cocycle
                    .values()
                    .map(|coefficient| {
                        coefficient
                            .coefficients()
                            .into_iter()
                            .map(|value| value.to_string())
                            .collect()
                    })
                    .collect(),
            });
        }
    }
    degree_offsets.push(basis.len());

    let mut products = Vec::new();
    for left_degree in 0..=exact_through_degree {
        for right_degree in 0..=exact_through_degree - left_degree {
            let target_degree = left_degree + right_degree;
            let left_dimension = result.dimension(left_degree)?;
            let right_dimension = result.dimension(right_degree)?;
            let target_dimension = result.dimension(target_degree)?;
            for left_index in 0..left_dimension {
                let multiplication =
                    result.basis_left_multiplication(left_degree, left_index, right_degree)?;
                for right_index in 0..right_dimension {
                    let target_terms = (0..target_dimension)
                        .filter_map(|target_index| {
                            let coefficient = multiplication
                                .get(target_index, right_index)
                                .expect("Noether returned the checked multiplication shape");
                            (!coefficient.is_zero()).then(|| CohomologyProductTermRecord {
                                basis_id: basis[degree_offsets[target_degree] + target_index]
                                    .id
                                    .clone(),
                                coefficient: coefficient
                                    .coefficients()
                                    .into_iter()
                                    .map(|value| value.to_string())
                                    .collect(),
                            })
                        })
                        .collect();
                    products.push(CohomologyProductRecord {
                        left_id: basis[degree_offsets[left_degree] + left_index].id.clone(),
                        right_id: basis[degree_offsets[right_degree] + right_index].id.clone(),
                        target_terms,
                    });
                }
            }
        }
    }

    let algebra = result.algebra_truncation()?;
    let algebra_record = CohomologyAlgebraTruncationRecord {
        exact_through_degree,
        generators: algebra
            .generators()
            .iter()
            .map(|generator| CohomologyAlgebraGeneratorRecord {
                name: generator.name().to_owned(),
                display_name: generator.display_name().to_owned(),
                degree: generator.degree(),
                basis_id: basis[degree_offsets[generator.degree()] + generator.basis_index()]
                    .id
                    .clone(),
                operation_witness: generator.operation_witness().map(|witness| {
                    let operation = witness.operation();
                    CohomologyGeneratorOperationWitnessRecord {
                        source_generator_index: witness.source_generator_index(),
                        operation: AdmissibleCohomologyOperationRecord {
                            prime: operation.prime(),
                            display_name: operation.display_name(),
                            square_sequence: operation.square_sequence().map(ToOwned::to_owned),
                            bocksteins: operation.bocksteins().map(ToOwned::to_owned),
                            reduced_powers: operation.reduced_powers().map(ToOwned::to_owned),
                        },
                        operation_coefficient: witness
                            .operation_coefficient()
                            .coefficients()
                            .into_iter()
                            .map(|value| value.to_string())
                            .collect(),
                        decomposable_correction: witness
                            .decomposable_correction()
                            .values()
                            .map(|coefficient| {
                                coefficient
                                    .coefficients()
                                    .into_iter()
                                    .map(|value| value.to_string())
                                    .collect()
                            })
                            .collect(),
                    }
                }),
            })
            .collect(),
        relations: algebra
            .presentation()
            .relations()
            .iter()
            .map(|relation| {
                let degree = relation
                    .homogeneous_degree()?
                    .expect("Noether presentations omit zero relations");
                Ok(CohomologyAlgebraRelationRecord {
                    degree: degree as usize,
                    terms: relation
                        .terms()
                        .iter()
                        .map(
                            |(monomial, coefficient)| CohomologyAlgebraRelationTermRecord {
                                coefficient: coefficient
                                    .coefficients()
                                    .into_iter()
                                    .map(|value| value.to_string())
                                    .collect(),
                                exponents: monomial.exponents().to_vec(),
                            },
                        )
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>>>()?,
        noether_result_sha256: hex::encode(Sha256::digest(algebra.canonical_bytes()?)),
        verified: true,
    };
    let extent = match algebra.certify_symonds_parameter_criterion() {
        Ok(complete) => {
            let certificate = complete
                .symonds_parameter_certificate()
                .expect("the selected Noether strategy retains parameter evidence");
            RingExtentRecord::CompletePresentation {
                certificate: CompletionCertificateRecord::SymondsParameterCriterion {
                    group_order: complete.group_order().to_string(),
                    generator_degree_bound: complete.generator_degree_bound(),
                    relation_degree_bound: complete.relation_degree_bound(),
                    computed_through_degree: exact_through_degree,
                    parameter_generator_indices: certificate.parameter_generator_indices().to_vec(),
                    elementary_abelian_restrictions: certificate
                        .elementary_abelian_restrictions()
                        .iter()
                        .map(|restriction| {
                            Ok(ElementaryAbelianRestrictionRecord {
                                permutation_degree: restriction.subgroup().degree(),
                                generators: restriction
                                    .subgroup()
                                    .generators()
                                    .iter()
                                    .map(|generator| generator.images().to_vec())
                                    .collect(),
                                rank: restriction.rank(),
                                restricted_parameters: restriction
                                    .restricted_parameters()
                                    .iter()
                                    .map(|parameter| {
                                        Ok(serde_json::from_str(&parameter.to_json()?)?)
                                    })
                                    .collect::<Result<Vec<_>>>()?,
                                quotient_generator_power_bounds: restriction
                                    .quotient_generator_power_bounds()
                                    .to_vec(),
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                    quotient_generator_power_bounds: certificate
                        .quotient_generator_power_bounds()
                        .to_vec(),
                    maximum_module_generator_degree: certificate.maximum_module_generator_degree(),
                    parameter_degree_bound: certificate.parameter_degree_bound(),
                    completion_degree_bound: certificate.completion_degree_bound(),
                    noether_result_sha256: hex::encode(Sha256::digest(complete.canonical_bytes()?)),
                    verified: true,
                },
            }
        }
        Err(
            FiniteGroupCohomologyError::CompletionBoundNotReached { .. }
            | FiniteGroupCohomologyError::ParameterEvidenceUnavailable { .. },
        ) => match algebra.certify_symonds_universal_bound() {
            Ok(complete) => RingExtentRecord::CompletePresentation {
                certificate: CompletionCertificateRecord::SymondsUniversalBound {
                    group_order: complete.group_order().to_string(),
                    generator_degree_bound: complete.generator_degree_bound(),
                    relation_degree_bound: complete.relation_degree_bound(),
                    computed_through_degree: exact_through_degree,
                    noether_result_sha256: hex::encode(Sha256::digest(complete.canonical_bytes()?)),
                    verified: true,
                },
            },
            Err(FiniteGroupCohomologyError::CompletionBoundNotReached { .. }) => {
                RingExtentRecord::ExactThroughDegree {
                    degree: exact_through_degree,
                    no_claim_above_degree: true,
                }
            }
            Err(error) => return Err(error.into()),
        },
        Err(error) => return Err(error.into()),
    };

    let (strategy, model) = match result.resolution_strategy() {
        FiniteGroupResolutionStrategy::MinimalPGroup => {
            ("minimal_p_group", "noether-canonical-free-resolution-v1")
        }
        FiniteGroupResolutionStrategy::NormalizedBar => {
            ("normalized_bar", "noether-canonical-free-resolution-v1")
        }
        FiniteGroupResolutionStrategy::SylowStableElements => {
            ("sylow_stable_elements", "noether-sylow-stable-elements-v1")
        }
    };
    Ok(ModularComputationRecord::Resolution {
        resolution: ResolutionEvidenceRecord {
            model: model.to_owned(),
            strategy: strategy.to_owned(),
            computed_through_degree: exact_through_degree,
            noether_result_sha256: hex::encode(Sha256::digest(result.canonical_bytes()?)),
            verified: true,
        },
        basis,
        products,
        algebra: algebra_record,
        extent,
    })
}

pub fn modular_census(
    source: &GroupSourceFile,
    maximum_order: usize,
    exact_through_degree: usize,
) -> Result<ModularCensus> {
    ensure!(
        maximum_order <= source.maximum_order,
        "requested order exceeds the source frontier"
    );
    let mut records = Vec::new();
    for group in source
        .groups
        .iter()
        .filter(|group| group.small_groups_order <= maximum_order)
    {
        for prime in prime_divisors(group.small_groups_order) {
            records.push(modular_record(group, prime, exact_through_degree)?);
        }
    }
    Ok(ModularCensus {
        maximum_order,
        initial_proof_degree: exact_through_degree,
        maximum_proof_degree: exact_through_degree,
        records,
    })
}

pub fn complete_modular_census(
    source: &GroupSourceFile,
    maximum_order: usize,
    initial_proof_degree: usize,
    maximum_proof_degree: usize,
) -> Result<ModularCensus> {
    ensure!(
        maximum_order <= source.maximum_order,
        "requested order exceeds the source frontier"
    );
    ensure!(
        initial_proof_degree <= maximum_proof_degree,
        "initial proof degree exceeds the maximum proof degree"
    );
    let mut records = Vec::new();
    for group in source
        .groups
        .iter()
        .filter(|group| group.small_groups_order <= maximum_order)
    {
        for prime in prime_divisors(group.small_groups_order) {
            let mut proof_degree = initial_proof_degree;
            loop {
                let record = modular_record(group, prime, proof_degree)?;
                if record.is_complete() {
                    records.push(record);
                    break;
                }
                ensure!(
                    proof_degree < maximum_proof_degree,
                    "{}/F_{} did not satisfy a completion criterion through degree {}",
                    group.lmfdb_label,
                    prime,
                    maximum_proof_degree
                );
                proof_degree += 1;
            }
        }
    }
    Ok(ModularCensus {
        maximum_order,
        initial_proof_degree,
        maximum_proof_degree,
        records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::load_source;

    #[test]
    fn klein_four_record_uses_the_complete_abelian_family() {
        let source = load_source().unwrap();
        let group = source
            .groups
            .iter()
            .find(|group| group.lmfdb_label == "4.2")
            .unwrap();
        let record = modular_record(group, 2, 3).unwrap();
        let ModularComputationRecord::FiniteAbelianClassifyingSpace { certificate } =
            record.computation
        else {
            panic!("the Klein four group should use the finite-abelian family")
        };
        assert_eq!(certificate.invariant_factors, ["2", "2"]);
        assert_eq!(
            certificate
                .generators
                .iter()
                .map(|generator| generator.degree)
                .collect::<Vec<_>>(),
            vec![1, 1]
        );
        assert_eq!(certificate.hilbert_function, vec![1, 2, 3, 4]);
        assert!(certificate.verified);
    }

    #[test]
    fn cyclic_four_record_has_a_named_complete_presentation() {
        let source = load_source().unwrap();
        let group = source
            .groups
            .iter()
            .find(|group| group.lmfdb_label == "4.1")
            .unwrap();
        let record = modular_record(group, 2, 4).unwrap();
        let ModularComputationRecord::FiniteAbelianClassifyingSpace { certificate } =
            record.computation
        else {
            panic!("C4 should use the finite-abelian family")
        };
        assert_eq!(certificate.invariant_factors, ["4"]);
        assert_eq!(
            certificate
                .generators
                .iter()
                .map(|generator| (generator.degree, generator.display_name.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "iota_1"), (2, "x_2")]
        );
        assert_eq!(certificate.hilbert_function, vec![1, 1, 1, 1, 1]);
        assert_eq!(
            certificate
                .operations
                .iter()
                .map(|operation| operation.operation.display_name.as_str())
                .collect::<Vec<_>>(),
            ["Sq^1", "Sq^2"]
        );
        assert!(
            certificate
                .operations
                .iter()
                .all(|operation| operation.operation.prime == 2)
        );
    }

    #[test]
    fn complete_abelian_census_keeps_the_initial_proof_depth() {
        let source = load_source().unwrap();
        let census = complete_modular_census(&source, 4, 3, 5).unwrap();
        assert!(census.records.iter().all(|record| record.is_complete()));
        assert!(
            census
                .records
                .iter()
                .all(|record| record.inspected_through_degree() == 3)
        );
    }

    #[test]
    fn complete_census_fails_when_the_proof_ceiling_is_exhausted() {
        let source = load_source().unwrap();
        assert!(complete_modular_census(&source, 6, 1, 1).is_err());
    }

    #[test]
    fn cyclic_two_record_has_a_verified_complete_presentation() {
        let source = load_source().unwrap();
        let group = source
            .groups
            .iter()
            .find(|group| group.lmfdb_label == "2.1")
            .unwrap();
        let record = modular_record(group, 2, 2).unwrap();
        let ModularComputationRecord::FiniteAbelianClassifyingSpace { certificate } =
            record.computation
        else {
            panic!("C2 should use the finite-abelian family")
        };
        assert_eq!(certificate.method, "finite_abelian_classifying_space");
        assert_eq!(certificate.invariant_factors, ["2"]);
        assert_eq!(certificate.hilbert_function, vec![1, 1, 1]);
        assert!(certificate.verified);
    }

    #[test]
    fn symmetric_three_record_uses_the_bar_oracle() {
        let source = load_source().unwrap();
        let group = source
            .groups
            .iter()
            .find(|group| group.lmfdb_label == "6.1")
            .unwrap();
        let record = modular_record(group, 3, 3).unwrap();
        let ModularComputationRecord::Resolution {
            resolution, basis, ..
        } = record.computation
        else {
            panic!("S3 is nonabelian")
        };
        assert_eq!(resolution.strategy, "normalized_bar");
        assert_eq!(
            (0..=3)
                .map(|degree| basis.iter().filter(|value| value.degree == degree).count())
                .collect::<Vec<_>>(),
            vec![1, 0, 0, 1]
        );
    }

    #[test]
    fn symmetric_three_parameter_certificate_is_fully_recorded() {
        let source = load_source().unwrap();
        let group = source
            .groups
            .iter()
            .find(|group| group.lmfdb_label == "6.1")
            .unwrap();
        let record = modular_record(group, 3, 5).unwrap();
        let ModularComputationRecord::Resolution {
            algebra,
            extent:
                RingExtentRecord::CompletePresentation {
                    certificate:
                        CompletionCertificateRecord::SymondsParameterCriterion {
                            parameter_generator_indices,
                            elementary_abelian_restrictions,
                            maximum_module_generator_degree,
                            parameter_degree_bound,
                            completion_degree_bound,
                            ..
                        },
                },
            ..
        } = record.computation
        else {
            panic!("S3 mod 3 should pass the parameter criterion through degree 5")
        };
        assert_eq!(parameter_generator_indices, [1]);
        assert_eq!(elementary_abelian_restrictions.len(), 1);
        assert_eq!(elementary_abelian_restrictions[0].rank, 1);
        assert_eq!(maximum_module_generator_degree, 3);
        assert_eq!(parameter_degree_bound, 4);
        assert_eq!(completion_degree_bound, 4);
        let degree_four = algebra
            .generators
            .iter()
            .find(|generator| generator.degree == 4)
            .unwrap();
        assert!(degree_four.display_name.starts_with("beta("));
        assert_eq!(
            degree_four
                .operation_witness
                .as_ref()
                .unwrap()
                .operation
                .display_name,
            "beta"
        );
    }

    #[test]
    fn alternating_four_record_uses_stable_elements() {
        let source = load_source().unwrap();
        let group = source
            .groups
            .iter()
            .find(|group| group.lmfdb_label == "12.3")
            .unwrap();
        let record = modular_record(group, 2, 4).unwrap();
        let ModularComputationRecord::Resolution {
            resolution, basis, ..
        } = record.computation
        else {
            panic!("A4 is nonabelian")
        };
        assert_eq!(resolution.strategy, "sylow_stable_elements");
        assert_eq!(resolution.model, "noether-sylow-stable-elements-v1");
        assert_eq!(
            (0..=4)
                .map(|degree| basis.iter().filter(|value| value.degree == degree).count())
                .collect::<Vec<_>>(),
            vec![1, 0, 1, 2, 1]
        );
    }
}
