//! TypeBase Volume I projection of the certified finite-Poset census.

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process,
    sync::Arc,
};

use anyhow::{Context, Result, ensure};
use arrow_array::{
    ArrayRef, BooleanArray, Int8Array, RecordBatch, StringArray, UInt8Array, UInt16Array,
    UInt32Array,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use parquet::{
    arrow::ArrowWriter,
    basic::{Compression, ZstdLevel},
    file::properties::WriterProperties,
};
use poset_census::records::{ComplexKey, ComplexRecord, PosetKey, PosetRecord};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::poset_groups::GroupIdentificationRecord;

const COMPONENT: &str = "typebase/volume-1/finite-posets";
const LABEL_SCHEME: &str = "order.order-complex-dimension.canonical-index/v1";
const SEARCH_BATCH_ROWS: usize = 50_000;
const LANDING_ROW_COUNT: usize = 30;

/// Build parameters for the finite-Poset Volume I component.
#[derive(Clone, Debug)]
pub struct PosetBuildOptions {
    pub data_root: PathBuf,
    pub output_root: PathBuf,
    pub maximum_order: usize,
    pub rows_per_detail_part: usize,
    pub group_identifications: Option<PathBuf>,
}

/// Completed immutable browser projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PosetBuildOutcome {
    pub directory: PathBuf,
    pub poset_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct PackedPosetKey {
    order: u8,
    entries: [u16; 10],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct PosetLabel {
    order: u8,
    dimension: i8,
    index: u32,
}

impl std::fmt::Display for PosetLabel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}.{}.{}",
            self.order, self.dimension, self.index
        )
    }
}

#[derive(Clone, Debug, Serialize)]
struct SearchRow {
    label: String,
    order: u8,
    dimension: i8,
    bucket_index: u32,
    cover_count: u16,
    comparable_pair_count: u16,
    minimal_count: u8,
    maximal_count: u8,
    component_count: u8,
    height: u8,
    width: u8,
    core_size: u8,
    is_connected: bool,
    is_lattice: bool,
    is_core: bool,
    is_self_dual: bool,
    automorphism_order: String,
    automorphism_lmfdb_label: Option<String>,
    core_label: String,
    opposite_label: String,
    homology_signature: String,
    has_torsion: bool,
}

#[derive(Debug)]
struct DetailRow {
    label: String,
    core_label: String,
    opposite_label: String,
    automorphism_lmfdb_label: Option<String>,
    poset_json: String,
    order_complex_json: String,
}

