use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use noether::category::{OrientedBasepoint, SignedBoundary, SteinerCategory, SteinerCell};
use serde::{Deserialize, Serialize};

use crate::{
    CATEGORICAL_HOMOTOPY_RECORD_VERSION, Census, IDENTITY_CONTRACT, IdentityCensus,
    LIGHT_ENRICHMENT_CONTRACT, LightCensus, MAPPING_ENRICHMENT_CONTRACT, MappingCensus,
    RECORD_VERSION,
    enrich::light_enrichment_record,
    records::{
        BasepointOrbitRecord, CategoricalHomologyRecord, CategoricalHomotopyRecord, CellRecord,
        HomotopyPosetRecord, MappingCategoryRecord, SteinerCategoryRecord, SteinerIdentityRecord,
        SteinerKey, poset_key,
    },
};

const INDEPENDENT_RECURSIVE_MAXIMUM_ATOMIC_SIZE: usize = 11;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IdentityValidationReport {
    pub contract: String,
    pub status: String,
    pub record_version: u32,
    pub atomic_size: usize,
    pub category_count: usize,
    pub records_reconstructed: bool,
    pub canonical_keys_verified: bool,
    pub accepted_count_verified: bool,
    pub candidate_accounting_verified: bool,
    pub completeness_method: String,
    pub independent_generator: Option<String>,
    pub independent_generator_keys_verified: Option<bool>,
    pub isomorphism_scope: IsomorphismScope,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnrichmentValidationReport {
    pub contract: String,
    pub status: String,
    pub record_version: u32,
    pub atomic_size: usize,
    pub identity_count: usize,
    pub enrichment_count: usize,
    pub upstream_identities_verified: bool,
    pub records_recomputed: bool,
    pub coverage_status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MappingValidationReport {
    pub contract: String,
    pub status: String,
    pub record_version: u32,
    pub atomic_size: usize,
    pub identity_count: usize,
    pub enrichment_count: usize,
    pub mapping_count: usize,
    pub upstream_identities_verified: bool,
    pub exact_cells_recomputed: bool,
    pub mappings_recomputed: bool,
    pub coverage_status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationReport {
    pub contract: String,
    pub status: String,
    pub record_version: u32,
    pub atomic_size: usize,
    pub category_count: usize,
    pub mapping_count: usize,
    pub basepoint_orbit_count: usize,
    pub homotopy_poset_count: usize,
    pub categorical_homology_count: usize,
    pub categorical_homotopy_count: usize,
    pub records_reconstructed: bool,
    pub canonical_keys_verified: bool,
    pub sidecars_verified: bool,
    pub accepted_count_verified: bool,
    pub independent_generator_keys_verified: bool,
    pub isomorphism_scope: IsomorphismScope,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IsomorphismScope {
    pub relation: String,
    pub status: String,
}

pub fn validate_identity_census(census: &IdentityCensus) -> Result<IdentityValidationReport> {
    let mut categories = BTreeMap::new();
    for record in &census.identities {
        record.check_shape()?;
        ensure!(
            record.atom_counts.iter().sum::<usize>() == census.atomic_size,
            "identity lies in the wrong atomic shell"
        );
        let category = record.reconstruct()?;
        let label = category.canonical_label()?;
        ensure!(
            label.category() == &category,
            "stored identity is not canonical"
        );
        ensure!(
            SteinerKey::from_canonical_label(&label) == record.key,
            "stored identity key does not match its canonical encoding"
        );
        if categories.insert(record.key.clone(), category).is_some() {
            bail!("duplicate identity key {:?}", record.key);
        }
    }
    let expected_count = accepted_count(census.atomic_size)?;
    ensure!(
        census.identities.len() as u64 == expected_count,
        "identity count disagrees with the accepted complete count"
    );
    ensure!(
        census.stats.accepted == census.identities.len()
            && census.stats.raw_candidates <= census.candidate_cap
            && census.stats.canonical_tests <= census.stats.orbit_representatives,
        "identity-generation accounting is inconsistent"
    );
    let (independent_generator, independent_generator_keys_verified) =
        if census.atomic_size <= INDEPENDENT_RECURSIVE_MAXIMUM_ATOMIC_SIZE {
            let (_, reference) = crate::enumerate::recursive_reference_representatives(
                census.atomic_size,
                census.candidate_cap,
            )?;
            ensure!(
                reference.keys().eq(categories.keys()),
                "independent generators disagree on canonical keys"
            );
            (Some("unpruned-cell-attachment/v1".to_owned()), Some(true))
        } else {
            (None, None)
        };
    Ok(IdentityValidationReport {
        contract: IDENTITY_CONTRACT.to_owned(),
        status: "passed".to_owned(),
        record_version: RECORD_VERSION,
        atomic_size: census.atomic_size,
        category_count: census.identities.len(),
        records_reconstructed: true,
        canonical_keys_verified: true,
        accepted_count_verified: true,
        candidate_accounting_verified: true,
        completeness_method: "hereditary-canonical-parent-exhaustion/v1".to_owned(),
        independent_generator,
        independent_generator_keys_verified,
        isomorphism_scope: IsomorphismScope {
            relation: "strict-isomorphism=categorical-equivalence=folk-equivalence".to_owned(),
            status: "complete".to_owned(),
        },
    })
}

pub fn validate_light_census(
    census: &LightCensus,
    identities: &[SteinerIdentityRecord],
) -> Result<EnrichmentValidationReport> {
    ensure!(
        census.records.len() == identities.len(),
        "light enrichment is incomplete"
    );
    let identity_by_key = identities
        .iter()
        .map(|record| (record.key.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let light_by_key = census
        .records
        .iter()
        .map(|record| (record.category.clone(), record))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        identity_by_key.len() == identities.len() && light_by_key.len() == census.records.len(),
        "duplicate identity or light-enrichment key"
    );
    ensure!(
        identity_by_key.keys().eq(light_by_key.keys()),
        "light-enrichment keys do not equal the identity shell"
    );
    for (key, identity) in identity_by_key {
        let category = identity.reconstruct()?;
        let canonical = category.canonical_label()?;
        let expected = light_enrichment_record(&canonical)?;
        ensure!(
            light_by_key.get(&key).copied() == Some(&expected),
            "light enrichment does not reconstruct for {key:?}"
        );
    }
    Ok(EnrichmentValidationReport {
        contract: LIGHT_ENRICHMENT_CONTRACT.to_owned(),
        status: "passed".to_owned(),
        record_version: RECORD_VERSION,
        atomic_size: census.atomic_size,
        identity_count: identities.len(),
        enrichment_count: census.records.len(),
        upstream_identities_verified: true,
        records_recomputed: true,
        coverage_status: "complete".to_owned(),
    })
}

pub fn validate_mapping_census(
    census: &MappingCensus,
    identities: &[SteinerIdentityRecord],
) -> Result<MappingValidationReport> {
    ensure!(
        census.records.len() == identities.len(),
        "mapping enrichment is incomplete"
    );
    let identity_by_key = identities
        .iter()
        .map(|record| (record.key.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let record_by_key = census
        .records
        .iter()
        .map(|record| (record.category.clone(), record))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        identity_by_key.len() == identities.len()
            && record_by_key.len() == census.records.len()
            && identity_by_key.keys().eq(record_by_key.keys()),
        "mapping-enrichment keys do not equal the identity shell"
    );
    let mut categories = BTreeMap::new();
    for (key, identity) in &identity_by_key {
        let category = identity.reconstruct()?;
        let canonical = category.canonical_label()?;
        ensure!(
            canonical.category() == &category
                && SteinerKey::from_canonical_label(&canonical) == *key,
            "mapping enrichment references a noncanonical identity"
        );
        let record = record_by_key[key];
        record.check_shape()?;
        ensure!(
            record.exact_cell_counts == category.exact_cell_counts()?,
            "exact-cell counts do not reconstruct for {key:?}"
        );
        categories.insert((*key).clone(), category);
    }
    let expected_mappings = identities
        .iter()
        .map(|identity| identity.atom_counts.first().copied().unwrap_or(0).pow(2))
        .sum::<usize>();
    ensure!(
        census.mappings.len() == expected_mappings,
        "mapping sidecar is incomplete"
    );
    for mapping in &census.mappings {
        validate_mapping(mapping, &categories)?;
    }
    Ok(MappingValidationReport {
        contract: MAPPING_ENRICHMENT_CONTRACT.to_owned(),
        status: "passed".to_owned(),
        record_version: RECORD_VERSION,
        atomic_size: census.atomic_size,
        identity_count: identities.len(),
        enrichment_count: census.records.len(),
        mapping_count: census.mappings.len(),
        upstream_identities_verified: true,
        exact_cells_recomputed: true,
        mappings_recomputed: true,
        coverage_status: "complete".to_owned(),
    })
}

fn accepted_count(atomic_size: usize) -> Result<u64> {
    let accepted: serde_json::Value = serde_json::from_str(include_str!(
        "../../../collections/strong-steiner-categories/validation/counts.json"
    ))?;
    accepted["counts"][atomic_size.to_string()]
        .as_u64()
        .context("accepted counts do not cover this atomic shell")
}

pub fn validate_census(census: &Census) -> Result<ValidationReport> {
    let mut categories = BTreeMap::new();
    for record in &census.categories {
        record.check_shape()?;
        ensure!(
            record.invariants.atomic_size == census.atomic_size,
            "category lies in the wrong atomic shell"
        );
        let category = reconstruct_category(record)?;
        let label = category.canonical_label()?;
        ensure!(
            label.category() == &category,
            "stored category is not canonical"
        );
        ensure!(
            SteinerKey::from_canonical_label(&label) == record.key,
            "stored category key does not match its canonical encoding"
        );
        if categories.insert(record.key.clone(), category).is_some() {
            bail!("duplicate category key {:?}", record.key);
        }
    }

    let expected_mappings = census
        .categories
        .iter()
        .map(|record| record.invariants.object_count.pow(2))
        .sum::<usize>();
    ensure!(
        census.mappings.len() == expected_mappings,
        "mapping sidecar is incomplete"
    );
    for mapping in &census.mappings {
        validate_mapping(mapping, &categories)?;
    }

    let posets = census
        .homotopy_posets
        .iter()
        .map(|record| (record.key.clone(), record))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        posets.len() == census.homotopy_posets.len(),
        "duplicate homotopy Poset key"
    );
    for record in &census.homotopy_posets {
        validate_homotopy_poset(record)?;
    }
    validate_basepoints(&census.basepoints, &categories, &posets)?;
    validate_categorical_homology(&census.categorical_homology, &categories)?;
    validate_categorical_homotopy(&census.categorical_homotopy, &categories)?;

    let expected_count = accepted_count(census.atomic_size)?;
    ensure!(
        census.categories.len() as u64 == expected_count,
        "category count disagrees with the accepted complete count"
    );
    let independent_generator_keys_verified =
        if census.atomic_size <= INDEPENDENT_RECURSIVE_MAXIMUM_ATOMIC_SIZE {
            let (_, reference) = crate::enumerate::recursive_reference_representatives(
                census.atomic_size,
                census.candidate_cap,
            )?;
            ensure!(
                reference.keys().eq(categories.keys()),
                "independent generators disagree on canonical keys"
            );
            true
        } else {
            false
        };

    Ok(ValidationReport {
        contract: crate::CONTRACT.to_owned(),
        status: "passed".to_owned(),
        record_version: RECORD_VERSION,
        atomic_size: census.atomic_size,
        category_count: census.categories.len(),
        mapping_count: census.mappings.len(),
        basepoint_orbit_count: census.basepoints.len(),
        homotopy_poset_count: census.homotopy_posets.len(),
        categorical_homology_count: census.categorical_homology.len(),
        categorical_homotopy_count: census.categorical_homotopy.len(),
        records_reconstructed: true,
        canonical_keys_verified: true,
        sidecars_verified: true,
        accepted_count_verified: true,
        independent_generator_keys_verified,
        isomorphism_scope: IsomorphismScope {
            relation: "strict-isomorphism=categorical-equivalence=folk-equivalence".to_owned(),
            status: "complete".to_owned(),
        },
    })
}

pub fn reconstruct_category(record: &SteinerCategoryRecord) -> Result<SteinerCategory> {
    let boundaries = record
        .boundaries
        .iter()
        .map(|degree| {
            degree
                .iter()
                .map(|boundary| {
                    SignedBoundary::new(
                        &boundary
                            .negative
                            .iter()
                            .map(|&atom| atom as usize)
                            .collect::<Vec<_>>(),
                        &boundary
                            .positive
                            .iter()
                            .map(|&atom| atom as usize)
                            .collect::<Vec<_>>(),
                    )
                })
                .collect()
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SteinerCategory::from_boundaries(
        record.atom_counts.clone(),
        boundaries,
    )?)
}

fn reconstruct_cell(category: &SteinerCategory, record: &CellRecord) -> Result<SteinerCell> {
    ensure!(
        record.negative_supports.len() == record.dimension + 1
            && record.positive_supports.len() == record.dimension + 1,
        "cell-record dimension does not match support lengths"
    );
    Ok(category.cell_from_supports(
        record
            .negative_supports
            .iter()
            .map(|support| support.iter().map(|&atom| atom as usize).collect())
            .collect(),
        record
            .positive_supports
            .iter()
            .map(|support| support.iter().map(|&atom| atom as usize).collect())
            .collect(),
    )?)
}

fn validate_mapping(
    record: &MappingCategoryRecord,
    categories: &BTreeMap<SteinerKey, SteinerCategory>,
) -> Result<()> {
    ensure!(
        record.record_version == RECORD_VERSION,
        "wrong mapping record version"
    );
    let category = categories
        .get(&record.category)
        .context("mapping references an absent category")?;
    let source = reconstruct_cell(category, &record.source)?;
    let target = reconstruct_cell(category, &record.target)?;
    let mapping = category.mapping_category(&source, &target)?;
    let mapping_model = mapping.strong_steiner_model()?;
    let mapping_label = mapping_model
        .as_ref()
        .map(SteinerCategory::canonical_label)
        .transpose()?;
    ensure!(
        mapping_label.as_ref().map(SteinerKey::from_canonical_label) == record.mapping_category
            && mapping_label
                .as_ref()
                .map(|label| label.category().atom_count())
                == record.mapping_atomic_size
            && mapping_label.is_some() == record.has_strong_steiner_model,
        "mapping strong-Steiner model does not reconstruct"
    );
    ensure!(
        mapping.dimension()? == record.relative_dimension
            && mapping.exact_cell_counts()? == record.exact_cell_counts,
        "mapping summary does not reconstruct"
    );
    ensure!(
        record.cells.len() == record.exact_cell_counts.len(),
        "mapping cell degrees missing"
    );
    for (degree, stored) in record.cells.iter().enumerate() {
        let reconstructed = stored
            .iter()
            .map(|cell| reconstruct_cell(category, cell))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            reconstructed == mapping.exact_cells(degree)?,
            "mapping cells do not reconstruct in degree {degree}"
        );
    }
    Ok(())
}

fn validate_homotopy_poset(record: &HomotopyPosetRecord) -> Result<()> {
    ensure!(
        record.record_version == RECORD_VERSION,
        "wrong homotopy Poset version"
    );
    let covers = record
        .covers
        .iter()
        .map(|edge| (edge[0], edge[1]))
        .collect::<Vec<_>>();
    let poset = noether::combin::Poset::new(record.order, &covers)?;
    let label = poset.canonical_label()?;
    ensure!(
        label.poset() == &poset,
        "stored homotopy Poset is not canonical"
    );
    ensure!(
        poset_key(&label) == record.key,
        "homotopy Poset key mismatch"
    );
    Ok(())
}

fn validate_basepoints(
    records: &[BasepointOrbitRecord],
    categories: &BTreeMap<SteinerKey, SteinerCategory>,
    posets: &BTreeMap<poset_census::records::PosetKey, &HomotopyPosetRecord>,
) -> Result<()> {
    let mut local_indices = BTreeMap::<(SteinerKey, usize), BTreeSet<usize>>::new();
    for record in records {
        ensure!(
            record.record_version == RECORD_VERSION,
            "wrong basepoint record version"
        );
        let category = categories
            .get(&record.category)
            .context("basepoint references an absent category")?;
        let pairs = record
            .pairs
            .iter()
            .map(|pair| {
                Ok((
                    reconstruct_cell(category, &pair[0])?,
                    reconstruct_cell(category, &pair[1])?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let basepoint = OrientedBasepoint::new(category, pairs)?;
        ensure!(
            basepoint.is_monomorphic(),
            "stored basepoint is not monomorphic"
        );
        ensure!(
            basepoint.dimension() == record.basepoint_dimension
                && basepoint.homotopy_degree() == record.homotopy_degree,
            "basepoint degree mismatch"
        );
        let homotopy = category.homotopy_poset(&basepoint)?;
        let label = homotopy.canonical_label()?;
        ensure!(
            poset_key(&label) == record.homotopy_poset_key
                && label.poset().order() == record.homotopy_poset_order,
            "basepoint homotopy Poset mismatch"
        );
        ensure!(
            posets.contains_key(&record.homotopy_poset_key),
            "basepoint homotopy Poset sidecar missing"
        );
        ensure!(
            local_indices
                .entry((record.category.clone(), record.basepoint_dimension))
                .or_default()
                .insert(record.local_index),
            "duplicate basepoint local index"
        );
    }
    for ((_category, _dimension), indices) in local_indices {
        ensure!(
            indices.iter().copied().eq(1..=indices.len()),
            "basepoint local indices are not consecutive"
        );
    }
    Ok(())
}

fn validate_categorical_homology(
    records: &[CategoricalHomologyRecord],
    categories: &BTreeMap<SteinerKey, SteinerCategory>,
) -> Result<()> {
    ensure!(
        records.len() == categories.len(),
        "categorical-homology sidecar is incomplete"
    );
    let mut seen = BTreeSet::new();
    for record in records {
        ensure!(
            record.record_version == RECORD_VERSION,
            "wrong categorical-homology version"
        );
        ensure!(
            seen.insert(record.category.clone()),
            "duplicate categorical-homology record"
        );
        let category = categories
            .get(&record.category)
            .context("categorical homology references an absent category")?;
        let presentation = category.categorical_homology();
        ensure!(
            record.model == presentation.model()
                && record.coefficient_monoid == presentation.coefficient_monoid(),
            "categorical-homology construction id mismatch"
        );
        let skeleton = presentation.skeleton(record.complete_through_degree)?;
        ensure!(
            skeleton.verify()
                && skeleton.simplex_counts() == record.simplex_counts
                && skeleton.thin_simplex_counts() == record.thin_simplex_counts,
            "categorical-homology skeleton summary mismatch"
        );
        ensure!(
            record.simplices.len() == record.complete_through_degree + 1,
            "categorical-homology simplex degrees missing"
        );
        for (degree, stored) in record.simplices.iter().enumerate() {
            let expected = skeleton
                .simplices(degree)
                .context("missing Street degree")?;
            ensure!(
                stored.len() == expected.len(),
                "Street-simplex count mismatch"
            );
            for (index, (stored, expected)) in stored.iter().zip(expected).enumerate() {
                let expected_images = expected
                    .images()
                    .iter()
                    .map(|degree| {
                        degree
                            .iter()
                            .map(|support| support.iter().map(|&atom| atom as u32).collect())
                            .collect()
                    })
                    .collect::<Vec<Vec<Vec<u32>>>>();
                ensure!(
                    stored.images == expected_images,
                    "Street-simplex image mismatch"
                );
                ensure!(stored.thin == expected.is_thin(), "thin marking mismatch");
                ensure!(
                    stored.faces == skeleton.faces(degree, index).unwrap_or(&[])
                        && stored.degeneracies
                            == skeleton.degeneracies(degree, index).unwrap_or(&[]),
                    "Street operator mismatch"
                );
            }
        }
    }
    Ok(())
}

fn validate_categorical_homotopy(
    records: &[CategoricalHomotopyRecord],
    categories: &BTreeMap<SteinerKey, SteinerCategory>,
) -> Result<()> {
    ensure!(
        records.len() == categories.len(),
        "categorical-homotopy-Poset sidecar is incomplete"
    );
    let mut seen = BTreeSet::new();
    for record in records {
        ensure!(
            record.record_version == CATEGORICAL_HOMOTOPY_RECORD_VERSION,
            "wrong categorical-homotopy-Poset version"
        );
        ensure!(
            seen.insert(record.category.clone()),
            "duplicate categorical-homotopy-Poset record"
        );
        let category = categories
            .get(&record.category)
            .context("categorical homotopy references an absent category")?;
        let presentation = category.categorical_homology();
        ensure!(
            record.spectrum_model == presentation.spectrum_model()
                && record.infinite_loop_model == presentation.infinite_loop_model()
                && record.higher_particle_sector_model
                    == "disjoint-permutation-sectors-of-cartesian-products",
            "categorical-homotopy construction id mismatch"
        );
        ensure!(
            record.component_poset_slices.len()
                == record.component_particle_number_complete_through + 1,
            "categorical-homotopy component slices are incomplete"
        );
        for (particle_number, stored) in record.component_poset_slices.iter().enumerate() {
            ensure!(
                stored.homotopy_degree == 0 && stored.particle_number == particle_number,
                "categorical-homotopy component slice has the wrong grading"
            );
            let slice = presentation.component_poset_slice(particle_number)?;
            ensure!(
                stored.poset_key.is_some()
                    == (slice.poset().order()
                        <= crate::enrich::CATEGORICAL_HOMOTOPY_POSET_LINK_MAX_ORDER),
                "categorical-homotopy Poset-key materialization policy mismatch"
            );
            let canonical = stored
                .poset_key
                .as_ref()
                .map(|_| slice.poset().canonical_label())
                .transpose()?;
            let expected_poset = canonical
                .as_ref()
                .map_or_else(|| slice.poset(), |label| label.poset());
            ensure!(
                canonical
                    .as_ref()
                    .is_none_or(|label| label.verify(slice.poset()))
                    && stored.poset_key == canonical.as_ref().map(poset_key)
                    && stored.poset_order == expected_poset.order()
                    && stored.covers
                        == expected_poset
                            .covers()
                            .edges()
                            .iter()
                            .map(|edge| [edge.source, edge.target])
                            .collect::<Vec<_>>(),
                "categorical-homotopy component Poset mismatch"
            );
            let mut expected_multisets = vec![Vec::new(); slice.poset().order()];
            for (original, multiset) in slice.object_multisets().iter().enumerate() {
                let stored_index = canonical.as_ref().map_or(original, |label| {
                    label.labeling().images()[original] as usize
                });
                expected_multisets[stored_index] = multiset
                    .iter()
                    .map(|&object| u32::try_from(object).context("object index exceeds u32"))
                    .collect::<Result<Vec<_>>>()?;
            }
            ensure!(
                stored.object_multisets == expected_multisets,
                "categorical-homotopy component elements mismatch"
            );
        }
    }
    Ok(())
}
