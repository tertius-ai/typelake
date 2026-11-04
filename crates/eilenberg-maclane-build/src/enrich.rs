use anyhow::{Result, bail, ensure};
use noether::{
    SemanticType,
    homalg::FinitelyGeneratedAbelianGroup,
    numthy::FiniteField,
    topology::{
        EilenbergMacLaneError, EilenbergMacLaneGeneratorOrigin, EilenbergMacLaneHomologyStrategy,
        EilenbergMacLaneIntegralCohomology, EilenbergMacLaneIntegralCohomologyPresentation,
        EilenbergMacLaneIntegralCohomologyPresentationStrategy,
        EilenbergMacLaneIntegralGeneratorOrigin, EilenbergMacLaneIntegralHomology,
        EilenbergMacLaneModularCohomology, EilenbergMacLaneModularCohomologyStrategy,
        EilenbergMacLaneModularCohomologyTruncation,
        EilenbergMacLaneModularCohomologyTruncationStrategy, EilenbergMacLaneSpace,
    },
};
use sha2::{Digest, Sha256};

use crate::{
    EilenbergMacLaneCensus, RECORD_VERSION,
    records::{
        AbelianGroupInvariantsRecord, AbelianGroupRecord, AbelianGroupSourceFile,
        EilenbergMacLaneRecord, FiniteAbelianGroupSourceRecord, GroupKindRecord,
        IntegralCohomologyRecord, IntegralCohomologyRingRecord, IntegralHomologyRecord,
        IntegralRingGeneratorOriginRecord, IntegralRingGeneratorRecord, IntegralRingRelationRecord,
        IntegralRingRelationTermRecord, InvariantCoverage, ModularCohomologyCoverageRecord,
        ModularCohomologyRecord, ModularGeneratorOriginRecord, ModularGeneratorRecord,
        ModularOperationRecord, ModularPresentationExtentRecord,
    },
    source::prime_divisors,
};

struct GroupParameter {
    label: String,
    group: FinitelyGeneratedAbelianGroup,
    record: AbelianGroupRecord,
    modular_primes: Vec<u64>,
}

pub fn build_census(
    source: &AbelianGroupSourceFile,
    maximum_finite_order: usize,
    maximum_delooping_degree: usize,
    exact_through_degree: usize,
) -> Result<EilenbergMacLaneCensus> {
    ensure!(
        maximum_finite_order <= source.maximum_finite_order,
        "requested finite-order frontier exceeds source"
    );
    ensure!(
        maximum_delooping_degree > 0,
        "maximum delooping degree must be positive"
    );
    let mut parameters = source
        .finite_groups
        .iter()
        .filter(|group| group.order <= maximum_finite_order)
        .map(finite_parameter)
        .collect::<Result<Vec<_>>>()?;
    if source.include_infinite_cyclic {
        parameters.push(infinite_cyclic_parameter());
    }

    let mut records = Vec::with_capacity(parameters.len() * maximum_delooping_degree);
    for parameter in &parameters {
        for delooping_degree in 1..=maximum_delooping_degree {
            records.push(build_record(
                parameter,
                delooping_degree,
                exact_through_degree,
            )?);
        }
    }
    Ok(EilenbergMacLaneCensus {
        maximum_finite_order,
        maximum_delooping_degree,
        exact_through_degree,
        records,
    })
}

fn finite_parameter(source: &FiniteAbelianGroupSourceRecord) -> Result<GroupParameter> {
    let group = source.group()?;
    Ok(GroupParameter {
        label: source.lmfdb_label.clone(),
        modular_primes: prime_divisors(source.order),
        record: AbelianGroupRecord {
            kind: GroupKindRecord::Finite,
            free_rank: group.free_rank(),
            torsion_coefficients: group
                .torsion_coefficients()
                .iter()
                .map(ToString::to_string)
                .collect(),
            order: Some(source.order),
            lmfdb_label: Some(source.lmfdb_label.clone()),
            lmfdb_url: Some(source.lmfdb_url()),
            structure_description: source.structure_description.clone(),
        },
        group,
    })
}

