//! Certified, database-independent finite strong-Steiner-category census.

pub mod enrich;
pub mod enumerate;
pub mod jsonl;
pub mod mapping_model;
pub mod records;
pub mod validate;

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail, ensure};
use noether::category::SteinerCanonicalLabel;

use crate::{
    enrich::{
        deduplicate_homotopy_posets, enrich_category, gray_product_record, identity_record,
        light_enrichment_record, rich_enrichment_record,
    },
    enumerate::EnumerationStats,
    records::{
        BasepointOrbitRecord, CategoricalHomologyRecord, CategoricalHomotopyRecord,
        GrayProductRecord, HomotopyPosetRecord, MappingCategoryRecord, SteinerCategoryRecord,
        SteinerIdentityRecord, SteinerKey, SteinerLightEnrichmentRecord,
        SteinerMappingEnrichmentRecord, SteinerRichEnrichmentRecord,
    },
};

pub const CONTRACT: &str = "strong-steiner-categories/v1";
pub const IDENTITY_CONTRACT: &str = "strong-steiner-categories/identities/v1";
pub const LIGHT_ENRICHMENT_CONTRACT: &str = "strong-steiner-categories/light-enrichments/v1";
pub const MAPPING_ENRICHMENT_CONTRACT: &str = "strong-steiner-categories/mapping-enrichments/v1";
pub const RICH_ENRICHMENT_CONTRACT: &str = "strong-steiner-categories/rich-enrichments/v1";
pub const RECORD_VERSION: u32 = 1;
pub const CANONICAL_ENCODING: &str = "steiner-boundaries/v1";
pub const DEFAULT_STREET_DEGREE: usize = 2;
pub const CATEGORICAL_HOMOTOPY_RECORD_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Generator {
    Reference,
    CellAttachment,
}

impl Generator {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Reference => "signed-boundary-matrices/v1",
            Self::CellAttachment => "canonical-cell-attachment/v1",
        }
    }
}

#[derive(Debug)]
pub struct Census {
    pub generator: Generator,
    pub atomic_size: usize,
    pub candidate_cap: u64,
    pub raw_candidate_count: u64,
    pub categories: Vec<SteinerCategoryRecord>,
    pub mappings: Vec<MappingCategoryRecord>,
    pub basepoints: Vec<BasepointOrbitRecord>,
    pub homotopy_posets: Vec<HomotopyPosetRecord>,
    pub categorical_homology: Vec<CategoricalHomologyRecord>,
    pub categorical_homotopy: Vec<CategoricalHomotopyRecord>,
}

#[derive(Debug)]
pub struct IdentityCensus {
    pub generator: Generator,
    pub atomic_size: usize,
    pub candidate_cap: u64,
    pub stats: EnumerationStats,
    pub identities: Vec<SteinerIdentityRecord>,
}

#[derive(Debug)]
pub struct LightCensus {
    pub atomic_size: usize,
    pub records: Vec<SteinerLightEnrichmentRecord>,
}

#[derive(Debug)]
pub struct MappingCensus {
    pub atomic_size: usize,
    pub records: Vec<SteinerMappingEnrichmentRecord>,
    pub mappings: Vec<MappingCategoryRecord>,
}

#[derive(Debug)]
pub struct RichCensus {
    pub atomic_size: usize,
    pub records: Vec<SteinerRichEnrichmentRecord>,
    pub mappings: Vec<MappingCategoryRecord>,
    pub basepoints: Vec<BasepointOrbitRecord>,
    pub homotopy_posets: Vec<HomotopyPosetRecord>,
    pub categorical_homology: Vec<CategoricalHomologyRecord>,
    pub categorical_homotopy: Vec<CategoricalHomotopyRecord>,
}

