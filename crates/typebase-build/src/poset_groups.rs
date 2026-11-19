//! Independently identified abstract automorphism groups for finite Posets.

use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use poset_census::records::{PosetKey, PosetRecord};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const BATCH_SIZE: usize = 500;
const ENRICHMENT: &str = "finite-posets/automorphism-abstract-groups/v1";

#[derive(Clone, Debug)]
pub struct GroupIdentificationOptions {
    pub data_root: PathBuf,
    pub output_root: PathBuf,
    pub maximum_order: usize,
    pub gap_executable: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupIdentificationOutcome {
    pub directory: PathBuf,
    pub poset_count: u64,
    pub identified_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GroupIdentificationRecord {
    pub record_version: u32,
    pub poset_key: PosetKey,
    pub automorphism_order: String,
    pub lmfdb_label: String,
    pub evidence: GroupIdentificationEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GroupIdentificationEvidence {
    OrderDeterminesIsomorphismClass,
    GapIdGroup {
        gap_version: String,
        small_group_order: String,
        small_group_id: usize,
    },
}

#[derive(Debug, Serialize)]
struct EnrichmentManifest {
    enrichment: &'static str,
    status: &'static str,
    maximum_order: usize,
    poset_count: u64,
    identified_count: u64,
    unidentified_count: u64,
    order_determined_count: u64,
    gap_identified_count: u64,
    gap_unavailable_count: u64,
    gap_version: String,
    output: OutputArtifact,
}

#[derive(Debug, Serialize)]
struct OutputArtifact {
    path: &'static str,
    row_count: u64,
    byte_count: u64,
    sha256: String,
}

enum PendingIdentification {
    Known(GroupIdentificationRecord),
    Gap(Box<PosetRecord>),
}

struct Counts {
    posets: u64,
    identified: u64,
    order_determined: u64,
    gap_identified: u64,
    gap_unavailable: u64,
}

/// Identify every abstract automorphism group supported by GAP's Small Groups library.
pub fn identify_poset_groups(
    options: &GroupIdentificationOptions,
) -> Result<GroupIdentificationOutcome> {
    ensure!(options.maximum_order <= 10, "Poset order exceeds Volume I");
    ensure!(
        !options.output_root.exists(),
        "group-identification output {} already exists",
        options.output_root.display()
    );
    for order in 0..=options.maximum_order {
        verify_source_order(&options.data_root, order)?;
    }

    let staging_guard = crate::static_component::StagingDirectory::new(&options.output_root)?;
    let staging = staging_guard.path.clone();

    let mut gap = GapIdentifier::spawn(&options.gap_executable)?;
    let gap_version = gap.version().to_owned();
    let records_path = staging.join("records.jsonl");
    let mut writer = BufWriter::new(File::create(&records_path)?);
    let mut counts = Counts {
        posets: 0,
        identified: 0,
        order_determined: 0,
        gap_identified: 0,
        gap_unavailable: 0,
    };
    let mut batch = Vec::with_capacity(BATCH_SIZE);

    for order in 0..=options.maximum_order {
        let path = census_order_directory(&options.data_root, order).join("posets.jsonl");
        let reader = BufReader::new(File::open(&path)?);
        for line in reader.lines() {
            let record: PosetRecord = serde_json::from_str(&line?)?;
            record.check_shape()?;
            let automorphism_order = record.automorphism_group.order.parse::<u64>()?;
            let pending = if automorphism_order == 1 || is_prime(automorphism_order) {
                PendingIdentification::Known(GroupIdentificationRecord {
                    record_version: 1,
                    poset_key: record.key,
                    automorphism_order: automorphism_order.to_string(),
                    lmfdb_label: format!("{automorphism_order}.1"),
                    evidence: GroupIdentificationEvidence::OrderDeterminesIsomorphismClass,
                })
            } else {
                PendingIdentification::Gap(Box::new(record))
            };
            batch.push(pending);
            if batch.len() == BATCH_SIZE {
                process_batch(&mut gap, &mut writer, &mut counts, &mut batch)?;
            }
        }
    }
    process_batch(&mut gap, &mut writer, &mut counts, &mut batch)?;
    gap.finish()?;
    writer.flush()?;
    writer.get_ref().sync_all()?;

    let (byte_count, sha256) = hash_file(&records_path)?;
    let manifest = EnrichmentManifest {
        enrichment: ENRICHMENT,
        status: "passed",
        maximum_order: options.maximum_order,
        poset_count: counts.posets,
        identified_count: counts.identified,
        unidentified_count: counts.posets - counts.identified,
        order_determined_count: counts.order_determined,
        gap_identified_count: counts.gap_identified,
        gap_unavailable_count: counts.gap_unavailable,
        gap_version,
        output: OutputArtifact {
            path: "records.jsonl",
            row_count: counts.identified,
            byte_count,
            sha256,
        },
    };
    write_json(&staging.join("manifest.json"), &manifest)?;
    sync_directory(&staging)?;
    staging_guard.install()?;
    Ok(GroupIdentificationOutcome {
        directory: options.output_root.clone(),
        poset_count: counts.posets,
        identified_count: counts.identified,
    })
}

fn process_batch(
    gap: &mut GapIdentifier,
    writer: &mut BufWriter<File>,
    counts: &mut Counts,
    batch: &mut Vec<PendingIdentification>,
) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }
    let candidates = batch
        .iter()
        .filter_map(|pending| match pending {
            PendingIdentification::Gap(record) => Some(record.as_ref()),
            PendingIdentification::Known(_) => None,
        })
        .collect::<Vec<_>>();
    let generator_sets = candidates
        .iter()
        .map(|record| record.automorphism_group.generators.as_slice())
        .collect::<Vec<_>>();
    let identified = gap.identify_batch(&generator_sets)?;
    let mut identified = identified.into_iter();
    for pending in batch.drain(..) {
        counts.posets += 1;
        let record = match pending {
            PendingIdentification::Known(record) => {
                counts.order_determined += 1;
                Some(record)
            }
            PendingIdentification::Gap(record) => {
                let record = *record;
                let result = identified.next().context("missing GAP batch result")?;
                match result {
                    Some((small_group_order, small_group_id)) => {
                        ensure!(
                            small_group_order == record.automorphism_group.order,
                            "GAP identified the wrong group order"
                        );
                        counts.gap_identified += 1;
                        Some(GroupIdentificationRecord {
                            record_version: 1,
                            poset_key: record.key,
                            automorphism_order: small_group_order.clone(),
                            lmfdb_label: format!("{small_group_order}.{small_group_id}"),
                            evidence: GroupIdentificationEvidence::GapIdGroup {
                                gap_version: gap.version().to_owned(),
                                small_group_order,
                                small_group_id,
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

pub(crate) struct GapIdentifier {
    child: Child,
    input: Option<BufWriter<ChildStdin>>,
    output: Receiver<std::io::Result<String>>,
    version: String,
}

impl GapIdentifier {
    pub(crate) fn spawn(executable: &Path) -> Result<Self> {
        let mut child = Command::new(executable)
            .args(["-q", "-b", "-T"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("starting GAP at {}", executable.display()))?;
        let mut input = BufWriter::new(child.stdin.take().context("GAP stdin unavailable")?);
        let stdout = child.stdout.take().context("GAP stdout unavailable")?;
        let (sender, output) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        input.write_all(b"Print(\"TYPEBASE_VERSION\\t\", GAPInfo.Version, \"\\n\");\n")?;
        input.flush()?;
        let line = match read_protocol_line(&output, "TYPEBASE_VERSION\t") {
            Ok(line) => line,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let version = line
            .strip_prefix("TYPEBASE_VERSION\t")
            .context("invalid GAP version handshake")?
            .to_owned();
        Ok(Self {
            child,
            input: Some(input),
            output,
            version,
        })
    }

    pub(crate) fn version(&self) -> &str {
        &self.version
    }

    pub(crate) fn identify_batch(
        &mut self,
        generator_sets: &[&[Vec<u32>]],
    ) -> Result<Vec<Option<(String, usize)>>> {
        let input = self.input.as_mut().context("GAP input closed")?;
        for (index, generators) in generator_sets.iter().enumerate() {
            let generators = generators
                .iter()
                .map(|generator| {
                    format!(
                        "[{}]",
                        generator
                            .iter()
                            .map(|image| (image + 1).to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            ensure!(!generators.is_empty(), "nontrivial group has no generators");
            writeln!(
                input,
                "g:=Group(List([{generators}],PermList));; if IdGroupsAvailable(Size(g)) then id:=IdGroup(g);; Print(\"TYPEBASE_ID\\t{index}\\t\",id[1],\"\\t\",id[2],\"\\n\"); else Print(\"TYPEBASE_ID\\t{index}\\t-\\n\"); fi;"
            )?;
        }
        input.flush()?;
        let mut results = Vec::with_capacity(generator_sets.len());
        for expected_index in 0..generator_sets.len() {
            let line = read_protocol_line(&self.output, "TYPEBASE_ID\t")?;
            let fields = line.split('\t').collect::<Vec<_>>();
            ensure!(fields.len() >= 3 && fields[0] == "TYPEBASE_ID");
            ensure!(fields[1].parse::<usize>()? == expected_index);
            if fields[2] == "-" {
                results.push(None);
            } else {
                ensure!(fields.len() == 4, "invalid GAP identification response");
                results.push(Some((fields[2].to_owned(), fields[3].parse()?)));
            }
        }
        Ok(results)
    }

    pub(crate) fn finish(mut self) -> Result<()> {
        if let Some(mut input) = self.input.take() {
            input.write_all(b"QUIT_GAP(0);\n")?;
            input.flush()?;
        }
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(status) = self.child.try_wait()? {
                ensure!(status.success(), "GAP exited unsuccessfully");
                return Ok(());
            }
            ensure!(Instant::now() < deadline, "GAP shutdown timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for GapIdentifier {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_protocol_line(reader: &Receiver<std::io::Result<String>>, prefix: &str) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let line = reader
            .recv_timeout(remaining)
            .context("GAP protocol timed out or closed its output")??;
        if line.starts_with(prefix) {
            return Ok(line.trim_end_matches(['\r', '\n']).to_owned());
        }
        if !line.is_empty() {
            bail!("unexpected GAP output: {line}");
        }
    }
}

fn verify_source_order(data_root: &Path, order: usize) -> Result<()> {
    let path = census_order_directory(data_root, order).join("validation.json");
    let validation: serde_json::Value = serde_json::from_reader(BufReader::new(File::open(path)?))?;
    ensure!(validation["contract"] == "finite-posets/v1");
    ensure!(validation["order"] == order);
    ensure!(validation["status"] == "passed");
    ensure!(validation["isomorphism_scope"]["status"] == "complete");
    Ok(())
}

fn census_order_directory(data_root: &Path, order: usize) -> PathBuf {
    data_root
        .join("census")
        .join("record-version=1")
        .join(format!("order={order}"))
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
    fn group_record_round_trips() {
        let record = GroupIdentificationRecord {
            record_version: 1,
            poset_key: PosetKey::new(vec!["1".into(), "2".into()]).unwrap(),
            automorphism_order: "2".into(),
            lmfdb_label: "2.1".into(),
            evidence: GroupIdentificationEvidence::OrderDeterminesIsomorphismClass,
        };
        let encoded = serde_json::to_string(&record).unwrap();
        assert_eq!(
            serde_json::from_str::<GroupIdentificationRecord>(&encoded).unwrap(),
            record
        );
    }
}
