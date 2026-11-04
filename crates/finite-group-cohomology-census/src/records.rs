use anyhow::{Context, Result, ensure};
use noether::{
    SemanticType,
    groups::{Permutation, PermutationGroup},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::SOURCE_CONTRACT;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroupSourceFile {
    pub contract: String,
    pub gap_version: String,
    pub smallgrp_version: String,
    pub maximum_order: usize,
    pub groups: Vec<FiniteGroupSourceRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteGroupSourceRecord {
    pub lmfdb_label: String,
    pub small_groups_order: usize,
    pub small_groups_index: usize,
    pub permutation_degree: usize,
    pub generators: Vec<Vec<u32>>,
}

impl FiniteGroupSourceRecord {
    pub fn reconstruct(&self) -> Result<PermutationGroup> {
        let generators = self
            .generators
            .iter()
            .map(|images| Permutation::from_images(images).map_err(anyhow::Error::from))
            .collect::<Result<Vec<_>>>()?;
        let group = PermutationGroup::from_generators(self.permutation_degree, &generators)?;
        ensure!(
            group.order().to_decimal() == self.small_groups_order.to_string(),
            "{} reconstructs with order {}, expected {}",
            self.lmfdb_label,
            group.order().to_decimal(),
            self.small_groups_order
        );
        Ok(group)
    }

    pub fn canonical_group_sha256(&self) -> Result<String> {
        Ok(hex::encode(Sha256::digest(
            self.reconstruct()?.canonical_bytes()?,
        )))
    }

    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.lmfdb_label == format!("{}.{}", self.small_groups_order, self.small_groups_index),
            "LMFDB and SmallGroups labels disagree"
        );
        ensure!(
            self.permutation_degree == self.small_groups_order,
            "first-edition source uses the regular permutation degree"
        );
        ensure!(
            self.generators
                .iter()
                .all(|generator| generator.len() == self.permutation_degree),
            "generator degree mismatch"
        );
        self.reconstruct().context("invalid source presentation")?;
        Ok(())
    }

    #[must_use]
    pub fn lmfdb_url(&self) -> String {
        format!("https://www.lmfdb.org/Groups/Abstract/{}", self.lmfdb_label)
    }
}

