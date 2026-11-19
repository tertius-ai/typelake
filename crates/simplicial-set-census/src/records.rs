use anyhow::{Result, ensure};
use noether::{
    SemanticType,
    homalg::{ChainCancellation, UniversalUnit},
    topology::{SimplicialFace, SimplicialSet},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::RECORD_VERSION;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SimplicialFaceKey {
    Cell {
        simplex: usize,
    },
    Degenerate {
        simplex_dimension: usize,
        simplex: usize,
        surjection: Vec<usize>,
    },
}

impl From<&SimplicialFace> for SimplicialFaceKey {
    fn from(value: &SimplicialFace) -> Self {
        match value {
            SimplicialFace::Cell(simplex) => Self::Cell { simplex: *simplex },
            SimplicialFace::Degenerate {
                simplex_dimension,
                simplex,
                surjection,
            } => Self::Degenerate {
                simplex_dimension: *simplex_dimension,
                simplex: *simplex,
                surjection: surjection.clone(),
            },
        }
    }
}

impl From<&SimplicialFaceKey> for SimplicialFace {
    fn from(value: &SimplicialFaceKey) -> Self {
        match value {
            SimplicialFaceKey::Cell { simplex } => Self::Cell(*simplex),
            SimplicialFaceKey::Degenerate {
                simplex_dimension,
                simplex,
                surjection,
            } => Self::Degenerate {
                simplex_dimension: *simplex_dimension,
                simplex: *simplex,
                surjection: surjection.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SimplicialSetKey {
    pub nondegenerate_faces: Vec<Vec<Vec<SimplicialFaceKey>>>,
}

impl SimplicialSetKey {
    #[must_use]
    pub fn from_value(value: &SimplicialSet) -> Self {
        let nondegenerate_faces = if value.is_empty() {
            Vec::new()
        } else {
            (0..=value.dimension() as usize)
                .map(|dimension| {
                    (0..value.count(dimension))
                        .map(|simplex| {
                            value
                                .faces_of(dimension, simplex)
                                .expect("dimension and simplex are in range")
                                .iter()
                                .map(SimplicialFaceKey::from)
                                .collect()
                        })
                        .collect()
                })
                .collect()
        };
        Self {
            nondegenerate_faces,
        }
    }

    pub fn reconstruct(&self) -> Result<SimplicialSet> {
        Ok(SimplicialSet::new(
            self.nondegenerate_faces
                .iter()
                .map(|level| {
                    level
                        .iter()
                        .map(|faces| faces.iter().map(SimplicialFace::from).collect())
                        .collect()
                })
                .collect(),
        )?)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AutomorphismGroupRecord {
    pub degree: usize,
    pub order: String,
    pub generators: Vec<Vec<u32>>,
    pub generators_verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimplicialSetIdentityRecord {
    pub record_version: u32,
    pub key: SimplicialSetKey,
    pub total_nondegenerate: usize,
    pub dimension: i64,
    pub nondegenerate_counts: Vec<usize>,
    pub canonical_bytes_sha256: String,
    pub automorphism_group: AutomorphismGroupRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntegralHomologyRecord {
    pub degree: usize,
    pub free_rank: usize,
    pub torsion_coefficients: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NormalizedBoundaryRecord {
    pub degree: usize,
    pub rows: usize,
    pub columns: usize,
    pub canonical_bytes_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainCancellationRecord {
    pub degree: i32,
    pub row: usize,
    pub column: usize,
    pub unit: i8,
}

impl ChainCancellationRecord {
    #[must_use]
    pub fn from_noether(value: &ChainCancellation) -> Self {
        Self {
            degree: value.degree(),
            row: value.row(),
            column: value.column(),
            unit: match value.unit() {
                UniversalUnit::One => 1,
                UniversalUnit::NegativeOne => -1,
            },
        }
    }

    pub fn to_noether(&self) -> Result<ChainCancellation> {
        let unit = match self.unit {
            1 => UniversalUnit::One,
            -1 => UniversalUnit::NegativeOne,
            _ => anyhow::bail!("chain-cancellation unit must be 1 or -1"),
        };
        Ok(ChainCancellation::new(
            self.degree,
            self.row,
            self.column,
            unit,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainModuleRecord {
    pub degree: i32,
    pub rank: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReducedDifferentialRecord {
    pub degree: i32,
    pub rows: usize,
    pub columns: usize,
    pub canonical_bytes_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NormalizedChainReductionRecord {
    pub method: String,
    pub coefficient_context: String,
    pub source_cells: usize,
    pub reduced_cells: usize,
    pub cancellations: Vec<ChainCancellationRecord>,
    pub reduced_modules: Vec<ChainModuleRecord>,
    pub reduced_differentials: Vec<ReducedDifferentialRecord>,
    pub witness_verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimplicialSetTopologyRecord {
    pub record_version: u32,
    pub simplicial_set: SimplicialSetKey,
    pub euler_characteristic: i64,
    pub normalized_boundaries: Vec<NormalizedBoundaryRecord>,
    pub normalized_chain_reduction: NormalizedChainReductionRecord,
    pub integral_homology: Vec<IntegralHomologyRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FiniteFieldContextRecord {
    pub characteristic: String,
    pub extension_degree: usize,
    pub defining_modulus: Option<Vec<u64>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CohomologyBasisRecord {
    pub id: String,
    pub degree: u32,
    pub cocycle_coefficients: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CohomologyProductTermRecord {
    pub basis_id: String,
    pub coefficient: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CohomologyProductRecord {
    pub left_id: String,
    pub right_id: String,
    pub terms: Vec<CohomologyProductTermRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CohomologyOperationRecord {
    pub operation: String,
    pub source_id: String,
    pub target_terms: Vec<CohomologyProductTermRecord>,
    pub evidence: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FiniteMultiplicationTableCertificate {
    pub method: String,
    pub maximum_nonzero_degree: u32,
    pub additive_dimensions: Vec<usize>,
    pub algebra_sha256: String,
    pub presentation_sha256: Option<String>,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "extent", rename_all = "snake_case")]
pub enum RingExtentRecord {
    CompletePresentation {
        certificate: FiniteMultiplicationTableCertificate,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimplicialSetCohomologyRecord {
    pub record_version: u32,
    pub simplicial_set: SimplicialSetKey,
    pub coefficient_context: FiniteFieldContextRecord,
    pub basis: Vec<CohomologyBasisRecord>,
    pub products: Vec<CohomologyProductRecord>,
    pub operations: Vec<CohomologyOperationRecord>,
    pub connected_presentation_json: Option<String>,
    pub extent: RingExtentRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimplicialSetSteenrodOperationsRecord {
    pub record_version: u32,
    pub simplicial_set: SimplicialSetKey,
    pub coefficient_context: FiniteFieldContextRecord,
    pub cohomology_record_sha256: String,
    pub operations: Vec<CohomologyOperationRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ElementaryHornCollapseRecord {
    pub simplex_dimension: usize,
    pub simplex: usize,
    pub face: usize,
    pub free_face: usize,
}

impl ElementaryHornCollapseRecord {
    #[must_use]
    pub fn from_noether(value: &noether::topology::SimplicialSetElementaryHornCollapse) -> Self {
        Self {
            simplex_dimension: value.simplex_dimension(),
            simplex: value.simplex(),
            face: value.face(),
            free_face: value.free_face(),
        }
    }

    #[must_use]
    pub const fn to_noether(&self) -> noether::topology::SimplicialSetElementaryHornCollapse {
        noether::topology::SimplicialSetElementaryHornCollapse::new(
            self.simplex_dimension,
            self.simplex,
            self.face,
            self.free_face,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ElementaryHornRelationRecord {
    pub step: ElementaryHornCollapseRecord,
    pub target: SimplicialSetKey,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GreedyHornCollapseRecord {
    pub target: SimplicialSetKey,
    pub source_cells: usize,
    pub target_cells: usize,
    pub steps: Vec<ElementaryHornCollapseRecord>,
    pub terminal: bool,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimplicialSetHomotopyRecord {
    pub record_version: u32,
    pub simplicial_set: SimplicialSetKey,
    pub method: String,
    pub elementary_horn_relations: Vec<ElementaryHornRelationRecord>,
    pub greedy_horn_collapse: GreedyHornCollapseRecord,
}

impl SimplicialSetIdentityRecord {
    pub fn from_canonical(value: &SimplicialSet) -> Result<Self> {
        ensure!(
            value.canonical_form()? == *value,
            "identity requires canonical form"
        );
        let automorphisms = value.automorphism_group()?;
        let record = Self {
            record_version: RECORD_VERSION,
            key: SimplicialSetKey::from_value(value),
            total_nondegenerate: value.total_cells(),
            dimension: value.dimension(),
            nondegenerate_counts: if value.is_empty() {
                Vec::new()
            } else {
                (0..=value.dimension() as usize)
                    .map(|dimension| value.count(dimension))
                    .collect()
            },
            canonical_bytes_sha256: hex::encode(Sha256::digest(value.canonical_bytes()?)),
            automorphism_group: AutomorphismGroupRecord {
                degree: automorphisms.degree(),
                order: automorphisms.order().to_decimal(),
                generators: automorphisms
                    .generators()
                    .iter()
                    .map(|generator| generator.images().to_vec())
                    .collect(),
                generators_verified: true,
            },
        };
        record.check_shape()?;
        Ok(record)
    }

    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.record_version == RECORD_VERSION,
            "wrong record version"
        );
        let value = self.key.reconstruct()?;
        ensure!(value.canonical_form()? == value, "key is not canonical");
        ensure!(
            value.total_cells() == self.total_nondegenerate,
            "wrong total"
        );
        ensure!(value.dimension() == self.dimension, "wrong dimension");
        ensure!(
            self.nondegenerate_counts
                == if value.is_empty() {
                    Vec::new()
                } else {
                    (0..=value.dimension() as usize)
                        .map(|degree| value.count(degree))
                        .collect::<Vec<_>>()
                },
            "dimension vector has wrong total"
        );
        ensure!(
            self.automorphism_group.degree == self.total_nondegenerate,
            "wrong automorphism degree"
        );
        ensure!(
            self.automorphism_group.generators_verified,
            "unverified generators"
        );
        ensure!(
            hex::encode(Sha256::digest(value.canonical_bytes()?)) == self.canonical_bytes_sha256,
            "canonical digest mismatch"
        );
        Ok(())
    }
}

#[cfg(test)]
mod dimension_vector_regression {
    use super::*;
    #[test]
    fn a_permuted_dimension_vector_is_rejected() {
        let interval = SimplicialSet::new(vec![
            vec![vec![], vec![]],
            vec![vec![SimplicialFace::Cell(1), SimplicialFace::Cell(0)]],
        ])
        .unwrap()
        .canonical_form()
        .unwrap();
        let mut record = SimplicialSetIdentityRecord::from_canonical(&interval).unwrap();
        record.nondegenerate_counts.reverse();
        assert!(record.check_shape().is_err());
    }
}
