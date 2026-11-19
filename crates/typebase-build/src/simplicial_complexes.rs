//! Static TypeBase projection of complete finite-simplicial-complex shells.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use simplicial_complex_census::{
    jsonl::{load_cohomology_shell_version, load_identity_shell, load_topology_shell},
    records::{
        SimplicialComplexCohomologyRecord, SimplicialComplexIdentityRecord, SimplicialComplexKey,
        SimplicialComplexTopologyRecord,
    },
};

use crate::static_component::{
    Artifact, StagingDirectory, inspect_file, write_json, write_zstd_json,
};

const COMPONENT: &str = "typebase/volume-1/finite-simplicial-complexes";
const COLLECTION: &str = "finite-simplicial-complexes/v1";
const LANDING_ROWS: usize = 30;

#[derive(Clone, Debug)]
pub struct SimplicialComplexBuildOptions {
    pub data_root: PathBuf,
    pub output_root: PathBuf,
    pub maximum_vertices: usize,
    pub f3_cohomology_version: u32,
    pub rows_per_detail_part: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplicialComplexBuildOutcome {
    pub directory: PathBuf,
    pub complex_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct Label {
    vertices: usize,
    dimension: i64,
    index: usize,
}

impl std::fmt::Display for Label {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}.{}.{}",
            self.vertices, self.dimension, self.index
        )
    }
}

#[derive(Clone, Debug, Serialize)]
struct SearchRow {
    standard_names: Vec<String>,
    label: String,
    path: String,
    vertices: usize,
    dimension: i64,
    bucket_index: usize,
    facet_count: usize,
    f_vector: Vec<usize>,
    euler_characteristic: i64,
    component_count: usize,
    automorphism_order: String,
    core_label: String,
    core_size: usize,
    is_core: bool,
    collapse_target_label: String,
    collapse_step_count: usize,
    homology_signature: String,
    has_torsion: bool,
    f2_dimension: usize,
    f3_dimension: usize,
    steenrod_square_count: usize,
    detail_path: String,
    detail_row: usize,
}

#[derive(Serialize)]
struct DetailRow<'a> {
    standard_names: Vec<String>,
    label: String,
    core_label: String,
    collapse_target_label: String,
    identity: &'a SimplicialComplexIdentityRecord,
    topology: &'a SimplicialComplexTopologyRecord,
    cohomology_f2: &'a SimplicialComplexCohomologyRecord,
    cohomology_f3: &'a SimplicialComplexCohomologyRecord,
}

#[derive(Serialize)]
struct LabelBucket {
    vertices: usize,
    dimension: i64,
    row_count: usize,
    detail_part_count: usize,
}

#[derive(Serialize)]
struct ComponentChecks {
    complete_identity_shells: bool,
    topology_rows_resolved: bool,
    cohomology_rows_resolved: bool,
    strong_core_labels_resolved: bool,
    collapse_target_labels_resolved: bool,
    labels_unique_and_complete: bool,
    search_and_detail_counts_match: bool,
}

#[derive(Serialize)]
struct ComponentManifest {
    component: &'static str,
    builder_source: poset_census::SourceIdentity,
    status: &'static str,
    volume: u32,
    collection: &'static str,
    maximum_vertices: usize,
    complex_count: usize,
    rows_per_detail_part: usize,
    landing_rows: usize,
    search_sharded_by_label_bucket: bool,
    label_scheme: &'static str,
    coefficient_fields: [&'static str; 2],
    label_buckets: Vec<LabelBucket>,
    inputs: Vec<Artifact>,
    outputs: Vec<Artifact>,
    checks: ComponentChecks,
}

#[derive(Serialize)]
struct BrowserCatalogueManifest<'a> {
    component: &'static str,
    builder_source: poset_census::SourceIdentity,
    status: &'static str,
    collection: &'static str,
    complex_count: usize,
    rows_per_detail_part: usize,
    landing_rows: usize,
    search_sharded_by_label_bucket: bool,
    label_scheme: &'static str,
    label_buckets: &'a [LabelBucket],
}