#[derive(Debug, Serialize)]
struct Artifact {
    kind: &'static str,
    path: String,
    row_count: u64,
    byte_count: u64,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct SourceOrder {
    order: usize,
    posets: SourceArtifact,
    complexes: SourceArtifact,
    validation: SourceArtifact,
}

#[derive(Debug, Serialize)]
struct SourceArtifact {
    path: String,
    byte_count: u64,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct LabelBucket {
    order: u8,
    dimension: i8,
    row_count: u32,
    detail_part_count: u32,
}

#[derive(Debug, Serialize)]
struct ComponentChecks {
    source_orders_certified: bool,
    labels_unique_and_complete: bool,
    core_labels_resolved: bool,
    opposite_labels_resolved: bool,
    order_complexes_resolved: bool,
    group_identifications_consistent: bool,
    search_and_detail_counts_match: bool,
}

#[derive(Debug, Serialize)]
struct ComponentManifest {
    component: &'static str,
    status: &'static str,
    volume: u32,
    collection: &'static str,
    census_record_version: u32,
    maximum_order: usize,
    label_scheme: &'static str,
    label_index_origin: u32,
    empty_poset_label: &'static str,
    rows_per_detail_part: usize,
    landing_path: &'static str,
    poset_count: u64,
    stores_labeled_posets: bool,
    source_orders: Vec<SourceOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_group_identifications: Option<SourceArtifact>,
    label_buckets: Vec<LabelBucket>,
    outputs: Vec<Artifact>,
    checks: ComponentChecks,
}

/// Build labels and static Parquet projections from certified census orders.
pub fn build_poset_component(options: &PosetBuildOptions) -> Result<PosetBuildOutcome> {
    ensure!(
        options.maximum_order <= 10,
        "Volume I labels currently support Poset orders through 10"
    );
    ensure!(
        options.rows_per_detail_part > 0,
        "detail part size must be positive"
    );
    ensure!(
        !options.output_root.exists(),
        "TypeBase output {} already exists; releases are immutable",
        options.output_root.display()
    );

    let mut labels = HashMap::<PackedPosetKey, PosetLabel>::new();
    let mut buckets = BTreeMap::<(u8, i8), u32>::new();
    let mut source_orders = Vec::new();
    let mut total_rows = 0_u64;
    for order in 0..=options.maximum_order {
        let directory = census_order_directory(&options.data_root, order);
        source_orders.push(inspect_source_order(&directory, order)?);
        let mut counters = BTreeMap::<i8, u32>::new();
        let mut previous_key = None::<PosetKey>;
        visit_posets(&directory.join("posets.jsonl"), |record, _line| {
            ensure!(
                record.order == order,
                "census order directory contains a mixed order"
            );
            if let Some(previous) = &previous_key {
                ensure!(
                    previous < &record.key,
                    "census Poset rows are not in strict canonical-key order"
                );
            }
            previous_key = Some(record.key.clone());
            let dimension = order_complex_dimension(record)?;
            let index = counters
                .entry(dimension)
                .and_modify(|value| *value += 1)
                .or_insert(1);
            let label = PosetLabel {
                order: u8::try_from(order)?,
                dimension,
                index: *index,
            };
            ensure!(
                labels.insert(pack_key(&record.key)?, label).is_none(),
                "duplicate Poset key while assigning TypeBase labels"
            );
            total_rows = total_rows
                .checked_add(1)
                .context("TypeBase Poset count overflow")?;
            Ok(())
        })?;
        for (dimension, count) in counters {
            buckets.insert((u8::try_from(order)?, dimension), count);
        }
    }
    let (group_identifications, source_group_identifications) =
        load_group_identifications(options.group_identifications.as_deref(), &labels)?;

    let parent = options
        .output_root
        .parent()
        .context("TypeBase output has no parent")?;
    fs::create_dir_all(parent)?;
    let file_name = options
        .output_root
        .file_name()
        .context("TypeBase output has no file name")?
        .to_string_lossy();
    let staging = parent.join(format!(".{file_name}.tmp-{}", process::id()));
    ensure!(
        !staging.exists(),
        "TypeBase staging directory already exists"
    );
    fs::create_dir(&staging)?;

    let mut outputs = Vec::new();
    let mut landing_rows = Vec::with_capacity(LANDING_ROW_COUNT);
    let mut detail_rows = 0_u64;
    let mut search_rows = 0_u64;
    for order in 0..=options.maximum_order {
        let source = census_order_directory(&options.data_root, order);
        let complexes = load_complexes(&source.join("simplicial-complexes.jsonl"))?;
        let search_path = staging
            .join("search")
            .join(format!("order={order}"))
            .join("posets.parquet");
        let mut search = SearchWriter::new(&staging, &search_path)?;
        let mut landing_candidates = Vec::new();
        let mut detail_buffers = BTreeMap::<i8, Vec<DetailRow>>::new();
        let mut detail_parts = BTreeMap::<i8, u32>::new();

        visit_posets(&source.join("posets.jsonl"), |record, line| {
            let label = label_for(&labels, &record.key)?;
            let core_label = label_for(&labels, &record.relations.has_core.target)?.to_string();
            let opposite_label = label_for(&labels, &record.relations.opposite.target)?.to_string();
            let complex = complexes
                .get(&record.relations.order_complex.target)
                .context("TypeBase Poset has an unresolved order-complex target")?;
            let lmfdb_label = abstract_group_label(record, &group_identifications)?;
            let search_row = SearchRow {
                label: label.to_string(),
                order: label.order,
                dimension: label.dimension,
                bucket_index: label.index,
                cover_count: u16::try_from(record.invariants.cover_count)?,
                comparable_pair_count: u16::try_from(
                    record.invariants.strict_comparable_pair_count,
                )?,
                minimal_count: u8::try_from(record.invariants.minimal_count)?,
                maximal_count: u8::try_from(record.invariants.maximal_count)?,
                component_count: u8::try_from(record.invariants.component_count)?,
                height: u8::try_from(record.invariants.height)?,
                width: u8::try_from(record.invariants.width)?,
                core_size: u8::try_from(record.relations.has_core.core_size)?,
                is_connected: record.invariants.is_connected,
                is_lattice: record.invariants.is_lattice,
                is_core: record.relations.has_core.is_core,
                is_self_dual: record.relations.opposite.is_self_dual,
                automorphism_order: record.automorphism_group.order.clone(),
                automorphism_lmfdb_label: lmfdb_label.clone(),
                core_label: core_label.clone(),
                opposite_label: opposite_label.clone(),
                homology_signature: homology_signature(complex),
                has_torsion: complex
                    .integral_homology
                    .iter()
                    .any(|group| !group.invariant_factors.is_empty()),
            };
            if landing_rows.len() < LANDING_ROW_COUNT {
                landing_candidates.push(search_row.clone());
            }
            search.push(search_row)?;
            search_rows += 1;

            let buffer = detail_buffers.entry(label.dimension).or_default();
            buffer.push(DetailRow {
                label: label.to_string(),
                core_label,
                opposite_label,
                automorphism_lmfdb_label: lmfdb_label,
                poset_json: line.trim_end_matches('\n').to_owned(),
                order_complex_json: serde_json::to_string(complex)?,
            });
            if buffer.len() == options.rows_per_detail_part {
                let part = detail_parts.entry(label.dimension).or_default();
                outputs.push(write_detail_part(
                    &staging,
                    order,
                    label.dimension,
                    *part,
                    buffer,
                )?);
                detail_rows += buffer.len() as u64;
                buffer.clear();
                *part += 1;
            }
            Ok(())
        })?;
        landing_candidates.sort_by_key(|row| (row.dimension, row.bucket_index));
        landing_rows.extend(
            landing_candidates
                .into_iter()
                .take(LANDING_ROW_COUNT - landing_rows.len()),
        );
        outputs.push(search.finish()?);
        for (dimension, buffer) in &mut detail_buffers {
            if buffer.is_empty() {
                continue;
            }
            let part = detail_parts.entry(*dimension).or_default();
            outputs.push(write_detail_part(
                &staging, order, *dimension, *part, buffer,
            )?);
            detail_rows += buffer.len() as u64;
            buffer.clear();
            *part += 1;
        }
    }
    ensure!(search_rows == total_rows && detail_rows == total_rows);
    ensure!(landing_rows.len() == LANDING_ROW_COUNT.min(total_rows as usize));
    let landing_path = staging.join("landing.json");
    write_json(&landing_path, &landing_rows)?;
    let (landing_byte_count, landing_sha256) = hash_file(&landing_path)?;
    outputs.push(Artifact {
        kind: "landing",
        path: "landing.json".to_owned(),
        row_count: landing_rows.len() as u64,
        byte_count: landing_byte_count,
        sha256: landing_sha256,
    });

    let label_buckets = buckets
        .into_iter()
        .map(|((order, dimension), row_count)| LabelBucket {
            order,
            dimension,
            row_count,
            detail_part_count: row_count.div_ceil(options.rows_per_detail_part as u32),
        })
        .collect::<Vec<_>>();
    let manifest = ComponentManifest {
        component: COMPONENT,
        status: "passed",
        volume: 1,
        collection: "finite-posets/v1",
        census_record_version: 1,
        maximum_order: options.maximum_order,
        label_scheme: LABEL_SCHEME,
        label_index_origin: 1,
        empty_poset_label: "0.-1.1",
        rows_per_detail_part: options.rows_per_detail_part,
        landing_path: "landing.json",
        poset_count: total_rows,
        stores_labeled_posets: false,
        source_orders,
        source_group_identifications,
        label_buckets,
        outputs,
        checks: ComponentChecks {
            source_orders_certified: true,
            labels_unique_and_complete: true,
            core_labels_resolved: true,
            opposite_labels_resolved: true,
            order_complexes_resolved: true,
            group_identifications_consistent: true,
            search_and_detail_counts_match: true,
        },
    };
    write_json(&staging.join("manifest.json"), &manifest)?;
    sync_directory(&staging)?;
    fs::rename(&staging, &options.output_root)?;
    sync_directory(parent)?;
    Ok(PosetBuildOutcome {
        directory: options.output_root.clone(),
        poset_count: total_rows,
    })
}

fn census_order_directory(data_root: &Path, order: usize) -> PathBuf {
    data_root
        .join("census")
        .join("record-version=1")
        .join(format!("order={order}"))
}

fn inspect_source_order(directory: &Path, order: usize) -> Result<SourceOrder> {
    let validation_path = directory.join("validation.json");
    let validation: serde_json::Value = serde_json::from_reader(BufReader::new(
        File::open(&validation_path)
            .with_context(|| format!("opening certified validation for source order {order}"))?,
    ))?;
    ensure!(validation["contract"] == "finite-posets/v1");
    ensure!(validation["order"] == order);
    ensure!(validation["status"] == "passed");
    ensure!(validation["isomorphism_scope"]["status"] == "complete");
    Ok(SourceOrder {
        order,
        posets: inspect_source_artifact(
            directory.join("posets.jsonl"),
            format!("census/record-version=1/order={order}/posets.jsonl"),
        )?,
        complexes: inspect_source_artifact(
            directory.join("simplicial-complexes.jsonl"),
            format!("census/record-version=1/order={order}/simplicial-complexes.jsonl"),
        )?,
        validation: inspect_source_artifact(
            validation_path,
            format!("census/record-version=1/order={order}/validation.json"),
        )?,
    })
}

fn inspect_source_artifact(path: PathBuf, published_path: String) -> Result<SourceArtifact> {
    let (byte_count, sha256) = hash_file(&path)?;
    Ok(SourceArtifact {
        path: published_path,
        byte_count,
        sha256,
    })
}

fn visit_posets(
    path: &Path,
    mut visit: impl FnMut(&PosetRecord, &str) -> Result<()>,
) -> Result<()> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        ensure!(
            line.ends_with('\n'),
            "source Poset JSONL lacks final newline"
        );
        let record: PosetRecord = serde_json::from_str(&line)?;
        record.check_shape()?;
        visit(&record, &line)?;
    }
    Ok(())
}

