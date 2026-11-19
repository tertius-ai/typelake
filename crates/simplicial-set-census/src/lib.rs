//! Certified, database-independent finite simplicial-set census.

pub mod enrich;
pub mod enumerate;
pub mod jsonl;
pub mod records;
pub mod validate;

use std::collections::BTreeMap;

use anyhow::Result;
use noether::topology::SimplicialSet;

use crate::{
    enumerate::EnumerationStats,
    records::{SimplicialSetIdentityRecord, SimplicialSetKey},
};

pub const CONTRACT: &str = "finite-simplicial-sets/v1";
pub const TOPOLOGY_CONTRACT: &str = "finite-simplicial-sets/topology-v2";
pub const COHOMOLOGY_CONTRACT: &str = "finite-simplicial-sets/cohomology-v1";
pub const STEENROD_OPERATIONS_CONTRACT: &str = "finite-simplicial-sets/steenrod-operations-v1";
pub const HOMOTOPY_CONTRACT: &str = "finite-simplicial-sets/homotopy-v1";
pub const RECORD_VERSION: u32 = 1;
pub const NOETHER_REVISION: &str = "eebc33d4f37ed47af83c88dbc9f8886b254f72c6";
/// Historical producer before the public 0.3.2 source import.
pub const PRERELEASE_NOETHER_REVISION: &str = "4c92c269c59da02a4d32c833c2ee093407f8e22f";
/// Producer of the immutable first-edition artifacts.
pub const LEGACY_NOETHER_REVISION: &str = "993dd0ff53ca7cf80e52a5d2f96b1659bce0b2a8";
pub const IDENTITY_NOETHER_REVISION: &str = "6c05804149d25305d8329f4e56afa83d7dc691fa";
pub const TOPOLOGY_NOETHER_REVISION: &str = "8911e74a15f9df93c65ad0074232db384ef83f91";
pub const COHOMOLOGY_NOETHER_REVISION: &str = "5ab3080d627b7b9633a085df8556c147b652c4ab";
pub const HOMOTOPY_NOETHER_REVISION: &str = "3603f10148fc609475c5bdc1641e9f1b88701c2b";

#[derive(Debug)]
pub struct IdentityCensus {
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub candidate_cap: u64,
    pub stats: EnumerationStats,
    pub records: Vec<SimplicialSetIdentityRecord>,
}

#[derive(Debug)]
pub struct TopologyCensus {
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub records: Vec<records::SimplicialSetTopologyRecord>,
}

#[derive(Debug)]
pub struct CohomologyCensus {
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub prime: u64,
    pub records: Vec<records::SimplicialSetCohomologyRecord>,
}

#[derive(Debug)]
pub struct HomotopyCensus {
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub records: Vec<records::SimplicialSetHomotopyRecord>,
}

#[derive(Debug)]
pub struct SteenrodOperationsCensus {
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub prime: u64,
    pub records: Vec<records::SimplicialSetSteenrodOperationsRecord>,
}

pub fn identity_census(
    total_nondegenerate: usize,
    maximum_dimension: usize,
    candidate_cap: u64,
    stats: EnumerationStats,
    representatives: BTreeMap<SimplicialSetKey, SimplicialSet>,
) -> Result<IdentityCensus> {
    let records = representatives
        .into_iter()
        .map(|(expected, value)| {
            let record = SimplicialSetIdentityRecord::from_canonical(&value)?;
            anyhow::ensure!(record.key == expected, "enrichment changed canonical key");
            Ok(record)
        })
        .collect::<Result<Vec<_>>>()?;
    anyhow::ensure!(stats.accepted == records.len(), "statistics/count mismatch");
    Ok(IdentityCensus {
        total_nondegenerate,
        maximum_dimension,
        candidate_cap,
        stats,
        records,
    })
}
