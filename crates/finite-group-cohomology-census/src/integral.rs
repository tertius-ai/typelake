//! Exact integral cohomology extents for finite groups.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use noether::{
    Integer, SemanticType,
    groups::{FiniteAbelianInvariantError, FiniteAbelianization, Permutation, PermutationGroup},
    homalg::{
        FiniteAbelianGroupCohomologyError, FiniteAbelianGroupIntegralCohomology,
        FiniteAbelianGroupIntegralCohomologyPresentation, FiniteCyclicGroupIntegralCohomology,
        FiniteGroupIntegralCohomology, FiniteGroupIntegralResolutionStrategy,
    },
    topology::{
        EilenbergMacLaneError, EilenbergMacLaneIntegralCohomologyPresentationStrategy,
        EilenbergMacLaneIntegralGeneratorOrigin,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    INTEGRAL_CONTRACT, IntegralCensus,
    records::{
        CyclicPeriodicResolutionCertificateRecord,
        FiniteAbelianIntegralClassifyingSpaceCertificateRecord,
        FiniteGroupIntegralCohomologyRecord, FiniteGroupSourceRecord, GroupSourceFile,
        IntegralAdditiveGroupRecord, IntegralCocycleTermRecord, IntegralCohomologyBasisRecord,
        IntegralCohomologyProductRecord, IntegralCohomologyProductTermRecord,
        IntegralCohomologyTruncationRecord, IntegralCompletionCertificateRecord,
        IntegralGeneratorOriginRecord, IntegralGeneratorRecord,
        IntegralGradedCommutativePresentationRecord, IntegralRelationRecord,
        IntegralRelationTermRecord, IntegralRingExtentRecord, IntegralTruncationCertificateRecord,
    },
};

pub const INTEGRAL_RECORD_VERSION: u32 = 4;
pub const DEFAULT_CENSUS_BAR_ENTRY_LIMIT: usize = 100_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralValidationReport {
    pub contract: String,
    pub maximum_order: usize,
    pub additive_through_degree: usize,
    pub bar_entry_limit: usize,
    pub source_groups: usize,
    pub complete_presentations: usize,
    pub complete_cyclic_presentations: usize,
    pub complete_rank_two_abelian_presentations: usize,
    pub exact_low_degree_truncations: usize,
    pub exact_sparse_bar_truncations: usize,
    pub exact_degrees: std::collections::BTreeMap<usize, usize>,
    pub pending_groups: usize,
    pub one_cyclic_group_per_order: bool,
    pub keys_unique_and_sorted: bool,
    pub source_groups_reconstructed: bool,
    pub family_certificates_recomputed: bool,
    pub universal_low_degree_recomputed: bool,
    pub additive_groups_recomputed: bool,
    pub extents_honest: bool,
}

pub fn integral_census(
    source: &GroupSourceFile,
    maximum_order: usize,
    additive_through_degree: usize,
    bar_entry_limit: usize,
) -> Result<IntegralCensus> {
    ensure!(
        additive_through_degree >= 2,
        "integral census degree must include the universal degree-two range"
    );
    ensure!(
        bar_entry_limit > 0,
        "integral sparse-bar budget must be positive"
    );
    let selected = source
        .groups
        .iter()
        .filter(|group| group.small_groups_order <= maximum_order)
        .collect::<Vec<_>>();
    let records = selected
        .iter()
        .map(|group| integral_record(group, additive_through_degree, bar_entry_limit))
        .collect::<Result<Vec<_>>>()?;
    Ok(IntegralCensus {
        maximum_order,
        additive_through_degree,
        bar_entry_limit,
        source_group_count: selected.len(),
        records,
    })
}

