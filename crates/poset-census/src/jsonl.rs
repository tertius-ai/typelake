use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{CONTRACT, Census, NOETHER_REVISION, validate::ValidationReport};

#[derive(Clone, Debug, Serialize)]
struct SourceRevision {
    revision: &'static str,
    dirty: bool,
}

#[derive(Clone, Debug, Serialize)]
struct ToolchainReport {
    rustc: &'static str,
    target: &'static str,
    profile: &'static str,
    noether_version: &'static str,
    flint_version: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct Parameters {
    order: usize,
    candidate_cap: u64,
    raw_candidate_count: u64,
    generator: &'static str,
    seed: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
struct FileReport {
    kind: &'static str,
    path: &'static str,
    row_count: usize,
    byte_count: u64,
    sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct InputReport {
    kind: &'static str,
    path: &'static str,
    sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct RunReport {
    contract: &'static str,
    run_id: String,
    operation: &'static str,
    started_unix_milliseconds: u128,
    elapsed_milliseconds: u128,
    revisions: Revisions,
    toolchain: ToolchainReport,
    parameters: Parameters,
    inputs: Vec<InputReport>,
    outputs: Vec<FileReport>,
    validation_sha256: String,
    peak_memory_bytes: Option<u64>,
    external_oracles: Vec<&'static str>,
    reconstruction_command: String,
}

#[derive(Clone, Debug, Serialize)]
struct Revisions {
    noether: SourceRevision,
    poset_census: SourceRevision,
}

/// Write a validated completed order under the explicit data root.
pub fn emit_completed_order(
    data_root: &Path,
    census: &Census,
    validation: &ValidationReport,
    started: SystemTime,
    elapsed: Duration,
) -> Result<PathBuf> {
    let version_root = data_root.join("census/record-version=1");
    let final_directory = version_root.join(format!("order={}", census.order));
    if final_directory.exists() {
        bail!(
            "completed census directory {} already exists; completed orders are immutable",
            final_directory.display()
        );
    }
    fs::create_dir_all(&version_root)
        .with_context(|| format!("creating {}", version_root.display()))?;
    let staging = version_root.join(format!(
        ".order={}.tmp-{}-{}",
        census.order,
        process::id(),
        unix_milliseconds(SystemTime::now())?
    ));
    fs::create_dir(&staging).with_context(|| format!("creating {}", staging.display()))?;

    let posets = write_jsonl(
        &staging.join("posets.jsonl"),
        &census.posets,
        "posets",
        "posets.jsonl",
    )?;
    let complexes = write_jsonl(
        &staging.join("simplicial-complexes.jsonl"),
        &census.complexes,
        "simplicial-complexes",
        "simplicial-complexes.jsonl",
    )?;
    let validation_artifact = write_json(
        &staging.join("validation.json"),
        validation,
        "validation",
        "validation.json",
    )?;
    let validation_sha256 = validation_artifact.sha256.clone();

    let report = RunReport {
        contract: CONTRACT,
        run_id: format!(
            "{}-order-{}-{}-{}",
            if census.generator == crate::Generator::Reference {
                "reference"
            } else {
                "orderly"
            },
            census.order,
            unix_milliseconds(started)?,
            process::id()
        ),
        operation: "emit",
        started_unix_milliseconds: unix_milliseconds(started)?,
        elapsed_milliseconds: elapsed.as_millis(),
        revisions: Revisions {
            noether: SourceRevision {
                revision: NOETHER_REVISION,
                dirty: false,
            },
            poset_census: SourceRevision {
                revision: env!("POSET_CENSUS_REVISION"),
                dirty: env!("POSET_CENSUS_DIRTY") == "true",
            },
        },
        toolchain: ToolchainReport {
            rustc: env!("POSET_CENSUS_RUSTC"),
            target: env!("POSET_CENSUS_TARGET"),
            profile: env!("POSET_CENSUS_PROFILE"),
            noether_version: noether::VERSION,
            flint_version: env!("POSET_CENSUS_FLINT_VERSION"),
        },
        parameters: Parameters {
            order: census.order,
            candidate_cap: census.candidate_cap,
            raw_candidate_count: census.raw_candidate_count,
            generator: census.generator.id(),
            seed: None,
        },
        inputs: vec![InputReport {
            kind: "accepted-counts",
            path: "collections/finite-posets/validation/counts.json",
            sha256: sha256(include_bytes!(
                "../../../collections/finite-posets/validation/counts.json"
            )),
        }],
        outputs: vec![posets, complexes, validation_artifact],
        validation_sha256,
        peak_memory_bytes: peak_memory_bytes(),
        external_oracles: vec!["GAP:skipped"],
        reconstruction_command: format!(
            "poset-census {} --order {} --candidate-cap {} --data-root <explicit-data-root>",
            census.generator.command(),
            census.order,
            census.candidate_cap
        ),
    };
    write_json(&staging.join("run.json"), &report, "run", "run.json")?;
    fs::rename(&staging, &final_directory).with_context(|| {
        format!(
            "publishing {} as {}",
            staging.display(),
            final_directory.display()
        )
    })?;
    Ok(final_directory)
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn write_jsonl<T: Serialize>(
    path: &Path,
    rows: &[T],
    kind: &'static str,
    relative_path: &'static str,
) -> Result<FileReport> {
    let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut writer = HashingWriter::new(BufWriter::new(file));
    for row in rows {
        serde_json::to_writer(&mut writer, row)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(writer.report(kind, relative_path, rows.len()))
}

fn write_json<T: Serialize>(
    path: &Path,
    value: &T,
    kind: &'static str,
    relative_path: &'static str,
) -> Result<FileReport> {
    let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut writer = HashingWriter::new(BufWriter::new(file));
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(writer.report(kind, relative_path, 1))
}

struct HashingWriter<W> {
    inner: W,
    digest: Sha256,
    bytes: u64,
}

impl<W> HashingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            digest: Sha256::new(),
            bytes: 0,
        }
    }

    fn report(self, kind: &'static str, path: &'static str, row_count: usize) -> FileReport {
        FileReport {
            kind,
            path,
            row_count,
            byte_count: self.bytes,
            sha256: hex::encode(self.digest.finalize()),
        }
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buffer)?;
        self.digest.update(&buffer[..written]);
        self.bytes += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn unix_milliseconds(time: SystemTime) -> Result<u128> {
    Ok(time
        .duration_since(UNIX_EPOCH)
        .context("system clock precedes Unix epoch")?
        .as_millis())
}

#[cfg(target_os = "linux")]
fn peak_memory_bytes() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kibibytes = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    kibibytes.checked_mul(1024)
}

#[cfg(not(target_os = "linux"))]
fn peak_memory_bytes() -> Option<u64> {
    None
}