pub fn identity_census(
    generator: Generator,
    atomic_size: usize,
    candidate_cap: u64,
    stats: EnumerationStats,
    representatives: BTreeMap<SteinerKey, SteinerCanonicalLabel>,
) -> Result<IdentityCensus> {
    ensure!(
        stats.accepted == representatives.len(),
        "enumeration statistics disagree with the representative count"
    );
    let identities = representatives
        .into_iter()
        .map(|(expected_key, representative)| {
            let record = identity_record(&representative)?;
            ensure!(
                record.key == expected_key,
                "identity construction changed canonical key {expected_key:?}"
            );
            Ok(record)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(IdentityCensus {
        generator,
        atomic_size,
        candidate_cap,
        stats,
        identities,
    })
}

pub fn light_census(
    atomic_size: usize,
    identities: &[SteinerIdentityRecord],
) -> Result<LightCensus> {
    let mut records = Vec::with_capacity(identities.len());
    for identity in identities {
        let category = identity.reconstruct()?;
        let canonical = category.canonical_label()?;
        ensure!(
            canonical.category() == &category
                && SteinerKey::from_canonical_label(&canonical) == identity.key,
            "identity is not in its claimed canonical form"
        );
        records.push(light_enrichment_record(&canonical)?);
    }
    Ok(LightCensus {
        atomic_size,
        records,
    })
}

pub fn mapping_census(
    atomic_size: usize,
    identities: &[SteinerIdentityRecord],
) -> Result<MappingCensus> {
    let mut records = Vec::with_capacity(identities.len());
    let mut mappings = Vec::new();
    for identity in identities {
        let category = identity.reconstruct()?;
        let canonical = category.canonical_label()?;
        ensure!(
            canonical.category() == &category
                && SteinerKey::from_canonical_label(&canonical) == identity.key,
            "identity is not in its claimed canonical form"
        );
        let record = SteinerMappingEnrichmentRecord {
            record_version: RECORD_VERSION,
            category: identity.key.clone(),
            exact_cell_counts: category.exact_cell_counts()?,
        };
        record.check_shape()?;
        mappings.extend(crate::enrich::mapping_records(&identity.key, &category)?);
        records.push(record);
    }
    Ok(MappingCensus {
        atomic_size,
        records,
        mappings,
    })
}

pub fn rich_census(
    atomic_size: usize,
    identities: &[SteinerIdentityRecord],
    lights: &[SteinerLightEnrichmentRecord],
    street_degree: usize,
) -> Result<RichCensus> {
    ensure!(
        identities.len() == lights.len(),
        "light enrichment is incomplete for the identity shell"
    );
    let light_by_key = lights
        .iter()
        .map(|record| (record.category.clone(), record))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        light_by_key.len() == lights.len(),
        "duplicate light-enrichment key"
    );
    let mut enriched = Vec::with_capacity(identities.len());
    for identity in identities {
        light_by_key
            .get(&identity.key)
            .context("identity has no light enrichment")?;
        let category = identity.reconstruct()?;
        let canonical = category.canonical_label()?;
        ensure!(
            canonical.category() == &category
                && SteinerKey::from_canonical_label(&canonical) == identity.key,
            "identity is not in its claimed canonical form"
        );
        enriched.push(rich_enrichment_record(&canonical, street_degree)?);
    }
    let mut homotopy_posets = BTreeMap::new();
    for record in enriched.iter().flat_map(|record| &record.homotopy_posets) {
        if let Some(existing) = homotopy_posets.insert(record.key.clone(), record.clone())
            && existing != *record
        {
            bail!("homotopy Poset key collision at {:?}", record.key);
        }
    }
    let mut records = Vec::with_capacity(enriched.len());
    let mut mappings = Vec::new();
    let mut basepoints = Vec::new();
    let mut categorical_homology = Vec::with_capacity(enriched.len());
    let mut categorical_homotopy = Vec::with_capacity(enriched.len());
    for record in enriched {
        records.push(record.rich);
        mappings.extend(record.mappings);
        basepoints.extend(record.basepoints);
        categorical_homology.push(record.categorical_homology);
        categorical_homotopy.push(record.categorical_homotopy);
    }
    Ok(RichCensus {
        atomic_size,
        records,
        mappings,
        basepoints,
        homotopy_posets: homotopy_posets.into_values().collect(),
        categorical_homology,
        categorical_homotopy,
    })
}

pub fn assemble_census(
    generator: Generator,
    atomic_size: usize,
    candidate_cap: u64,
    raw_candidate_count: u64,
    identities: &[SteinerIdentityRecord],
    lights: &[SteinerLightEnrichmentRecord],
    rich: &RichCensus,
) -> Result<Census> {
    ensure!(
        rich.atomic_size == atomic_size,
        "rich enrichment lies in the wrong shell"
    );
    let light_by_key = lights
        .iter()
        .map(|record| (record.category.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let rich_by_key = rich
        .records
        .iter()
        .map(|record| (record.category.clone(), record))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        light_by_key.len() == identities.len() && rich_by_key.len() == identities.len(),
        "enrichment layers are incomplete"
    );
    let categories = identities
        .iter()
        .map(|identity| {
            SteinerCategoryRecord::from_layers(
                identity,
                light_by_key
                    .get(&identity.key)
                    .context("missing light enrichment")?,
                rich_by_key
                    .get(&identity.key)
                    .context("missing rich enrichment")?,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Census {
        generator,
        atomic_size,
        candidate_cap,
        raw_candidate_count,
        categories,
        mappings: rich.mappings.clone(),
        basepoints: rich.basepoints.clone(),
        homotopy_posets: rich.homotopy_posets.clone(),
        categorical_homology: rich.categorical_homology.clone(),
        categorical_homotopy: rich.categorical_homotopy.clone(),
    })
}

pub fn enrich_census(
    generator: Generator,
    atomic_size: usize,
    candidate_cap: u64,
    raw_candidate_count: u64,
    representatives: BTreeMap<SteinerKey, SteinerCanonicalLabel>,
    street_degree: usize,
) -> Result<Census> {
    let mut enriched = Vec::with_capacity(representatives.len());
    for (expected_key, representative) in representatives {
        let record = enrich_category(&representative, street_degree)?;
        if record.category.key != expected_key {
            bail!("enrichment changed canonical key {expected_key:?}");
        }
        enriched.push(record);
    }
    let homotopy_posets = deduplicate_homotopy_posets(&enriched)?;
    let mut categories = Vec::with_capacity(enriched.len());
    let mut mappings = Vec::new();
    let mut basepoints = Vec::new();
    let mut categorical_homology = Vec::with_capacity(enriched.len());
    let mut categorical_homotopy = Vec::with_capacity(enriched.len());
    for record in enriched {
        categories.push(record.category);
        mappings.extend(record.mappings);
        basepoints.extend(record.basepoints);
        categorical_homology.push(record.categorical_homology);
        categorical_homotopy.push(record.categorical_homotopy);
    }
    Ok(Census {
        generator,
        atomic_size,
        candidate_cap,
        raw_candidate_count,
        categories,
        mappings,
        basepoints,
        homotopy_posets,
        categorical_homology,
        categorical_homotopy,
    })
}

pub fn gray_table(
    representatives: &BTreeMap<SteinerKey, SteinerCanonicalLabel>,
) -> Result<Vec<GrayProductRecord>> {
    let mut output = Vec::new();
    for left in representatives.values() {
        for right in representatives.values() {
            output.push(gray_product_record(left, right)?);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layered_pipeline_reassembles_the_monolithic_record() {
        let atomic_size = 3;
        let candidate_cap = 100_000;
        let (stats, representatives) =
            crate::enumerate::orderly_representatives(atomic_size, candidate_cap).unwrap();
        let expected = enrich_census(
            Generator::CellAttachment,
            atomic_size,
            candidate_cap,
            stats.raw_candidates,
            representatives.clone(),
            2,
        )
        .unwrap();
        let identities = identity_census(
            Generator::CellAttachment,
            atomic_size,
            candidate_cap,
            stats.clone(),
            representatives,
        )
        .unwrap();
        crate::validate::validate_identity_census(&identities).unwrap();
        let light = light_census(atomic_size, &identities.identities).unwrap();
        crate::validate::validate_light_census(&light, &identities.identities).unwrap();
        let rich = rich_census(atomic_size, &identities.identities, &light.records, 2).unwrap();
        let assembled = assemble_census(
            Generator::CellAttachment,
            atomic_size,
            candidate_cap,
            stats.raw_candidates,
            &identities.identities,
            &light.records,
            &rich,
        )
        .unwrap();
        crate::validate::validate_census(&assembled).unwrap();
        assert_eq!(assembled.categories, expected.categories);
        assert_eq!(assembled.mappings, expected.mappings);
        assert_eq!(assembled.basepoints, expected.basepoints);
        assert_eq!(assembled.homotopy_posets, expected.homotopy_posets);
        assert_eq!(
            assembled.categorical_homology,
            expected.categorical_homology
        );
        assert_eq!(
            assembled.categorical_homotopy,
            expected.categorical_homotopy
        );
    }
}
