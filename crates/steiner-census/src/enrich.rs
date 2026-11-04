use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use noether::{
    category::{OrientedBasepoint, SteinerCanonicalLabel, SteinerCategory},
    groups::{FiniteGroup, Permutation, PermutationGroup, iterate_elements},
};
use poset_census::records::PosetKey;

use crate::{
    CATEGORICAL_HOMOTOPY_RECORD_VERSION, RECORD_VERSION,
    records::{
        AutomorphismGroupRecord, BasepointOrbitRecord, CategoricalHomologyRecord,
        CategoricalHomotopyComponentSliceRecord, CategoricalHomotopyRecord, CellRecord,
        DualityRecord, GrayProductRecord, HomotopyPosetRecord, IntegralHomologyRecord,
        LightCategoryInvariants, MappingCategoryRecord, NamedFamilyRecord, SteinerCategoryRecord,
        SteinerIdentityRecord, SteinerKey, SteinerLightEnrichmentRecord,
        SteinerRichEnrichmentRecord, StreetSimplexRecord, boundaries, poset_key,
    },
};

#[derive(Debug)]
pub struct EnrichedCategory {
    pub category: SteinerCategoryRecord,
    pub mappings: Vec<MappingCategoryRecord>,
    pub basepoints: Vec<BasepointOrbitRecord>,
    pub homotopy_posets: Vec<HomotopyPosetRecord>,
    pub categorical_homology: CategoricalHomologyRecord,
    pub categorical_homotopy: CategoricalHomotopyRecord,
}

#[derive(Debug)]
pub struct RichEnrichedCategory {
    pub rich: SteinerRichEnrichmentRecord,
    pub mappings: Vec<MappingCategoryRecord>,
    pub basepoints: Vec<BasepointOrbitRecord>,
    pub homotopy_posets: Vec<HomotopyPosetRecord>,
    pub categorical_homology: CategoricalHomologyRecord,
    pub categorical_homotopy: CategoricalHomotopyRecord,
}

fn verify_canonical(canonical: &SteinerCanonicalLabel) -> Result<()> {
    ensure!(
        canonical.verify(canonical.category()),
        "canonical evidence did not verify"
    );
    ensure!(
        canonical.category().canonical_form()? == *canonical.category(),
        "enrichment requires a canonical category"
    );
    Ok(())
}

pub fn identity_record(canonical: &SteinerCanonicalLabel) -> Result<SteinerIdentityRecord> {
    verify_canonical(canonical)?;
    let record = SteinerIdentityRecord {
        record_version: RECORD_VERSION,
        key: SteinerKey::from_canonical_label(canonical),
        atom_counts: canonical.category().atom_counts().to_vec(),
        boundaries: boundaries(canonical.category()),
    };
    record.check_shape()?;
    Ok(record)
}

pub fn light_enrichment_record(
    canonical: &SteinerCanonicalLabel,
) -> Result<SteinerLightEnrichmentRecord> {
    verify_canonical(canonical)?;
    let category = canonical.category();
    let key = SteinerKey::from_canonical_label(canonical);
    let homology = category
        .polygraphic_homology_invariants()?
        .into_iter()
        .map(|(degree, value)| IntegralHomologyRecord {
            degree,
            free_rank: value.free_rank(),
            torsion: value
                .torsion_coefficients()
                .iter()
                .map(ToString::to_string)
                .collect(),
        })
        .collect::<Vec<_>>();
    let precedence = category.precedence_poset()?;
    let mut precedence_component_sizes = precedence
        .connected_components()
        .into_iter()
        .map(|component| component.len())
        .collect::<Vec<_>>();
    precedence_component_sizes.sort_unstable_by(|left, right| right.cmp(left));
    let mut skeleton_components = generating_skeleton_components(category)?;
    skeleton_components.sort_unstable_by(|left, right| right.cmp(left));
    let record = SteinerLightEnrichmentRecord {
        record_version: RECORD_VERSION,
        category: key.clone(),
        invariants: LightCategoryInvariants {
            atomic_size: category.atom_count(),
            dimension: category.dimension(),
            object_count: category.atom_count_in_degree(0),
            generating_arrow_count: category.atom_count_in_degree(1),
            boundary_nonzero_counts: boundary_nonzero_counts(category),
            precedence_height: precedence.height(),
            precedence_width: precedence.width(),
            precedence_component_sizes,
            generating_skeleton_component_sizes: skeleton_components.clone(),
            is_connected: skeleton_components.len() == 1,
            is_empty: category.atom_count() == 0,
            is_discrete: category.dimension() <= 0,
            polygraphic_euler_characteristic: category
                .atom_counts()
                .iter()
                .enumerate()
                .map(|(degree, &count)| {
                    if degree % 2 == 0 {
                        count as i64
                    } else {
                        -(count as i64)
                    }
                })
                .sum(),
            integral_polygraphic_homology: homology,
        },
        automorphism_group: automorphism_record(canonical.automorphism_group())?,
        recognized_families: recognized_families(category)
            .with_context(|| format!("recognizing families for {key:?}"))?,
        dualities: duality_records(category)
            .with_context(|| format!("constructing dualities for {key:?}"))?,
    };
    record.check_shape()?;
    Ok(record)
}