fn infinite_cyclic_parameter() -> GroupParameter {
    GroupParameter {
        label: "Z".to_owned(),
        group: FinitelyGeneratedAbelianGroup::free(1),
        record: AbelianGroupRecord {
            kind: GroupKindRecord::InfiniteCyclic,
            free_rank: 1,
            torsion_coefficients: Vec::new(),
            order: None,
            lmfdb_label: None,
            lmfdb_url: None,
            structure_description: "Z".to_owned(),
        },
        // There is no distinguished prime for Z; F2 and F3 are the declared
        // first-edition comparison coefficients.
        modular_primes: vec![2, 3],
    }
}

fn build_record(
    parameter: &GroupParameter,
    delooping_degree: usize,
    exact_through_degree: usize,
) -> Result<EilenbergMacLaneRecord> {
    let space = EilenbergMacLaneSpace::new(parameter.group.clone(), delooping_degree)?;
    let integral_homology = match space.integral_homology(exact_through_degree) {
        Ok(result) => InvariantCoverage::Available {
            value: integral_homology_record(&result)?,
        },
        Err(EilenbergMacLaneError::UnsupportedOperation(error)) => InvariantCoverage::Pending {
            operation: error.operation,
            context: error.context,
            detail: error
                .detail
                .unwrap_or_else(|| "native exact playbook is not implemented".to_owned()),
            no_claim: true,
        },
        Err(error) => bail!(error),
    };
    let integral_cohomology = match space.integral_cohomology(exact_through_degree) {
        Ok(result) => InvariantCoverage::Available {
            value: integral_cohomology_record(&result)?,
        },
        Err(EilenbergMacLaneError::UnsupportedOperation(error)) => InvariantCoverage::Pending {
            operation: error.operation,
            context: error.context,
            detail: error
                .detail
                .unwrap_or_else(|| "native exact playbook is not implemented".to_owned()),
            no_claim: true,
        },
        Err(error) => bail!(error),
    };
    let integral_cohomology_ring = match space.integral_cohomology_presentation() {
        Ok(result) => InvariantCoverage::Available {
            value: integral_cohomology_ring_record(&result, exact_through_degree)?,
        },
        Err(EilenbergMacLaneError::UnsupportedOperation(error)) => InvariantCoverage::Pending {
            operation: error.operation,
            context: error.context,
            detail: error.detail.unwrap_or_else(|| {
                "native exact integral-ring playbook is not implemented".to_owned()
            }),
            no_claim: true,
        },
        Err(error) => bail!(error),
    };
    let modular_cohomology = parameter
        .modular_primes
        .iter()
        .copied()
        .map(|prime| {
            let field = FiniteField::prime(prime)?;
            let coverage = match space.modular_cohomology(field.clone()) {
                Ok(result) => InvariantCoverage::Available {
                    value: modular_cohomology_record(&result, exact_through_degree)?,
                },
                Err(EilenbergMacLaneError::UnsupportedOperation(_)) => {
                    match space.modular_cohomology_through_degree(field, exact_through_degree) {
                        Ok(result) => InvariantCoverage::Available {
                            value: bounded_modular_cohomology_record(&result)?,
                        },
                        Err(EilenbergMacLaneError::UnsupportedOperation(error)) => {
                            InvariantCoverage::Pending {
                                operation: error.operation,
                                context: error.context,
                                detail: error.detail.unwrap_or_else(|| {
                                    "native bounded modular playbook is not implemented".to_owned()
                                }),
                                no_claim: true,
                            }
                        }
                        Err(error) => bail!(error),
                    }
                }
                Err(error) => bail!(error),
            };
            Ok(ModularCohomologyCoverageRecord { prime, coverage })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(EilenbergMacLaneRecord {
        record_version: RECORD_VERSION,
        label: format!("{}.{}", parameter.label, delooping_degree),
        group: parameter.record.clone(),
        delooping_degree,
        space_canonical_sha256: semantic_sha256(&space)?,
        integral_homology,
        integral_cohomology,
        integral_cohomology_ring,
        modular_cohomology,
    })
}

fn integral_homology_record(
    result: &EilenbergMacLaneIntegralHomology,
) -> Result<IntegralHomologyRecord> {
    result.verify()?;
    Ok(IntegralHomologyRecord {
        exact_through_degree: result.max_degree(),
        groups: result
            .groups()
            .iter()
            .enumerate()
            .map(|(degree, group)| AbelianGroupInvariantsRecord::from_group(degree, group))
            .collect(),
        strategy: strategy_name(result.strategy()).to_owned(),
        noether_result_sha256: semantic_sha256(result)?,
        verified: true,
    })
}

fn integral_cohomology_record(
    result: &EilenbergMacLaneIntegralCohomology,
) -> Result<IntegralCohomologyRecord> {
    result.verify()?;
    Ok(IntegralCohomologyRecord {
        exact_through_degree: result.max_degree(),
        groups: result
            .groups()
            .iter()
            .enumerate()
            .map(|(degree, group)| AbelianGroupInvariantsRecord::from_group(degree, group))
            .collect(),
        method: "integral_universal_coefficient".to_owned(),
        source_homology_sha256: semantic_sha256(result.homology())?,
        noether_result_sha256: semantic_sha256(result)?,
        verified: true,
    })
}

fn integral_cohomology_ring_record(
    result: &EilenbergMacLaneIntegralCohomologyPresentation,
    inspect_through_degree: usize,
) -> Result<IntegralCohomologyRingRecord> {
    result.verify()?;
    let generators = result
        .generators()
        .iter()
        .map(|generator| IntegralRingGeneratorRecord {
            id: generator.id().to_owned(),
            degree: generator.degree(),
            display_name: generator.display_name().to_owned(),
            origin: match generator.origin() {
                EilenbergMacLaneIntegralGeneratorOrigin::FreeFundamentalClass { component } => {
                    IntegralRingGeneratorOriginRecord::FreeFundamentalClass {
                        component: *component,
                    }
                }
                EilenbergMacLaneIntegralGeneratorOrigin::IntegralBockstein {
                    component,
                    modulus,
                } => IntegralRingGeneratorOriginRecord::IntegralBockstein {
                    component: *component,
                    modulus: modulus.to_string(),
                },
                EilenbergMacLaneIntegralGeneratorOrigin::CyclicTorClass {
                    left_component,
                    right_component,
                    order,
                } => IntegralRingGeneratorOriginRecord::CyclicTorClass {
                    left_component: *left_component,
                    right_component: *right_component,
                    order: order.to_string(),
                },
                EilenbergMacLaneIntegralGeneratorOrigin::SecondDeloopingFundamentalClass {
                    component,
                } => IntegralRingGeneratorOriginRecord::SecondDeloopingFundamentalClass {
                    component: *component,
                },
            },
        })
        .collect();
    let relations = result
        .relations()
        .iter()
        .map(|relation| IntegralRingRelationRecord {
            terms: relation
                .terms()
                .iter()
                .map(|term| IntegralRingRelationTermRecord {
                    coefficient: term.coefficient().to_string(),
                    generator_exponents: term.exponents().to_vec(),
                })
                .collect(),
        })
        .collect();
    let additive_groups = (0..=inspect_through_degree)
        .map(|degree| {
            Ok(AbelianGroupInvariantsRecord::from_group(
                degree,
                &result.additive_group(degree)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(IntegralCohomologyRingRecord {
        coefficient: "Z".to_owned(),
        graded_commutative: true,
        presentation_extent: "complete_closed_form".to_owned(),
        generators,
        relations,
        additive_groups_through_degree: inspect_through_degree,
        additive_groups,
        strategy: integral_ring_strategy_name(result.strategy()).to_owned(),
        noether_result_sha256: semantic_sha256(result)?,
        verified: true,
    })
}

fn integral_ring_strategy_name(
    strategy: EilenbergMacLaneIntegralCohomologyPresentationStrategy,
) -> &'static str {
    match strategy {
        EilenbergMacLaneIntegralCohomologyPresentationStrategy::TrivialSpace => "trivial_space",
        EilenbergMacLaneIntegralCohomologyPresentationStrategy::FreeAndCyclicClassifyingSpace => {
            "free_and_cyclic_classifying_space"
        }
        EilenbergMacLaneIntegralCohomologyPresentationStrategy::FreeAndRankTwoAbelianClassifyingSpace => {
            "free_and_rank_two_abelian_classifying_space"
        }
        EilenbergMacLaneIntegralCohomologyPresentationStrategy::FreeAbelianSecondDelooping => {
            "free_abelian_second_delooping"
        }
    }
}

fn strategy_name(strategy: EilenbergMacLaneHomologyStrategy) -> &'static str {
    match strategy {
        EilenbergMacLaneHomologyStrategy::TrivialSpace => "trivial_space",
        EilenbergMacLaneHomologyStrategy::ClassifyingSpaceKunneth => "classifying_space_kunneth",
        EilenbergMacLaneHomologyStrategy::FreeAbelianSecondDelooping => {
            "free_abelian_second_delooping"
        }
    }
}

fn modular_cohomology_record(
    result: &EilenbergMacLaneModularCohomology,
    inspect_through_degree: usize,
) -> Result<ModularCohomologyRecord> {
    result.verify()?;
    let through_degree = u32::try_from(inspect_through_degree)?;
    Ok(ModularCohomologyRecord {
        coefficient: format!("F{}", result.field().characteristic()),
        presentation_extent: ModularPresentationExtentRecord::CompleteClosedForm,
        presentation: serde_json::from_str(&result.presentation().to_json()?)?,
        generators: modular_generator_records(result.generators()),
        operations_scope: "primary_operations_on_presentation_generators".to_owned(),
        operations: result
            .operations()
            .iter()
            .map(|operation| {
                Ok(ModularOperationRecord {
                    operation: operation.operation().to_string(),
                    source_generator: operation.source_generator(),
                    target: serde_json::from_str(&operation.target().to_json()?)?,
                })
            })
            .collect::<Result<Vec<_>>>()?,
        hilbert_function_through_degree: inspect_through_degree,
        hilbert_function: result.presentation().hilbert_function(through_degree)?,
        strategy: modular_strategy_name(result.strategy()).to_owned(),
        noether_result_sha256: semantic_sha256(result)?,
        verified: true,
    })
}

fn bounded_modular_cohomology_record(
    result: &EilenbergMacLaneModularCohomologyTruncation,
) -> Result<ModularCohomologyRecord> {
    result.verify()?;
    let through_degree = u32::try_from(result.exact_through_degree())?;
    Ok(ModularCohomologyRecord {
        coefficient: format!("F{}", result.field().characteristic()),
        presentation_extent: ModularPresentationExtentRecord::ExactThroughDegree {
            degree: result.exact_through_degree(),
            no_claim_above_degree: true,
        },
        presentation: serde_json::from_str(&result.presentation().to_json()?)?,
        generators: modular_generator_records(result.generators()),
        operations_scope: "generator_origins_only".to_owned(),
        operations: Vec::new(),
        hilbert_function_through_degree: result.exact_through_degree(),
        hilbert_function: result.presentation().hilbert_function(through_degree)?,
        strategy: bounded_modular_strategy_name(result.strategy()).to_owned(),
        noether_result_sha256: semantic_sha256(result)?,
        verified: true,
    })
}

fn modular_generator_records(
    generators: &[noether::topology::EilenbergMacLaneCohomologyGenerator],
) -> Vec<ModularGeneratorRecord> {
    generators
        .iter()
        .map(|generator| ModularGeneratorRecord {
            id: generator.id().to_owned(),
            degree: generator.degree(),
            display_name: generator.display_name().to_owned(),
            origin: match generator.origin() {
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
                    sequence: operation.square_sequence().expect("mod-two generator operation").to_vec(),
                    fundamental_class_lifts_integrally: *fundamental_class_lifts_integrally,
                },
                EilenbergMacLaneGeneratorOrigin::AdmissibleOddPrimaryOperation {
                    component,
                    operation,
                    fundamental_class_lifts_integrally,
                } => ModularGeneratorOriginRecord::AdmissibleOddPrimaryOperation {
                    component: *component,
                    bocksteins: operation.bocksteins().expect("odd-primary generator operation").to_vec(),
                    reduced_powers: operation.reduced_powers().expect("odd-primary generator operation").to_vec(),
                    fundamental_class_lifts_integrally: *fundamental_class_lifts_integrally,
                },
                EilenbergMacLaneGeneratorOrigin::HigherBockstein {
                    component,
                    primary_exponent,
                } => ModularGeneratorOriginRecord::HigherBockstein {
                    component: *component,
                    primary_exponent: *primary_exponent,
                },
                EilenbergMacLaneGeneratorOrigin::AdmissibleSteenrodSquareOnHigherPrimaryClass {
                    component,
                    primary_exponent,
                    source_is_higher_bockstein,
                    operation,
                } => ModularGeneratorOriginRecord::AdmissibleSteenrodSquareOnHigherPrimaryClass {
                    component: *component,
                    primary_exponent: *primary_exponent,
                    source_is_higher_bockstein: *source_is_higher_bockstein,
                    sequence: operation.square_sequence().expect("mod-two generator operation").to_vec(),
                },
                EilenbergMacLaneGeneratorOrigin::AdmissibleOddPrimaryOperationOnHigherPrimaryClass {
                    component,
                    primary_exponent,
                    source_is_higher_bockstein,
                    operation,
                } => ModularGeneratorOriginRecord::AdmissibleOddPrimaryOperationOnHigherPrimaryClass {
                    component: *component,
                    primary_exponent: *primary_exponent,
                    source_is_higher_bockstein: *source_is_higher_bockstein,
                    bocksteins: operation.bocksteins().expect("odd-primary generator operation").to_vec(),
                    reduced_powers: operation.reduced_powers().expect("odd-primary generator operation").to_vec(),
                },
            },
        })
        .collect()
}

fn modular_strategy_name(strategy: EilenbergMacLaneModularCohomologyStrategy) -> &'static str {
    match strategy {
        EilenbergMacLaneModularCohomologyStrategy::TrivialSpace => "trivial_space",
        EilenbergMacLaneModularCohomologyStrategy::ClassifyingSpaceDecomposition => {
            "classifying_space_decomposition"
        }
        EilenbergMacLaneModularCohomologyStrategy::FreeAbelianSecondDelooping => {
            "free_abelian_second_delooping"
        }
    }
}

fn bounded_modular_strategy_name(
    strategy: EilenbergMacLaneModularCohomologyTruncationStrategy,
) -> &'static str {
    match strategy {
        EilenbergMacLaneModularCohomologyTruncationStrategy::TrivialSpace => "trivial_space",
        EilenbergMacLaneModularCohomologyTruncationStrategy::CartanSerreAdmissibleSquares => {
            "cartan_serre_admissible_squares"
        }
        EilenbergMacLaneModularCohomologyTruncationStrategy::CartanSerreAdmissibleOddPrimaryOperations => {
            "cartan_serre_admissible_odd_primary_operations"
        }
    }
}

fn semantic_sha256(value: &impl SemanticType) -> Result<String> {
    Ok(hex::encode(Sha256::digest(value.canonical_bytes()?)))
}

#[cfg(test)]
mod tests {
    use crate::source::{load_source, validate_source};

    use super::*;

    #[test]
    fn pilot_box_is_complete_in_parameters_and_honest_in_coverage() {
        let source = load_source().unwrap();
        validate_source(&source).unwrap();
        let census = build_census(&source, 3, 4, 8).unwrap();
        // trivial, C2, C3, and Z, each in four delooping degrees
        assert_eq!(census.records.len(), 16);
        assert_eq!(
            census
                .records
                .iter()
                .filter(|record| record.integral_homology.is_available())
                .count(),
            8
        );
        assert_eq!(
            census
                .records
                .iter()
                .filter(|record| record.integral_cohomology.is_available())
                .count(),
            8
        );
        assert_eq!(
            census
                .records
                .iter()
                .filter(|record| record.integral_cohomology_ring.is_available())
                .count(),
            8
        );
        assert_eq!(census.records.first().unwrap().label, "1.1.1");
        assert_eq!(census.records.last().unwrap().label, "Z.4");
        assert_eq!(
            census
                .records
                .iter()
                .flat_map(|record| &record.modular_cohomology)
                .filter(|record| record.coverage.is_available())
                .count(),
            16
        );

        let z_three = census
            .records
            .iter()
            .find(|record| record.label == "Z.3")
            .unwrap();
        let mod_two = z_three
            .modular_cohomology
            .iter()
            .find(|record| record.prime == 2)
            .unwrap();
        let InvariantCoverage::Available { value } = &mod_two.coverage else {
            panic!("K(Z,3) mod-two cohomology should be available");
        };
        assert_eq!(
            value.presentation_extent,
            ModularPresentationExtentRecord::ExactThroughDegree {
                degree: 8,
                no_claim_above_degree: true,
            }
        );
        assert_eq!(
            value
                .generators
                .iter()
                .map(|generator| generator.display_name.as_str())
                .collect::<Vec<_>>(),
            ["iota_3", "Sq^2(iota_3)"]
        );
        assert_eq!(value.operations_scope, "generator_origins_only");
        assert!(value.operations.is_empty());

        let mod_three = z_three
            .modular_cohomology
            .iter()
            .find(|record| record.prime == 3)
            .unwrap();
        let InvariantCoverage::Available { value } = &mod_three.coverage else {
            panic!("K(Z,3) mod-three cohomology should be available");
        };
        assert_eq!(
            value
                .generators
                .iter()
                .map(|generator| generator.display_name.as_str())
                .collect::<Vec<_>>(),
            ["iota_3", "P^1(iota_3)", "beta P^1(iota_3)"]
        );
        assert_eq!(
            value.strategy,
            "cartan_serre_admissible_odd_primary_operations"
        );
    }

    #[test]
    fn higher_primary_generator_provenance_survives_projection() {
        let source = load_source().unwrap();
        let census = build_census(&source, 9, 2, 16).unwrap();

        let binary = census
            .records
            .iter()
            .find(|record| record.label == "4.1.2")
            .unwrap();
        let InvariantCoverage::Available { value } = &binary.modular_cohomology[0].coverage else {
            panic!("K(C4,2) mod-two cohomology should be available");
        };
        assert!(matches!(
            value.generators[1].origin,
            ModularGeneratorOriginRecord::HigherBockstein {
                component: 1,
                primary_exponent: 2
            }
        ));
        assert!(matches!(
            &value.generators[2].origin,
            ModularGeneratorOriginRecord::AdmissibleSteenrodSquareOnHigherPrimaryClass {
                source_is_higher_bockstein: true,
                sequence,
                ..
            } if sequence == &[2]
        ));

        let odd = census
            .records
            .iter()
            .find(|record| record.label == "9.1.2")
            .unwrap();
        let InvariantCoverage::Available { value } = &odd.modular_cohomology[0].coverage else {
            panic!("K(C9,2) mod-three cohomology should be available");
        };
        assert!(matches!(
            &value.generators[3].origin,
            ModularGeneratorOriginRecord::AdmissibleOddPrimaryOperationOnHigherPrimaryClass {
                primary_exponent: 2,
                source_is_higher_bockstein: true,
                bocksteins,
                reduced_powers,
                ..
            } if bocksteins == &[true, false] && reduced_powers == &[1]
        ));
    }

    #[test]
    fn first_box_has_fifty_one_complete_integral_rings() {
        let source = load_source().unwrap();
        let census = build_census(&source, 32, 4, 16).unwrap();
        assert_eq!(
            census
                .records
                .iter()
                .filter(|record| record.integral_cohomology_ring.is_available())
                .count(),
            51
        );

        let cyclic = census
            .records
            .iter()
            .find(|record| record.label == "6.2.1")
            .unwrap();
        let InvariantCoverage::Available { value } = &cyclic.integral_cohomology_ring else {
            panic!("K(C6,1) should have a complete integral ring");
        };
        assert_eq!(value.generators[0].display_name, "beta_6(iota_1)");
        assert_eq!(
            value.relations[0].terms[0],
            IntegralRingRelationTermRecord {
                coefficient: "6".to_owned(),
                generator_exponents: vec![1]
            }
        );

        let noncyclic = census
            .records
            .iter()
            .find(|record| record.label == "4.2.1")
            .unwrap();
        let InvariantCoverage::Available { value } = &noncyclic.integral_cohomology_ring else {
            panic!("K(C2 x C2,1) should have a complete integral ring");
        };
        assert_eq!(
            value.strategy,
            "free_and_rank_two_abelian_classifying_space"
        );
        assert!(matches!(
            value.generators[2].origin,
            IntegralRingGeneratorOriginRecord::CyclicTorClass {
                left_component: 1,
                right_component: 2,
                ref order,
            } if order == "2"
        ));
        assert_eq!(
            value.relations[3].terms,
            vec![
                IntegralRingRelationTermRecord {
                    coefficient: "1".to_owned(),
                    generator_exponents: vec![0, 0, 2],
                },
                IntegralRingRelationTermRecord {
                    coefficient: "-1".to_owned(),
                    generator_exponents: vec![2, 1, 0],
                },
                IntegralRingRelationTermRecord {
                    coefficient: "-1".to_owned(),
                    generator_exponents: vec![1, 2, 0],
                },
            ]
        );

        let projective = census
            .records
            .iter()
            .find(|record| record.label == "Z.2")
            .unwrap();
        let InvariantCoverage::Available { value } = &projective.integral_cohomology_ring else {
            panic!("K(Z,2) should have a complete integral ring");
        };
        assert_eq!(value.generators[0].display_name, "iota_2");
        assert!(value.relations.is_empty());
    }
}