fn load_complexes(path: &Path) -> Result<HashMap<ComplexKey, ComplexRecord>> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = String::new();
    let mut complexes = HashMap::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        ensure!(
            line.ends_with('\n'),
            "source complex JSONL lacks final newline"
        );
        let record: ComplexRecord = serde_json::from_str(&line)?;
        ensure!(
            complexes.insert(record.key.clone(), record).is_none(),
            "duplicate source complex key"
        );
    }
    Ok(complexes)
}

fn pack_key(key: &PosetKey) -> Result<PackedPosetKey> {
    ensure!(key.order() <= 10, "Volume I Poset key exceeds order 10");
    let mut entries = [0_u16; 10];
    for (target, source) in entries.iter_mut().zip(key.entries()) {
        *target = source.parse::<u16>()?;
    }
    Ok(PackedPosetKey {
        order: u8::try_from(key.order())?,
        entries,
    })
}

fn order_complex_dimension(record: &PosetRecord) -> Result<i8> {
    if record.order == 0 {
        return Ok(-1);
    }
    i8::try_from(record.invariants.height - 1).context("order-complex dimension exceeds i8")
}

fn label_for(labels: &HashMap<PackedPosetKey, PosetLabel>, key: &PosetKey) -> Result<PosetLabel> {
    labels
        .get(&pack_key(key)?)
        .copied()
        .context("TypeBase relation target has no Poset label")
}

