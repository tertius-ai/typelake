//! Independently identified abstract automorphism groups for Steiner categories.

use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use steiner_census::records::{SteinerCategoryRecord, SteinerKey};

use crate::poset_groups::{GapIdentifier, GroupIdentificationEvidence};

const BATCH_SIZE: usize = 500;
const ENRICHMENT: &str = "strong-steiner-categories/automorphism-abstract-groups/v1";

#[derive(Clone, Debug)]
pub struct SteinerGroupIdentificationOptions {
    pub data_root: PathBuf,
    pub output_root: PathBuf,
    pub maximum_atomic_size: usize,
    pub gap_executable: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SteinerGroupIdentificationOutcome {
    pub directory: PathBuf,
    pub category_count: u64,
    pub identified_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SteinerGroupIdentificationRecord {
    pub record_version: u32,
    pub category_key: SteinerKey,
    pub automorphism_order: String,
    pub lmfdb_label: String,
    pub evidence: GroupIdentificationEvidence,
}

#[derive(Debug, Serialize)]
struct Manifest {
    enrichment: &'static str,
    status: &'static str,
    maximum_atomic_size: usize,
    category_count: u64,
    identified_count: u64,
    unidentified_count: u64,
    order_determined_count: u64,
    gap_identified_count: u64,
    gap_unavailable_count: u64,
    gap_version: String,
    output_path: &'static str,
    output_byte_count: u64,
    output_sha256: String,
}

enum Pending {
    Known(SteinerGroupIdentificationRecord),
    Gap(Box<SteinerCategoryRecord>),
}

#[derive(Default)]
struct Counts {
    categories: u64,
    identified: u64,
    order_determined: u64,
    gap_identified: u64,
    gap_unavailable: u64,
}

/// Identify every abstract automorphism group supported by GAP's Small Groups library.
pub fn identify_steiner_groups(
    options: &SteinerGroupIdentificationOptions,
) -> Result<SteinerGroupIdentificationOutcome> {
    ensure!(
        !options.output_root.exists(),
        "group-identification output {} already exists",
        options.output_root.display()
    );
    let parent = options
        .output_root
        .parent()
        .context("group-identification output has no parent")?;
    fs::create_dir_all(parent)?;
    let file_name = options
        .output_root
        .file_name()
        .context("group-identification output has no file name")?
        .to_string_lossy();
    let staging = parent.join(format!(".{file_name}.tmp-{}", process::id()));
    ensure!(!staging.exists(), "group-identification staging exists");
    fs::create_dir(&staging)?;

    let mut gap = GapIdentifier::spawn(&options.gap_executable)?;
    let gap_version = gap.version().to_owned();
    let records_path = staging.join("records.jsonl");
    let mut writer = BufWriter::new(File::create(&records_path)?);
    let mut counts = Counts::default();
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    for atomic_size in 0..=options.maximum_atomic_size {
        verify_source_shell(&options.data_root, atomic_size)?;
        let path = options
            .data_root
            .join("census/record-version=1")
            .join(format!("atomic-size={atomic_size}/categories.jsonl"));
        for line in BufReader::new(File::open(path)?).lines() {
            let record: SteinerCategoryRecord = serde_json::from_str(&line?)?;
            record.check_shape()?;
            let order = record.automorphism_group.order.parse::<u64>()?;
            if order == 1 || is_prime(order) {
                batch.push(Pending::Known(SteinerGroupIdentificationRecord {
                    record_version: 1,
                    category_key: record.key,
                    automorphism_order: order.to_string(),
                    lmfdb_label: format!("{order}.1"),
                    evidence: GroupIdentificationEvidence::OrderDeterminesIsomorphismClass,
                }));
            } else {
                batch.push(Pending::Gap(Box::new(record)));
            }
            if batch.len() == BATCH_SIZE {
                process_batch(&mut gap, &mut writer, &mut counts, &mut batch)?;
            }
        }
    }
    process_batch(&mut gap, &mut writer, &mut counts, &mut batch)?;
    gap.finish()?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    let (output_byte_count, output_sha256) = hash_file(&records_path)?;
    write_json(
        &staging.join("manifest.json"),
        &Manifest {
            enrichment: ENRICHMENT,
            status: "passed",
            maximum_atomic_size: options.maximum_atomic_size,
            category_count: counts.categories,
            identified_count: counts.identified,
            unidentified_count: counts.categories - counts.identified,
            order_determined_count: counts.order_determined,
            gap_identified_count: counts.gap_identified,
            gap_unavailable_count: counts.gap_unavailable,
            gap_version,
            output_path: "records.jsonl",
            output_byte_count,
            output_sha256,
        },
    )?;
    fs::rename(&staging, &options.output_root)?;
    Ok(SteinerGroupIdentificationOutcome {
        directory: options.output_root.clone(),
        category_count: counts.categories,
        identified_count: counts.identified,
    })
}

fn process_batch(
    gap: &mut GapIdentifier,
    writer: &mut BufWriter<File>,
    counts: &mut Counts,
    batch: &mut Vec<Pending>,
) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }
    let generator_sets = batch
        .iter()
        .filter_map(|pending| match pending {
            Pending::Known(_) => None,
            Pending::Gap(record) => Some(record.automorphism_group.generators.as_slice()),
        })
        .collect::<Vec<_>>();
    let mut identified = gap.identify_batch(&generator_sets)?.into_iter();
    for pending in batch.drain(..) {
        counts.categories += 1;
        let record = match pending {
            Pending::Known(record) => {
                counts.order_determined += 1;
                Some(record)
            }
            Pending::Gap(record) => {
                let result = identified.next().context("missing GAP batch result")?;
                match result {
                    Some((order, id)) => {
                        ensure!(order == record.automorphism_group.order);
                        counts.gap_identified += 1;
                        Some(SteinerGroupIdentificationRecord {
                            record_version: 1,
                            category_key: record.key,
                            automorphism_order: order.clone(),
                            lmfdb_label: format!("{order}.{id}"),
                            evidence: GroupIdentificationEvidence::GapIdGroup {
                                gap_version: gap.version().to_owned(),
                                small_group_order: order,
                                small_group_id: id,
                            },
                        })
                    }
                    None => {
                        counts.gap_unavailable += 1;
                        None
                    }
                }
            }
        };
        if let Some(record) = record {
            serde_json::to_writer(&mut *writer, &record)?;
            writer.write_all(b"\n")?;
            counts.identified += 1;
        }
    }
    ensure!(identified.next().is_none(), "extra GAP batch result");
    Ok(())
}

fn verify_source_shell(data_root: &Path, atomic_size: usize) -> Result<()> {
    let path = data_root
        .join("census/record-version=1")
        .join(format!("atomic-size={atomic_size}/validation.json"));
    let validation: serde_json::Value = serde_json::from_reader(BufReader::new(File::open(path)?))?;
    ensure!(validation["contract"] == "strong-steiner-categories/v1");
    ensure!(validation["atomic_size"] == atomic_size);
    ensure!(validation["status"] == "passed");
    ensure!(validation["isomorphism_scope"]["status"] == "complete");
    Ok(())
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

fn hash_file(path: &Path) -> Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes += count as u64;
    }
    Ok((bytes, hex::encode(hasher.finalize())))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}
