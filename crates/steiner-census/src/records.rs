use anyhow::{Result, ensure};
use noether::{
    category::{
        SignedBoundary, SteinerCanonicalLabel, SteinerCategory, SteinerCell, StreetSimplex,
    },
    combin::PosetCanonicalLabel,
};
use poset_census::records::PosetKey;
use serde::{Deserialize, Serialize};

use crate::RECORD_VERSION;

/// Canonical unsigned-word encoding supplied by Noether.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SteinerKey(Vec<String>);

impl SteinerKey {
    pub fn new(words: Vec<String>) -> Result<Self> {
        ensure!(!words.is_empty(), "a SteinerKey cannot be empty");
        for word in &words {
            ensure!(
                word == "0"
                    || (!word.is_empty()
                        && !word.starts_with('0')
                        && word.bytes().all(|byte| byte.is_ascii_digit())),
                "invalid unsigned decimal SteinerKey word {word:?}"
            );
        }
        Ok(Self(words))
    }

    #[must_use]
    pub fn from_canonical_label(label: &SteinerCanonicalLabel) -> Self {
        Self(label.encoding().iter().map(ToString::to_string).collect())
    }

    #[must_use]
    pub fn words(&self) -> &[String] {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BoundaryRecord {
    pub negative: Vec<u32>,
    pub positive: Vec<u32>,
}

/// Minimal canonical identity for one finite strong Steiner category.
///
/// This record is sufficient to reconstruct the category and its canonical
/// key.  Derived invariants deliberately live in separately versioned
/// enrichment records.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SteinerIdentityRecord {
    pub record_version: u32,
    pub key: SteinerKey,
    pub atom_counts: Vec<usize>,
    pub boundaries: Vec<Vec<BoundaryRecord>>,
}

impl SteinerIdentityRecord {
    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.record_version == RECORD_VERSION,
            "wrong identity record version"
        );
        check_boundary_shape(&self.atom_counts, &self.boundaries)
    }

    pub fn reconstruct(&self) -> Result<SteinerCategory> {
        self.check_shape()?;
        let boundaries = self
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
            self.atom_counts.clone(),
            boundaries,
        )?)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CellRecord {
    pub dimension: usize,
    pub negative_supports: Vec<Vec<u32>>,
    pub positive_supports: Vec<Vec<u32>>,
}