fn abstract_group_label(
    record: &PosetRecord,
    identifications: &HashMap<PackedPosetKey, GroupIdentificationRecord>,
) -> Result<Option<String>> {
    if let Some(label) = record
        .automorphism_group
        .abstract_group
        .as_ref()
        .and_then(|group| group.lmfdb_label.as_ref())
    {
        return Ok(Some(label.clone()));
    }
    if let Some(identification) = identifications.get(&pack_key(&record.key)?) {
        ensure!(
            identification.automorphism_order == record.automorphism_group.order,
            "abstract-group enrichment disagrees with the automorphism order"
        );
        return Ok(Some(identification.lmfdb_label.clone()));
    }
    let Ok(order) = record.automorphism_group.order.parse::<u64>() else {
        return Ok(None);
    };
    Ok((order == 1 || is_prime(order)).then(|| format!("{order}.1")))
}

fn load_group_identifications(
    path: Option<&Path>,
    labels: &HashMap<PackedPosetKey, PosetLabel>,
) -> Result<(
    HashMap<PackedPosetKey, GroupIdentificationRecord>,
    Option<SourceArtifact>,
)> {
    let Some(path) = path else {
        return Ok((HashMap::new(), None));
    };
    let manifest_path = path
        .parent()
        .context("group-identification records have no parent")?
        .join("manifest.json");
    let manifest: serde_json::Value = serde_json::from_reader(BufReader::new(
        File::open(&manifest_path).context("opening group-identification manifest")?,
    ))?;
    ensure!(manifest["enrichment"] == "finite-posets/automorphism-abstract-groups/v1");
    ensure!(manifest["status"] == "passed");
    let mut records = HashMap::new();
    let reader = BufReader::new(File::open(path)?);
    for line in reader.lines() {
        let record: GroupIdentificationRecord = serde_json::from_str(&line?)?;
        ensure!(record.record_version == 1);
        let key = pack_key(&record.poset_key)?;
        ensure!(
            labels.contains_key(&key),
            "group-identification record is outside the selected Poset census"
        );
        ensure!(
            record
                .lmfdb_label
                .starts_with(&format!("{}.", record.automorphism_order)),
            "LMFDB group label disagrees with its order"
        );
        ensure!(
            records.insert(key, record).is_none(),
            "duplicate group-identification Poset key"
        );
    }
    let expected = manifest["identified_count"]
        .as_u64()
        .context("group-identification manifest lacks identified_count")?;
    ensure!(records.len() as u64 == expected);
    let artifact = inspect_source_artifact(
        path.to_owned(),
        "enrichments/automorphism-abstract-groups/v1/records.jsonl".to_owned(),
    )?;
    ensure!(artifact.byte_count == manifest["output"]["byte_count"]);
    ensure!(artifact.sha256 == manifest["output"]["sha256"]);
    Ok((records, Some(artifact)))
}

