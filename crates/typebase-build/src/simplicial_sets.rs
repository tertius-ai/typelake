//! Static TypeBase projection of the first finite-simplicial-set frontier.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::File,
    io::{BufRead, BufReader, Lines},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use sha2::Digest;
use simplicial_set_census::records::{
    ElementaryHornCollapseRecord, SimplicialSetCohomologyRecord, SimplicialSetHomotopyRecord,
    SimplicialSetIdentityRecord, SimplicialSetKey, SimplicialSetSteenrodOperationsRecord,
    SimplicialSetTopologyRecord,
};

use crate::static_component::{
    Artifact, StagingDirectory, inspect_file, write_json, write_zstd_json,
};

const COMPONENT: &str = "typebase/volume-1/finite-simplicial-sets";
const COLLECTION: &str = "finite-simplicial-sets/v1";
const LANDING_ROWS: usize = 30;

#[derive(Clone, Debug)]
pub struct SimplicialSetBuildOptions {
    pub data_root: PathBuf,
    pub output_root: PathBuf,
    pub maximum_total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub maximum_dimension_at_top_total: usize,
    pub rows_per_detail_part: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplicialSetBuildOutcome {
    pub directory: PathBuf,
    pub simplicial_set_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Label {
    total_nondegenerate: usize,
    dimension: i64,
    index: usize,
    catalogue_index: usize,
}

impl std::fmt::Display for Label {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}.{}.{}",
            self.total_nondegenerate, self.dimension, self.index
        )
    }
}

#[derive(Serialize)]
struct SearchRow {
    standard_names: Vec<String>,
    label: String,
    path: String,
    total_nondegenerate: usize,
    dimension: i64,
    bucket_index: usize,
    nondegenerate_counts: Vec<usize>,
    euler_characteristic: i64,
    automorphism_order: String,
    normalized_source_cells: usize,
    normalized_reduced_cells: usize,
    chain_cancellation_count: usize,
    homology_signature: String,
    has_torsion: bool,
    immediate_horn_collapse_count: usize,
    greedy_target_label: String,
    greedy_step_count: usize,
    witnessed_component_label: String,
    witnessed_component_size: usize,
    f2_dimension: usize,
    f3_dimension: usize,
    steenrod_square_count: usize,
    odd_primary_operation_count: usize,
    detail_path: String,
    detail_row: usize,
    #[serde(skip)]
    catalogue_index: usize,
}

#[derive(Serialize)]
struct HornTargetLabel {
    step: ElementaryHornCollapseRecord,
    target_label: String,
}

#[derive(Serialize)]
struct DetailRow {
    standard_names: Vec<String>,
    label: String,
    elementary_horn_relations: Vec<HornTargetLabel>,
    greedy_target_label: String,
    identity: SimplicialSetIdentityRecord,
    topology: SimplicialSetTopologyRecord,
    cohomology_f2: SimplicialSetCohomologyRecord,
    cohomology_f3: SimplicialSetCohomologyRecord,
    steenrod_operations_f3: SimplicialSetSteenrodOperationsRecord,
    homotopy: SimplicialSetHomotopyRecord,
}

#[derive(Serialize)]
struct FrontierShell {
    total_nondegenerate: usize,
    maximum_dimension: usize,
    row_count: usize,
}

#[derive(Serialize)]
struct LabelBucket {
    total_nondegenerate: usize,
    dimension: i64,
    row_count: usize,
    detail_part_count: usize,
}