impl CellRecord {
    #[must_use]
    pub fn from_cell(cell: &SteinerCell) -> Self {
        Self {
            dimension: cell.dimension(),
            negative_supports: cell
                .negative_supports()
                .iter()
                .map(|support| support.iter().map(|&atom| atom as u32).collect())
                .collect(),
            positive_supports: cell
                .positive_supports()
                .iter()
                .map(|support| support.iter().map(|&atom| atom as u32).collect())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntegralHomologyRecord {
    pub degree: i32,
    pub free_rank: usize,
    pub torsion: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AutomorphismGroupRecord {
    pub degree: usize,
    pub generators: Vec<Vec<u32>>,
    pub order: String,
    pub orbits: Vec<Vec<u32>>,
    pub generators_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lmfdb_label: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DualityRecord {
    pub dimensions: Vec<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SteinerKey>,
    pub is_self_dual: bool,
    pub remains_strong_steiner: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SteinerCategoryRecord {
    pub record_version: u32,
    pub key: SteinerKey,
    pub atom_counts: Vec<usize>,
    pub boundaries: Vec<Vec<BoundaryRecord>>,
    pub invariants: CategoryInvariants,
    pub automorphism_group: AutomorphismGroupRecord,
    pub recognized_families: Vec<NamedFamilyRecord>,
    pub dualities: Vec<DualityRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NamedFamilyRecord {
    pub family: String,
    pub parameter: usize,
}

impl SteinerCategoryRecord {
    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.record_version == RECORD_VERSION,
            "wrong record version"
        );
        ensure!(
            self.invariants.atomic_size == self.atom_counts.iter().sum::<usize>(),
            "atomic size does not match atom counts"
        );
        check_boundary_shape(&self.atom_counts, &self.boundaries)?;
        ensure!(
            self.automorphism_group.degree == self.invariants.atomic_size,
            "automorphism degree does not match atomic size"
        );
        ensure!(
            self.automorphism_group.generators_verified,
            "unverified automorphism generators"
        );
        Ok(())
    }

    pub fn from_layers(
        identity: &SteinerIdentityRecord,
        light: &SteinerLightEnrichmentRecord,
        rich: &SteinerRichEnrichmentRecord,
    ) -> Result<Self> {
        ensure!(
            identity.key == light.category && identity.key == rich.category,
            "Steiner enrichment layers reference different identities"
        );
        ensure!(
            identity.atom_counts.iter().sum::<usize>() == light.invariants.atomic_size,
            "light enrichment atomic size does not match its identity"
        );
        let record = Self {
            record_version: RECORD_VERSION,
            key: identity.key.clone(),
            atom_counts: identity.atom_counts.clone(),
            boundaries: identity.boundaries.clone(),
            invariants: light
                .invariants
                .clone()
                .with_exact_cell_counts(rich.exact_cell_counts.clone()),
            automorphism_group: light.automorphism_group.clone(),
            recognized_families: light.recognized_families.clone(),
            dualities: light.dualities.clone(),
        };
        record.check_shape()?;
        Ok(record)
    }
}

fn check_boundary_shape(atom_counts: &[usize], boundaries: &[Vec<BoundaryRecord>]) -> Result<()> {
    ensure!(
        boundaries.len() == atom_counts.len(),
        "boundary degree count does not match atom counts"
    );
    ensure!(
        boundaries.first().is_none_or(Vec::is_empty),
        "degree zero has a boundary"
    );
    ensure!(
        boundaries
            .iter()
            .enumerate()
            .skip(1)
            .all(|(degree, columns)| columns.len() == atom_counts[degree]),
        "boundary column count does not match atom counts"
    );
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CategoryInvariants {
    pub atomic_size: usize,
    pub dimension: i64,
    pub object_count: usize,
    pub generating_arrow_count: usize,
    pub boundary_nonzero_counts: Vec<usize>,
    pub precedence_height: usize,
    pub precedence_width: usize,
    pub precedence_component_sizes: Vec<usize>,
    pub generating_skeleton_component_sizes: Vec<usize>,
    pub is_connected: bool,
    pub exact_cell_counts: Vec<usize>,
    pub is_empty: bool,
    pub is_discrete: bool,
    pub polygraphic_euler_characteristic: i64,
    pub integral_polygraphic_homology: Vec<IntegralHomologyRecord>,
}

/// Uniform inexpensive enrichment for every identity in a complete shell.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SteinerLightEnrichmentRecord {
    pub record_version: u32,
    pub category: SteinerKey,
    pub invariants: LightCategoryInvariants,
    pub automorphism_group: AutomorphismGroupRecord,
    pub recognized_families: Vec<NamedFamilyRecord>,
    pub dualities: Vec<DualityRecord>,
}

impl SteinerLightEnrichmentRecord {
    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.record_version == RECORD_VERSION,
            "wrong light-enrichment record version"
        );
        ensure!(
            self.automorphism_group.degree == self.invariants.atomic_size,
            "automorphism degree does not match atomic size"
        );
        ensure!(
            self.automorphism_group.generators_verified,
            "unverified automorphism generators"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LightCategoryInvariants {
    pub atomic_size: usize,
    pub dimension: i64,
    pub object_count: usize,
    pub generating_arrow_count: usize,
    pub boundary_nonzero_counts: Vec<usize>,
    pub precedence_height: usize,
    pub precedence_width: usize,
    pub precedence_component_sizes: Vec<usize>,
    pub generating_skeleton_component_sizes: Vec<usize>,
    pub is_connected: bool,
    pub is_empty: bool,
    pub is_discrete: bool,
    pub polygraphic_euler_characteristic: i64,
    pub integral_polygraphic_homology: Vec<IntegralHomologyRecord>,
}

impl LightCategoryInvariants {
    #[must_use]
    pub fn with_exact_cell_counts(self, exact_cell_counts: Vec<usize>) -> CategoryInvariants {
        CategoryInvariants {
            atomic_size: self.atomic_size,
            dimension: self.dimension,
            object_count: self.object_count,
            generating_arrow_count: self.generating_arrow_count,
            boundary_nonzero_counts: self.boundary_nonzero_counts,
            precedence_height: self.precedence_height,
            precedence_width: self.precedence_width,
            precedence_component_sizes: self.precedence_component_sizes,
            generating_skeleton_component_sizes: self.generating_skeleton_component_sizes,
            is_connected: self.is_connected,
            exact_cell_counts,
            is_empty: self.is_empty,
            is_discrete: self.is_discrete,
            polygraphic_euler_characteristic: self.polygraphic_euler_characteristic,
            integral_polygraphic_homology: self.integral_polygraphic_homology,
        }
    }
}

/// Complete exact-cell and object-pair mapping enrichment for one identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SteinerMappingEnrichmentRecord {
    pub record_version: u32,
    pub category: SteinerKey,
    pub exact_cell_counts: Vec<usize>,
}

impl SteinerMappingEnrichmentRecord {
    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.record_version == RECORD_VERSION,
            "wrong mapping-enrichment record version"
        );
        Ok(())
    }
}

/// Expensive per-category summary owned by the rich enrichment lane.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SteinerRichEnrichmentRecord {
    pub record_version: u32,
    pub category: SteinerKey,
    pub exact_cell_counts: Vec<usize>,
}

impl SteinerRichEnrichmentRecord {
    pub fn check_shape(&self) -> Result<()> {
        ensure!(
            self.record_version == RECORD_VERSION,
            "wrong rich-enrichment record version"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InvariantCoverage {
    pub invariant: String,
    pub status: String,
    pub scope: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageUpstream {
    pub contract: String,
    pub atomic_size: usize,
    pub validation_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageManifest {
    pub contract: String,
    pub status: String,
    pub atomic_size: usize,
    pub category_count: usize,
    pub upstream: Vec<CoverageUpstream>,
    pub invariants: Vec<InvariantCoverage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MappingCategoryRecord {
    pub record_version: u32,
    pub category: SteinerKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping_category: Option<SteinerKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping_atomic_size: Option<usize>,
    pub has_strong_steiner_model: bool,
    pub source: CellRecord,
    pub target: CellRecord,
    pub relative_dimension: i64,
    pub exact_cell_counts: Vec<usize>,
    pub cells: Vec<Vec<CellRecord>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BasepointOrbitRecord {
    pub record_version: u32,
    pub category: SteinerKey,
    pub basepoint_dimension: usize,
    pub homotopy_degree: usize,
    pub local_index: usize,
    pub pairs: Vec<[CellRecord; 2]>,
    pub orbit_size: String,
    pub stabilizer_order: String,
    pub homotopy_poset_key: PosetKey,
    pub homotopy_poset_order: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HomotopyPosetRecord {
    pub record_version: u32,
    pub key: PosetKey,
    pub order: usize,
    pub covers: Vec<[u32; 2]>,
}

impl HomotopyPosetRecord {
    #[must_use]
    pub fn from_label(label: &PosetCanonicalLabel) -> Self {
        let poset = label.poset();
        Self {
            record_version: RECORD_VERSION,
            key: poset_key(label),
            order: poset.order(),
            covers: poset
                .covers()
                .edges()
                .iter()
                .map(|edge| [edge.source, edge.target])
                .collect(),
        }
    }
}

#[must_use]
pub fn poset_key(label: &PosetCanonicalLabel) -> PosetKey {
    PosetKey::new(
        label
            .incidence_sequence()
            .iter()
            .map(ToString::to_string)
            .collect(),
    )
    .expect("Noether canonical incidence words are unsigned decimal integers")
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreetSimplexRecord {
    pub images: Vec<Vec<Vec<u32>>>,
    pub thin: bool,
    pub faces: Vec<usize>,
    pub degeneracies: Vec<usize>,
}

impl StreetSimplexRecord {
    #[must_use]
    pub fn from_simplex(simplex: &StreetSimplex, faces: &[usize], degeneracies: &[usize]) -> Self {
        Self {
            images: simplex
                .images()
                .iter()
                .map(|degree| {
                    degree
                        .iter()
                        .map(|support| support.iter().map(|&atom| atom as u32).collect())
                        .collect()
                })
                .collect(),
            thin: simplex.is_thin(),
            faces: faces.to_vec(),
            degeneracies: degeneracies.to_vec(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CategoricalHomotopyComponentSliceRecord {
    pub homotopy_degree: usize,
    pub particle_number: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poset_key: Option<PosetKey>,
    pub poset_order: usize,
    pub covers: Vec<[u32; 2]>,
    pub object_multisets: Vec<Vec<u32>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CategoricalHomologyRecord {
    pub record_version: u32,
    pub category: SteinerKey,
    pub model: String,
    pub coefficient_monoid: String,
    pub complete_through_degree: usize,
    pub simplex_counts: Vec<usize>,
    pub thin_simplex_counts: Vec<usize>,
    pub simplices: Vec<Vec<StreetSimplexRecord>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CategoricalHomotopyRecord {
    pub record_version: u32,
    pub category: SteinerKey,
    pub spectrum_model: String,
    pub infinite_loop_model: String,
    pub component_particle_number_complete_through: usize,
    pub component_poset_slices: Vec<CategoricalHomotopyComponentSliceRecord>,
    pub higher_particle_sector_model: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GrayProductRecord {
    pub record_version: u32,
    pub left: SteinerKey,
    pub right: SteinerKey,
    pub product: SteinerKey,
    pub product_atom_counts: Vec<usize>,
}

#[must_use]
pub fn boundaries(category: &SteinerCategory) -> Vec<Vec<BoundaryRecord>> {
    (0..category.atom_counts().len())
        .map(|degree| {
            if degree == 0 {
                return Vec::new();
            }
            (0..category.atom_count_in_degree(degree))
                .map(|atom| {
                    let boundary = category
                        .boundary(noether::category::AtomId::new(degree, atom))
                        .expect("stored category has valid atoms");
                    BoundaryRecord {
                        negative: boundary
                            .negative()
                            .iter()
                            .map(|&value| value as u32)
                            .collect(),
                        positive: boundary
                            .positive()
                            .iter()
                            .map(|&value| value as u32)
                            .collect(),
                    }
                })
                .collect()
        })
        .collect()
}