pub fn rich_enrichment_record(
    canonical: &SteinerCanonicalLabel,
    street_degree: usize,
) -> Result<RichEnrichedCategory> {
    verify_canonical(canonical)?;
    let category = canonical.category();
    let key = SteinerKey::from_canonical_label(canonical);
    let rich = SteinerRichEnrichmentRecord {
        record_version: RECORD_VERSION,
        category: key.clone(),
        exact_cell_counts: category.exact_cell_counts()?,
    };
    rich.check_shape()?;
    let mappings = mapping_records(&key, category)
        .with_context(|| format!("constructing mapping categories for {key:?}"))?;
    let (basepoints, homotopy_posets) =
        basepoint_records(&key, category, canonical.automorphism_group())
            .with_context(|| format!("constructing homotopy Posets for {key:?}"))?;
    let categorical_homology = categorical_homology_record(&key, category, street_degree)
        .with_context(|| format!("constructing categorical homology for {key:?}"))?;
    let categorical_homotopy = categorical_homotopy_record(&key, category)
        .with_context(|| format!("constructing categorical homotopy Posets for {key:?}"))?;
    Ok(RichEnrichedCategory {
        rich,
        mappings,
        basepoints,
        homotopy_posets,
        categorical_homology,
        categorical_homotopy,
    })
}

pub fn enrich_category(
    canonical: &SteinerCanonicalLabel,
    street_degree: usize,
) -> Result<EnrichedCategory> {
    let identity = identity_record(canonical)?;
    let light = light_enrichment_record(canonical)?;
    let rich = rich_enrichment_record(canonical, street_degree)?;
    let category = SteinerCategoryRecord::from_layers(&identity, &light, &rich.rich)?;
    Ok(EnrichedCategory {
        category,
        mappings: rich.mappings,
        basepoints: rich.basepoints,
        homotopy_posets: rich.homotopy_posets,
        categorical_homology: rich.categorical_homology,
        categorical_homotopy: rich.categorical_homotopy,
    })
}

fn recognized_families(category: &SteinerCategory) -> Result<Vec<NamedFamilyRecord>> {
    let mut output = Vec::new();
    if category.dimension() < 0 {
        output.push(NamedFamilyRecord {
            family: "empty".to_owned(),
            parameter: 0,
        });
        return Ok(output);
    }
    if category.dimension() == 0 {
        output.push(NamedFamilyRecord {
            family: "discrete".to_owned(),
            parameter: category.atom_count_in_degree(0),
        });
    }
    let dimension = category.dimension() as usize;
    for (family, candidate) in [
        ("globe", SteinerCategory::globe(dimension)?),
        ("oriental", SteinerCategory::oriental(dimension)?),
        ("cube", SteinerCategory::cube(dimension)?),
    ] {
        if candidate.is_isomorphic(category)? {
            output.push(NamedFamilyRecord {
                family: family.to_owned(),
                parameter: dimension,
            });
        }
    }
    Ok(output)
}