fn is_prime(value: u64) -> bool {
    if value < 2 {
        return false;
    }
    if value.is_multiple_of(2) {
        return value == 2;
    }
    let mut divisor = 3_u64;
    while divisor <= value / divisor {
        if value.is_multiple_of(divisor) {
            return false;
        }
        divisor += 2;
    }
    true
}

fn homology_signature(complex: &ComplexRecord) -> String {
    complex
        .integral_homology
        .iter()
        .map(|group| {
            format!(
                "{}:{}:{}",
                group.degree,
                group.free_rank,
                group.invariant_factors.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

struct SearchWriter {
    projection_root: PathBuf,
    path: PathBuf,
    schema: SchemaRef,
    writer: ArrowWriter<File>,
    rows: Vec<SearchRow>,
    row_count: u64,
}

impl SearchWriter {
    fn new(projection_root: &Path, path: &Path) -> Result<Self> {
        fs::create_dir_all(path.parent().context("search output has no parent")?)?;
        let schema = search_schema();
        let writer = ArrowWriter::try_new(
            File::create(path)?,
            schema.clone(),
            Some(parquet_properties(SEARCH_BATCH_ROWS)?),
        )?;
        Ok(Self {
            projection_root: projection_root.to_owned(),
            path: path.to_owned(),
            schema,
            writer,
            rows: Vec::with_capacity(SEARCH_BATCH_ROWS),
            row_count: 0,
        })
    }

    fn push(&mut self, row: SearchRow) -> Result<()> {
        self.rows.push(row);
        if self.rows.len() == SEARCH_BATCH_ROWS {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if self.rows.is_empty() {
            return Ok(());
        }
        let batch = search_batch(self.schema.clone(), &self.rows)?;
        self.writer.write(&batch)?;
        self.row_count += self.rows.len() as u64;
        self.rows.clear();
        Ok(())
    }

    fn finish(mut self) -> Result<Artifact> {
        self.flush()?;
        self.writer.close()?;
        artifact_for_parquet(&self.projection_root, &self.path, "search", self.row_count)
    }
}

fn search_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("label", DataType::Utf8, false),
        Field::new("order", DataType::UInt8, false),
        Field::new("dimension", DataType::Int8, false),
        Field::new("bucket_index", DataType::UInt32, false),
        Field::new("cover_count", DataType::UInt16, false),
        Field::new("comparable_pair_count", DataType::UInt16, false),
        Field::new("minimal_count", DataType::UInt8, false),
        Field::new("maximal_count", DataType::UInt8, false),
        Field::new("component_count", DataType::UInt8, false),
        Field::new("height", DataType::UInt8, false),
        Field::new("width", DataType::UInt8, false),
        Field::new("core_size", DataType::UInt8, false),
        Field::new("is_connected", DataType::Boolean, false),
        Field::new("is_lattice", DataType::Boolean, false),
        Field::new("is_core", DataType::Boolean, false),
        Field::new("is_self_dual", DataType::Boolean, false),
        Field::new("automorphism_order", DataType::Utf8, false),
        Field::new("automorphism_lmfdb_label", DataType::Utf8, true),
        Field::new("core_label", DataType::Utf8, false),
        Field::new("opposite_label", DataType::Utf8, false),
        Field::new("homology_signature", DataType::Utf8, false),
        Field::new("has_torsion", DataType::Boolean, false),
    ]))
}

fn search_batch(schema: SchemaRef, rows: &[SearchRow]) -> Result<RecordBatch> {
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.label.as_str()),
        )),
        Arc::new(UInt8Array::from_iter_values(
            rows.iter().map(|row| row.order),
        )),
        Arc::new(Int8Array::from_iter_values(
            rows.iter().map(|row| row.dimension),
        )),
        Arc::new(UInt32Array::from_iter_values(
            rows.iter().map(|row| row.bucket_index),
        )),
        Arc::new(UInt16Array::from_iter_values(
            rows.iter().map(|row| row.cover_count),
        )),
        Arc::new(UInt16Array::from_iter_values(
            rows.iter().map(|row| row.comparable_pair_count),
        )),
        Arc::new(UInt8Array::from_iter_values(
            rows.iter().map(|row| row.minimal_count),
        )),
        Arc::new(UInt8Array::from_iter_values(
            rows.iter().map(|row| row.maximal_count),
        )),
        Arc::new(UInt8Array::from_iter_values(
            rows.iter().map(|row| row.component_count),
        )),
        Arc::new(UInt8Array::from_iter_values(
            rows.iter().map(|row| row.height),
        )),
        Arc::new(UInt8Array::from_iter_values(
            rows.iter().map(|row| row.width),
        )),
        Arc::new(UInt8Array::from_iter_values(
            rows.iter().map(|row| row.core_size),
        )),
        Arc::new(BooleanArray::from(
            rows.iter().map(|row| row.is_connected).collect::<Vec<_>>(),
        )),
        Arc::new(BooleanArray::from(
            rows.iter().map(|row| row.is_lattice).collect::<Vec<_>>(),
        )),
        Arc::new(BooleanArray::from(
            rows.iter().map(|row| row.is_core).collect::<Vec<_>>(),
        )),
        Arc::new(BooleanArray::from(
            rows.iter().map(|row| row.is_self_dual).collect::<Vec<_>>(),
        )),
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.automorphism_order.as_str()),
        )),
        Arc::new(StringArray::from_iter(
            rows.iter()
                .map(|row| row.automorphism_lmfdb_label.as_deref()),
        )),
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.core_label.as_str()),
        )),
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.opposite_label.as_str()),
        )),
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.homology_signature.as_str()),
        )),
        Arc::new(BooleanArray::from(
            rows.iter().map(|row| row.has_torsion).collect::<Vec<_>>(),
        )),
    ];
    Ok(RecordBatch::try_new(schema, arrays)?)
}

