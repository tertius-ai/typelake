use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    EilenbergMacLaneCensus, INVARIANT_CONTRACT, LEGACY_NOETHER_REVISION, NOETHER_REVISION,
    records::EilenbergMacLaneRecord,
    source::{SourceValidationReport, source_sha256},
    validate::{ValidationReport, validate_records_sorted},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub path: String,
    pub row_count: usize,
    pub byte_count: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub contract: String,
    pub noether_revision: String,
    pub source_sha256: String,
    pub maximum_finite_order: usize,
    pub includes_infinite_cyclic: bool,
    pub maximum_delooping_degree: usize,
    pub exact_through_degree: usize,
    pub source_validation: SourceValidationReport,
    pub validation: ValidationReport,
    pub artifacts: Vec<Artifact>,
}

pub fn artifact_path(
    root: &Path,
    maximum_finite_order: usize,
    maximum_delooping_degree: usize,
    exact_through_degree: usize,
) -> PathBuf {
    root.join("eilenberg-maclane")
        .join("invariants-v8")
        .join(format!("order-through-{maximum_finite_order}-plus-Z"))
        .join(format!("delooping-through-{maximum_delooping_degree}"))
        .join(format!("degree-{exact_through_degree}"))
}

pub fn emit_census(
    root: &Path,
    census: &EilenbergMacLaneCensus,
    source_validation: SourceValidationReport,
    validation: ValidationReport,
) -> Result<Manifest> {
    validate_records_sorted(&census.records)?;
    ensure!(
        validation.symbolic_spaces == census.records.len(),
        "manifest and validation row counts disagree"
    );
    let destination = artifact_path(
        root,
        census.maximum_finite_order,
        census.maximum_delooping_degree,
        census.exact_through_degree,
    );
    ensure!(!destination.exists(), "immutable artifact already exists");
    let parent = destination
        .parent()
        .context("artifact path has no parent")?;
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(
        ".degree-{}.{}.tmp",
        census.exact_through_degree,
        std::process::id()
    ));
    ensure!(!staging.exists(), "staging path already exists");
    fs::create_dir(&staging)?;

    let records_path = staging.join("records.jsonl");
    let mut writer = BufWriter::new(File::create(&records_path)?);
    let mut digest = Sha256::new();
    let mut bytes = 0u64;
    for record in &census.records {
        let mut line = serde_json::to_vec(record)?;
        line.push(b'\n');
        writer.write_all(&line)?;
        digest.update(&line);
        bytes += u64::try_from(line.len())?;
    }
    writer.flush()?;

    let manifest = Manifest {
        contract: INVARIANT_CONTRACT.to_owned(),
        noether_revision: NOETHER_REVISION.to_owned(),
        source_sha256: source_sha256(),
        maximum_finite_order: census.maximum_finite_order,
        includes_infinite_cyclic: true,
        maximum_delooping_degree: census.maximum_delooping_degree,
        exact_through_degree: census.exact_through_degree,
        source_validation,
        validation,
        artifacts: vec![Artifact {
            path: "records.jsonl".to_owned(),
            row_count: census.records.len(),
            byte_count: bytes,
            sha256: hex::encode(digest.finalize()),
        }],
    };
    fs::write(
        staging.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::rename(&staging, &destination)?;
    Ok(manifest)
}

pub fn load_census(
    root: &Path,
    maximum_finite_order: usize,
    maximum_delooping_degree: usize,
    exact_through_degree: usize,
) -> Result<(Manifest, EilenbergMacLaneCensus)> {
    let directory = artifact_path(
        root,
        maximum_finite_order,
        maximum_delooping_degree,
        exact_through_degree,
    );
    let manifest: Manifest = serde_json::from_slice(&fs::read(directory.join("manifest.json"))?)?;
    ensure!(manifest.contract == INVARIANT_CONTRACT, "manifest contract");
    ensure!(
        manifest.noether_revision == NOETHER_REVISION
            || manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "Noether revision"
    );
    ensure!(manifest.source_sha256 == source_sha256(), "source hash");
    ensure!(manifest.artifacts.len() == 1, "artifact inventory");
    ensure!(
        manifest.maximum_finite_order == maximum_finite_order
            && manifest.maximum_delooping_degree == maximum_delooping_degree
            && manifest.exact_through_degree == exact_through_degree,
        "requested shell differs from manifest"
    );
    let artifact = &manifest.artifacts[0];
    let bytes = fs::read(directory.join(&artifact.path))?;
    ensure!(
        u64::try_from(bytes.len())? == artifact.byte_count,
        "artifact byte count"
    );
    ensure!(
        hex::encode(Sha256::digest(&bytes)) == artifact.sha256,
        "artifact hash"
    );
    let records = BufReader::new(bytes.as_slice())
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<Result<Vec<EilenbergMacLaneRecord>>>()?;
    ensure!(records.len() == artifact.row_count, "artifact row count");
    validate_records_sorted(&records)?;
    let census = EilenbergMacLaneCensus {
        maximum_finite_order,
        maximum_delooping_degree,
        exact_through_degree,
        records,
    };
    let source = crate::source::load_source()?;
    ensure!(
        crate::validate::validate_census(&source, &census)? == manifest.validation,
        "stored completeness report differs from record replay"
    );
    Ok((manifest, census))
}

#[cfg(test)]
mod tests {
    use crate::{
        enrich::build_census,
        source::{load_source, validate_source},
        validate::validate_census,
    };

    use super::*;

    #[test]
    fn pilot_artifact_round_trips_hashes_and_recomputes() {
        let source = load_source().unwrap();
        let source_validation = validate_source(&source).unwrap();
        let census = build_census(&source, 3, 4, 8).unwrap();
        let validation = validate_census(&source, &census).unwrap();
        let root = tempfile::tempdir().unwrap();
        let emitted = emit_census(root.path(), &census, source_validation, validation).unwrap();
        let (loaded_manifest, loaded) = load_census(root.path(), 3, 4, 8).unwrap();
        assert_eq!(loaded_manifest, emitted);
        assert_eq!(loaded.records, census.records);
        validate_census(&source, &loaded).unwrap();
    }
}