fn boundary_nonzero_counts(category: &SteinerCategory) -> Vec<usize> {
    (0..category.atom_counts().len())
        .map(|degree| {
            (0..category.atom_count_in_degree(degree))
                .map(|atom| {
                    if degree == 0 {
                        0
                    } else {
                        let boundary = category
                            .boundary(noether::category::AtomId::new(degree, atom))
                            .expect("stored atom is valid");
                        boundary.negative().len() + boundary.positive().len()
                    }
                })
                .sum()
        })
        .collect()
}

fn generating_skeleton_components(category: &SteinerCategory) -> Result<Vec<usize>> {
    let object_count = category.atom_count_in_degree(0);
    let mut parent = (0..object_count).collect::<Vec<_>>();
    fn root(parent: &mut [usize], mut value: usize) -> usize {
        while parent[value] != value {
            parent[value] = parent[parent[value]];
            value = parent[value];
        }
        value
    }
    for arrow in 0..category.atom_count_in_degree(1) {
        let boundary = category.boundary(noether::category::AtomId::new(1, arrow))?;
        for &left in boundary.negative() {
            for &right in boundary.positive() {
                let left_root = root(&mut parent, left);
                let right_root = root(&mut parent, right);
                parent[right_root] = left_root;
            }
        }
    }
    let mut sizes = BTreeMap::new();
    for object in 0..object_count {
        *sizes.entry(root(&mut parent, object)).or_insert(0usize) += 1;
    }
    Ok(sizes.into_values().collect())
}

fn automorphism_record(group: &PermutationGroup) -> Result<AutomorphismGroupRecord> {
    let mut seen = vec![false; group.degree()];
    let mut orbits = Vec::new();
    for point in 0..group.degree() {
        if seen[point] {
            continue;
        }
        let mut orbit = group
            .orbit(point as u32)?
            .into_iter()
            .inspect(|&value| {
                seen[value as usize] = true;
            })
            .collect::<Vec<_>>();
        orbit.sort_unstable();
        orbits.push(orbit);
    }
    let order = FiniteGroup::order(group).to_string();
    let lmfdb_label = order
        .parse::<u64>()
        .ok()
        .filter(|&value| value == 1 || is_prime(value))
        .map(|value| format!("{value}.1"));
    Ok(AutomorphismGroupRecord {
        degree: group.degree(),
        generators: group
            .generators()
            .iter()
            .map(|generator| generator.images().to_vec())
            .collect(),
        order,
        orbits,
        generators_verified: true,
        lmfdb_label,
    })
}

fn is_prime(value: u64) -> bool {
    if value < 2 {
        return false;
    }
    if value.is_multiple_of(2) {
        return value == 2;
    }
    let mut divisor = 3;
    while divisor <= value / divisor {
        if value.is_multiple_of(divisor) {
            return false;
        }
        divisor += 2;
    }
    true
}