fn detail_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("label", DataType::Utf8, false),
        Field::new("core_label", DataType::Utf8, false),
        Field::new("opposite_label", DataType::Utf8, false),
        Field::new("automorphism_lmfdb_label", DataType::Utf8, true),
        Field::new("poset_json", DataType::Utf8, false),
        Field::new("order_complex_json", DataType::Utf8, false),
    ]))
}

fn write_detail_part(
    staging: &Path,
    order: usize,
    dimension: i8,
    part: u32,
    rows: &[DetailRow],
) -> Result<Artifact> {
    let path = staging
        .join("details")
        .join(format!("order={order}"))
        .join(format!("dimension={dimension}"))
        .join(format!("part-{part:05}.parquet"));
    fs::create_dir_all(path.parent().context("detail output has no parent")?)?;
    let schema = detail_schema();
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.label.as_str()),
        )),
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.core_label.as_str()),
        )),
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.opposite_label.as_str()),
        )),
        Arc::new(StringArray::from_iter(
            rows.iter()
                .map(|row| row.automorphism_lmfdb_label.as_deref()),
        )),
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.poset_json.as_str()),
        )),
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|row| row.order_complex_json.as_str()),
        )),
    ];
    let batch = RecordBatch::try_new(schema.clone(), arrays)?;
    let mut writer = ArrowWriter::try_new(
        File::create(&path)?,
        schema,
        Some(parquet_properties(rows.len())?),
    )?;
    writer.write(&batch)?;
    writer.close()?;
    artifact_for_parquet(staging, &path, "details", rows.len() as u64)
}

