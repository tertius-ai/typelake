//! Static TypeBase Volume I projection of strong Steiner categories.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result, ensure};
use noether::combin::Poset;
use poset_census::records::{PosetKey, PosetRecord};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use steiner_census::records::{
    BasepointOrbitRecord, BoundaryRecord, CategoricalHomologyRecord, CategoricalHomotopyRecord,
    CellRecord, GrayProductRecord, HomotopyPosetRecord, IntegralHomologyRecord,
    MappingCategoryRecord, SteinerCategoryRecord, SteinerKey,
};

use crate::steiner_groups::SteinerGroupIdentificationRecord;

const COMPONENT: &str = "typebase/volume-1/strong-steiner-categories";
const LABEL_SCHEME: &str = "atomic-size.dimension.canonical-index/v1";
const GALLERY_ROWS_PER_PART: usize = 48;

#[derive(Clone, Debug)]
pub struct SteinerBuildOptions {
    pub data_root: PathBuf,
    pub output_root: PathBuf,
    pub maximum_atomic_size: usize,
    pub poset_data_root: PathBuf,
    pub group_identifications: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SteinerBuildOutcome {
    pub directory: PathBuf,
    pub category_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct SteinerLabel {
    atomic_size: usize,
    dimension: i64,
    index: usize,
}

impl std::fmt::Display for SteinerLabel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}.{}.{}",
            self.atomic_size, self.dimension, self.index
        )
    }
}

#[derive(Clone, Debug, Serialize)]
struct SearchRow {
    label: String,
    atomic_size: usize,
    dimension: i64,
    bucket_index: usize,
    atom_counts: Vec<usize>,
    object_count: usize,
    generating_arrow_count: usize,
    exact_cell_counts: Vec<usize>,
    total_nonidentity_cells: usize,
    is_empty: bool,
    is_discrete: bool,
    is_connected: bool,
    is_self_dual: bool,
    automorphism_order: String,
    automorphism_lmfdb_label: Option<String>,
    polygraphic_homology_signature: String,
    has_polygraphic_torsion: bool,
    basepoint_orbit_count: usize,
    maximum_homotopy_poset_order: usize,
    has_interesting_homotopy_poset: bool,
    street_simplex_counts: Vec<usize>,
    recognized_families: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct GalleryRow {
    label: String,
    atomic_size: usize,
    dimension: i64,
    bucket_index: usize,
    boundaries: Vec<Vec<BoundaryRecord>>,
    object_count: usize,
    generating_arrow_count: usize,
    is_empty: bool,
    is_discrete: bool,
}

#[derive(Clone, Debug, Serialize)]
struct GalleryPart {
    path: String,
    row_count: usize,
    first_label: String,
    last_label: String,
}

#[derive(Clone, Debug, Serialize)]
struct GalleryIndex {
    record_version: u32,
    category_count: usize,
    rows_per_part: usize,
    parts: Vec<GalleryPart>,
}

#[derive(Clone, Debug, Serialize)]
struct LabeledDuality {
    dimensions: Vec<usize>,
    target_label: Option<String>,
    is_self_dual: bool,
    remains_strong_steiner: bool,
}

#[derive(Clone, Debug, Serialize)]
struct DetailRecord<'a> {
    label: String,
    category: &'a SteinerCategoryRecord,
    dualities: Vec<LabeledDuality>,
    mappings: Vec<LabeledMapping<'a>>,
    basepoints: Vec<LabeledBasepoint<'a>>,
    homotopy_posets: Vec<&'a HomotopyPosetRecord>,
    categorical_homology: &'a CategoricalHomologyRecord,
    categorical_homotopy: LabeledCategoricalHomotopy<'a>,
    automorphism_lmfdb_label: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct LabeledCategoricalHomotopy<'a> {
    #[serde(flatten)]
    record: &'a CategoricalHomotopyRecord,
    component_poset_links: Vec<CategoricalComponentPosetLink>,
}

#[derive(Clone, Debug, Serialize)]
struct CategoricalComponentPosetLink {
    particle_number: usize,
    poset_label: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct LabeledMapping<'a> {
    #[serde(flatten)]
    record: &'a MappingCategoryRecord,
    mapping_label: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct LabeledBasepoint<'a> {
    #[serde(flatten)]
    record: &'a BasepointOrbitRecord,
    description: String,
    homotopy_poset_label: Option<String>,
    homotopy_poset_summary: HomotopyPosetSummary,
}