impl GroupSourceFile {
    pub fn check_shape(&self) -> Result<()> {
        ensure!(self.contract == SOURCE_CONTRACT, "source contract mismatch");
        ensure!(self.maximum_order == 32, "unexpected source frontier");
        ensure!(!self.gap_version.is_empty(), "missing GAP version");
        ensure!(
            !self.smallgrp_version.is_empty(),
            "missing SmallGrp version"
        );
        for group in &self.groups {
            group.check_shape()?;
        }
        ensure!(
            self.groups.windows(2).all(|pair| {
                (pair[0].small_groups_order, pair[0].small_groups_index)
                    < (pair[1].small_groups_order, pair[1].small_groups_index)
            }),
            "source groups are not uniquely sorted"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteFieldContextRecord {
    pub characteristic: String,
    pub extension_degree: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionEvidenceRecord {
    pub model: String,
    pub strategy: String,
    pub computed_through_degree: usize,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohomologyBasisRecord {
    pub id: String,
    pub degree: usize,
    pub canonical_model_cocycle: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohomologyProductTermRecord {
    pub basis_id: String,
    pub coefficient: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohomologyProductRecord {
    pub left_id: String,
    pub right_id: String,
    pub target_terms: Vec<CohomologyProductTermRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohomologyAlgebraGeneratorRecord {
    pub name: String,
    pub display_name: String,
    pub degree: usize,
    pub basis_id: String,
    pub operation_witness: Option<CohomologyGeneratorOperationWitnessRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissibleCohomologyOperationRecord {
    pub prime: u64,
    pub display_name: String,
    pub square_sequence: Option<Vec<u32>>,
    pub bocksteins: Option<Vec<bool>>,
    pub reduced_powers: Option<Vec<u32>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohomologyGeneratorOperationWitnessRecord {
    pub source_generator_index: usize,
    pub operation: AdmissibleCohomologyOperationRecord,
    pub operation_coefficient: Vec<String>,
    pub decomposable_correction: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohomologyAlgebraRelationTermRecord {
    pub coefficient: Vec<String>,
    pub exponents: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohomologyAlgebraRelationRecord {
    pub degree: usize,
    pub terms: Vec<CohomologyAlgebraRelationTermRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohomologyAlgebraTruncationRecord {
    pub exact_through_degree: usize,
    pub generators: Vec<CohomologyAlgebraGeneratorRecord>,
    pub relations: Vec<CohomologyAlgebraRelationRecord>,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModularGeneratorOriginRecord {
    FundamentalClass {
        component: usize,
    },
    Bockstein {
        source_generator: usize,
    },
    PeriodicCyclicClass {
        component: usize,
        primary_exponent: usize,
    },
    AdmissibleSteenrodSquare {
        component: usize,
        operation: AdmissibleCohomologyOperationRecord,
        fundamental_class_lifts_integrally: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModularGeneratorRecord {
    pub id: String,
    pub degree: u32,
    pub display_name: String,
    pub origin: ModularGeneratorOriginRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModularOperationRecord {
    pub operation: AdmissibleCohomologyOperationRecord,
    pub source_generator: usize,
    pub target: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteAbelianClassifyingSpaceRecord {
    pub method: String,
    pub invariant_factors: Vec<String>,
    pub presentation: serde_json::Value,
    pub generators: Vec<ModularGeneratorRecord>,
    pub operations_scope: String,
    pub operations: Vec<ModularOperationRecord>,
    pub hilbert_function_through_degree: usize,
    pub hilbert_function: Vec<usize>,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ElementaryAbelianRestrictionRecord {
    pub permutation_degree: usize,
    pub generators: Vec<Vec<u32>>,
    pub rank: usize,
    pub restricted_parameters: Vec<serde_json::Value>,
    pub quotient_generator_power_bounds: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompletionCertificateRecord {
    SymondsUniversalBound {
        group_order: String,
        generator_degree_bound: usize,
        relation_degree_bound: usize,
        computed_through_degree: usize,
        noether_result_sha256: String,
        verified: bool,
    },
    SymondsParameterCriterion {
        group_order: String,
        generator_degree_bound: usize,
        relation_degree_bound: usize,
        computed_through_degree: usize,
        parameter_generator_indices: Vec<usize>,
        elementary_abelian_restrictions: Vec<ElementaryAbelianRestrictionRecord>,
        quotient_generator_power_bounds: Vec<u32>,
        maximum_module_generator_degree: usize,
        parameter_degree_bound: usize,
        completion_degree_bound: usize,
        noether_result_sha256: String,
        verified: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RingExtentRecord {
    CompletePresentation {
        certificate: CompletionCertificateRecord,
    },
    ExactThroughDegree {
        degree: usize,
        no_claim_above_degree: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModularComputationRecord {
    FiniteAbelianClassifyingSpace {
        certificate: FiniteAbelianClassifyingSpaceRecord,
    },
    Resolution {
        resolution: ResolutionEvidenceRecord,
        basis: Vec<CohomologyBasisRecord>,
        products: Vec<CohomologyProductRecord>,
        algebra: CohomologyAlgebraTruncationRecord,
        extent: RingExtentRecord,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteGroupModularCohomologyRecord {
    pub record_version: u32,
    pub group_label: String,
    pub group_canonical_sha256: String,
    pub coefficient_context: FiniteFieldContextRecord,
    pub computation: ModularComputationRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralAdditiveGroupRecord {
    pub degree: usize,
    pub free_rank: usize,
    pub torsion_coefficients: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralCocycleTermRecord {
    pub coordinate: usize,
    pub coefficient: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralCohomologyBasisRecord {
    pub id: String,
    pub degree: usize,
    pub order: Option<String>,
    pub canonical_model_cocycle: Vec<IntegralCocycleTermRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralCohomologyProductTermRecord {
    pub basis_id: String,
    pub coefficient: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralCohomologyProductRecord {
    pub left_id: String,
    pub right_id: String,
    pub target_terms: Vec<IntegralCohomologyProductTermRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralCohomologyTruncationRecord {
    pub resolution_strategy: String,
    pub computed_through_degree: usize,
    pub basis: Vec<IntegralCohomologyBasisRecord>,
    pub products: Vec<IntegralCohomologyProductRecord>,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IntegralGeneratorOriginRecord {
    IntegralBockstein {
        component: usize,
        modulus: String,
        source_class: String,
        source_degree: usize,
    },
    CyclicTorClass {
        left_component: usize,
        right_component: usize,
        order: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralGeneratorRecord {
    pub id: String,
    pub degree: usize,
    pub display_name: String,
    pub origin: IntegralGeneratorOriginRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralRelationTermRecord {
    pub coefficient: String,
    pub generator_exponents: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralRelationRecord {
    pub terms: Vec<IntegralRelationTermRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralGradedCommutativePresentationRecord {
    pub coefficient_context: String,
    pub generators: Vec<IntegralGeneratorRecord>,
    pub relations: Vec<IntegralRelationRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CyclicPeriodicResolutionCertificateRecord {
    pub group_order: String,
    pub period: usize,
    pub odd_differential: String,
    pub even_differential: String,
    pub presentation_formula: String,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteAbelianIntegralClassifyingSpaceCertificateRecord {
    pub method: String,
    pub invariant_factors: Vec<String>,
    pub presentation_strategy: String,
    pub theorem_reference: String,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IntegralCompletionCertificateRecord {
    CyclicPeriodicResolution {
        #[serde(flatten)]
        certificate: CyclicPeriodicResolutionCertificateRecord,
    },
    FiniteAbelianClassifyingSpace {
        #[serde(flatten)]
        certificate: FiniteAbelianIntegralClassifyingSpaceCertificateRecord,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IntegralRingExtentRecord {
    CompletePresentation {
        certificate: IntegralCompletionCertificateRecord,
    },
    ExactThroughDegree {
        degree: usize,
        no_claim_above_degree: bool,
        certificate: IntegralTruncationCertificateRecord,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IntegralTruncationCertificateRecord {
    UniversalLowDegree {
        method: String,
        abelianization_invariant_factors: Vec<String>,
        noether_result_sha256: String,
        verified: bool,
    },
    SparseNormalizedBar {
        method: String,
        census_sparse_entry_limit: usize,
        noether_result_sha256: String,
        verified: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteGroupIntegralCohomologyRecord {
    pub record_version: u32,
    pub group_label: String,
    pub group_canonical_sha256: String,
    pub presentation: Option<IntegralGradedCommutativePresentationRecord>,
    pub truncation: Option<IntegralCohomologyTruncationRecord>,
    pub additive_groups_through_degree: Vec<IntegralAdditiveGroupRecord>,
    pub extent: IntegralRingExtentRecord,
}

impl FiniteGroupModularCohomologyRecord {
    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.record_version == crate::RECORD_VERSION,
            "record version"
        );
        ensure!(
            self.coefficient_context.extension_degree == 1,
            "only prime-field records are accepted"
        );
        match &self.computation {
            ModularComputationRecord::FiniteAbelianClassifyingSpace { certificate } => {
                certificate.check_shape(&self.group_label)?;
            }
            ModularComputationRecord::Resolution {
                resolution,
                basis,
                products,
                algebra,
                extent,
            } => check_resolution_shape(
                &self.group_label,
                resolution,
                basis,
                products,
                algebra,
                extent,
            )?,
        }
        Ok(())
    }

    #[must_use]
    pub const fn inspected_through_degree(&self) -> usize {
        match &self.computation {
            ModularComputationRecord::FiniteAbelianClassifyingSpace { certificate } => {
                certificate.hilbert_function_through_degree
            }
            ModularComputationRecord::Resolution { extent, .. } => match extent {
                RingExtentRecord::CompletePresentation { certificate } => {
                    certificate.computed_through_degree()
                }
                RingExtentRecord::ExactThroughDegree { degree, .. } => *degree,
            },
        }
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        match &self.computation {
            ModularComputationRecord::FiniteAbelianClassifyingSpace { .. } => true,
            ModularComputationRecord::Resolution { extent, .. } => extent.is_complete(),
        }
    }

    #[must_use]
    pub const fn is_finite_abelian_family(&self) -> bool {
        matches!(
            &self.computation,
            ModularComputationRecord::FiniteAbelianClassifyingSpace { .. }
        )
    }
}

impl FiniteAbelianClassifyingSpaceRecord {
    fn check_shape(&self, group_label: &str) -> Result<()> {
        ensure!(
            self.method == "finite_abelian_classifying_space",
            "unknown finite-abelian completion method"
        );
        ensure!(self.verified, "finite-abelian certificate is not verified");
        ensure!(
            self.invariant_factors
                .iter()
                .all(|factor| factor.parse::<usize>().is_ok_and(|factor| factor > 1)),
            "invalid finite-abelian invariant factor"
        );
        ensure!(
            self.invariant_factors.windows(2).all(|pair| {
                let left = pair[0].parse::<usize>().ok();
                let right = pair[1].parse::<usize>().ok();
                left.zip(right)
                    .is_some_and(|(left, right)| right.is_multiple_of(left))
            }),
            "finite-abelian invariant factors are not in divisibility order"
        );
        let group_order = self
            .invariant_factors
            .iter()
            .try_fold(1_usize, |order, factor| {
                order.checked_mul(factor.parse::<usize>().ok()?)
            });
        let label_order = group_label
            .split_once('.')
            .context("group label has no order component")?
            .0
            .parse::<usize>()?;
        ensure!(
            group_order == Some(label_order),
            "finite-abelian invariant factors disagree with the group label"
        );
        ensure!(
            self.presentation
                .get("type")
                .and_then(serde_json::Value::as_str)
                == Some("noether/commalg/GradedCommutativePresentation@1"),
            "finite-abelian presentation shape"
        );
        ensure!(
            self.operations_scope == "primary_operations_on_presentation_generators",
            "finite-abelian operation scope"
        );
        ensure!(
            self.hilbert_function.len() == self.hilbert_function_through_degree + 1
                && self.hilbert_function.first() == Some(&1),
            "finite-abelian Hilbert-function extent"
        );
        let generator_ids = self
            .generators
            .iter()
            .map(|generator| generator.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        ensure!(
            generator_ids.len() == self.generators.len()
                && self.generators.iter().all(|generator| {
                    !generator.id.is_empty()
                        && generator.degree > 0
                        && !generator.display_name.is_empty()
                        && match &generator.origin {
                            ModularGeneratorOriginRecord::FundamentalClass { component } => {
                                *component > 0
                            }
                            ModularGeneratorOriginRecord::Bockstein { source_generator } => {
                                *source_generator < self.generators.len()
                            }
                            ModularGeneratorOriginRecord::PeriodicCyclicClass {
                                component,
                                primary_exponent,
                            } => *component > 0 && *primary_exponent > 0,
                            ModularGeneratorOriginRecord::AdmissibleSteenrodSquare {
                                component,
                                operation,
                                ..
                            } => {
                                *component > 0
                                    && operation.prime == 2
                                    && operation.square_sequence.as_ref().is_some_and(|sequence| {
                                        !sequence.is_empty()
                                            && sequence.iter().all(|square| *square > 0)
                                    })
                            }
                        }
                }),
            "finite-abelian generator metadata"
        );
        ensure!(
            self.operations.iter().all(|operation| {
                operation.source_generator < self.generators.len()
                    && admissible_operation_shape(&operation.operation)
                    && operation
                        .target
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        == Some("noether/commalg/GradedCommutativePolynomial@1")
            }),
            "finite-abelian operation shape"
        );
        ensure!(
            self.noether_result_sha256.len() == 64
                && self
                    .noether_result_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "invalid finite-abelian result hash"
        );
        Ok(())
    }
}

fn check_resolution_shape(
    group_label: &str,
    resolution: &ResolutionEvidenceRecord,
    basis: &[CohomologyBasisRecord],
    products: &[CohomologyProductRecord],
    algebra: &CohomologyAlgebraTruncationRecord,
    extent: &RingExtentRecord,
) -> Result<()> {
    let extent_degree = match extent {
        RingExtentRecord::CompletePresentation { certificate } => {
            certificate.computed_through_degree()
        }
        RingExtentRecord::ExactThroughDegree { degree, .. } => *degree,
    };
    ensure!(resolution.verified, "resolution is not verified");
    ensure!(algebra.verified, "algebra truncation is not verified");
    ensure!(
        resolution.computed_through_degree == extent_degree,
        "resolution and ring extents disagree"
    );
    ensure!(
        algebra.exact_through_degree == extent_degree,
        "algebra and ring extents disagree"
    );
    ensure!(
        basis.iter().enumerate().all(|(index, value)| value.id
            == format!("h{}.{}", value.degree, index_in_degree(basis, index) + 1)),
        "basis ids are not canonical"
    );
    ensure!(
        products.iter().all(
            |product| basis.iter().any(|value| value.id == product.left_id)
                && basis.iter().any(|value| value.id == product.right_id)
                && product
                    .target_terms
                    .iter()
                    .all(|term| basis.iter().any(|value| value.id == term.basis_id))
        ),
        "product references an unknown basis id"
    );
    ensure!(
        algebra
            .generators
            .iter()
            .enumerate()
            .all(|(generator_index, generator)| {
                generator.degree > 0
                    && generator.degree <= extent_degree
                    && !generator.display_name.is_empty()
                    && basis.iter().any(|value| {
                        value.id == generator.basis_id && value.degree == generator.degree
                    })
                    && generator.operation_witness.as_ref().is_none_or(|witness| {
                        witness.source_generator_index < generator_index
                            && witness.operation.prime >= 2
                            && !witness.operation.display_name.is_empty()
                            && admissible_operation_shape(&witness.operation)
                            && witness
                                .operation_coefficient
                                .iter()
                                .any(|coefficient| coefficient != "0")
                            && witness.decomposable_correction.len()
                                == basis
                                    .iter()
                                    .filter(|value| value.degree == generator.degree)
                                    .count()
                            && witness
                                .decomposable_correction
                                .iter()
                                .all(|coefficient| !coefficient.is_empty())
                    })
            }),
        "algebra generator references an incompatible basis class"
    );
    ensure!(
        algebra
            .generators
            .iter()
            .enumerate()
            .all(|(index, generator)| algebra.generators[..index]
                .iter()
                .all(|earlier| earlier.name != generator.name)),
        "algebra generator names are not unique"
    );
    ensure!(
        algebra.relations.iter().all(|relation| {
            relation.degree > 0
                && relation.degree <= extent_degree
                && !relation.terms.is_empty()
                && relation.terms.iter().all(|term| {
                    term.exponents.len() == algebra.generators.len()
                        && term.exponents.iter().zip(&algebra.generators).try_fold(
                            0_usize,
                            |degree, (exponent, generator)| {
                                usize::try_from(*exponent)
                                    .ok()
                                    .and_then(|exponent| exponent.checked_mul(generator.degree))
                                    .and_then(|term_degree| degree.checked_add(term_degree))
                            },
                        ) == Some(relation.degree)
                })
        }),
        "algebra relation is not homogeneous inside the exact range"
    );
    match extent {
        RingExtentRecord::CompletePresentation { certificate } => {
            let label_order = group_label
                .split_once('.')
                .context("group label has no order component")?
                .0
                .parse::<usize>()?;
            match certificate {
                CompletionCertificateRecord::SymondsUniversalBound {
                    group_order,
                    generator_degree_bound,
                    relation_degree_bound,
                    computed_through_degree,
                    noether_result_sha256,
                    verified,
                } => {
                    let group_order = group_order.parse::<usize>()?;
                    ensure!(*verified, "completion certificate is not verified");
                    ensure!(
                        *computed_through_degree == algebra.exact_through_degree,
                        "completion certificate and algebra extents disagree"
                    );
                    ensure!(
                        group_order == label_order,
                        "completion certificate group order disagrees with the label"
                    );
                    ensure!(
                        *generator_degree_bound == group_order.saturating_sub(1),
                        "Symonds generator bound does not match the group order"
                    );
                    ensure!(
                        *relation_degree_bound
                            == generator_degree_bound.checked_mul(2).context(
                                "Symonds relation bound overflowed while checking the record",
                            )?,
                        "Symonds relation bound does not match the generator bound"
                    );
                    ensure!(
                        *computed_through_degree >= *relation_degree_bound,
                        "computed range does not reach the Symonds relation bound"
                    );
                    ensure!(
                        noether_result_sha256.len() == 64,
                        "invalid completion-certificate hash"
                    );
                }
                CompletionCertificateRecord::SymondsParameterCriterion {
                    group_order,
                    generator_degree_bound,
                    relation_degree_bound,
                    computed_through_degree,
                    parameter_generator_indices,
                    elementary_abelian_restrictions,
                    quotient_generator_power_bounds,
                    maximum_module_generator_degree,
                    parameter_degree_bound,
                    completion_degree_bound,
                    noether_result_sha256,
                    verified,
                } => {
                    ensure!(*verified, "completion certificate is not verified");
                    ensure!(
                        group_order.parse::<usize>()? == label_order,
                        "completion certificate group order disagrees with the label"
                    );
                    ensure!(
                        *computed_through_degree == algebra.exact_through_degree,
                        "completion certificate and algebra extents disagree"
                    );
                    ensure!(
                        !parameter_generator_indices.is_empty()
                            && parameter_generator_indices
                                .iter()
                                .all(|index| *index < algebra.generators.len())
                            && parameter_generator_indices
                                .iter()
                                .copied()
                                .collect::<std::collections::BTreeSet<_>>()
                                .len()
                                == parameter_generator_indices.len(),
                        "invalid Symonds parameter indices"
                    );
                    ensure!(
                        *parameter_degree_bound
                            == parameter_generator_indices
                                .iter()
                                .try_fold(1usize, |bound, index| bound.checked_add(
                                    algebra.generators[*index].degree.saturating_sub(1),
                                ),)
                                .context("Symonds parameter-degree bound overflow")?,
                        "Symonds parameter-degree bound mismatch"
                    );
                    ensure!(
                        *completion_degree_bound
                            == (*maximum_module_generator_degree).max(*parameter_degree_bound)
                            && *computed_through_degree >= *completion_degree_bound,
                        "Symonds completion stopping inequality"
                    );
                    ensure!(
                        quotient_generator_power_bounds.len() == algebra.generators.len()
                            && quotient_generator_power_bounds
                                .iter()
                                .all(|bound| *bound > 0),
                        "invalid approximation quotient power bounds"
                    );
                    ensure!(
                        !elementary_abelian_restrictions.is_empty()
                            && elementary_abelian_restrictions.iter().all(|restriction| {
                                restriction.permutation_degree == label_order
                                    && restriction.rank > 0
                                    && restriction.generators.iter().all(|generator| {
                                        generator.len() == restriction.permutation_degree
                                    })
                                    && restriction.restricted_parameters.len()
                                        == parameter_generator_indices.len()
                                    && restriction.restricted_parameters.iter().all(|parameter| {
                                        parameter.get("type").and_then(serde_json::Value::as_str)
                                            == Some("noether/commalg/GradedCommutativePolynomial@1")
                                    })
                                    && !restriction.quotient_generator_power_bounds.is_empty()
                                    && restriction
                                        .quotient_generator_power_bounds
                                        .iter()
                                        .all(|bound| *bound > 0)
                            }),
                        "invalid elementary-abelian restriction evidence"
                    );
                    ensure!(
                        *generator_degree_bound
                            == algebra
                                .generators
                                .iter()
                                .map(|generator| generator.degree)
                                .max()
                                .unwrap_or(0)
                            && *relation_degree_bound
                                == algebra
                                    .relations
                                    .iter()
                                    .map(|relation| relation.degree)
                                    .max()
                                    .unwrap_or(0),
                        "complete-presentation actual degree bounds mismatch"
                    );
                    ensure!(
                        noether_result_sha256.len() == 64,
                        "invalid completion-certificate hash"
                    );
                }
            }
        }
        RingExtentRecord::ExactThroughDegree {
            no_claim_above_degree,
            ..
        } => ensure!(
            *no_claim_above_degree,
            "truncation made a higher-degree claim"
        ),
    }
    Ok(())
}

fn admissible_operation_shape(operation: &AdmissibleCohomologyOperationRecord) -> bool {
    if operation.display_name.is_empty() {
        return false;
    }
    match (
        &operation.square_sequence,
        &operation.bocksteins,
        &operation.reduced_powers,
    ) {
        (Some(_), None, None) => operation.prime == 2,
        (None, Some(bocksteins), Some(powers)) => {
            operation.prime > 2 && bocksteins.len() == powers.len().saturating_add(1)
        }
        _ => false,
    }
}

impl RingExtentRecord {
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::CompletePresentation { .. })
    }
}

impl CompletionCertificateRecord {
    #[must_use]
    pub const fn computed_through_degree(&self) -> usize {
        match self {
            Self::SymondsUniversalBound {
                computed_through_degree,
                ..
            }
            | Self::SymondsParameterCriterion {
                computed_through_degree,
                ..
            } => *computed_through_degree,
        }
    }

    #[must_use]
    pub const fn is_parameter_criterion(&self) -> bool {
        matches!(self, Self::SymondsParameterCriterion { .. })
    }
}

fn index_in_degree(basis: &[CohomologyBasisRecord], index: usize) -> usize {
    basis[..index]
        .iter()
        .filter(|value| value.degree == basis[index].degree)
        .count()
}
