use anyhow::{Result, ensure};
use noether::{Integer, SemanticType, homalg::FinitelyGeneratedAbelianGroup};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbelianGroupSourceFile {
    pub contract: String,
    pub gap_version: String,
    pub smallgrp_version: String,
    pub maximum_finite_order: usize,
    pub include_infinite_cyclic: bool,
    pub finite_groups: Vec<FiniteAbelianGroupSourceRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteAbelianGroupSourceRecord {
    pub lmfdb_label: String,
    pub order: usize,
    pub small_groups_index: usize,
    pub gap_abelian_invariants: Vec<u64>,
    pub structure_description: String,
}

impl FiniteAbelianGroupSourceRecord {
    pub fn group(&self) -> Result<FinitelyGeneratedAbelianGroup> {
        Ok(FinitelyGeneratedAbelianGroup::from_cyclic_summands(
            0,
            self.gap_abelian_invariants
                .iter()
                .copied()
                .map(Integer::from)
                .collect(),
        )?)
    }

    #[must_use]
    pub fn lmfdb_url(&self) -> String {
        format!("https://www.lmfdb.org/Groups/Abstract/{}", self.lmfdb_label)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupKindRecord {
    Finite,
    InfiniteCyclic,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbelianGroupRecord {
    pub kind: GroupKindRecord,
    pub free_rank: usize,
    pub torsion_coefficients: Vec<String>,
    pub order: Option<usize>,
    pub lmfdb_label: Option<String>,
    pub lmfdb_url: Option<String>,
    pub structure_description: String,
}

impl AbelianGroupRecord {
    pub fn group(&self) -> Result<FinitelyGeneratedAbelianGroup> {
        let torsion = self
            .torsion_coefficients
            .iter()
            .map(|value| Integer::from_str(value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FinitelyGeneratedAbelianGroup::new(self.free_rank, torsion)?)
    }

    pub fn check_shape(&self) -> Result<()> {
        let group = self.group()?;
        group.verify()?;
        match self.kind {
            GroupKindRecord::Finite => {
                ensure!(self.free_rank == 0, "finite group has positive free rank");
                ensure!(self.order.is_some(), "finite group lacks order");
                ensure!(self.lmfdb_label.is_some(), "finite group lacks LMFDB label");
                ensure!(self.lmfdb_url.is_some(), "finite group lacks LMFDB URL");
            }
            GroupKindRecord::InfiniteCyclic => {
                ensure!(self.free_rank == 1, "infinite cyclic group has wrong rank");
                ensure!(
                    self.torsion_coefficients.is_empty(),
                    "infinite cyclic group has torsion"
                );
                ensure!(
                    self.order.is_none(),
                    "infinite cyclic group has finite order"
                );
                ensure!(self.lmfdb_label.is_none(), "Z has an LMFDB group label");
                ensure!(self.lmfdb_url.is_none(), "Z has an LMFDB group URL");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbelianGroupInvariantsRecord {
    pub degree: usize,
    pub free_rank: usize,
    pub torsion_coefficients: Vec<String>,
}

impl AbelianGroupInvariantsRecord {
    pub fn from_group(degree: usize, group: &FinitelyGeneratedAbelianGroup) -> Self {
        Self {
            degree,
            free_rank: group.free_rank(),
            torsion_coefficients: group
                .torsion_coefficients()
                .iter()
                .map(ToString::to_string)
                .collect(),
        }
    }

    pub fn group(&self) -> Result<FinitelyGeneratedAbelianGroup> {
        let torsion = self
            .torsion_coefficients
            .iter()
            .map(|value| Integer::from_str(value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FinitelyGeneratedAbelianGroup::new(self.free_rank, torsion)?)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralHomologyRecord {
    pub exact_through_degree: usize,
    pub groups: Vec<AbelianGroupInvariantsRecord>,
    pub strategy: String,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralCohomologyRecord {
    pub exact_through_degree: usize,
    pub groups: Vec<AbelianGroupInvariantsRecord>,
    pub method: String,
    pub source_homology_sha256: String,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IntegralRingGeneratorOriginRecord {
    FreeFundamentalClass {
        component: usize,
    },
    IntegralBockstein {
        component: usize,
        modulus: String,
    },
    CyclicTorClass {
        left_component: usize,
        right_component: usize,
        order: String,
    },
    SecondDeloopingFundamentalClass {
        component: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralRingGeneratorRecord {
    pub id: String,
    pub degree: u32,
    pub display_name: String,
    pub origin: IntegralRingGeneratorOriginRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralRingRelationTermRecord {
    pub coefficient: String,
    pub generator_exponents: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralRingRelationRecord {
    pub terms: Vec<IntegralRingRelationTermRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralCohomologyRingRecord {
    pub coefficient: String,
    pub graded_commutative: bool,
    pub presentation_extent: String,
    pub generators: Vec<IntegralRingGeneratorRecord>,
    pub relations: Vec<IntegralRingRelationRecord>,
    pub additive_groups_through_degree: usize,
    pub additive_groups: Vec<AbelianGroupInvariantsRecord>,
    pub strategy: String,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModularCohomologyRecord {
    pub coefficient: String,
    pub presentation_extent: ModularPresentationExtentRecord,
    pub presentation: serde_json::Value,
    pub generators: Vec<ModularGeneratorRecord>,
    pub operations_scope: String,
    pub operations: Vec<ModularOperationRecord>,
    pub hilbert_function_through_degree: usize,
    pub hilbert_function: Vec<usize>,
    pub strategy: String,
    pub noether_result_sha256: String,
    pub verified: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModularPresentationExtentRecord {
    CompleteClosedForm,
    ExactThroughDegree {
        degree: usize,
        no_claim_above_degree: bool,
    },
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
        sequence: Vec<u32>,
        fundamental_class_lifts_integrally: bool,
    },
    AdmissibleOddPrimaryOperation {
        component: usize,
        bocksteins: Vec<bool>,
        reduced_powers: Vec<u32>,
        fundamental_class_lifts_integrally: bool,
    },
    HigherBockstein {
        component: usize,
        primary_exponent: usize,
    },
    AdmissibleSteenrodSquareOnHigherPrimaryClass {
        component: usize,
        primary_exponent: usize,
        source_is_higher_bockstein: bool,
        sequence: Vec<u32>,
    },
    AdmissibleOddPrimaryOperationOnHigherPrimaryClass {
        component: usize,
        primary_exponent: usize,
        source_is_higher_bockstein: bool,
        bocksteins: Vec<bool>,
        reduced_powers: Vec<u32>,
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
    pub operation: String,
    pub source_generator: usize,
    pub target: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum InvariantCoverage<T> {
    Available {
        value: T,
    },
    Pending {
        operation: String,
        context: String,
        detail: String,
        no_claim: bool,
    },
}

impl<T> InvariantCoverage<T> {
    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    pub fn check_pending_shape(&self) -> Result<()> {
        if let Self::Pending {
            operation,
            context,
            detail,
            no_claim,
        } = self
        {
            ensure!(!operation.is_empty(), "pending operation is empty");
            ensure!(!context.is_empty(), "pending context is empty");
            ensure!(!detail.is_empty(), "pending detail is empty");
            ensure!(*no_claim, "pending coverage makes an implicit claim");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModularCohomologyCoverageRecord {
    pub prime: u64,
    pub coverage: InvariantCoverage<ModularCohomologyRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EilenbergMacLaneRecord {
    pub record_version: u32,
    pub label: String,
    pub group: AbelianGroupRecord,
    pub delooping_degree: usize,
    pub space_canonical_sha256: String,
    pub integral_homology: InvariantCoverage<IntegralHomologyRecord>,
    pub integral_cohomology: InvariantCoverage<IntegralCohomologyRecord>,
    pub integral_cohomology_ring: InvariantCoverage<IntegralCohomologyRingRecord>,
    pub modular_cohomology: Vec<ModularCohomologyCoverageRecord>,
}

impl EilenbergMacLaneRecord {
    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.record_version == crate::RECORD_VERSION,
            "record version mismatch"
        );
        ensure!(self.delooping_degree > 0, "zero delooping degree");
        ensure!(
            self.space_canonical_sha256.len() == 64
                && self
                    .space_canonical_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "space hash shape"
        );
        self.group.check_shape()?;
        self.integral_homology.check_pending_shape()?;
        self.integral_cohomology.check_pending_shape()?;
        self.integral_cohomology_ring.check_pending_shape()?;
        ensure!(
            self.modular_cohomology
                .windows(2)
                .all(|pair| pair[0].prime < pair[1].prime),
            "modular primes are not unique and sorted"
        );
        for modular in &self.modular_cohomology {
            ensure!(modular.prime >= 2, "invalid modular prime");
            modular.coverage.check_pending_shape()?;
            if let InvariantCoverage::Available { value } = &modular.coverage {
                ensure!(value.verified, "modular cohomology is not verified");
                ensure!(
                    value.coefficient == format!("F{}", modular.prime),
                    "modular coefficient does not match its prime"
                );
                match value.presentation_extent {
                    ModularPresentationExtentRecord::CompleteClosedForm => ensure!(
                        ![
                            "cartan_serre_admissible_squares",
                            "cartan_serre_admissible_odd_primary_operations",
                        ]
                        .contains(&value.strategy.as_str()),
                        "bounded modular strategy is marked complete"
                    ),
                    ModularPresentationExtentRecord::ExactThroughDegree {
                        degree,
                        no_claim_above_degree,
                    } => {
                        ensure!(
                            degree == value.hilbert_function_through_degree,
                            "bounded modular presentation and Hilbert extents disagree"
                        );
                        ensure!(
                            no_claim_above_degree,
                            "bounded modular presentation makes an implicit claim above its extent"
                        );
                        ensure!(
                            [
                                "cartan_serre_admissible_squares",
                                "cartan_serre_admissible_odd_primary_operations",
                            ]
                            .contains(&value.strategy.as_str()),
                            "bounded modular presentation strategy"
                        );
                        ensure!(
                            value
                                .generators
                                .iter()
                                .all(|generator| usize::try_from(generator.degree)
                                    .is_ok_and(|generator_degree| generator_degree <= degree)),
                            "bounded modular generator exceeds its extent"
                        );
                    }
                }
                ensure!(
                    value
                        .presentation
                        .get("type")
                        .and_then(|value| value.as_str())
                        == Some("noether/commalg/GradedCommutativePresentation@1"),
                    "modular presentation shape"
                );
                match value.presentation_extent {
                    ModularPresentationExtentRecord::CompleteClosedForm => ensure!(
                        value.operations_scope == "primary_operations_on_presentation_generators",
                        "complete modular operation scope"
                    ),
                    ModularPresentationExtentRecord::ExactThroughDegree { .. } => {
                        ensure!(
                            value.operations_scope == "generator_origins_only"
                                && value.operations.is_empty(),
                            "bounded modular operation scope"
                        )
                    }
                }
                ensure!(
                    value.hilbert_function.len() == value.hilbert_function_through_degree + 1,
                    "modular Hilbert-function extent"
                );
                ensure!(
                    value.hilbert_function.first() == Some(&1),
                    "modular degree-zero cohomology"
                );
                let generator_ids = value
                    .generators
                    .iter()
                    .map(|generator| generator.id.as_str())
                    .collect::<std::collections::BTreeSet<_>>();
                ensure!(
                    value
                        .generators
                        .iter()
                        .all(|generator| !generator.id.is_empty()
                            && generator.degree > 0
                            && !generator.display_name.is_empty()),
                    "modular generator shape"
                );
                ensure!(
                    generator_ids.len() == value.generators.len(),
                    "duplicate modular generator id"
                );
                ensure!(
                    value
                        .generators
                        .iter()
                        .all(|generator| match &generator.origin {
                            ModularGeneratorOriginRecord::FundamentalClass { component } => {
                                *component > 0
                            }
                            ModularGeneratorOriginRecord::PeriodicCyclicClass {
                                component,
                                primary_exponent,
                            } => *component > 0 && *primary_exponent > 0,
                            ModularGeneratorOriginRecord::Bockstein { source_generator } => {
                                *source_generator < value.generators.len()
                            }
                            ModularGeneratorOriginRecord::AdmissibleSteenrodSquare {
                                component,
                                sequence,
                                fundamental_class_lifts_integrally,
                            } => {
                                *component > 0
                                    && !sequence.is_empty()
                                    && sequence.iter().all(|degree| *degree > 0)
                                    && sequence.windows(2).all(|pair| pair[1] <= pair[0] / 2)
                                    && (!*fundamental_class_lifts_integrally
                                        || sequence.last() != Some(&1))
                            }
                            ModularGeneratorOriginRecord::AdmissibleOddPrimaryOperation {
                                component,
                                bocksteins,
                                reduced_powers,
                                fundamental_class_lifts_integrally,
                            } => {
                                *component > 0
                                    && bocksteins.len() == reduced_powers.len() + 1
                                    && (bocksteins.iter().any(|exponent| *exponent)
                                        || !reduced_powers.is_empty())
                                    && reduced_powers.iter().all(|power| *power > 0)
                                    && reduced_powers.iter().enumerate().all(|(index, power)| {
                                        reduced_powers.get(index + 1).is_none_or(|next_power| {
                                            let epsilon = u64::from(bocksteins[index + 1]);
                                            u64::from(*power)
                                                >= modular.prime * u64::from(*next_power) + epsilon
                                        })
                                    })
                                    && (!*fundamental_class_lifts_integrally
                                        || bocksteins.last() != Some(&true))
                            }
                            ModularGeneratorOriginRecord::HigherBockstein {
                                component,
                                primary_exponent,
                            } => *component > 0 && *primary_exponent > 1,
                            ModularGeneratorOriginRecord::AdmissibleSteenrodSquareOnHigherPrimaryClass {
                                component,
                                primary_exponent,
                                sequence,
                                ..
                            } => {
                                *component > 0
                                    && *primary_exponent > 1
                                    && !sequence.is_empty()
                                    && sequence.iter().all(|degree| *degree > 0)
                                    && sequence.windows(2).all(|pair| pair[1] <= pair[0] / 2)
                                    && sequence.last() != Some(&1)
                            }
                            ModularGeneratorOriginRecord::AdmissibleOddPrimaryOperationOnHigherPrimaryClass {
                                component,
                                primary_exponent,
                                bocksteins,
                                reduced_powers,
                                ..
                            } => {
                                *component > 0
                                    && *primary_exponent > 1
                                    && bocksteins.len() == reduced_powers.len() + 1
                                    && (bocksteins.iter().any(|exponent| *exponent)
                                        || !reduced_powers.is_empty())
                                    && reduced_powers.iter().all(|power| *power > 0)
                                    && reduced_powers.iter().enumerate().all(|(index, power)| {
                                        reduced_powers.get(index + 1).is_none_or(|next_power| {
                                            let epsilon = u64::from(bocksteins[index + 1]);
                                            u64::from(*power)
                                                >= modular.prime * u64::from(*next_power) + epsilon
                                        })
                                    })
                                    && bocksteins.last() != Some(&true)
                            }
                        }),
                    "modular generator origin"
                );
                ensure!(
                    value.operations.iter().all(|operation| {
                        ["beta", "Sq^1", "Sq^2", "P^1"].contains(&operation.operation.as_str())
                            && operation.source_generator < value.generators.len()
                            && operation
                                .target
                                .get("type")
                                .and_then(|value| value.as_str())
                                == Some("noether/commalg/GradedCommutativePolynomial@1")
                    }),
                    "modular operation shape"
                );
                ensure!(
                    [
                        "trivial_space",
                        "classifying_space_decomposition",
                        "free_abelian_second_delooping",
                        "cartan_serre_admissible_squares",
                        "cartan_serre_admissible_odd_primary_operations",
                    ]
                    .contains(&value.strategy.as_str()),
                    "modular strategy"
                );
                ensure!(
                    value.noether_result_sha256.len() == 64
                        && value
                            .noether_result_sha256
                            .bytes()
                            .all(|byte| byte.is_ascii_hexdigit()),
                    "modular result hash shape"
                );
            }
        }
        if let InvariantCoverage::Available { value } = &self.integral_homology {
            ensure!(value.verified, "integral homology is not verified");
            ensure!(
                value.groups.len() == value.exact_through_degree + 1,
                "integral homology degree rows"
            );
            ensure!(
                value
                    .groups
                    .iter()
                    .enumerate()
                    .all(|(degree, group)| group.degree == degree),
                "integral homology degree indexing"
            );
            ensure!(
                value.noether_result_sha256.len() == 64
                    && value
                        .noether_result_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "homology hash shape"
            );
            for group in &value.groups {
                group.group()?.verify()?;
            }
        }
        if let InvariantCoverage::Available { value } = &self.integral_cohomology {
            ensure!(value.verified, "integral cohomology is not verified");
            ensure!(
                value.groups.len() == value.exact_through_degree + 1,
                "integral cohomology degree rows"
            );
            ensure!(
                value
                    .groups
                    .iter()
                    .enumerate()
                    .all(|(degree, group)| group.degree == degree),
                "integral cohomology degree indexing"
            );
            ensure!(
                value.method == "integral_universal_coefficient",
                "integral cohomology method"
            );
            ensure!(
                [
                    value.source_homology_sha256.as_str(),
                    value.noether_result_sha256.as_str(),
                ]
                .iter()
                .all(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())),
                "integral cohomology hash shape"
            );
            for group in &value.groups {
                group.group()?.verify()?;
            }
        }
        if let InvariantCoverage::Available { value } = &self.integral_cohomology_ring {
            ensure!(value.verified, "integral cohomology ring is not verified");
            ensure!(value.coefficient == "Z", "integral ring coefficient");
            ensure!(
                value.graded_commutative,
                "integral ring is not graded-commutative"
            );
            ensure!(
                value.presentation_extent == "complete_closed_form",
                "integral ring is not a complete closed form"
            );
            ensure!(
                value.additive_groups.len() == value.additive_groups_through_degree + 1
                    && value
                        .additive_groups
                        .iter()
                        .enumerate()
                        .all(|(degree, group)| group.degree == degree),
                "integral ring additive-group extent"
            );
            for group in &value.additive_groups {
                group.group()?.verify()?;
            }
            let generator_ids = value
                .generators
                .iter()
                .map(|generator| generator.id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            ensure!(
                generator_ids.len() == value.generators.len()
                    && value.generators.iter().all(|generator| {
                        !generator.id.is_empty()
                            && generator.degree > 0
                            && !generator.display_name.is_empty()
                            && match &generator.origin {
                                IntegralRingGeneratorOriginRecord::FreeFundamentalClass {
                                    component,
                                }
                                | IntegralRingGeneratorOriginRecord::SecondDeloopingFundamentalClass {
                                    component,
                                } => *component > 0,
                                IntegralRingGeneratorOriginRecord::IntegralBockstein {
                                    component,
                                    modulus,
                                } => {
                                    *component > 0
                                        && Integer::from_str(modulus)
                                            .is_ok_and(|value| value > 1)
                                }
                                IntegralRingGeneratorOriginRecord::CyclicTorClass {
                                    left_component,
                                    right_component,
                                    order,
                                } => {
                                    *left_component > 0
                                        && *right_component > *left_component
                                        && Integer::from_str(order)
                                            .is_ok_and(|value| value > 1)
                                }
                            }
                    }),
                "integral ring generator shape"
            );
            for relation in &value.relations {
                ensure!(!relation.terms.is_empty(), "empty integral ring relation");
                let mut relation_degree = None;
                for term in &relation.terms {
                    let coefficient = Integer::from_str(&term.coefficient)?;
                    ensure!(coefficient != 0, "zero term in integral ring relation");
                    ensure!(
                        term.generator_exponents.len() == value.generators.len(),
                        "integral relation exponent arity"
                    );
                    let degree = term
                        .generator_exponents
                        .iter()
                        .zip(&value.generators)
                        .try_fold(0u64, |total, (exponent, generator)| {
                            total.checked_add(u64::from(*exponent) * u64::from(generator.degree))
                        });
                    let degree = degree
                        .ok_or_else(|| anyhow::anyhow!("integral relation degree exceeds u64"))?;
                    ensure!(
                        relation_degree.is_none_or(|expected| expected == degree),
                        "inhomogeneous integral ring relation"
                    );
                    relation_degree = Some(degree);
                }
            }
            ensure!(
                [
                    "trivial_space",
                    "free_and_cyclic_classifying_space",
                    "free_and_rank_two_abelian_classifying_space",
                    "free_abelian_second_delooping",
                ]
                .contains(&value.strategy.as_str()),
                "integral ring strategy"
            );
            ensure!(
                value.noether_result_sha256.len() == 64
                    && value
                        .noether_result_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "integral ring hash shape"
            );
            if let InvariantCoverage::Available { value: additive } = &self.integral_cohomology {
                ensure!(
                    additive.exact_through_degree == value.additive_groups_through_degree
                        && additive.groups == value.additive_groups,
                    "integral ring additive groups disagree with universal coefficients"
                );
            }
        }
        Ok(())
    }

    pub fn space_hash(&self) -> Result<String> {
        let space = noether::topology::EilenbergMacLaneSpace::new(
            self.group.group()?,
            self.delooping_degree,
        )?;
        Ok(hex::encode(sha2::Sha256::digest(space.canonical_bytes()?)))
    }
}