fn parquet_properties(row_group_size: usize) -> Result<WriterProperties> {
    Ok(WriterProperties::builder()
        .set_compression(Compression::ZSTD(ZstdLevel::try_new(9)?))
        .set_max_row_group_row_count(Some(row_group_size.max(1)))
        .build())
}

fn artifact_for_parquet(
    projection_root: &Path,
    path: &Path,
    kind: &'static str,
    row_count: u64,
) -> Result<Artifact> {
    let (byte_count, sha256) = hash_file(path)?;
    let relative = path
        .strip_prefix(projection_root)
        .context("Parquet artifact is outside its projection root")?
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    Ok(Artifact {
        kind,
        path: relative,
        row_count,
        byte_count,
        sha256,
    })
}

fn hash_file(path: &Path) -> Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut byte_count = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
        byte_count += read as u64;
    }
    Ok((byte_count, hex::encode(digest.finalize())))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_text_includes_empty_dimension() {
        assert_eq!(
            PosetLabel {
                order: 0,
                dimension: -1,
                index: 1,
            }
            .to_string(),
            "0.-1.1"
        );
    }

    #[test]
    fn only_unique_prime_order_groups_are_inferred() {
        assert!(is_prime(2));
        assert!(is_prime(97));
        assert!(!is_prime(1));
        assert!(!is_prime(8));
    }
}
