use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    LEGACY_NOETHER_REVISION, MODULAR_CONTRACT, ModularCensus, NOETHER_REVISION,
    records::FiniteGroupModularCohomologyRecord,
    source::source_sha256,
    validate::{ModularValidationReport, validate_records_sorted},
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
pub struct ModularManifest {
    pub contract: String,
    pub noether_revision: String,
    pub source_sha256: String,
    pub maximum_order: usize,
    pub initial_proof_degree: usize,
    pub maximum_proof_degree: usize,
    pub proof_depths: BTreeMap<usize, usize>,
    pub groups: usize,
    pub coefficient_rows: usize,
    pub finite_abelian_presentations: usize,
    pub symonds_presentations: usize,
    pub symonds_parameter_presentations: usize,
    pub symonds_universal_presentations: usize,
    pub complete_presentations: usize,
    pub exact_truncations: usize,
    pub validation: ModularValidationReport,
    pub artifacts: Vec<Artifact>,
}

pub fn modular_path(
    root: &Path,
    maximum_order: usize,
    initial_proof_degree: usize,
    maximum_proof_degree: usize,
) -> PathBuf {
    root.join("finite-group-cohomology")
        .join("modular-v7")
        .join(format!("order-through-{maximum_order}"))
        .join(format!(
            "proof-degree-{initial_proof_degree}-through-{maximum_proof_degree}"
        ))
}

pub fn emit_modular_census(
    root: &Path,
    census: &ModularCensus,
    validation: ModularValidationReport,
) -> Result<ModularManifest> {
    validate_records_sorted(&census.records)?;
    let destination = modular_path(
        root,
        census.maximum_order,
        census.initial_proof_degree,
        census.maximum_proof_degree,
    );
    ensure!(!destination.exists(), "immutable artifact already exists");
    let parent = destination
        .parent()
        .context("artifact path has no parent")?;
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(
        ".proof-degree-{}-through-{}.{}.tmp",
        census.initial_proof_degree,
        census.maximum_proof_degree,
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
    let artifact = Artifact {
        path: "records.jsonl".to_owned(),
        row_count: census.records.len(),
        byte_count: bytes,
        sha256: hex::encode(digest.finalize()),
    };
    let complete_presentations = census
        .records
        .iter()
        .filter(|record| record.is_complete())
        .count();
    ensure!(
        complete_presentations == validation.complete_presentations,
        "manifest and validation completion counts disagree"
    );
    ensure!(
        complete_presentations == census.records.len(),
        "release artifact requires complete presentations in every row"
    );
    let manifest = ModularManifest {
        contract: MODULAR_CONTRACT.to_owned(),
        noether_revision: NOETHER_REVISION.to_owned(),
        source_sha256: source_sha256(),
        maximum_order: census.maximum_order,
        initial_proof_degree: census.initial_proof_degree,
        maximum_proof_degree: census.maximum_proof_degree,
        proof_depths: validation.proof_depths.clone(),
        groups: validation.groups,
        coefficient_rows: census.records.len(),
        finite_abelian_presentations: validation.finite_abelian_presentations,
        symonds_presentations: validation.symonds_presentations,
        symonds_parameter_presentations: validation.symonds_parameter_presentations,
        symonds_universal_presentations: validation.symonds_universal_presentations,
        complete_presentations,
        exact_truncations: census.records.len() - complete_presentations,
        validation,
        artifacts: vec![artifact],
    };
    fs::write(
        staging.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::rename(&staging, &destination)?;
    Ok(manifest)
}

pub fn load_modular_census(
    root: &Path,
    maximum_order: usize,
    initial_proof_degree: usize,
    maximum_proof_degree: usize,
) -> Result<(ModularManifest, ModularCensus)> {
    let directory = modular_path(
        root,
        maximum_order,
        initial_proof_degree,
        maximum_proof_degree,
    );
    let manifest: ModularManifest =
        serde_json::from_slice(&fs::read(directory.join("manifest.json"))?)?;
    ensure!(manifest.contract == MODULAR_CONTRACT, "manifest contract");
    ensure!(
        (manifest.noether_revision == NOETHER_REVISION
            || manifest.noether_revision == crate::PRERELEASE_NOETHER_REVISION)
            || manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "Noether revision"
    );
    ensure!(manifest.source_sha256 == source_sha256(), "source hash");
    ensure!(
        manifest.maximum_order == maximum_order
            && manifest.initial_proof_degree == initial_proof_degree
            && manifest.maximum_proof_degree == maximum_proof_degree,
        "manifest shell"
    );
    ensure!(manifest.artifacts.len() == 1, "artifact inventory");
    let artifact = &manifest.artifacts[0];
    let bytes = fs::read(directory.join(&artifact.path))?;
    ensure!(
        u64::try_from(bytes.len())? == artifact.byte_count,
        "artifact bytes"
    );
    ensure!(
        hex::encode(Sha256::digest(&bytes)) == artifact.sha256,
        "artifact hash"
    );
    let records = BufReader::new(bytes.as_slice())
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<Result<Vec<FiniteGroupModularCohomologyRecord>>>()?;
    ensure!(records.len() == artifact.row_count, "artifact rows");
    validate_records_sorted(&records)?;
    Ok((
        manifest,
        ModularCensus {
            maximum_order,
            initial_proof_degree,
            maximum_proof_degree,
            records,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{enrich::modular_census, source::load_source, validate::validate_modular_census};

    #[test]
    fn pilot_artifact_round_trips_and_hashes() {
        let source = load_source().unwrap();
        let census = modular_census(&source, 3, 3).unwrap();
        let validation = validate_modular_census(&source, &census).unwrap();
        let root = tempfile::tempdir().unwrap();
        let emitted = emit_modular_census(root.path(), &census, validation).unwrap();
        assert_eq!(emitted.finite_abelian_presentations, 2);
        assert_eq!(emitted.symonds_presentations, 0);
        assert_eq!(emitted.complete_presentations, 2);
        assert_eq!(emitted.exact_truncations, 0);
        let (loaded_manifest, loaded) = load_modular_census(root.path(), 3, 3, 3).unwrap();
        assert_eq!(loaded_manifest, emitted);
        assert_eq!(loaded.records, census.records);
        assert_eq!(
            validate_modular_census(&source, &loaded).unwrap(),
            loaded_manifest.validation
        );
    }
}