#[derive(Serialize)]
struct ComponentChecks {
    complete_identity_frontier: bool,
    topology_rows_resolved: bool,
    cohomology_rows_resolved: bool,
    odd_primary_sidecars_linked: bool,
    horn_targets_resolved: bool,
    witnessed_components_reconciled: bool,
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
    simplicial_set_count: usize,
    rows_per_detail_part: usize,
    landing_rows: usize,
    search_sharded_by_label_bucket: bool,
    label_scheme: &'static str,
    coefficient_fields: [&'static str; 2],
    frontier: Vec<FrontierShell>,
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
    simplicial_set_count: usize,
    rows_per_detail_part: usize,
    landing_rows: usize,
    search_sharded_by_label_bucket: bool,
    label_scheme: &'static str,
    label_buckets: &'a [LabelBucket],
}

pub fn build_simplicial_set_component(
    options: &SimplicialSetBuildOptions,
) -> Result<SimplicialSetBuildOutcome> {
    ensure!(
        options.maximum_total_nondegenerate == 7
            && options.maximum_dimension == 3
            && options.maximum_dimension_at_top_total == 2,
        "the selected first edition has frontier (total <= 6, dimension <= 3) plus (total = 7, dimension <= 2)"
    );
    ensure!(
        options.rows_per_detail_part > 0,
        "detail part size must be positive"
    );
    let names = crate::named_spaces::simplicial_sets()?;
    let shells = frontier(options);
    let mut labels = HashMap::<SimplicialSetKey, Label>::new();
    let mut bucket_counts = BTreeMap::<(usize, i64), usize>::new();
    let mut inputs = Vec::new();
    for &(total, maximum_dimension) in &shells {
        let shell_inputs = check_identity_source(&options.data_root, total, maximum_dimension)?;
        let identity_path = identity_directory(&options.data_root, total, maximum_dimension)
            .join("identities.jsonl");
        let mut lines = jsonl_lines(&identity_path)?;
        let mut observed_identity_rows = 0usize;
        let mut previous_key = None;
        while let Some(identity) = next_record::<SimplicialSetIdentityRecord>(&mut lines)? {
            identity.check_shape()?;
            ensure!(
                previous_key.as_ref().is_none_or(|key| key < &identity.key),
                "simplicial-set keys are not strictly sorted"
            );
            previous_key = Some(identity.key.clone());
            let index = bucket_counts
                .entry((total, identity.dimension))
                .and_modify(|count| *count += 1)
                .or_insert(1);
            let catalogue_index = labels.len();
            ensure!(
                labels
                    .insert(
                        identity.key,
                        Label {
                            total_nondegenerate: total,
                            dimension: identity.dimension,
                            index: *index,
                            catalogue_index,
                        },
                    )
                    .is_none(),
                "duplicate simplicial-set key"
            );
            observed_identity_rows += 1;
        }
        ensure!(
            observed_identity_rows == shell_inputs.row_count,
            "identity JSONL row count differs from its immutable manifest"
        );
        inputs.extend(shell_inputs.artifacts);
    }

    let staging = StagingDirectory::new(&options.output_root)?;
    let mut outputs = Vec::new();
    let mut search_rows = Vec::with_capacity(labels.len());
    let mut union_find = UnionFind::new(labels.len());
    let mut detail_rows_written = 0usize;
    let mut frontier_rows = Vec::new();
    for &(total, maximum_dimension) in &shells {
        let sources = check_enrichment_sources(&options.data_root, total, maximum_dimension)?;
        inputs.extend(sources.artifacts);
        frontier_rows.push(FrontierShell {
            total_nondegenerate: total,
            maximum_dimension,
            row_count: sources.row_count,
        });
        let mut identity_lines = jsonl_lines(&sources.identity)?;
        let mut topology_lines = jsonl_lines(&sources.topology)?;
        let mut f2_lines = jsonl_lines(&sources.cohomology_f2)?;
        let mut f3_lines = jsonl_lines(&sources.cohomology_f3)?;
        let mut odd_lines = jsonl_lines(&sources.steenrod_f3)?;
        let mut homotopy_lines = jsonl_lines(&sources.homotopy)?;
        let mut buffers = BTreeMap::<i64, Vec<DetailRow>>::new();
        let mut parts = BTreeMap::<i64, usize>::new();
        let mut observed_rows = 0usize;
        while let Some(identity) = next_record::<SimplicialSetIdentityRecord>(&mut identity_lines)?
        {
            let topology = required_next::<SimplicialSetTopologyRecord>(&mut topology_lines)?;
            let f2 = required_next::<SimplicialSetCohomologyRecord>(&mut f2_lines)?;
            let f3 = required_next::<SimplicialSetCohomologyRecord>(&mut f3_lines)?;
            let odd = required_next::<SimplicialSetSteenrodOperationsRecord>(&mut odd_lines)?;
            let homotopy = required_next::<SimplicialSetHomotopyRecord>(&mut homotopy_lines)?;
            ensure!(
                identity.key == topology.simplicial_set
                    && identity.key == f2.simplicial_set
                    && identity.key == f3.simplicial_set
                    && identity.key == odd.simplicial_set
                    && identity.key == homotopy.simplicial_set,
                "simplicial-set enrichment keys differ"
            );
            ensure!(
                odd.cohomology_record_sha256
                    == hex::encode(sha2::Sha256::digest(serde_json::to_vec(&f3)?)),
                "Steenrod cohomology-record digest mismatch"
            );
            let label = labels[&identity.key];
            let elementary_horn_relations = homotopy
                .elementary_horn_relations
                .iter()
                .map(|relation| {
                    let target = labels
                        .get(&relation.target)
                        .context("horn-collapse target lies outside the complete frontier")?;
                    union_find.union(label.catalogue_index, target.catalogue_index);
                    Ok(HornTargetLabel {
                        step: relation.step.clone(),
                        target_label: target.to_string(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let greedy_target = labels
                .get(&homotopy.greedy_horn_collapse.target)
                .context("greedy horn-collapse target lies outside the complete frontier")?;
            union_find.union(label.catalogue_index, greedy_target.catalogue_index);
            let part = parts.entry(identity.dimension).or_default();
            let buffer = buffers.entry(identity.dimension).or_default();
            let detail_file = detail_path(total, identity.dimension, *part);
            let standard_names = names.get(&identity.key).cloned().unwrap_or_default();
            search_rows.push(SearchRow {
                standard_names: standard_names.clone(),
                label: label.to_string(),
                path: format!("/SimplicialSet/{label}"),
                total_nondegenerate: total,
                dimension: identity.dimension,
                bucket_index: label.index,
                nondegenerate_counts: identity.nondegenerate_counts.clone(),
                euler_characteristic: topology.euler_characteristic,
                automorphism_order: identity.automorphism_group.order.clone(),
                normalized_source_cells: topology.normalized_chain_reduction.source_cells,
                normalized_reduced_cells: topology.normalized_chain_reduction.reduced_cells,
                chain_cancellation_count: topology.normalized_chain_reduction.cancellations.len(),
                homology_signature: homology_signature(&topology),
                has_torsion: topology
                    .integral_homology
                    .iter()
                    .any(|group| !group.torsion_coefficients.is_empty()),
                immediate_horn_collapse_count: elementary_horn_relations.len(),
                greedy_target_label: greedy_target.to_string(),
                greedy_step_count: homotopy.greedy_horn_collapse.steps.len(),
                witnessed_component_label: String::new(),
                witnessed_component_size: 0,
                f2_dimension: f2.basis.len(),
                f3_dimension: f3.basis.len(),
                steenrod_square_count: f2.operations.len(),
                odd_primary_operation_count: odd.operations.len(),
                detail_path: detail_file,
                detail_row: buffer.len(),
                catalogue_index: label.catalogue_index,
            });
            buffer.push(DetailRow {
                standard_names,
                label: label.to_string(),
                elementary_horn_relations,
                greedy_target_label: greedy_target.to_string(),
                identity,
                topology,
                cohomology_f2: f2,
                cohomology_f3: f3,
                steenrod_operations_f3: odd,
                homotopy,
            });
            if buffer.len() == options.rows_per_detail_part {
                outputs.push(write_zstd_json(
                    &staging.path,
                    &detail_path(total, label.dimension, *part),
                    "details",
                    u64::try_from(buffer.len())?,
                    buffer,
                )?);
                detail_rows_written += buffer.len();
                buffer.clear();
                *part += 1;
            }
            observed_rows += 1;
        }
        ensure!(observed_rows == sources.row_count);
        ensure!(next_record::<SimplicialSetTopologyRecord>(&mut topology_lines)?.is_none());
        ensure!(next_record::<SimplicialSetCohomologyRecord>(&mut f2_lines)?.is_none());
        ensure!(next_record::<SimplicialSetCohomologyRecord>(&mut f3_lines)?.is_none());
        ensure!(next_record::<SimplicialSetSteenrodOperationsRecord>(&mut odd_lines)?.is_none());
        ensure!(next_record::<SimplicialSetHomotopyRecord>(&mut homotopy_lines)?.is_none());
        for (dimension, buffer) in &mut buffers {
            if buffer.is_empty() {
                continue;
            }
            let part = parts.entry(*dimension).or_default();
            outputs.push(write_zstd_json(
                &staging.path,
                &detail_path(total, *dimension, *part),
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
    populate_components(&mut search_rows, &mut union_find);
    search_rows.sort_by_key(|row| (row.total_nondegenerate, row.dimension, row.bucket_index));
    ensure!(
        search_rows
            .iter()
            .map(|row| row.label.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            == labels.len(),
        "TypeBase simplicial-set labels are not unique"
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
            .entry((row.total_nondegenerate, row.dimension))
            .or_default()
            .push(row);
    }
    for ((total, dimension), rows) in search_buckets {
        outputs.push(write_zstd_json(
            &staging.path,
            &format!("search/total={total}/dimension={dimension}.json.zst"),
            "search_bucket",
            u64::try_from(rows.len())?,
            &rows,
        )?);
    }

    let label_buckets: Vec<LabelBucket> = bucket_counts
        .into_iter()
        .map(
            |((total_nondegenerate, dimension), row_count)| LabelBucket {
                total_nondegenerate,
                dimension,
                row_count,
                detail_part_count: row_count.div_ceil(options.rows_per_detail_part),
            },
        )
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
            simplicial_set_count: labels.len(),
            rows_per_detail_part: options.rows_per_detail_part,
            landing_rows,
            search_sharded_by_label_bucket: true,
            label_scheme: "nondegenerate-simplices.dimension.canonical-index/v1",
            label_buckets: &label_buckets,
        },
    )?);
    let manifest = ComponentManifest {
        component: COMPONENT,
        builder_source: poset_census::source_identity(),
        status: "passed",
        volume: 1,
        collection: COLLECTION,
        simplicial_set_count: labels.len(),
        rows_per_detail_part: options.rows_per_detail_part,
        landing_rows,
        search_sharded_by_label_bucket: true,
        label_scheme: "nondegenerate-simplices.dimension.canonical-index/v1",
        coefficient_fields: ["F2", "F3"],
        frontier: frontier_rows,
        label_buckets,
        inputs,
        outputs,
        checks: ComponentChecks {
            complete_identity_frontier: true,
            topology_rows_resolved: true,
            cohomology_rows_resolved: true,
            odd_primary_sidecars_linked: true,
            horn_targets_resolved: true,
            witnessed_components_reconciled: true,
            labels_unique_and_complete: true,
            search_and_detail_counts_match: true,
        },
    };
    write_json(&staging.path, "manifest.json", "manifest", 1, &manifest)?;
    staging.install()?;
    Ok(SimplicialSetBuildOutcome {
        directory: options.output_root.clone(),
        simplicial_set_count: labels.len(),
    })
}

fn frontier(options: &SimplicialSetBuildOptions) -> Vec<(usize, usize)> {
    (0..options.maximum_total_nondegenerate)
        .map(|total| (total, options.maximum_dimension))
        .chain(std::iter::once((
            options.maximum_total_nondegenerate,
            options.maximum_dimension_at_top_total,
        )))
        .collect()
}

fn identity_directory(root: &Path, total: usize, maximum_dimension: usize) -> PathBuf {
    root.join("finite-simplicial-sets/v1")
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total}"))
}

fn enrichment_directory(
    root: &Path,
    enrichment: &str,
    total: usize,
    maximum_dimension: usize,
) -> PathBuf {
    root.join("finite-simplicial-sets/v1/enrichments")
        .join(enrichment)
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total}"))
}

struct CheckedInputs {
    row_count: usize,
    artifacts: Vec<Artifact>,
}

struct EnrichmentSources {
    row_count: usize,
    identity: PathBuf,
    topology: PathBuf,
    cohomology_f2: PathBuf,
    cohomology_f3: PathBuf,
    steenrod_f3: PathBuf,
    homotopy: PathBuf,
    artifacts: Vec<Artifact>,
}

fn check_identity_source(
    root: &Path,
    total: usize,
    maximum_dimension: usize,
) -> Result<CheckedInputs> {
    let directory = identity_directory(root, total, maximum_dimension);
    let manifest_path = directory.join("manifest.json");
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    ensure!(manifest["contract"] == "finite-simplicial-sets/v1");
    check_source_revision(&manifest)?;
    ensure!(manifest["total_nondegenerate"] == total);
    ensure!(manifest["maximum_dimension"] == maximum_dimension);
    let record = manifest_artifact(&manifest, "identities.jsonl")?;
    let row_count = usize::try_from(record.row_count)?;
    let prefix = format!("finite-simplicial-sets/v1/dimension-{maximum_dimension}/total-{total}");
    let observed = inspect_file(
        &directory.join("identities.jsonl"),
        &format!("{prefix}/identities.jsonl"),
        "identities",
        record.row_count,
    )?;
    ensure_source_artifact(&observed, &record)?;
    Ok(CheckedInputs {
        row_count,
        artifacts: vec![
            inspect_file(
                &manifest_path,
                &format!("{prefix}/manifest.json"),
                "identity_manifest",
                1,
            )?,
            observed,
        ],
    })
}

fn check_enrichment_sources(
    root: &Path,
    total: usize,
    maximum_dimension: usize,
) -> Result<EnrichmentSources> {
    let identity_directory = identity_directory(root, total, maximum_dimension);
    let identity = identity_directory.join("identities.jsonl");
    let identity_manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(identity_directory.join("manifest.json"))?)?;
    let identity_record = manifest_artifact(&identity_manifest, "identities.jsonl")?;
    let identity_sha = identity_record.sha256.clone();
    let row_count = usize::try_from(identity_record.row_count)?;
    let specifications = [
        (
            "topology-v2",
            "topology.jsonl",
            "finite-simplicial-sets/topology-v2",
            "topology",
            None,
        ),
        (
            "cohomology-f2-v1",
            "cohomology.jsonl",
            "finite-simplicial-sets/cohomology-v1",
            "cohomology",
            Some("F2"),
        ),
        (
            "cohomology-f3-v1",
            "cohomology.jsonl",
            "finite-simplicial-sets/cohomology-v1",
            "cohomology",
            Some("F3"),
        ),
        (
            "steenrod-operations-f3-v1",
            "steenrod-operations.jsonl",
            "finite-simplicial-sets/steenrod-operations-v1",
            "steenrod_operations",
            Some("F3"),
        ),
        (
            "homotopy-v1",
            "homotopy.jsonl",
            "finite-simplicial-sets/homotopy-v1",
            "homotopy",
            None,
        ),
    ];
    let mut artifacts = Vec::new();
    let mut paths = Vec::new();
    for (enrichment, file_name, contract, kind, coefficient) in specifications {
        let directory = enrichment_directory(root, enrichment, total, maximum_dimension);
        let manifest_path = directory.join("manifest.json");
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
        ensure!(manifest["contract"] == contract);
        check_source_revision(&manifest)?;
        ensure!(manifest["total_nondegenerate"] == total);
        ensure!(manifest["maximum_dimension"] == maximum_dimension);
        ensure!(manifest["identity_sha256"] == identity_sha);
        if let Some(coefficient) = coefficient {
            ensure!(manifest["coefficient"] == coefficient);
        }
        let expected = manifest_artifact(&manifest, file_name)?;
        ensure!(usize::try_from(expected.row_count)? == row_count);
        let prefix = format!(
            "finite-simplicial-sets/v1/enrichments/{enrichment}/dimension-{maximum_dimension}/total-{total}"
        );
        let observed = inspect_file(
            &directory.join(file_name),
            &format!("{prefix}/{file_name}"),
            kind,
            expected.row_count,
        )?;
        ensure_source_artifact(&observed, &expected)?;
        artifacts.push(inspect_file(
            &manifest_path,
            &format!("{prefix}/manifest.json"),
            &format!("{kind}_manifest"),
            1,
        )?);
        artifacts.push(observed);
        paths.push(directory.join(file_name));
    }
    let f3_cohomology = manifest_artifact(
        &serde_json::from_slice::<serde_json::Value>(&std::fs::read(
            enrichment_directory(root, "cohomology-f3-v1", total, maximum_dimension)
                .join("manifest.json"),
        )?)?,
        "cohomology.jsonl",
    )?;
    let steenrod_manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
        enrichment_directory(root, "steenrod-operations-f3-v1", total, maximum_dimension)
            .join("manifest.json"),
    )?)?;
    ensure!(steenrod_manifest["cohomology_sha256"] == f3_cohomology.sha256);
    Ok(EnrichmentSources {
        row_count,
        identity,
        topology: paths[0].clone(),
        cohomology_f2: paths[1].clone(),
        cohomology_f3: paths[2].clone(),
        steenrod_f3: paths[3].clone(),
        homotopy: paths[4].clone(),
        artifacts,
    })
}

struct SourceArtifact {
    row_count: u64,
    byte_count: u64,
    sha256: String,
}

fn manifest_artifact(manifest: &serde_json::Value, path: &str) -> Result<SourceArtifact> {
    let artifact = manifest["artifacts"]
        .as_array()
        .context("source manifest has no artifact array")?
        .iter()
        .find(|artifact| artifact["path"] == path)
        .context("source manifest omits the required JSONL artifact")?;
    Ok(SourceArtifact {
        row_count: artifact["row_count"]
            .as_u64()
            .context("source artifact row count")?,
        byte_count: artifact["byte_count"]
            .as_u64()
            .context("source artifact byte count")?,
        sha256: artifact["sha256"]
            .as_str()
            .context("source artifact hash")?
            .to_owned(),
    })
}

fn ensure_source_artifact(observed: &Artifact, expected: &SourceArtifact) -> Result<()> {
    ensure!(
        observed.row_count == expected.row_count
            && observed.byte_count == expected.byte_count
            && observed.sha256 == expected.sha256,
        "source artifact differs from its immutable manifest"
    );
    Ok(())
}

fn jsonl_lines(path: &Path) -> Result<Lines<BufReader<File>>> {
    Ok(BufReader::new(File::open(path)?).lines())
}

fn next_record<T: DeserializeOwned>(lines: &mut Lines<BufReader<File>>) -> Result<Option<T>> {
    lines
        .next()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .transpose()
}

fn required_next<T: DeserializeOwned>(lines: &mut Lines<BufReader<File>>) -> Result<T> {
    next_record(lines)?.context("enrichment JSONL ended before its identity source")
}

fn detail_path(total: usize, dimension: i64, part: usize) -> String {
    format!("details/total={total}/dimension={dimension}/part-{part:05}.json.zst")
}

fn homology_signature(topology: &SimplicialSetTopologyRecord) -> String {
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

fn populate_components(search_rows: &mut [SearchRow], union_find: &mut UnionFind) {
    let mut component_sizes = HashMap::<usize, usize>::new();
    let mut component_labels = HashMap::<usize, (usize, String)>::new();
    for row in search_rows.iter() {
        let root = union_find.find(row.catalogue_index);
        *component_sizes.entry(root).or_default() += 1;
        component_labels
            .entry(root)
            .and_modify(|(catalogue_index, label)| {
                if row.catalogue_index < *catalogue_index {
                    *catalogue_index = row.catalogue_index;
                    *label = row.label.clone();
                }
            })
            .or_insert_with(|| (row.catalogue_index, row.label.clone()));
    }
    for row in search_rows {
        let root = union_find.find(row.catalogue_index);
        row.witnessed_component_label = component_labels[&root].1.clone();
        row.witnessed_component_size = component_sizes[&root];
    }
}

struct UnionFind {
    parents: Vec<usize>,
    ranks: Vec<u8>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parents: (0..size).collect(),
            ranks: vec![0; size],
        }
    }

    fn find(&mut self, value: usize) -> usize {
        if self.parents[value] != value {
            self.parents[value] = self.find(self.parents[value]);
        }
        self.parents[value]
    }

    fn union(&mut self, left: usize, right: usize) {
        let mut left_root = self.find(left);
        let mut right_root = self.find(right);
        if left_root == right_root {
            return;
        }
        if self.ranks[left_root] < self.ranks[right_root] {
            std::mem::swap(&mut left_root, &mut right_root);
        }
        self.parents[right_root] = left_root;
        if self.ranks[left_root] == self.ranks[right_root] {
            self.ranks[left_root] += 1;
        }
    }
}

fn check_source_revision(manifest: &serde_json::Value) -> Result<()> {
    let revision = manifest["noether_revision"]
        .as_str()
        .context("missing producer revision")?;
    ensure!(
        simplicial_set_census::jsonl::accepted_identity_noether_revision(revision),
        "unsupported source producer revision"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_edition_frontier_and_labels_are_explicit() {
        let options = SimplicialSetBuildOptions {
            data_root: PathBuf::new(),
            output_root: PathBuf::new(),
            maximum_total_nondegenerate: 7,
            maximum_dimension: 3,
            maximum_dimension_at_top_total: 2,
            rows_per_detail_part: 128,
        };
        assert_eq!(frontier(&options).last(), Some(&(7, 2)));
        assert_eq!(frontier(&options).len(), 8);
        assert_eq!(
            Label {
                total_nondegenerate: 0,
                dimension: -1,
                index: 1,
                catalogue_index: 0,
            }
            .to_string(),
            "0.-1.1"
        );
    }

    #[test]
    fn witnessed_components_use_the_first_canonical_catalogue_member() {
        let mut rows = vec![
            SearchRow {
                standard_names: Vec::new(),
                label: "2.1.2".to_owned(),
                path: String::new(),
                total_nondegenerate: 2,
                dimension: 1,
                bucket_index: 2,
                nondegenerate_counts: Vec::new(),
                euler_characteristic: 0,
                automorphism_order: "1".to_owned(),
                normalized_source_cells: 0,
                normalized_reduced_cells: 0,
                chain_cancellation_count: 0,
                homology_signature: String::new(),
                has_torsion: false,
                immediate_horn_collapse_count: 0,
                greedy_target_label: String::new(),
                greedy_step_count: 0,
                witnessed_component_label: String::new(),
                witnessed_component_size: 0,
                f2_dimension: 0,
                f3_dimension: 0,
                steenrod_square_count: 0,
                odd_primary_operation_count: 0,
                detail_path: String::new(),
                detail_row: 0,
                catalogue_index: 0,
            },
            SearchRow {
                standard_names: Vec::new(),
                label: "1.0.1".to_owned(),
                catalogue_index: 1,
                ..empty_search_row()
            },
        ];
        let mut union_find = UnionFind::new(2);
        union_find.union(0, 1);
        populate_components(&mut rows, &mut union_find);
        assert!(
            rows.iter()
                .all(|row| row.witnessed_component_label == "2.1.2")
        );
        assert!(rows.iter().all(|row| row.witnessed_component_size == 2));
    }

    fn empty_search_row() -> SearchRow {
        SearchRow {
            standard_names: Vec::new(),
            label: String::new(),
            path: String::new(),
            total_nondegenerate: 0,
            dimension: 0,
            bucket_index: 0,
            nondegenerate_counts: Vec::new(),
            euler_characteristic: 0,
            automorphism_order: String::new(),
            normalized_source_cells: 0,
            normalized_reduced_cells: 0,
            chain_cancellation_count: 0,
            homology_signature: String::new(),
            has_torsion: false,
            immediate_horn_collapse_count: 0,
            greedy_target_label: String::new(),
            greedy_step_count: 0,
            witnessed_component_label: String::new(),
            witnessed_component_size: 0,
            f2_dimension: 0,
            f3_dimension: 0,
            steenrod_square_count: 0,
            odd_primary_operation_count: 0,
            detail_path: String::new(),
            detail_row: 0,
            catalogue_index: 0,
        }
    }
}
