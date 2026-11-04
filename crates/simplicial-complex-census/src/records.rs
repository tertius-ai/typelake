use std::cmp::Ordering;

use anyhow::{Result, ensure};
use noether::{SemanticType, topology::SimplicialComplex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::RECORD_VERSION;

/// Stable structured identity of a canonical finite simplicial complex.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SimplicialComplexKey {
    facets: Vec<Vec<u32>>,
}

impl SimplicialComplexKey {
    #[must_use]
    pub fn from_canonical(complex: &SimplicialComplex) -> Self {
        Self {
            facets: complex
                .facets()
                .into_iter()
                .map(|facet| facet.vertices().to_vec())
                .collect(),
        }
    }

    #[must_use]
    pub fn facets(&self) -> &[Vec<u32>] {
        &self.facets
    }

    #[must_use]
    pub fn reconstruct(&self) -> SimplicialComplex {
        SimplicialComplex::from_facets(&self.facets)
    }
}

impl PartialOrd for SimplicialComplexKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SimplicialComplexKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.facets.cmp(&other.facets)
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
pub struct SimplicialComplexIdentityRecord {
    pub record_version: u32,
    pub key: SimplicialComplexKey,
    pub vertices: usize,
    pub dimension: i64,
    pub facets: Vec<Vec<u32>>,
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
pub struct DominatedVertexRecord {
    pub vertex: u32,
    pub dominator: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ElementaryCollapseRecord {
    pub free_face: Vec<u32>,
    pub coface: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StrongCoreRelationRecord {
    pub target: SimplicialComplexKey,
    pub source_vertices: usize,
    pub target_vertices: usize,
    pub steps: Vec<DominatedVertexRecord>,
    pub reduction_json: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GreedyCollapseRelationRecord {
    pub target: SimplicialComplexKey,
    pub steps: Vec<ElementaryCollapseRecord>,
    pub sequence_json: String,
    pub terminal: bool,
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimplicialComplexTopologyRecord {
    pub record_version: u32,
    pub complex: SimplicialComplexKey,
    pub f_vector: Vec<usize>,
    pub component_sizes: Vec<usize>,
    pub euler_characteristic: i64,
    pub integral_homology: Vec<IntegralHomologyRecord>,
    pub strong_core: StrongCoreRelationRecord,
    pub greedy_collapse: GreedyCollapseRelationRecord,
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
    ExactThroughDegree {
        degree: u32,
        no_claim_above_degree: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimplicialComplexCohomologyRecord {
    pub record_version: u32,
    pub complex: SimplicialComplexKey,
    pub coefficient_context: FiniteFieldContextRecord,
    pub computed_on_strong_core: SimplicialComplexKey,
    pub basis: Vec<CohomologyBasisRecord>,
    pub products: Vec<CohomologyProductRecord>,
    pub operations: Vec<CohomologyOperationRecord>,
    pub connected_presentation_json: Option<String>,
    pub extent: RingExtentRecord,
}

impl SimplicialComplexIdentityRecord {
    pub fn from_canonical(complex: &SimplicialComplex) -> Result<Self> {
        let canonical = complex.canonical_form()?;
        ensure!(
            &canonical == complex,
            "identity records require a canonical complex"
        );
        let key = SimplicialComplexKey::from_canonical(complex);
        let automorphisms = complex.automorphism_group()?;
        let record = Self {
            record_version: RECORD_VERSION,
            key,
            vertices: complex.count(0),
            dimension: complex.dimension(),
            facets: complex
                .facets()
                .into_iter()
                .map(|facet| facet.vertices().to_vec())
                .collect(),
            canonical_bytes_sha256: hex::encode(Sha256::digest(complex.canonical_bytes()?)),
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
        ensure!(self.key.facets() == self.facets, "key/facet mismatch");
        let complex = self.key.reconstruct();
        ensure!(complex.count(0) == self.vertices, "wrong vertex count");
        ensure!(complex.dimension() == self.dimension, "wrong dimension");
        ensure!(
            complex.canonical_form()? == complex,
            "record is not in canonical form"
        );
        ensure!(
            self.automorphism_group.degree == self.vertices,
            "wrong automorphism degree"
        );
        ensure!(
            self.automorphism_group.generators_verified,
            "unverified automorphism generators"
        );
        ensure!(
            hex::encode(Sha256::digest(complex.canonical_bytes()?)) == self.canonical_bytes_sha256,
            "canonical byte digest mismatch"
        );
        Ok(())
    }
}