pub fn build_simplicial_complex_component(
    options: &SimplicialComplexBuildOptions,
) -> Result<SimplicialComplexBuildOutcome> {
    ensure!(
        options.maximum_vertices <= 6,
        "first edition ends at six vertices"
    );
    ensure!(
        options.rows_per_detail_part > 0,
        "detail part size must be positive"
    );

    let mut identities_by_vertices = Vec::new();
    let mut labels = HashMap::<SimplicialComplexKey, Label>::new();
    let mut bucket_counts = BTreeMap::<(usize, i64), usize>::new();
    for vertices in 0..=options.maximum_vertices {
        let identities = load_identity_shell(&options.data_root, vertices)?;
        ensure!(
            identities.windows(2).all(|pair| pair[0].key < pair[1].key),
            "simplicial-complex keys are not strictly sorted"
        );
        for identity in &identities {
            let index = bucket_counts
                .entry((vertices, identity.dimension))
                .and_modify(|count| *count += 1)
                .or_insert(1);
            ensure!(
                labels
                    .insert(
                        identity.key.clone(),
                        Label {
                            vertices,
                            dimension: identity.dimension,
                            index: *index,
                        },
                    )
                    .is_none(),
                "duplicate simplicial-complex key"
            );
        }
        identities_by_vertices.push(identities);
    }

    let staging = StagingDirectory::new(&options.output_root)?;
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut search_rows = Vec::with_capacity(labels.len());
    let mut detail_rows_written = 0usize;
    for (vertices, identities) in identities_by_vertices.iter().enumerate() {
        let topology = load_topology_shell(&options.data_root, vertices)?;
        let f2 = load_cohomology_shell_version(&options.data_root, vertices, 2, 2)?;
        let f3 = load_cohomology_shell_version(
            &options.data_root,
            vertices,
            3,
            options.f3_cohomology_version,
        )?;
        ensure!(
            identities.len() == topology.len()
                && identities.len() == f2.len()
                && identities.len() == f3.len(),
            "simplicial-complex enrichment row counts differ"
        );
        inputs.extend(source_artifacts(
            &options.data_root,
            vertices,
            identities.len(),
            options.f3_cohomology_version,
        )?);

        let mut detail_buffers = BTreeMap::<i64, Vec<DetailRow<'_>>>::new();
        let mut detail_parts = BTreeMap::<i64, usize>::new();
        for (((identity, topology), f2), f3) in identities.iter().zip(&topology).zip(&f2).zip(&f3) {
            ensure!(
                identity.key == topology.complex
                    && identity.key == f2.complex
                    && identity.key == f3.complex,
                "simplicial-complex enrichment keys differ"
            );
            let label = labels[&identity.key];
            let core_label = labels
                .get(&topology.strong_core.target)
                .context("strong core lies outside the complete frontier")?
                .to_string();
            let collapse_target_label = labels
                .get(&topology.greedy_collapse.target)
                .context("collapse target lies outside the complete frontier")?
                .to_string();
            let part = detail_parts.entry(identity.dimension).or_default();
            let buffer = detail_buffers.entry(identity.dimension).or_default();
            let detail_file = detail_path(vertices, identity.dimension, *part);
            let detail_row = buffer.len();
            let standard_names =
                crate::named_spaces::simplicial_complex(vertices, &identity.facets);
            search_rows.push(SearchRow {
                standard_names: standard_names.clone(),
                label: label.to_string(),
                path: format!("/SimplicialComplex/{label}"),
                vertices,
                dimension: identity.dimension,
                bucket_index: label.index,
                facet_count: identity.facets.len(),
                f_vector: topology.f_vector.clone(),
                euler_characteristic: topology.euler_characteristic,
                component_count: topology.component_sizes.len(),
                automorphism_order: identity.automorphism_group.order.clone(),
                core_label: core_label.clone(),
                core_size: topology.strong_core.target_vertices,
                is_core: topology.strong_core.steps.is_empty(),
                collapse_target_label: collapse_target_label.clone(),
                collapse_step_count: topology.greedy_collapse.steps.len(),
                homology_signature: homology_signature(topology),
                has_torsion: topology
                    .integral_homology
                    .iter()
                    .any(|group| !group.torsion_coefficients.is_empty()),
                f2_dimension: f2.basis.len(),
                f3_dimension: f3.basis.len(),
                steenrod_square_count: f2.operations.len(),
                detail_path: detail_file,
                detail_row,
            });
            buffer.push(DetailRow {
                standard_names,
                label: label.to_string(),
                core_label,
                collapse_target_label,
                identity,
                topology,
                cohomology_f2: f2,
                cohomology_f3: f3,
            });
            if buffer.len() == options.rows_per_detail_part {
                outputs.push(write_zstd_json(
                    &staging.path,
                    &detail_path(vertices, identity.dimension, *part),
                    "details",
                    u64::try_from(buffer.len())?,
                    buffer,
                )?);
                detail_rows_written += buffer.len();
                buffer.clear();
                *part += 1;
            }
        }
        for (dimension, buffer) in &mut detail_buffers {
            if buffer.is_empty() {
                continue;
            }
            let part = detail_parts.entry(*dimension).or_default();
            outputs.push(write_zstd_json(
                &staging.path,
                &detail_path(vertices, *dimension, *part),
                "details",
                u64::try_from(buffer.len())?,
                buffer,
            )?);
            detail_rows_written += buffer.len();
            buffer.clear();
            *part += 1;
        }
    }
    ensure!(detail_rows_written == labels.len());
    search_rows.sort_by_key(|row| (row.vertices, row.dimension, row.bucket_index));
    ensure!(
        search_rows
            .iter()
            .map(|row| row.label.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            == labels.len(),
        "TypeBase simplicial-complex labels are not unique"
    );
    outputs.push(write_zstd_json(
        &staging.path,
        "search.json.zst",
        "search",
        u64::try_from(search_rows.len())?,
        &search_rows,
    )?);
    let landing_rows = search_rows.len().min(LANDING_ROWS);
    outputs.push(write_json(
        &staging.path,
        "landing.json",
        "landing",
        u64::try_from(landing_rows)?,
        &&search_rows[..landing_rows],
    )?);
    let mut search_buckets = BTreeMap::<(usize, i64), Vec<&SearchRow>>::new();
    for row in &search_rows {
        search_buckets
            .entry((row.vertices, row.dimension))
            .or_default()
            .push(row);
    }
    for ((vertices, dimension), rows) in search_buckets {
        outputs.push(write_zstd_json(
            &staging.path,
            &format!("search/vertices={vertices}/dimension={dimension}.json.zst"),
            "search_bucket",
            u64::try_from(rows.len())?,
            &rows,
        )?);
    }

    let label_buckets: Vec<LabelBucket> = bucket_counts
        .into_iter()
        .map(|((vertices, dimension), row_count)| LabelBucket {
            vertices,
            dimension,
            row_count,
            detail_part_count: row_count.div_ceil(options.rows_per_detail_part),
        })
        .collect();
    outputs.push(write_json(
        &staging.path,
        "catalogue.json",
        "catalogue_manifest",
        1,
        &BrowserCatalogueManifest {
            component: COMPONENT,
            builder_source: poset_census::source_identity(),
            status: "passed",
            collection: COLLECTION,
            complex_count: labels.len(),
            rows_per_detail_part: options.rows_per_detail_part,
            landing_rows,
            search_sharded_by_label_bucket: true,
            label_scheme: "vertices.dimension.canonical-index/v1",
            label_buckets: &label_buckets,
        },
    )?);
    let manifest = ComponentManifest {
        component: COMPONENT,
        builder_source: poset_census::source_identity(),
        status: "passed",
        volume: 1,
        collection: COLLECTION,
        maximum_vertices: options.maximum_vertices,
        complex_count: labels.len(),
        rows_per_detail_part: options.rows_per_detail_part,
        landing_rows,
        search_sharded_by_label_bucket: true,
        label_scheme: "vertices.dimension.canonical-index/v1",
        coefficient_fields: ["F2", "F3"],
        label_buckets,
        inputs,
        outputs,
        checks: ComponentChecks {
            complete_identity_shells: true,
            topology_rows_resolved: true,
            cohomology_rows_resolved: true,
            strong_core_labels_resolved: true,
            collapse_target_labels_resolved: true,
            labels_unique_and_complete: true,
            search_and_detail_counts_match: true,
        },
    };
    write_json(&staging.path, "manifest.json", "manifest", 1, &manifest)?;
    staging.install()?;
    Ok(SimplicialComplexBuildOutcome {
        directory: options.output_root.clone(),
        complex_count: labels.len(),
    })
}

fn detail_path(vertices: usize, dimension: i64, part: usize) -> String {
    format!("details/vertices={vertices}/dimension={dimension}/part-{part:05}.json.zst")
}

fn homology_signature(topology: &SimplicialComplexTopologyRecord) -> String {
    topology
        .integral_homology
        .iter()
        .map(|group| {
            format!(
                "{}:{}:{}",
                group.degree,
                group.free_rank,
                group.torsion_coefficients.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn source_artifacts(
    data_root: &Path,
    vertices: usize,
    rows: usize,
    f3_version: u32,
) -> Result<Vec<Artifact>> {
    let version = data_root.join("finite-simplicial-complexes/v1");
    let sources = [
        (
            version.join(format!("vertices-{vertices}/manifest.json")),
            format!("finite-simplicial-complexes/v1/vertices-{vertices}/manifest.json"),
            "identity_manifest",
            1,
        ),
        (
            version.join(format!("vertices-{vertices}/identities.jsonl")),
            format!("finite-simplicial-complexes/v1/vertices-{vertices}/identities.jsonl"),
            "identities",
            rows,
        ),
        (
            version.join(format!(
                "enrichments/topology-v1/vertices-{vertices}/manifest.json"
            )),
            format!(
                "finite-simplicial-complexes/v1/enrichments/topology-v1/vertices-{vertices}/manifest.json"
            ),
            "topology_manifest",
            1,
        ),
        (
            version.join(format!(
                "enrichments/topology-v1/vertices-{vertices}/topology.jsonl"
            )),
            format!(
                "finite-simplicial-complexes/v1/enrichments/topology-v1/vertices-{vertices}/topology.jsonl"
            ),
            "topology",
            rows,
        ),
        (
            version.join(format!(
                "enrichments/cohomology-f2-v2/vertices-{vertices}/manifest.json"
            )),
            format!(
                "finite-simplicial-complexes/v1/enrichments/cohomology-f2-v2/vertices-{vertices}/manifest.json"
            ),
            "cohomology_manifest",
            1,
        ),
        (
            version.join(format!(
                "enrichments/cohomology-f2-v2/vertices-{vertices}/cohomology.jsonl"
            )),
            format!(
                "finite-simplicial-complexes/v1/enrichments/cohomology-f2-v2/vertices-{vertices}/cohomology.jsonl"
            ),
            "cohomology",
            rows,
        ),
        (
            version.join(format!(
                "enrichments/cohomology-f3-v{f3_version}/vertices-{vertices}/manifest.json"
            )),
            format!(
                "finite-simplicial-complexes/v1/enrichments/cohomology-f3-v{f3_version}/vertices-{vertices}/manifest.json"
            ),
            "cohomology_manifest",
            1,
        ),
        (
            version.join(format!(
                "enrichments/cohomology-f3-v{f3_version}/vertices-{vertices}/cohomology.jsonl"
            )),
            format!(
                "finite-simplicial-complexes/v1/enrichments/cohomology-f3-v{f3_version}/vertices-{vertices}/cohomology.jsonl"
            ),
            "cohomology",
            rows,
        ),
    ];
    ensure!(
        sources.iter().all(|(path, _, _, _)| path.is_file()),
        "selected simplicial-complex source artifact is missing"
    );
    sources
        .into_iter()
        .map(|(path, published, kind, row_count)| {
            inspect_file(&path, &published, kind, u64::try_from(row_count)?)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_and_detail_paths_include_the_empty_dimension() {
        assert_eq!(
            Label {
                vertices: 0,
                dimension: -1,
                index: 1
            }
            .to_string(),
            "0.-1.1"
        );
        assert_eq!(
            detail_path(0, -1, 0),
            "details/vertices=0/dimension=-1/part-00000.json.zst"
        );
    }
}
