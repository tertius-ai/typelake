use std::{
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct Artifact {
    pub kind: String,
    pub path: String,
    pub row_count: u64,
    pub byte_count: u64,
    pub sha256: String,
}

pub(crate) struct StagingDirectory {
    pub path: PathBuf,
    destination: PathBuf,
    parent: PathBuf,
}

impl StagingDirectory {
    pub fn new(destination: &Path) -> Result<Self> {
        ensure!(
            !destination.exists(),
            "TypeBase output {} already exists; releases are immutable",
            destination.display()
        );
        let parent = destination
            .parent()
            .context("TypeBase output has no parent")?
            .to_owned();
        fs::create_dir_all(&parent)?;
        let file_name = destination
            .file_name()
            .context("TypeBase output has no file name")?
            .to_string_lossy();
        let path = parent.join(format!(".{file_name}.tmp-{}", process::id()));
        ensure!(!path.exists(), "TypeBase staging directory already exists");
        fs::create_dir(&path)?;
        Ok(Self {
            path,
            destination: destination.to_owned(),
            parent,
        })
    }

    pub fn install(self) -> Result<()> {
        sync_directory(&self.path)?;
        fs::rename(&self.path, &self.destination)?;
        sync_directory(&self.parent)
    }
}

pub(crate) fn write_json(
    root: &Path,
    relative_path: &str,
    kind: &str,
    row_count: u64,
    value: &impl Serialize,
) -> Result<Artifact> {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    let mut writer = BufWriter::new(File::create(&path)?);
    writer.write_all(&bytes)?;
    writer.flush()?;
    artifact(kind, relative_path, row_count, &bytes)
}

pub(crate) fn write_zstd_json(
    root: &Path,
    relative_path: &str,
    kind: &str,
    row_count: u64,
    value: &impl Serialize,
) -> Result<Artifact> {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(value)?;
    let compressed = zstd::stream::encode_all(bytes.as_slice(), 10)?;
    let mut writer = BufWriter::new(File::create(&path)?);
    writer.write_all(&compressed)?;
    writer.flush()?;
    artifact(kind, relative_path, row_count, &compressed)
}

pub(crate) fn inspect_file(
    path: &Path,
    published_path: &str,
    kind: &str,
    row_count: u64,
) -> Result<Artifact> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut byte_count = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
        byte_count = byte_count
            .checked_add(u64::try_from(read)?)
            .context("artifact byte count overflow")?;
    }
    Ok(Artifact {
        kind: kind.to_owned(),
        path: published_path.to_owned(),
        row_count,
        byte_count,
        sha256: hex::encode(digest.finalize()),
    })
}

fn artifact(kind: &str, path: &str, row_count: u64, bytes: &[u8]) -> Result<Artifact> {
    Ok(Artifact {
        kind: kind.to_owned(),
        path: path.to_owned(),
        row_count,
        byte_count: u64::try_from(bytes.len())?,
        sha256: hex::encode(Sha256::digest(bytes)),
    })
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}