pub fn integral_record(
    source: &FiniteGroupSourceRecord,
    additive_through_degree: usize,
    bar_entry_limit: usize,
) -> Result<FiniteGroupIntegralCohomologyRecord> {
    ensure!(
        additive_through_degree >= 2,
        "integral record degree must include the universal degree-two range"
    );
    ensure!(
        bar_entry_limit > 0,
        "integral sparse-bar budget must be positive"
    );
    let group = reconstruct(source)?;
    match group.complete_abelian_integral_cohomology() {
        Ok(result) => return record_from_result(source, &result, additive_through_degree),
        Err(FiniteAbelianGroupCohomologyError::AbelianInvariants(
            FiniteAbelianInvariantError::NonAbelian,
        ))
        | Err(FiniteAbelianGroupCohomologyError::EilenbergMacLane(
            EilenbergMacLaneError::UnsupportedOperation(_),
        )) => {}
        Err(error) => return Err(error.into()),
    };
    let exact_degree = census_bar_degree(
        source.small_groups_order,
        additive_through_degree,
        bar_entry_limit,
    );
    if exact_degree <= 2 {
        universal_low_degree_record(source, &group)
    } else {
        sparse_bar_record(source, &group, exact_degree, bar_entry_limit)
    }
}

pub fn validate_integral_census(
    source: &GroupSourceFile,
    census: &IntegralCensus,
) -> Result<IntegralValidationReport> {
    let selected = source
        .groups
        .iter()
        .filter(|group| group.small_groups_order <= census.maximum_order)
        .collect::<Vec<_>>();
    ensure!(
        census.source_group_count == selected.len(),
        "integral source-group count mismatch"
    );
    validate_records_sorted(&census.records)?;
    let expected = selected
        .iter()
        .map(|group| {
            integral_record(
                group,
                census.additive_through_degree,
                census.bar_entry_limit,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        census.records == expected,
        "integral finite-abelian-family recomputation mismatch"
    );
    let cyclic_orders =
        census
            .records
            .iter()
            .filter(|record| {
                matches!(
                    &record.extent,
                    IntegralRingExtentRecord::CompletePresentation {
                        certificate:
                            IntegralCompletionCertificateRecord::CyclicPeriodicResolution { .. }
                    }
                )
            })
            .map(|record| parse_group_key(&record.group_label).0)
            .collect::<BTreeSet<_>>();
    ensure!(
        cyclic_orders == (1..=census.maximum_order).collect(),
        "the source list does not contain exactly one certified cyclic group in each order"
    );
    let complete_cyclic_presentations = cyclic_orders.len();
    let complete_rank_two_abelian_presentations = census
        .records
        .iter()
        .filter(|record| {
            matches!(
                &record.extent,
                IntegralRingExtentRecord::CompletePresentation {
                    certificate:
                        IntegralCompletionCertificateRecord::FiniteAbelianClassifyingSpace { .. }
                }
            )
        })
        .count();
    let complete_presentations = census
        .records
        .iter()
        .filter(|record| {
            matches!(
                record.extent,
                IntegralRingExtentRecord::CompletePresentation { .. }
            )
        })
        .count();
    let exact_low_degree_truncations = census.records.len() - complete_presentations;
    let exact_sparse_bar_truncations = census
        .records
        .iter()
        .filter(|record| record.truncation.is_some())
        .count();
    let exact_low_degree_truncations = exact_low_degree_truncations - exact_sparse_bar_truncations;
    let exact_degrees = census.records.iter().fold(
        std::collections::BTreeMap::<usize, usize>::new(),
        |mut counts, record| {
            if let IntegralRingExtentRecord::ExactThroughDegree { degree, .. } = record.extent {
                *counts.entry(degree).or_default() += 1;
            }
            counts
        },
    );
    Ok(IntegralValidationReport {
        contract: INTEGRAL_CONTRACT.to_owned(),
        maximum_order: census.maximum_order,
        additive_through_degree: census.additive_through_degree,
        bar_entry_limit: census.bar_entry_limit,
        source_groups: selected.len(),
        complete_presentations,
        complete_cyclic_presentations,
        complete_rank_two_abelian_presentations,
        exact_low_degree_truncations,
        exact_sparse_bar_truncations,
        exact_degrees,
        pending_groups: 0,
        one_cyclic_group_per_order: true,
        keys_unique_and_sorted: true,
        source_groups_reconstructed: true,
        family_certificates_recomputed: true,
        universal_low_degree_recomputed: true,
        additive_groups_recomputed: true,
        extents_honest: true,
    })
}

pub fn validate_records_sorted(records: &[FiniteGroupIntegralCohomologyRecord]) -> Result<()> {
    ensure!(
        records
            .windows(2)
            .all(|pair| parse_group_key(&pair[0].group_label)
                < parse_group_key(&pair[1].group_label)),
        "integral records are not uniquely sorted"
    );
    for record in records {
        check_record_shape(record)?;
    }
    Ok(())
}

fn reconstruct(source: &FiniteGroupSourceRecord) -> Result<PermutationGroup> {
    let generators = source
        .generators
        .iter()
        .map(|images| Permutation::from_images(images).map_err(anyhow::Error::from))
        .collect::<Result<Vec<_>>>()?;
    let group = PermutationGroup::from_generators(source.permutation_degree, &generators)?;
    ensure!(
        group.order().to_decimal() == source.small_groups_order.to_string(),
        "{} reconstructs with the wrong order in the integral Noether revision",
        source.lmfdb_label
    );
    Ok(group)
}

fn census_bar_degree(group_order: usize, requested_degree: usize, entry_limit: usize) -> usize {
    let alphabet_size = group_order.saturating_sub(1);
    let mut exact_degree = 2;
    for degree in 3..=requested_degree {
        let Some(boundary_degree) = degree.checked_add(1) else {
            break;
        };
        let Ok(exponent) = u32::try_from(boundary_degree) else {
            break;
        };
        let Some(columns) = alphabet_size.checked_pow(exponent) else {
            break;
        };
        let Some(entries) = columns.checked_mul(boundary_degree.saturating_add(1)) else {
            break;
        };
        if entries > entry_limit {
            break;
        }
        exact_degree = degree;
    }
    exact_degree
}

fn sparse_bar_record(
    source: &FiniteGroupSourceRecord,
    group: &PermutationGroup,
    exact_degree: usize,
    bar_entry_limit: usize,
) -> Result<FiniteGroupIntegralCohomologyRecord> {
    let result = group.integral_cohomology(exact_degree)?;
    result.verify()?;
    ensure!(
        result.resolution_strategy() == FiniteGroupIntegralResolutionStrategy::NormalizedBar,
        "Noether did not select the sparse normalized-bar integral strategy"
    );
    let additive_groups_through_degree = (0..=exact_degree)
        .map(|degree| {
            let additive_group = result.additive_group(degree)?;
            Ok(IntegralAdditiveGroupRecord {
                degree,
                free_rank: additive_group.free_rank(),
                torsion_coefficients: additive_group
                    .torsion_coefficients()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let mut basis = Vec::new();
    let mut degree_offsets = Vec::with_capacity(exact_degree + 2);
    for degree in 0..=exact_degree {
        degree_offsets.push(basis.len());
        let orders = result.basis_orders(degree)?;
        let cocycles = result
            .basis_cocycles(degree)?
            .context("normalized-bar result omitted its canonical cocycles")?;
        ensure!(
            orders.len() == cocycles.len(),
            "integral basis shape mismatch"
        );
        for (index, (order, cocycle)) in orders.iter().zip(cocycles).enumerate() {
            basis.push(IntegralCohomologyBasisRecord {
                id: format!("h{degree}.{}", index + 1),
                degree,
                order: order.as_ref().map(ToString::to_string),
                canonical_model_cocycle: cocycle
                    .values()
                    .enumerate()
                    .filter_map(|(coordinate, coefficient)| {
                        (coefficient != 0).then(|| IntegralCocycleTermRecord {
                            coordinate,
                            coefficient: coefficient.to_string(),
                        })
                    })
                    .collect(),
            });
        }
    }
    degree_offsets.push(basis.len());

    let mut products = Vec::new();
    for left_degree in 0..=exact_degree {
        for right_degree in 0..=exact_degree - left_degree {
            let target_degree = left_degree + right_degree;
            let left_count = degree_offsets[left_degree + 1] - degree_offsets[left_degree];
            let right_count = degree_offsets[right_degree + 1] - degree_offsets[right_degree];
            let target_count = degree_offsets[target_degree + 1] - degree_offsets[target_degree];
            for left_index in 0..left_count {
                for right_index in 0..right_count {
                    let coordinates = result.basis_cup_product(
                        left_degree,
                        left_index,
                        right_degree,
                        right_index,
                    )?;
                    ensure!(
                        coordinates.len() == target_count,
                        "integral product coordinate shape mismatch"
                    );
                    let target_terms = coordinates
                        .into_iter()
                        .enumerate()
                        .filter_map(|(target_index, coefficient)| {
                            (coefficient != 0).then(|| IntegralCohomologyProductTermRecord {
                                basis_id: basis[degree_offsets[target_degree] + target_index]
                                    .id
                                    .clone(),
                                coefficient: coefficient.to_string(),
                            })
                        })
                        .collect();
                    products.push(IntegralCohomologyProductRecord {
                        left_id: basis[degree_offsets[left_degree] + left_index].id.clone(),
                        right_id: basis[degree_offsets[right_degree] + right_index].id.clone(),
                        target_terms,
                    });
                }
            }
        }
    }

    let noether_result_sha256 = hex::encode(Sha256::digest(result.canonical_bytes()?));
    Ok(FiniteGroupIntegralCohomologyRecord {
        record_version: INTEGRAL_RECORD_VERSION,
        group_label: source.lmfdb_label.clone(),
        group_canonical_sha256: hex::encode(Sha256::digest(group.canonical_bytes()?)),
        presentation: None,
        truncation: Some(IntegralCohomologyTruncationRecord {
            resolution_strategy: "sparse_normalized_bar".to_owned(),
            computed_through_degree: exact_degree,
            basis,
            products,
            noether_result_sha256: noether_result_sha256.clone(),
            verified: true,
        }),
        additive_groups_through_degree,
        extent: IntegralRingExtentRecord::ExactThroughDegree {
            degree: exact_degree,
            no_claim_above_degree: true,
            certificate: IntegralTruncationCertificateRecord::SparseNormalizedBar {
                method: "sparse_normalized_bar_with_universal_unit_dmt".to_owned(),
                census_sparse_entry_limit: bar_entry_limit,
                noether_result_sha256,
                verified: true,
            },
        },
    })
}

fn universal_low_degree_record(
    source: &FiniteGroupSourceRecord,
    group: &PermutationGroup,
) -> Result<FiniteGroupIntegralCohomologyRecord> {
    let result = group.integral_cohomology(2)?;
    result.verify()?;
    ensure!(
        result.resolution_strategy() == FiniteGroupIntegralResolutionStrategy::UniversalLowDegree,
        "Noether did not select the universal low-degree integral strategy"
    );
    let additive_groups_through_degree = (0..=2)
        .map(|degree| {
            let additive_group = result.additive_group(degree)?;
            Ok(IntegralAdditiveGroupRecord {
                degree,
                free_rank: additive_group.free_rank(),
                torsion_coefficients: additive_group
                    .torsion_coefficients()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let abelianization_invariant_factors = group
        .abelianization_invariant_factors()?
        .iter()
        .map(ToString::to_string)
        .collect();
    Ok(FiniteGroupIntegralCohomologyRecord {
        record_version: INTEGRAL_RECORD_VERSION,
        group_label: source.lmfdb_label.clone(),
        group_canonical_sha256: hex::encode(Sha256::digest(group.canonical_bytes()?)),
        presentation: None,
        truncation: None,
        additive_groups_through_degree,
        extent: IntegralRingExtentRecord::ExactThroughDegree {
            degree: 2,
            no_claim_above_degree: true,
            certificate: IntegralTruncationCertificateRecord::UniversalLowDegree {
                method: "universal_low_degree_via_abelianization_ext".to_owned(),
                abelianization_invariant_factors,
                noether_result_sha256: hex::encode(Sha256::digest(result.canonical_bytes()?)),
                verified: true,
            },
        },
    })
}

fn record_from_result(
    source: &FiniteGroupSourceRecord,
    result: &FiniteAbelianGroupIntegralCohomologyPresentation,
    additive_through_degree: usize,
) -> Result<FiniteGroupIntegralCohomologyRecord> {
    result.verify()?;
    let presentation = result.classifying_space_cohomology();
    presentation.verify()?;
    let generators = presentation
        .generators()
        .iter()
        .map(|generator| {
            let origin = match generator.origin() {
                EilenbergMacLaneIntegralGeneratorOrigin::IntegralBockstein {
                    component,
                    modulus,
                } => IntegralGeneratorOriginRecord::IntegralBockstein {
                    component: *component,
                    modulus: modulus.to_string(),
                    source_class: if result.invariant_factors().len() == 1 {
                        "iota_1".to_owned()
                    } else {
                        format!("iota_1^({component})")
                    },
                    source_degree: 1,
                },
                EilenbergMacLaneIntegralGeneratorOrigin::CyclicTorClass {
                    left_component,
                    right_component,
                    order,
                } => IntegralGeneratorOriginRecord::CyclicTorClass {
                    left_component: *left_component,
                    right_component: *right_component,
                    order: order.to_string(),
                },
                other => {
                    return Err(anyhow::anyhow!(
                        "unexpected integral generator origin for finite group: {other:?}"
                    ));
                }
            };
            Ok(IntegralGeneratorRecord {
                id: generator.id().to_owned(),
                degree: usize::try_from(generator.degree())?,
                display_name: generator.display_name().to_owned(),
                origin,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let relations = presentation
        .relations()
        .iter()
        .map(|relation| IntegralRelationRecord {
            terms: relation
                .terms()
                .iter()
                .map(|term| IntegralRelationTermRecord {
                    coefficient: term.coefficient().to_string(),
                    generator_exponents: term.exponents().to_vec(),
                })
                .collect(),
        })
        .collect();
    let additive_groups_through_degree = (0..=additive_through_degree)
        .map(|degree| {
            let group = result.additive_group(degree)?;
            Ok(IntegralAdditiveGroupRecord {
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
    let certificate = if result.invariant_factors().len() <= 1 {
        let cyclic = result.group().complete_cyclic_integral_cohomology()?;
        cyclic.verify()?;
        let order = cyclic.order().to_string();
        let presentation_formula = if cyclic.generator_id().is_some() {
            format!("Z[u2_1]/({order}*u2_1), |u2_1|=2")
        } else {
            "Z".to_owned()
        };
        IntegralCompletionCertificateRecord::CyclicPeriodicResolution {
            certificate: CyclicPeriodicResolutionCertificateRecord {
                group_order: order.clone(),
                period: 2,
                odd_differential: "g - 1".to_owned(),
                even_differential: if order == "1" {
                    "1".to_owned()
                } else {
                    format!("1 + g + ... + g^({order} - 1)")
                },
                presentation_formula,
                noether_result_sha256: hex::encode(Sha256::digest(cyclic.canonical_bytes()?)),
                verified: true,
            },
        }
    } else {
        let presentation_strategy = match presentation.strategy() {
            EilenbergMacLaneIntegralCohomologyPresentationStrategy::FreeAndRankTwoAbelianClassifyingSpace => {
                "free_and_rank_two_abelian_classifying_space"
            }
            other => {
                return Err(anyhow::anyhow!(
                    "unexpected rank-two integral presentation strategy: {other:?}"
                ));
            }
        };
        IntegralCompletionCertificateRecord::FiniteAbelianClassifyingSpace {
            certificate: FiniteAbelianIntegralClassifyingSpaceCertificateRecord {
                method: result.completion_strategy().to_owned(),
                invariant_factors: result
                    .invariant_factors()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                presentation_strategy: presentation_strategy.to_owned(),
                theorem_reference: "Roberts 2013, Theorem 6.3.1".to_owned(),
                noether_result_sha256: hex::encode(Sha256::digest(result.canonical_bytes()?)),
                verified: true,
            },
        }
    };
    Ok(FiniteGroupIntegralCohomologyRecord {
        record_version: INTEGRAL_RECORD_VERSION,
        group_label: source.lmfdb_label.clone(),
        group_canonical_sha256: hex::encode(Sha256::digest(result.group().canonical_bytes()?)),
        presentation: Some(IntegralGradedCommutativePresentationRecord {
            coefficient_context: "Z".to_owned(),
            generators,
            relations,
        }),
        truncation: None,
        additive_groups_through_degree,
        extent: IntegralRingExtentRecord::CompletePresentation { certificate },
    })
}

fn check_record_shape(record: &FiniteGroupIntegralCohomologyRecord) -> Result<()> {
    ensure!(record.record_version == INTEGRAL_RECORD_VERSION);
    ensure!(
        record
            .additive_groups_through_degree
            .iter()
            .enumerate()
            .all(|(degree, group)| group.degree == degree),
        "integral additive groups are not indexed consecutively"
    );
    ensure!(
        record.group_canonical_sha256.len() == 64,
        "integral group hash shape"
    );
    if let Some(presentation) = &record.presentation {
        ensure!(presentation.coefficient_context == "Z");
        let generator_count = presentation.generators.len();
        for relation in &presentation.relations {
            ensure!(!relation.terms.is_empty(), "empty integral relation");
            ensure!(
                relation
                    .terms
                    .iter()
                    .all(|term| term.generator_exponents.len() == generator_count),
                "integral relation arity"
            );
            let degrees = relation
                .terms
                .iter()
                .map(|term| {
                    term.generator_exponents
                        .iter()
                        .zip(&presentation.generators)
                        .try_fold(0usize, |degree, (exponent, generator)| {
                            degree
                                .checked_add(
                                    usize::try_from(*exponent)?
                                        .checked_mul(generator.degree)
                                        .ok_or_else(|| {
                                            anyhow::anyhow!("integral relation degree overflow")
                                        })?,
                                )
                                .ok_or_else(|| anyhow::anyhow!("integral relation degree overflow"))
                        })
                })
                .collect::<Result<BTreeSet<_>>>()?;
            ensure!(degrees.len() == 1, "inhomogeneous integral relation");
        }
    }
    match &record.extent {
        IntegralRingExtentRecord::CompletePresentation { certificate } => {
            ensure!(
                record.presentation.is_some(),
                "complete integral record has no presentation"
            );
            ensure!(
                record.truncation.is_none(),
                "complete integral record also stores a finite truncation"
            );
            match certificate {
                IntegralCompletionCertificateRecord::CyclicPeriodicResolution { certificate } => {
                    ensure!(certificate.verified, "integral certificate is unverified");
                    ensure!(certificate.period == 2, "cyclic resolution period");
                    ensure!(
                        certificate.noether_result_sha256.len() == 64,
                        "integral result hash shape"
                    );
                }
                IntegralCompletionCertificateRecord::FiniteAbelianClassifyingSpace {
                    certificate,
                } => {
                    ensure!(certificate.verified, "integral certificate is unverified");
                    ensure!(
                        certificate.method == "finite_abelian_classifying_space",
                        "finite-abelian integral method"
                    );
                    ensure!(
                        certificate.invariant_factors.len() == 2,
                        "rank-two integral certificate factors"
                    );
                    ensure!(
                        certificate.presentation_strategy
                            == "free_and_rank_two_abelian_classifying_space",
                        "rank-two integral presentation strategy"
                    );
                    ensure!(
                        certificate.noether_result_sha256.len() == 64,
                        "integral result hash shape"
                    );
                }
            }
        }
        IntegralRingExtentRecord::ExactThroughDegree {
            degree,
            no_claim_above_degree,
            certificate,
        } => {
            ensure!(
                record.presentation.is_none(),
                "truncated integral record claims a presentation"
            );
            ensure!(*no_claim_above_degree);
            ensure!(record.additive_groups_through_degree.len() == degree + 1);
            match certificate {
                IntegralTruncationCertificateRecord::UniversalLowDegree {
                    method,
                    abelianization_invariant_factors,
                    noether_result_sha256,
                    verified,
                } => {
                    ensure!(*degree == 2 && record.truncation.is_none());
                    ensure!(
                        method == "universal_low_degree_via_abelianization_ext"
                            && *verified
                            && noether_result_sha256.len() == 64,
                        "universal low-degree integral certificate shape"
                    );
                    ensure!(
                        abelianization_invariant_factors
                            .iter()
                            .all(|factor| canonical_integer_text_greater_than_one(factor)),
                        "universal low-degree abelianization factors are not canonical"
                    );
                }
                IntegralTruncationCertificateRecord::SparseNormalizedBar {
                    method,
                    census_sparse_entry_limit,
                    noether_result_sha256,
                    verified,
                } => {
                    ensure!(*degree > 2 && *census_sparse_entry_limit > 0);
                    ensure!(
                        method == "sparse_normalized_bar_with_universal_unit_dmt"
                            && *verified
                            && noether_result_sha256.len() == 64,
                        "sparse normalized-bar certificate shape"
                    );
                    let truncation = record
                        .truncation
                        .as_ref()
                        .context("sparse normalized-bar record has no ring truncation")?;
                    check_integral_truncation_shape(truncation, *degree)?;
                    ensure!(
                        truncation.noether_result_sha256 == *noether_result_sha256,
                        "integral truncation and certificate hashes disagree"
                    );
                }
            }
        }
    }
    Ok(())
}

fn check_integral_truncation_shape(
    truncation: &IntegralCohomologyTruncationRecord,
    exact_degree: usize,
) -> Result<()> {
    ensure!(
        truncation.resolution_strategy == "sparse_normalized_bar"
            && truncation.computed_through_degree == exact_degree
            && truncation.verified
            && truncation.noether_result_sha256.len() == 64,
        "integral truncation evidence shape"
    );
    let mut local_indices = BTreeMap::<usize, usize>::new();
    let mut basis_degrees = BTreeMap::<&str, usize>::new();
    let mut basis_positions = BTreeMap::<&str, usize>::new();
    ensure!(
        truncation
            .basis
            .windows(2)
            .all(|pair| pair[0].degree <= pair[1].degree),
        "integral basis is not ordered by degree"
    );
    for (position, basis) in truncation.basis.iter().enumerate() {
        let local = local_indices.entry(basis.degree).or_default();
        ensure!(
            basis.degree <= exact_degree && basis.id == format!("h{}.{}", basis.degree, *local + 1),
            "integral basis ids are not canonical"
        );
        *local += 1;
        ensure!(
            basis
                .order
                .as_ref()
                .is_none_or(|order| canonical_integer_text_greater_than_one(order)),
            "integral basis order is not canonical"
        );
        ensure!(
            basis
                .canonical_model_cocycle
                .windows(2)
                .all(|pair| pair[0].coordinate < pair[1].coordinate)
                && basis
                    .canonical_model_cocycle
                    .iter()
                    .all(|term| canonical_nonzero_integer_text(&term.coefficient)),
            "integral cocycle is not in canonical sparse form"
        );
        ensure!(
            basis_degrees.insert(&basis.id, basis.degree).is_none(),
            "duplicate integral basis id"
        );
        basis_positions.insert(&basis.id, position);
    }
    ensure!(
        local_indices.get(&0) == Some(&1),
        "integral truncation has no unique degree-zero unit"
    );
    let expected_products =
        local_indices
            .iter()
            .try_fold(0usize, |total, (left, left_count)| {
                local_indices
                    .iter()
                    .try_fold(total, |total, (right, right_count)| {
                        if left + right <= exact_degree {
                            total
                                .checked_add(
                                    left_count
                                        .checked_mul(*right_count)
                                        .context("integral product count overflow")?,
                                )
                                .context("integral product count overflow")
                        } else {
                            Ok(total)
                        }
                    })
            })?;
    ensure!(
        truncation.products.len() == expected_products,
        "integral product table is incomplete"
    );
    let mut product_pairs = BTreeSet::new();
    for product in &truncation.products {
        let left_degree = *basis_degrees
            .get(product.left_id.as_str())
            .context("integral product has unknown left basis id")?;
        let right_degree = *basis_degrees
            .get(product.right_id.as_str())
            .context("integral product has unknown right basis id")?;
        let target_degree = left_degree + right_degree;
        ensure!(target_degree <= exact_degree);
        ensure!(
            product_pairs.insert((product.left_id.as_str(), product.right_id.as_str())),
            "duplicate integral product pair"
        );
        ensure!(
            product.target_terms.windows(2).all(|pair| {
                basis_positions.get(pair[0].basis_id.as_str())
                    < basis_positions.get(pair[1].basis_id.as_str())
            }),
            "integral product terms are not in canonical basis order"
        );
        ensure!(
            product.target_terms.iter().all(|term| {
                canonical_nonzero_integer_text(&term.coefficient)
                    && basis_degrees.get(term.basis_id.as_str()) == Some(&target_degree)
            }),
            "integral product has an invalid target term"
        );
    }
    Ok(())
}

fn canonical_nonzero_integer_text(text: &str) -> bool {
    text.parse::<Integer>()
        .is_ok_and(|value| value != 0 && value.to_string() == text)
}

fn canonical_integer_text_greater_than_one(text: &str) -> bool {
    text.parse::<Integer>()
        .is_ok_and(|value| value > 1 && value.to_string() == text)
}

fn parse_group_key(label: &str) -> (usize, usize) {
    let (order, index) = label.split_once('.').unwrap_or(("0", "0"));
    (order.parse().unwrap_or(0), index.parse().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{load_source, validate_source};

    #[test]
    fn supported_finite_abelian_families_are_complete() {
        let source = load_source().unwrap();
        validate_source(&source).unwrap();
        let census = integral_census(&source, 4, 8, DEFAULT_CENSUS_BAR_ENTRY_LIMIT).unwrap();
        let report = validate_integral_census(&source, &census).unwrap();
        assert_eq!(report.source_groups, 5);
        assert_eq!(report.complete_cyclic_presentations, 4);
        assert_eq!(report.complete_rank_two_abelian_presentations, 1);
        assert_eq!(report.complete_presentations, 5);
        assert_eq!(report.exact_low_degree_truncations, 0);
        assert_eq!(report.pending_groups, 0);
        assert_eq!(census.records[0].group_label, "1.1");
        assert!(
            census.records[0]
                .presentation
                .as_ref()
                .unwrap()
                .generators
                .is_empty()
        );
        let cyclic_four = census
            .records
            .iter()
            .find(|record| record.group_label.starts_with("4."))
            .unwrap();
        assert_eq!(
            cyclic_four.presentation.as_ref().unwrap().generators[0].display_name,
            "beta_4(iota_1)"
        );
        assert_eq!(
            cyclic_four.presentation.as_ref().unwrap().relations[0].terms[0].coefficient,
            "4"
        );
        let klein_four = census
            .records
            .iter()
            .find(|record| record.group_label == "4.2")
            .unwrap();
        assert_eq!(
            klein_four
                .presentation
                .as_ref()
                .unwrap()
                .generators
                .iter()
                .map(|generator| generator.degree)
                .collect::<Vec<_>>(),
            [2, 2, 3]
        );
        assert_eq!(
            klein_four.presentation.as_ref().unwrap().relations[3]
                .terms
                .len(),
            3
        );
    }

    #[test]
    fn nonabelian_groups_receive_an_exact_integral_degree_two_record() {
        let source = load_source().unwrap();
        let census = integral_census(&source, 6, 8, 1).unwrap();
        let report = validate_integral_census(&source, &census).unwrap();
        assert_eq!(report.source_groups, 8);
        assert_eq!(report.complete_presentations, 7);
        assert_eq!(report.exact_low_degree_truncations, 1);
        assert_eq!(report.pending_groups, 0);
        let truncated = census
            .records
            .iter()
            .find(|record| {
                matches!(
                    record.extent,
                    IntegralRingExtentRecord::ExactThroughDegree { .. }
                )
            })
            .unwrap();
        assert!(truncated.presentation.is_none());
        assert_eq!(truncated.additive_groups_through_degree.len(), 3);
    }

    #[test]
    fn census_budget_selects_a_sparse_bar_ring_truncation() {
        let source = load_source().unwrap();
        let nonabelian_order_six = source
            .groups
            .iter()
            .find(|group| group.small_groups_order == 6 && group.lmfdb_label != "6.2")
            .unwrap();
        let record = integral_record(nonabelian_order_six, 4, 100_000).unwrap();
        assert!(matches!(
            record.extent,
            IntegralRingExtentRecord::ExactThroughDegree { degree: 4, .. }
        ));
        let truncation = record.truncation.as_ref().unwrap();
        assert_eq!(truncation.computed_through_degree, 4);
        assert!(!truncation.basis.is_empty());
        check_record_shape(&record).unwrap();
    }

    #[test]
    fn census_budget_has_a_deterministic_order_frontier() {
        assert_eq!(census_bar_degree(6, 16, 100_000), 4);
        assert_eq!(census_bar_degree(8, 16, 100_000), 3);
        assert_eq!(census_bar_degree(12, 16, 100_000), 3);
        assert_eq!(census_bar_degree(13, 16, 100_000), 2);
    }
}
