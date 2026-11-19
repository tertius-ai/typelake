//! Database-independent finite-poset census construction.

pub mod enrich;
pub mod enumerate;
pub mod jsonl;
pub mod records;
pub mod shard;
pub mod validate;

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use noether::combin::PosetCanonicalLabel;

use crate::{
    enrich::enrich_poset_with_core_cache, records::ComplexRecord, records::PosetKey,
    records::PosetRecord,
};

/// The record contract implemented by this builder.
pub const CONTRACT: &str = "finite-posets/v1";
/// The exact Noether source used by the builder.
pub const NOETHER_REVISION: &str = "eebc33d4f37ed47af83c88dbc9f8886b254f72c6";
/// Historical producer before the public 0.3.2 source import.
pub const PRERELEASE_NOETHER_REVISION: &str = "4c92c269c59da02a4d32c833c2ee093407f8e22f";
/// Producer of the immutable first-edition artifacts.
pub const LEGACY_NOETHER_REVISION: &str = "253d73734b217fad6fc5839704b2f5688672bb55";
/// The current on-disk record version.
pub const RECORD_VERSION: u32 = 1;

/// Complete generator used to obtain the canonical representatives.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Generator {
    Reference,
    HereditaryOrderly,
}

impl Generator {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Reference => "fixed-linear-extension-subsets/v1",
            Self::HereditaryOrderly => "hereditary-downset-orderly/v1",
        }
    }

    #[must_use]
    pub const fn command(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::HereditaryOrderly => "orderly",
        }
    }
}

/// Fully enriched in-memory records for one order.
#[derive(Debug)]
pub struct Census {
    pub generator: Generator,
    pub order: usize,
    pub candidate_cap: u64,
    pub raw_candidate_count: u64,
    pub posets: Vec<PosetRecord>,
    pub complexes: Vec<ComplexRecord>,
}

/// Enrich canonical representatives and deduplicate their linked complexes.
pub fn enrich_census(
    order: usize,
    candidate_cap: u64,
    raw_candidate_count: u64,
    representatives: BTreeMap<PosetKey, PosetCanonicalLabel>,
) -> Result<Census> {
    enrich_census_with_generator(
        Generator::Reference,
        order,
        candidate_cap,
        raw_candidate_count,
        representatives,
    )
}

/// Enrich representatives while recording the complete generator that made them.
pub fn enrich_census_with_generator(
    generator: Generator,
    order: usize,
    candidate_cap: u64,
    raw_candidate_count: u64,
    representatives: BTreeMap<PosetKey, PosetCanonicalLabel>,
) -> Result<Census> {
    let mut posets = Vec::with_capacity(representatives.len());
    let mut complexes = BTreeMap::new();
    let mut core_homology = BTreeMap::new();

    for (expected_key, canonical) in representatives {
        let (record, complex) = enrich_poset_with_core_cache(&canonical, &mut core_homology)?;
        if record.key != expected_key {
            bail!(
                "enrichment changed canonical key {expected_key:?} to {:?}",
                record.key
            );
        }
        if let Some(existing) = complexes.insert(complex.key.clone(), complex.clone())
            && existing != complex
        {
            bail!("order-complex digest collision at {}", complex.key);
        }
        posets.push(record);
    }

    Ok(Census {
        generator,
        order,
        candidate_cap,
        raw_candidate_count,
        posets,
        complexes: complexes.into_values().collect(),
    })
}

/// Exact source identity of the producer, including uncommitted source bytes.
#[derive(Clone, Debug, serde::Serialize)]
pub struct SourceIdentity {
    pub revision: &'static str,
    pub source_sha256: &'static str,
    pub dirty: bool,
}

/// Source identity embedded at compile time, independent of the runtime checkout.
pub fn source_identity() -> SourceIdentity {
    SourceIdentity {
        revision: env!("POSET_CENSUS_REVISION"),
        source_sha256: env!("TYPELAKE_SOURCE_SHA256"),
        dirty: env!("POSET_CENSUS_DIRTY") == "true",
    }
}