fn duality_records(category: &SteinerCategory) -> Result<Vec<DualityRecord>> {
    let dimension = category.dimension().max(0) as usize;
    if dimension == 0 {
        return Ok(Vec::new());
    }
    let masks = 1usize
        .checked_shl(u32::try_from(dimension).context("duality dimension exceeds u32")?)
        .context("duality mask count overflow")?;
    let mut output = Vec::with_capacity(masks - 1);
    for mask in 1..masks {
        let dimensions = (1..=dimension)
            .filter(|degree| mask & (1usize << (degree - 1)) != 0)
            .collect::<Vec<_>>();
        match category.dual(&dimensions) {
            Ok(dual) => {
                let label = dual.canonical_label()?;
                output.push(DualityRecord {
                    is_self_dual: label.category() == category,
                    dimensions,
                    target: Some(SteinerKey::from_canonical_label(&label)),
                    remains_strong_steiner: true,
                });
            }
            Err(noether::category::CategoryError::CyclicPrecedence) => {
                output.push(DualityRecord {
                    dimensions,
                    target: None,
                    is_self_dual: false,
                    remains_strong_steiner: false,
                });
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(output)
}

/// Materialize every ordered object-pair mapping category and its exact cells.
pub fn mapping_records(
    key: &SteinerKey,
    category: &SteinerCategory,
) -> Result<Vec<MappingCategoryRecord>> {
    let objects = category.exact_cells(0)?;
    let mut output = Vec::new();
    for source in &objects {
        for target in &objects {
            let mapping = category.mapping_category(source, target)?;
            let mapping_model = mapping.strong_steiner_model().with_context(|| {
                format!(
                    "recovering Steiner model for mapping category of {key:?} from {source:?} to {target:?}"
                )
            })?;
            let mapping_label = mapping_model
                .as_ref()
                .map(SteinerCategory::canonical_label)
                .transpose()?;
            let exact_cell_counts = mapping.exact_cell_counts()?;
            let cells = (0..exact_cell_counts.len())
                .map(|degree| {
                    mapping
                        .exact_cells(degree)
                        .map(|cells| cells.iter().map(CellRecord::from_cell).collect::<Vec<_>>())
                })
                .collect::<Result<Vec<_>, _>>()?;
            output.push(MappingCategoryRecord {
                record_version: RECORD_VERSION,
                category: key.clone(),
                mapping_category: mapping_label.as_ref().map(SteinerKey::from_canonical_label),
                mapping_atomic_size: mapping_label
                    .as_ref()
                    .map(|label| label.category().atom_count()),
                has_strong_steiner_model: mapping_label.is_some(),
                source: CellRecord::from_cell(source),
                target: CellRecord::from_cell(target),
                relative_dimension: mapping.dimension()?,
                exact_cell_counts,
                cells,
            });
        }
    }
    Ok(output)
}

type BasepointKey = Vec<(CellRecord, CellRecord)>;

fn basepoint_key(basepoint: &OrientedBasepoint) -> BasepointKey {
    basepoint
        .pairs()
        .iter()
        .map(|(source, target)| (CellRecord::from_cell(source), CellRecord::from_cell(target)))
        .collect()
}

fn transform_basepoint(
    category: &SteinerCategory,
    basepoint: &OrientedBasepoint,
    permutation: &Permutation,
) -> Result<OrientedBasepoint> {
    OrientedBasepoint::new(
        category,
        basepoint
            .pairs()
            .iter()
            .map(|(source, target)| {
                Ok((
                    source.apply_automorphism(permutation)?,
                    target.apply_automorphism(permutation)?,
                ))
            })
            .collect::<Result<Vec<_>, noether::category::CategoryError>>()?,
    )
    .map_err(Into::into)
}

fn basepoint_records(
    key: &SteinerKey,
    category: &SteinerCategory,
    group: &PermutationGroup,
) -> Result<(Vec<BasepointOrbitRecord>, Vec<HomotopyPosetRecord>)> {
    let mut records = Vec::new();
    let mut posets = BTreeMap::<PosetKey, HomotopyPosetRecord>::new();
    let maximum = category.dimension().max(0) as usize;
    for dimension in 0..=maximum {
        let all = category.monomorphic_oriented_basepoints(dimension)?;
        let by_key = all
            .into_iter()
            .map(|basepoint| (basepoint_key(&basepoint), basepoint))
            .collect::<BTreeMap<_, _>>();
        let mut seen = BTreeSet::new();
        let mut representatives = Vec::new();
        for (candidate_key, candidate) in &by_key {
            if seen.contains(candidate_key) {
                continue;
            }
            let mut orbit = BTreeMap::new();
            let mut stabilizer_count = 0usize;
            let mut element_count = 0usize;
            for permutation in iterate_elements(group) {
                element_count = element_count
                    .checked_add(1)
                    .context("group size overflow")?;
                let image = transform_basepoint(category, candidate, &permutation)?;
                let image_key = basepoint_key(&image);
                if image_key == *candidate_key {
                    stabilizer_count += 1;
                }
                orbit.entry(image_key).or_insert(image);
            }
            ensure!(
                orbit.len().checked_mul(stabilizer_count) == Some(element_count),
                "basepoint orbit--stabilizer verification failed"
            );
            seen.extend(orbit.keys().cloned());
            let (minimum, representative) = orbit
                .into_iter()
                .next()
                .context("basepoint orbit was empty")?;
            ensure!(
                by_key.contains_key(&minimum),
                "automorphism produced an unenumerated monomorphic basepoint"
            );
            representatives.push((
                minimum,
                representative,
                element_count / stabilizer_count,
                stabilizer_count,
            ));
        }
        representatives.sort_by(|left, right| left.0.cmp(&right.0));
        for (offset, (_basepoint_key, basepoint, orbit_size, stabilizer_order)) in
            representatives.into_iter().enumerate()
        {
            let homotopy = category.homotopy_poset(&basepoint)?;
            let homotopy_label = homotopy.canonical_label()?;
            let poset_record = HomotopyPosetRecord::from_label(&homotopy_label);
            posets
                .entry(poset_record.key.clone())
                .or_insert_with(|| poset_record.clone());
            records.push(BasepointOrbitRecord {
                record_version: RECORD_VERSION,
                category: key.clone(),
                basepoint_dimension: dimension,
                homotopy_degree: dimension + 1,
                local_index: offset + 1,
                pairs: basepoint
                    .pairs()
                    .iter()
                    .map(|(source, target)| {
                        [CellRecord::from_cell(source), CellRecord::from_cell(target)]
                    })
                    .collect(),
                orbit_size: orbit_size.to_string(),
                stabilizer_order: stabilizer_order.to_string(),
                homotopy_poset_key: poset_key(&homotopy_label),
                homotopy_poset_order: homotopy_label.poset().order(),
            });
        }
    }
    Ok((records, posets.into_values().collect()))
}

const CATEGORICAL_HOMOTOPY_COMPONENT_PARTICLES: usize = 2;
pub(crate) const CATEGORICAL_HOMOTOPY_POSET_LINK_MAX_ORDER: usize = 10;

fn categorical_homology_record(
    key: &SteinerKey,
    category: &SteinerCategory,
    degree: usize,
) -> Result<CategoricalHomologyRecord> {
    let presentation = category.categorical_homology();
    let skeleton = presentation.skeleton(degree)?;
    ensure!(
        skeleton.verify(),
        "Street-nerve skeleton failed simplicial identities"
    );
    let mut simplices = Vec::with_capacity(degree + 1);
    for simplicial_degree in 0..=degree {
        let rows = skeleton
            .simplices(simplicial_degree)
            .context("missing Street-nerve degree")?
            .iter()
            .enumerate()
            .map(|(index, simplex)| {
                StreetSimplexRecord::from_simplex(
                    simplex,
                    skeleton.faces(simplicial_degree, index).unwrap_or(&[]),
                    skeleton
                        .degeneracies(simplicial_degree, index)
                        .unwrap_or(&[]),
                )
            })
            .collect();
        simplices.push(rows);
    }
    Ok(CategoricalHomologyRecord {
        record_version: RECORD_VERSION,
        category: key.clone(),
        model: presentation.model().to_owned(),
        coefficient_monoid: presentation.coefficient_monoid().to_owned(),
        complete_through_degree: degree,
        simplex_counts: skeleton.simplex_counts(),
        thin_simplex_counts: skeleton.thin_simplex_counts(),
        simplices,
    })
}

fn categorical_homotopy_record(
    key: &SteinerKey,
    category: &SteinerCategory,
) -> Result<CategoricalHomotopyRecord> {
    let presentation = category.categorical_homology();
    let mut component_poset_slices = Vec::new();
    for particle_number in 0..=CATEGORICAL_HOMOTOPY_COMPONENT_PARTICLES {
        let slice = presentation.component_poset_slice(particle_number)?;
        let canonical = (slice.poset().order() <= CATEGORICAL_HOMOTOPY_POSET_LINK_MAX_ORDER)
            .then(|| slice.poset().canonical_label())
            .transpose()?;
        if let Some(canonical) = &canonical {
            ensure!(
                canonical.verify(slice.poset()),
                "categorical-homology component Poset label did not verify"
            );
        }
        let poset = canonical
            .as_ref()
            .map_or_else(|| slice.poset(), |label| label.poset());
        let mut object_multisets = vec![Vec::new(); poset.order()];
        for (original, multiset) in slice.object_multisets().iter().enumerate() {
            let stored_index = canonical.as_ref().map_or(original, |label| {
                label.labeling().images()[original] as usize
            });
            object_multisets[stored_index] = multiset
                .iter()
                .map(|&object| u32::try_from(object).context("object index exceeds u32"))
                .collect::<Result<Vec<_>>>()?;
        }
        component_poset_slices.push(CategoricalHomotopyComponentSliceRecord {
            homotopy_degree: 0,
            particle_number,
            poset_key: canonical.as_ref().map(poset_key),
            poset_order: poset.order(),
            covers: poset
                .covers()
                .edges()
                .iter()
                .map(|edge| [edge.source, edge.target])
                .collect(),
            object_multisets,
        });
    }
    Ok(CategoricalHomotopyRecord {
        record_version: CATEGORICAL_HOMOTOPY_RECORD_VERSION,
        category: key.clone(),
        spectrum_model: presentation.spectrum_model().to_owned(),
        infinite_loop_model: presentation.infinite_loop_model().to_owned(),
        component_particle_number_complete_through: CATEGORICAL_HOMOTOPY_COMPONENT_PARTICLES,
        component_poset_slices,
        higher_particle_sector_model: "disjoint-permutation-sectors-of-cartesian-products"
            .to_owned(),
    })
}

pub fn gray_product_record(
    left: &SteinerCanonicalLabel,
    right: &SteinerCanonicalLabel,
) -> Result<GrayProductRecord> {
    let product = left.category().gray_tensor(right.category())?;
    product.verify()?;
    let product = product.canonical_label()?;
    Ok(GrayProductRecord {
        record_version: RECORD_VERSION,
        left: SteinerKey::from_canonical_label(left),
        right: SteinerKey::from_canonical_label(right),
        product: SteinerKey::from_canonical_label(&product),
        product_atom_counts: product.category().atom_counts().to_vec(),
    })
}

pub fn deduplicate_homotopy_posets(
    enriched: &[EnrichedCategory],
) -> Result<Vec<HomotopyPosetRecord>> {
    let mut output = BTreeMap::new();
    for record in enriched.iter().flat_map(|record| &record.homotopy_posets) {
        if let Some(existing) = output.insert(record.key.clone(), record.clone())
            && existing != *record
        {
            bail!("homotopy Poset key collision at {:?}", record.key);
        }
    }
    Ok(output.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(category: SteinerCategory) -> SteinerCanonicalLabel {
        category
            .canonical_form()
            .unwrap()
            .canonical_label()
            .unwrap()
    }

    #[test]
    fn arrow_enrichment_has_mapping_and_homotopy_sidecars() {
        let label = canonical(SteinerCategory::globe(1).unwrap());
        let enriched = enrich_category(&label, 3).unwrap();
        assert_eq!(enriched.category.atom_counts, vec![2, 1]);
        assert_eq!(enriched.mappings.len(), 4);
        assert_eq!(enriched.basepoints.len(), 2);
        assert_eq!(
            enriched
                .basepoints
                .iter()
                .map(|basepoint| basepoint.homotopy_poset_order)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([0, 1])
        );
        assert_eq!(enriched.categorical_homology.simplex_counts[0], 2);
        assert_eq!(
            enriched
                .categorical_homotopy
                .component_poset_slices
                .iter()
                .map(|slice| (slice.particle_number, slice.poset_order))
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 2), (2, 3)]
        );
    }

    #[test]
    fn two_oriental_enrichment_retains_nontrivial_mapping_category() {
        let label = canonical(SteinerCategory::oriental(2).unwrap());
        let enriched = enrich_category(&label, 3).unwrap();
        assert_eq!(enriched.category.atom_counts, vec![3, 3, 1]);
        assert!(
            enriched
                .mappings
                .iter()
                .any(|mapping| mapping.exact_cell_counts == vec![2, 1])
        );
        assert!(enriched.basepoints.iter().any(|basepoint| {
            basepoint.homotopy_degree == 1 && basepoint.homotopy_poset_order == 2
        }));
    }
}