#[derive(Clone, Debug, Serialize)]
struct HomotopyPosetSummary {
    shape: String,
    component_count: usize,
    core_order: usize,
    integral_homology: Vec<IntegralHomologyRecord>,
    has_nontrivial_reduced_homology: bool,
    is_interesting: bool,
}

#[derive(Clone, Debug, Serialize)]
struct LabelBucket {
    atomic_size: usize,
    dimension: i64,
    row_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct SourceShell {
    atomic_size: usize,
    category_count: usize,
    validation_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct Artifact {
    kind: String,
    path: String,
    row_count: usize,
    byte_count: u64,
    sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct Manifest {
    component: &'static str,
    status: &'static str,
    volume: u32,
    collection: &'static str,
    census_record_version: u32,
    maximum_atomic_size: usize,
    label_scheme: &'static str,
    label_index_origin: usize,
    empty_category_label: &'static str,
    category_count: usize,
    stores_labeled_categories: bool,
    street_nerve_complete_through_degree: usize,
    categorical_homotopy_particle_number_complete_through: usize,
    group_identifications_sha256: Option<String>,
    label_buckets: Vec<LabelBucket>,
    source_shells: Vec<SourceShell>,
    catalogue_path: &'static str,
    gallery_index_path: &'static str,
    interesting_path: &'static str,
    gray_table_path: &'static str,
    outputs: Vec<Artifact>,
}

#[derive(Clone, Debug, Deserialize)]
struct GraySourceRow {
    left_name: String,
    right_name: String,
    #[serde(flatten)]
    product: GrayProductRecord,
}

#[derive(Clone, Debug, Serialize)]
struct GrayCatalogueRow {
    left_name: String,
    right_name: String,
    left_label: String,
    right_label: String,
    product_label: Option<String>,
    product_key: SteinerKey,
    product_atom_counts: Vec<usize>,
    within_complete_region: bool,
}

#[derive(Debug)]
struct Shell {
    atomic_size: usize,
    categories: Vec<SteinerCategoryRecord>,
    mappings: Vec<MappingCategoryRecord>,
    basepoints: Vec<BasepointOrbitRecord>,
    homotopy_posets: Vec<HomotopyPosetRecord>,
    categorical_homology: Vec<CategoricalHomologyRecord>,
    categorical_homotopy: Vec<CategoricalHomotopyRecord>,
    validation_sha256: String,
}

pub fn build_steiner_component(options: &SteinerBuildOptions) -> Result<SteinerBuildOutcome> {
    ensure!(
        !options.output_root.exists(),
        "TypeBase output {} already exists; releases are immutable",
        options.output_root.display()
    );
    let mut shells = Vec::new();
    for atomic_size in 0..=options.maximum_atomic_size {
        shells.push(load_shell(&options.data_root, atomic_size)?);
    }
    let poset_labels = load_poset_labels(&options.poset_data_root, &shells)?;
    let (group_identifications, group_identifications_sha256) =
        load_group_identifications(options.group_identifications.as_deref(), &shells)?;

    let mut labels = HashMap::<SteinerKey, SteinerLabel>::new();
    let mut buckets = BTreeMap::<(usize, i64), usize>::new();
    for shell in &shells {
        let mut previous = None::<&SteinerKey>;
        let mut counters = BTreeMap::<i64, usize>::new();
        for category in &shell.categories {
            if let Some(previous) = previous {
                ensure!(
                    previous < &category.key,
                    "source categories are not in canonical-key order"
                );
            }
            previous = Some(&category.key);
            ensure!(
                category.invariants.atomic_size == shell.atomic_size,
                "source shell contains a mixed atomic size"
            );
            let index = counters
                .entry(category.invariants.dimension)
                .and_modify(|index| *index += 1)
                .or_insert(1);
            let label = SteinerLabel {
                atomic_size: shell.atomic_size,
                dimension: category.invariants.dimension,
                index: *index,
            };
            ensure!(
                labels.insert(category.key.clone(), label).is_none(),
                "duplicate category key"
            );
        }
        for (dimension, count) in counters {
            buckets.insert((shell.atomic_size, dimension), count);
        }
    }
    ensure!(
        labels
            .values()
            .find(|label| label.atomic_size == 0)
            .is_some_and(|label| label.to_string() == "0.-1.1"),
        "the empty category did not receive label 0.-1.1"
    );

    let parent = options
        .output_root
        .parent()
        .context("TypeBase output has no parent")?;
    fs::create_dir_all(parent)?;
    let name = options
        .output_root
        .file_name()
        .context("TypeBase output has no file name")?
        .to_string_lossy();
    let staging = parent.join(format!(".{name}.tmp-{}", process::id()));
    ensure!(
        !staging.exists(),
        "TypeBase staging directory already exists"
    );
    fs::create_dir(&staging)?;
    fs::create_dir(staging.join("details"))?;
    fs::create_dir(staging.join("gallery"))?;

    let mut search = Vec::with_capacity(labels.len());
    let mut gallery = Vec::with_capacity(labels.len());
    let mut interesting = Vec::<String>::new();
    let mut outputs = Vec::new();
    for shell in &shells {
        let mappings = group_by_category(&shell.mappings, |record| &record.category);
        let basepoints = group_by_category(&shell.basepoints, |record| &record.category);
        let categorical = shell
            .categorical_homology
            .iter()
            .map(|record| (record.category.clone(), record))
            .collect::<HashMap<_, _>>();
        let categorical_homotopy = shell
            .categorical_homotopy
            .iter()
            .map(|record| (record.category.clone(), record))
            .collect::<HashMap<_, _>>();
        let posets = shell
            .homotopy_posets
            .iter()
            .map(|record| (record.key.clone(), record))
            .collect::<HashMap<_, _>>();
        let mut poset_summaries = HashMap::new();
        for record in &shell.homotopy_posets {
            if !poset_summaries.contains_key(&record.key) {
                poset_summaries.insert(record.key.clone(), summarize_homotopy_poset(record)?);
            }
        }
        for category in &shell.categories {
            let label = label_for(&labels, &category.key)?;
            let category_mappings = mappings.get(&category.key).map_or(&[][..], Vec::as_slice);
            let category_basepoints = basepoints.get(&category.key).map_or(&[][..], Vec::as_slice);
            let category_homology = categorical
                .get(&category.key)
                .copied()
                .context("category has no categorical-homology sidecar")?;
            let category_homotopy = categorical_homotopy
                .get(&category.key)
                .copied()
                .context("category has no categorical-homotopy-Poset sidecar")?;
            let family_names = category
                .recognized_families
                .iter()
                .map(|family| format!("{} {}", family.family, family.parameter))
                .collect::<Vec<_>>();
            let has_interesting_homotopy_poset = category_basepoints.iter().any(|basepoint| {
                poset_summaries
                    .get(&basepoint.homotopy_poset_key)
                    .is_some_and(|summary| summary.is_interesting)
            });
            if !family_names.is_empty()
                || has_interesting_homotopy_poset
                || matches!(
                    label.to_string().as_str(),
                    "4.1.1" | "5.1.3" | "5.1.4" | "5.1.5"
                )
            {
                interesting.push(label.to_string());
            }
            let automorphism_lmfdb_label = group_identifications
                .get(&category.key)
                .map(|record| record.lmfdb_label.clone())
                .or_else(|| category.automorphism_group.lmfdb_label.clone());
            search.push(SearchRow {
                label: label.to_string(),
                atomic_size: label.atomic_size,
                dimension: label.dimension,
                bucket_index: label.index,
                atom_counts: category.atom_counts.clone(),
                object_count: category.invariants.object_count,
                generating_arrow_count: category.invariants.generating_arrow_count,
                exact_cell_counts: category.invariants.exact_cell_counts.clone(),
                total_nonidentity_cells: category.invariants.exact_cell_counts.iter().sum(),
                is_empty: category.invariants.is_empty,
                is_discrete: category.invariants.is_discrete,
                is_connected: category.invariants.is_connected,
                is_self_dual: category
                    .dualities
                    .iter()
                    .any(|duality| duality.is_self_dual),
                automorphism_order: category.automorphism_group.order.clone(),
                automorphism_lmfdb_label: automorphism_lmfdb_label.clone(),
                polygraphic_homology_signature: homology_signature(category),
                has_polygraphic_torsion: category
                    .invariants
                    .integral_polygraphic_homology
                    .iter()
                    .any(|group| !group.torsion.is_empty()),
                basepoint_orbit_count: category_basepoints.len(),
                maximum_homotopy_poset_order: category_basepoints
                    .iter()
                    .map(|basepoint| basepoint.homotopy_poset_order)
                    .max()
                    .unwrap_or(0),
                has_interesting_homotopy_poset,
                street_simplex_counts: category_homology.simplex_counts.clone(),
                recognized_families: family_names,
            });
            gallery.push(GalleryRow {
                label: label.to_string(),
                atomic_size: label.atomic_size,
                dimension: label.dimension,
                bucket_index: label.index,
                boundaries: category.boundaries.clone(),
                object_count: category.invariants.object_count,
                generating_arrow_count: category.invariants.generating_arrow_count,
                is_empty: category.invariants.is_empty,
                is_discrete: category.invariants.is_discrete,
            });

            let dualities = category
                .dualities
                .iter()
                .map(|duality| {
                    Ok(LabeledDuality {
                        dimensions: duality.dimensions.clone(),
                        target_label: duality
                            .target
                            .as_ref()
                            .map(|target| label_for(&labels, target).map(|label| label.to_string()))
                            .transpose()?,
                        is_self_dual: duality.is_self_dual,
                        remains_strong_steiner: duality.remains_strong_steiner,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let homotopy_posets = category_basepoints
                .iter()
                .map(|basepoint| {
                    posets
                        .get(&basepoint.homotopy_poset_key)
                        .copied()
                        .context("basepoint has no homotopy-Poset sidecar")
                })
                .collect::<Result<Vec<_>>>()?;
            let labeled_mappings = category_mappings
                .iter()
                .map(|record| {
                    Ok(LabeledMapping {
                        record,
                        mapping_label: record
                            .mapping_category
                            .as_ref()
                            .and_then(|target| labels.get(target))
                            .map(ToString::to_string),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let labeled_basepoints = category_basepoints
                .iter()
                .map(|record| {
                    Ok(LabeledBasepoint {
                        record,
                        description: basepoint_description(record),
                        homotopy_poset_label: poset_labels.get(&record.homotopy_poset_key).cloned(),
                        homotopy_poset_summary: poset_summaries
                            .get(&record.homotopy_poset_key)
                            .cloned()
                            .context("basepoint has no homotopy-Poset summary")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let labeled_categorical_homotopy = LabeledCategoricalHomotopy {
                record: category_homotopy,
                component_poset_links: category_homotopy
                    .component_poset_slices
                    .iter()
                    .map(|slice| CategoricalComponentPosetLink {
                        particle_number: slice.particle_number,
                        poset_label: slice
                            .poset_key
                            .as_ref()
                            .and_then(|key| poset_labels.get(key))
                            .cloned(),
                    })
                    .collect(),
            };
            let path = staging.join("details").join(format!("{label}.json"));
            write_json(
                &path,
                &DetailRecord {
                    label: label.to_string(),
                    category,
                    dualities,
                    mappings: labeled_mappings,
                    basepoints: labeled_basepoints,
                    homotopy_posets,
                    categorical_homology: category_homology,
                    categorical_homotopy: labeled_categorical_homotopy,
                    automorphism_lmfdb_label,
                },
            )?;
            outputs.push(artifact(&staging, &path, "detail", 1)?);
        }
    }
    search.sort_by_key(|row| (row.atomic_size, row.dimension, row.bucket_index));
    gallery.sort_by_key(|row| {
        (
            row.is_discrete,
            row.atomic_size,
            row.dimension,
            row.bucket_index,
        )
    });
    let gallery_labels = gallery
        .iter()
        .map(|row| row.label.as_str())
        .collect::<HashSet<_>>();
    ensure!(
        gallery.len() == search.len()
            && search
                .iter()
                .all(|row| gallery_labels.contains(row.label.as_str())),
        "gallery projection does not match the catalogue labels"
    );
    interesting.sort();
    interesting.dedup();
    let catalogue_path = staging.join("catalogue.json");
    write_json(&catalogue_path, &search)?;
    outputs.push(artifact(
        &staging,
        &catalogue_path,
        "catalogue",
        search.len(),
    )?);
    let mut gallery_parts = Vec::new();
    for (index, rows) in gallery.chunks(GALLERY_ROWS_PER_PART).enumerate() {
        let relative_path = format!("gallery/part-{index:05}.json");
        let path = staging.join(&relative_path);
        write_json(&path, &rows)?;
        outputs.push(artifact(&staging, &path, "gallery-part", rows.len())?);
        gallery_parts.push(GalleryPart {
            path: relative_path,
            row_count: rows.len(),
            first_label: rows.first().context("empty gallery part")?.label.clone(),
            last_label: rows.last().context("empty gallery part")?.label.clone(),
        });
    }
    let gallery_index_path = staging.join("gallery/index.json");
    write_json(
        &gallery_index_path,
        &GalleryIndex {
            record_version: 1,
            category_count: gallery.len(),
            rows_per_part: GALLERY_ROWS_PER_PART,
            parts: gallery_parts,
        },
    )?;
    outputs.push(artifact(
        &staging,
        &gallery_index_path,
        "gallery-index",
        gallery.len(),
    )?);
    let interesting_path = staging.join("interesting.json");
    write_json(&interesting_path, &interesting)?;
    outputs.push(artifact(
        &staging,
        &interesting_path,
        "interesting",
        interesting.len(),
    )?);
    let gray_source: Vec<GraySourceRow> =
        read_jsonl(&options.data_root.join("gray-products.jsonl"))?;
    ensure!(
        gray_source.len() == 25,
        "the fixed Gray table is not 5 by 5"
    );
    let gray_rows = gray_source
        .into_iter()
        .map(|row| {
            let product_atomic_size = row.product.product_atom_counts.iter().sum::<usize>();
            Ok(GrayCatalogueRow {
                left_name: row.left_name,
                right_name: row.right_name,
                left_label: label_for(&labels, &row.product.left)?.to_string(),
                right_label: label_for(&labels, &row.product.right)?.to_string(),
                product_label: labels.get(&row.product.product).map(ToString::to_string),
                product_key: row.product.product,
                product_atom_counts: row.product.product_atom_counts,
                within_complete_region: product_atomic_size <= options.maximum_atomic_size,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let gray_path = staging.join("gray-table.json");
    write_json(&gray_path, &gray_rows)?;
    outputs.push(artifact(
        &staging,
        &gray_path,
        "gray-table",
        gray_rows.len(),
    )?);

    let street_degree = shells
        .iter()
        .flat_map(|shell| &shell.categorical_homology)
        .map(|record| record.complete_through_degree)
        .min()
        .unwrap_or(0);
    ensure!(
        shells
            .iter()
            .flat_map(|shell| &shell.categorical_homology)
            .all(|record| record.complete_through_degree == street_degree),
        "Street-nerve materialization degree is not uniform"
    );
    let categorical_homotopy_particle_number = shells
        .iter()
        .flat_map(|shell| &shell.categorical_homotopy)
        .map(|record| record.component_particle_number_complete_through)
        .min()
        .unwrap_or(0);
    ensure!(
        shells
            .iter()
            .flat_map(|shell| &shell.categorical_homotopy)
            .all(|record| {
                record.component_particle_number_complete_through
                    == categorical_homotopy_particle_number
            }),
        "categorical-homotopy particle-number bound is not uniform"
    );
    let manifest = Manifest {
        component: COMPONENT,
        status: "passed",
        volume: 1,
        collection: "strong-steiner-categories/v1",
        census_record_version: 1,
        maximum_atomic_size: options.maximum_atomic_size,
        label_scheme: LABEL_SCHEME,
        label_index_origin: 1,
        empty_category_label: "0.-1.1",
        category_count: search.len(),
        stores_labeled_categories: false,
        street_nerve_complete_through_degree: street_degree,
        categorical_homotopy_particle_number_complete_through: categorical_homotopy_particle_number,
        group_identifications_sha256,
        label_buckets: buckets
            .into_iter()
            .map(|((atomic_size, dimension), row_count)| LabelBucket {
                atomic_size,
                dimension,
                row_count,
            })
            .collect(),
        source_shells: shells
            .iter()
            .map(|shell| SourceShell {
                atomic_size: shell.atomic_size,
                category_count: shell.categories.len(),
                validation_sha256: shell.validation_sha256.clone(),
            })
            .collect(),
        catalogue_path: "catalogue.json",
        gallery_index_path: "gallery/index.json",
        interesting_path: "interesting.json",
        gray_table_path: "gray-table.json",
        outputs,
    };
    write_json(&staging.join("manifest.json"), &manifest)?;
    fs::rename(&staging, &options.output_root)?;
    Ok(SteinerBuildOutcome {
        directory: options.output_root.clone(),
        category_count: search.len(),
    })
}

fn load_shell(data_root: &Path, atomic_size: usize) -> Result<Shell> {
    let directory = data_root
        .join("census/record-version=1")
        .join(format!("atomic-size={atomic_size}"));
    let validation_path = directory.join("validation.json");
    let validation: serde_json::Value = serde_json::from_reader(BufReader::new(
        File::open(&validation_path)
            .with_context(|| format!("opening validation for atomic size {atomic_size}"))?,
    ))?;
    ensure!(validation["contract"] == "strong-steiner-categories/v1");
    ensure!(validation["status"] == "passed");
    ensure!(validation["atomic_size"] == atomic_size);
    ensure!(validation["isomorphism_scope"]["status"] == "complete");
    let (_, validation_sha256) = hash_file(&validation_path)?;
    let categories: Vec<SteinerCategoryRecord> = read_jsonl(&directory.join("categories.jsonl"))?;
    for record in &categories {
        record.check_shape()?;
    }
    ensure!(
        validation["category_count"] == categories.len(),
        "validation category count mismatch"
    );
    Ok(Shell {
        atomic_size,
        categories,
        mappings: read_zstd_jsonl(&directory.join("mapping-categories.jsonl.zst"))?,
        basepoints: read_zstd_jsonl(&directory.join("monomorphic-basepoints.jsonl.zst"))?,
        homotopy_posets: read_zstd_jsonl(&directory.join("homotopy-posets.jsonl.zst"))?,
        categorical_homology: read_zstd_jsonl(&directory.join("categorical-homology.jsonl.zst"))?,
        categorical_homotopy: read_zstd_jsonl(
            &directory.join("categorical-homotopy-posets.jsonl.zst"),
        )?,
        validation_sha256,
    })
}

fn load_poset_labels(data_root: &Path, shells: &[Shell]) -> Result<HashMap<PosetKey, String>> {
    let basepoint_maximum = shells
        .iter()
        .flat_map(|shell| &shell.homotopy_posets)
        .map(|poset| poset.order)
        .max()
        .unwrap_or(0);
    let categorical_maximum = shells
        .iter()
        .flat_map(|shell| &shell.categorical_homotopy)
        .flat_map(|record| &record.component_poset_slices)
        .map(|slice| slice.poset_order)
        .max()
        .unwrap_or(0);
    let maximum_order = basepoint_maximum.max(categorical_maximum);
    let mut labels = HashMap::new();
    for order in 0..=maximum_order {
        let path = data_root
            .join("census/record-version=1")
            .join(format!("order={order}/posets.jsonl"));
        if !path.exists() {
            continue;
        }
        let posets: Vec<PosetRecord> = read_jsonl(&path).with_context(|| {
            format!("loading finite-Poset labels through order {maximum_order}")
        })?;
        let mut counters = BTreeMap::<i64, usize>::new();
        for poset in posets {
            let dimension = if order == 0 {
                -1
            } else {
                i64::try_from(poset.invariants.height - 1)?
            };
            let index = counters
                .entry(dimension)
                .and_modify(|index| *index += 1)
                .or_insert(1);
            ensure!(
                labels
                    .insert(poset.key, format!("{order}.{dimension}.{index}"))
                    .is_none(),
                "duplicate Poset key while resolving homotopy links"
            );
        }
    }
    Ok(labels)
}

fn load_group_identifications(
    root: Option<&Path>,
    shells: &[Shell],
) -> Result<(
    HashMap<SteinerKey, SteinerGroupIdentificationRecord>,
    Option<String>,
)> {
    let Some(root) = root else {
        return Ok((HashMap::new(), None));
    };
    let manifest: serde_json::Value =
        serde_json::from_reader(BufReader::new(File::open(root.join("manifest.json"))?))?;
    ensure!(manifest["enrichment"] == "strong-steiner-categories/automorphism-abstract-groups/v1");
    ensure!(manifest["status"] == "passed");
    let expected = shells
        .iter()
        .flat_map(|shell| &shell.categories)
        .map(|category| {
            (
                category.key.clone(),
                category.automorphism_group.order.as_str(),
            )
        })
        .collect::<HashMap<_, _>>();
    let records_path = root.join("records.jsonl");
    let records: Vec<SteinerGroupIdentificationRecord> = read_jsonl(&records_path)?;
    let mut output = HashMap::new();
    for record in records {
        ensure!(record.record_version == 1);
        let expected_order = expected
            .get(&record.category_key)
            .context("group identification references an unselected category")?;
        ensure!(
            **expected_order == record.automorphism_order,
            "group identification has the wrong order"
        );
        ensure!(
            output.insert(record.category_key.clone(), record).is_none(),
            "duplicate Steiner-category group identification"
        );
    }
    let (_, sha256) = hash_file(&records_path)?;
    Ok((output, Some(sha256)))
}

fn group_by_category<T>(
    records: &[T],
    key: impl Fn(&T) -> &SteinerKey,
) -> HashMap<SteinerKey, Vec<T>>
where
    T: Clone,
{
    let mut output = HashMap::new();
    for record in records {
        output
            .entry(key(record).clone())
            .or_insert_with(Vec::new)
            .push(record.clone());
    }
    output
}

fn label_for(labels: &HashMap<SteinerKey, SteinerLabel>, key: &SteinerKey) -> Result<SteinerLabel> {
    labels
        .get(key)
        .copied()
        .context("relation target has no selected Steiner-category label")
}

fn homology_signature(category: &SteinerCategoryRecord) -> String {
    category
        .invariants
        .integral_polygraphic_homology
        .iter()
        .map(|group| {
            format!(
                "{}:{}:{}",
                group.degree,
                group.free_rank,
                group.torsion.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn basepoint_description(basepoint: &BasepointOrbitRecord) -> String {
    basepoint
        .pairs
        .iter()
        .enumerate()
        .map(|(dimension, pair)| {
            let separator = if dimension == 0 { "→" } else { "⇉" };
            format!(
                "{} {separator} {}",
                cell_description(&pair[0]),
                cell_description(&pair[1])
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn cell_description(cell: &CellRecord) -> String {
    let top_support = cell
        .positive_supports
        .get(cell.dimension)
        .map_or(&[][..], Vec::as_slice);
    if cell.dimension == 0 {
        return top_support
            .first()
            .map_or_else(|| "empty object".to_owned(), |object| format!("v{object}"));
    }
    if top_support.is_empty() {
        return "identity".to_owned();
    }
    let prefix = match cell.dimension {
        1 => "a".to_owned(),
        2 => "α".to_owned(),
        dimension => format!("cell{dimension}."),
    };
    if top_support.len() == 1 {
        return format!("{prefix}{}", top_support[0]);
    }
    format!(
        "{}[{}]",
        match cell.dimension {
            1 => "path",
            2 => "2-cell",
            _ => "composite",
        },
        top_support
            .iter()
            .map(|atom| format!("{prefix}{atom}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn summarize_homotopy_poset(record: &HomotopyPosetRecord) -> Result<HomotopyPosetSummary> {
    let relations = record
        .covers
        .iter()
        .map(|cover| (cover[0], cover[1]))
        .collect::<Vec<_>>();
    let poset = Poset::new(record.order, &relations)?;
    let components = poset.connected_components();
    let reduction = poset.core()?;
    let core_order = reduction.core().order();
    let integral_homology = reduction
        .graded_homology_invariants()?
        .iter()
        .enumerate()
        .map(|(degree, group)| IntegralHomologyRecord {
            degree: i32::try_from(degree).expect("homology degree fits i32"),
            free_rank: group.free_rank(),
            torsion: group
                .torsion_coefficients()
                .iter()
                .map(ToString::to_string)
                .collect(),
        })
        .collect::<Vec<_>>();
    let has_nontrivial_reduced_homology = integral_homology.iter().any(|group| {
        !group.torsion.is_empty()
            || if group.degree == 0 {
                group.free_rank > 1
            } else {
                group.free_rank > 0
            }
    });
    let is_interesting = core_order > components.len();
    let is_four_point_crown = record.order == 4
        && record.covers.len() == 4
        && poset.height() == 2
        && poset.minimal_elements().len() == 2
        && poset.maximal_elements().len() == 2;
    let is_crown_plus_point = record.order == 5
        && record.covers.len() == 4
        && poset.height() == 2
        && components.len() == 2
        && components.iter().any(|component| component.len() == 1);
    let shape = if record.order == 0 {
        "empty".to_owned()
    } else if record.order == 1 {
        "point".to_owned()
    } else if record.covers.is_empty() {
        format!("{}-point antichain", record.order)
    } else if poset.height() == record.order {
        format!("{}-point chain", record.order)
    } else if is_four_point_crown {
        "four-point crown (K₂,₂)".to_owned()
    } else if is_crown_plus_point {
        "four-point crown + isolated point".to_owned()
    } else if components.len() > 1 {
        format!("disconnected Poset ({} components)", components.len())
    } else if core_order == 1 {
        "contractible Poset".to_owned()
    } else {
        format!("nontrivial core on {core_order} points")
    };
    Ok(HomotopyPosetSummary {
        shape,
        component_count: components.len(),
        core_order,
        integral_homology,
        has_nontrivial_reduced_homology,
        is_interesting,
    })
}

fn read_jsonl<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let reader = BufReader::new(File::open(path)?);
    read_lines(reader, path)
}

fn read_zstd_jsonl<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let decoder = zstd::stream::read::Decoder::new(File::open(path)?)?;
    read_lines(BufReader::new(decoder), path)
}

fn read_lines<T: DeserializeOwned>(reader: impl BufRead, path: &Path) -> Result<Vec<T>> {
    reader
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let line = line.with_context(|| format!("reading {}:{}", path.display(), index + 1))?;
            serde_json::from_str(&line)
                .with_context(|| format!("decoding {}:{}", path.display(), index + 1))
        })
        .collect()
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    serde_json::to_writer(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn artifact(root: &Path, path: &Path, kind: &str, rows: usize) -> Result<Artifact> {
    let (byte_count, sha256) = hash_file(path)?;
    Ok(Artifact {
        kind: kind.to_owned(),
        path: path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/"),
        row_count: rows,
        byte_count,
        sha256,
    })
}

fn hash_file(path: &Path) -> Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut count = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
        count += read as u64;
    }
    Ok((count, hex::encode(digest.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_cell(object: u32) -> CellRecord {
        CellRecord {
            dimension: 0,
            negative_supports: vec![vec![object]],
            positive_supports: vec![vec![object]],
        }
    }

    #[test]
    fn basepoint_description_names_its_oriented_boundary() -> Result<()> {
        let basepoint = BasepointOrbitRecord {
            record_version: 1,
            category: SteinerKey::new(vec!["1".to_owned()])?,
            basepoint_dimension: 0,
            homotopy_degree: 1,
            local_index: 7,
            pairs: vec![[object_cell(0), object_cell(1)]],
            orbit_size: "1".to_owned(),
            stabilizer_order: "1".to_owned(),
            homotopy_poset_key: PosetKey::new(vec!["1".to_owned()])?,
            homotopy_poset_order: 1,
        };
        assert_eq!(basepoint_description(&basepoint), "v0 → v1");
        Ok(())
    }

    #[test]
    fn four_point_crown_is_a_circle_and_an_interesting_core() -> Result<()> {
        let record = HomotopyPosetRecord {
            record_version: 1,
            key: PosetKey::new(vec!["1".to_owned(), "2".to_owned()])?,
            order: 4,
            covers: vec![[0, 2], [0, 3], [1, 2], [1, 3]],
        };
        let summary = summarize_homotopy_poset(&record)?;
        assert_eq!(summary.shape, "four-point crown (K₂,₂)");
        assert_eq!(summary.core_order, 4);
        assert!(summary.is_interesting);
        assert_eq!(summary.integral_homology[1].free_rank, 1);
        Ok(())
    }

    #[test]
    fn crown_with_an_isolated_point_remains_interesting() -> Result<()> {
        let record = HomotopyPosetRecord {
            record_version: 1,
            key: PosetKey::new(vec!["1".to_owned(), "2".to_owned(), "3".to_owned()])?,
            order: 5,
            covers: vec![[0, 2], [0, 3], [1, 2], [1, 3]],
        };
        let summary = summarize_homotopy_poset(&record)?;
        assert_eq!(summary.shape, "four-point crown + isolated point");
        assert_eq!(summary.component_count, 2);
        assert!(summary.is_interesting);
        Ok(())
    }
}
