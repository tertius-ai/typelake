//! Certified, database-independent finite simplicial-complex census.

pub mod enrich;
pub mod enumerate;
pub mod jsonl;
pub mod records;
pub mod validate;

use anyhow::Result;

use crate::{
    enumerate::EnumerationStats,
    records::{SimplicialComplexIdentityRecord, SimplicialComplexKey},
};

pub const CONTRACT: &str = "finite-simplicial-complexes/v1";
pub const COHOMOLOGY_CONTRACT: &str = "finite-simplicial-complexes/cohomology-v2";
pub const RECORD_VERSION: u32 = 1;
/// Noether commit whose supplied-object surface this contract requires.
pub const NOETHER_REVISION: &str = "eebc33d4f37ed47af83c88dbc9f8886b254f72c6";
/// Historical producer before the public 0.3.2 source import.
pub const PRERELEASE_NOETHER_REVISION: &str = "4c92c269c59da02a4d32c833c2ee093407f8e22f";
/// Producer of the immutable first-edition artifacts.
pub const LEGACY_NOETHER_REVISION: &str = "d2834a6b28c532100ad821b78d5eb74a82be32ec";
/// Noether commit used to construct the installed first-edition identity and
/// topology artifacts. Loaders accept both this revision and the current one.
pub const FIRST_EDITION_NOETHER_REVISION: &str = "6c05804149d25305d8329f4e56afa83d7dc691fa";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Generator {
    Reference,
    CanonicalVertexLink,
}

impl Generator {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Reference => "labeled-downward-closed-subsets/v1",
            Self::CanonicalVertexLink => "canonical-vertex-link/v1",
        }
    }
}

#[derive(Debug)]
pub struct IdentityCensus {
    pub generator: Generator,
    pub vertices: usize,
    pub candidate_cap: u64,
    pub stats: EnumerationStats,
    pub records: Vec<SimplicialComplexIdentityRecord>,
}

#[derive(Debug)]
pub struct TopologyCensus {
    pub vertices: usize,
    pub records: Vec<records::SimplicialComplexTopologyRecord>,
}

#[derive(Debug)]
pub struct CohomologyCensus {
    pub vertices: usize,
    pub prime: u64,
    pub records: Vec<records::SimplicialComplexCohomologyRecord>,
}

pub fn identity_census(
    generator: Generator,
    vertices: usize,
    candidate_cap: u64,
    stats: EnumerationStats,
    representatives: std::collections::BTreeMap<
        SimplicialComplexKey,
        noether::topology::SimplicialComplex,
    >,
) -> Result<IdentityCensus> {
    let records = representatives
        .into_iter()
        .map(|(expected, complex)| {
            let record = SimplicialComplexIdentityRecord::from_canonical(&complex)?;
            anyhow::ensure!(record.key == expected, "enrichment changed canonical key");
            Ok(record)
        })
        .collect::<Result<Vec<_>>>()?;
    anyhow::ensure!(stats.accepted == records.len(), "statistics/count mismatch");
    Ok(IdentityCensus {
        generator,
        vertices,
        candidate_cap,
        stats,
        records,
    })
}
