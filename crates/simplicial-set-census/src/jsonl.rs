use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    COHOMOLOGY_CONTRACT, COHOMOLOGY_NOETHER_REVISION, CONTRACT, CohomologyCensus,
    HOMOTOPY_CONTRACT, HOMOTOPY_NOETHER_REVISION, HomotopyCensus, IDENTITY_NOETHER_REVISION,
    IdentityCensus, LEGACY_NOETHER_REVISION, NOETHER_REVISION, STEENROD_OPERATIONS_CONTRACT,
    SteenrodOperationsCensus, TOPOLOGY_CONTRACT, TOPOLOGY_NOETHER_REVISION, TopologyCensus,
    records::{
        SimplicialSetCohomologyRecord, SimplicialSetHomotopyRecord, SimplicialSetIdentityRecord,
        SimplicialSetSteenrodOperationsRecord, SimplicialSetTopologyRecord,
    },
    validate::{
        CohomologyValidationReport, HomotopyValidationReport, SteenrodOperationsValidationReport,
        TopologyValidationReport, ValidationReport,
    },
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Artifact {
    pub path: String,
    pub row_count: u64,
    pub byte_count: u64,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
struct IdentityManifestRead {
    contract: String,
    noether_revision: String,
    total_nondegenerate: usize,
    maximum_dimension: usize,
    artifacts: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
struct TopologyManifestRead {
    contract: String,
    noether_revision: String,
    identity_noether_revision: String,
    total_nondegenerate: usize,
    maximum_dimension: usize,
    identity_sha256: String,
    artifacts: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
struct CohomologyManifestRead {
    contract: String,
    noether_revision: String,
    identity_noether_revision: String,
    total_nondegenerate: usize,
    maximum_dimension: usize,
    coefficient: String,
    identity_sha256: String,
    artifacts: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
struct HomotopyManifestRead {
    contract: String,
    noether_revision: String,
    identity_noether_revision: String,
    total_nondegenerate: usize,
    maximum_dimension: usize,
    identity_sha256: String,
    artifacts: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
struct SteenrodOperationsManifestRead {
    contract: String,
    noether_revision: String,
    identity_noether_revision: String,
    cohomology_noether_revision: String,
    total_nondegenerate: usize,
    maximum_dimension: usize,
    coefficient: String,
    identity_sha256: String,
    cohomology_sha256: String,
    artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TopologyManifest {
    pub contract: &'static str,
    pub noether_revision: &'static str,
    pub identity_noether_revision: String,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub identity_sha256: String,
    pub artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IdentityManifest {
    pub contract: &'static str,
    pub noether_revision: &'static str,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub candidate_cap: u64,
    pub stats: crate::enumerate::EnumerationStats,
    pub artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CohomologyManifest {
    pub contract: &'static str,
    pub noether_revision: &'static str,
    pub identity_noether_revision: String,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub coefficient: String,
    pub identity_sha256: String,
    pub artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HomotopyManifest {
    pub contract: &'static str,
    pub noether_revision: &'static str,
    pub identity_noether_revision: String,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub identity_sha256: String,
    pub artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SteenrodOperationsManifest {
    pub contract: &'static str,
    pub noether_revision: &'static str,
    pub identity_noether_revision: String,
    pub cohomology_noether_revision: String,
    pub total_nondegenerate: usize,
    pub maximum_dimension: usize,
    pub coefficient: String,
    pub identity_sha256: String,
    pub cohomology_sha256: String,
    pub artifacts: Vec<Artifact>,
}

pub fn emit_identity_shell(
    data_root: &Path,
    census: &IdentityCensus,
    validation: &ValidationReport,
) -> Result<PathBuf> {
    ensure!(
        validation.status == "complete" && validation.contract == CONTRACT,
        "refusing unvalidated output"
    );
    ensure!(
        validation.total_nondegenerate == census.total_nondegenerate
            && validation.maximum_dimension == census.maximum_dimension,
        "validation shell mismatch"
    );
    let collection = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{}", census.maximum_dimension));
    fs::create_dir_all(&collection)?;
    let destination = collection.join(format!("total-{}", census.total_nondegenerate));
    if destination.exists() {
        bail!(
            "immutable identity shell already exists at {}; choose a new data root or remove it deliberately",
            destination.display()
        );
    }
    let staging = collection.join(format!(
        ".total-{}.tmp-{}",
        census.total_nondegenerate,
        std::process::id()
    ));
    fs::create_dir(&staging)?;
    let records_path = staging.join("identities.jsonl");
    {
        let file = File::create(&records_path)?;
        let mut writer = BufWriter::new(file);
        for record in &census.records {
            serde_json::to_writer(&mut writer, record)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }
    let validation_path = staging.join("validation.json");
    write_pretty_json(&validation_path, validation)?;
    let manifest = IdentityManifest {
        contract: CONTRACT,
        noether_revision: NOETHER_REVISION,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        candidate_cap: census.candidate_cap,
        stats: census.stats.clone(),
        artifacts: vec![
            artifact(&records_path, census.records.len() as u64)?,
            artifact(&validation_path, 1)?,
        ],
    };
    write_pretty_json(&staging.join("manifest.json"), &manifest)?;
    fs::rename(&staging, &destination)?;
    Ok(destination)
}

pub fn load_identity_shell(
    data_root: &Path,
    total_nondegenerate: usize,
    maximum_dimension: usize,
) -> Result<Vec<SimplicialSetIdentityRecord>> {
    let directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(directory.join("manifest.json"))?)?;
    ensure!(manifest.contract == CONTRACT, "identity contract mismatch");
    ensure!(
        accepted_identity_noether_revision(&manifest.noether_revision),
        "identity Noether revision mismatch"
    );
    ensure!(
        manifest.total_nondegenerate == total_nondegenerate
            && manifest.maximum_dimension == maximum_dimension,
        "identity shell mismatch"
    );
    let expected = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    let path = directory.join("identities.jsonl");
    ensure!(
        artifact(&path, expected.row_count)? == *expected,
        "identity artifact hash mismatch"
    );
    let records = BufReader::new(File::open(path)?)
        .lines()
        .map(|line| {
            let record: SimplicialSetIdentityRecord = serde_json::from_str(&line?)?;
            record.check_shape()?;
            Ok(record)
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        records.len() as u64 == expected.row_count,
        "identity row count mismatch"
    );
    Ok(records)
}

pub fn emit_topology_shell(
    data_root: &Path,
    census: &TopologyCensus,
    validation: &TopologyValidationReport,
) -> Result<PathBuf> {
    ensure!(
        validation.status == "complete" && validation.contract == TOPOLOGY_CONTRACT,
        "refusing unvalidated output"
    );
    ensure!(
        validation.total_nondegenerate == census.total_nondegenerate
            && validation.maximum_dimension == census.maximum_dimension,
        "validation shell mismatch"
    );
    let identity_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{}", census.maximum_dimension))
        .join(format!("total-{}", census.total_nondegenerate));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT,
        "identity contract mismatch"
    );
    ensure!(
        accepted_identity_noether_revision(&identity_manifest.noether_revision),
        "identity Noether revision mismatch"
    );
    ensure!(
        identity_manifest.total_nondegenerate == census.total_nondegenerate
            && identity_manifest.maximum_dimension == census.maximum_dimension,
        "identity shell mismatch"
    );
    let identity = identity_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    ensure!(
        artifact(
            &identity_directory.join("identities.jsonl"),
            identity.row_count
        )? == *identity,
        "identity artifact hash mismatch"
    );
    let collection = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join("topology-v2")
        .join(format!("dimension-{}", census.maximum_dimension));
    fs::create_dir_all(&collection)?;
    let destination = collection.join(format!("total-{}", census.total_nondegenerate));
    if destination.exists() {
        bail!(
            "immutable topology shell already exists at {}; remove it deliberately before rebuilding",
            destination.display()
        );
    }
    let staging = collection.join(format!(
        ".total-{}.tmp-{}",
        census.total_nondegenerate,
        std::process::id()
    ));
    fs::create_dir(&staging)?;
    let records_path = staging.join("topology.jsonl");
    {
        let file = File::create(&records_path)?;
        let mut writer = BufWriter::new(file);
        for record in &census.records {
            serde_json::to_writer(&mut writer, record)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }
    let validation_path = staging.join("validation.json");
    write_pretty_json(&validation_path, validation)?;
    let manifest = TopologyManifest {
        contract: TOPOLOGY_CONTRACT,
        noether_revision: NOETHER_REVISION,
        identity_noether_revision: identity_manifest.noether_revision,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        identity_sha256: identity.sha256.clone(),
        artifacts: vec![
            artifact(&records_path, census.records.len() as u64)?,
            artifact(&validation_path, 1)?,
        ],
    };
    write_pretty_json(&staging.join("manifest.json"), &manifest)?;
    fs::rename(&staging, &destination)?;
    Ok(destination)
}

pub fn load_topology_shell(
    data_root: &Path,
    total_nondegenerate: usize,
    maximum_dimension: usize,
) -> Result<Vec<SimplicialSetTopologyRecord>> {
    let identity_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT,
        "identity contract mismatch"
    );
    ensure!(
        identity_manifest.total_nondegenerate == total_nondegenerate
            && identity_manifest.maximum_dimension == maximum_dimension,
        "identity shell mismatch"
    );

    let directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join("topology-v2")
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let manifest: TopologyManifestRead =
        serde_json::from_reader(File::open(directory.join("manifest.json"))?)?;
    ensure!(
        manifest.contract == TOPOLOGY_CONTRACT,
        "topology contract mismatch"
    );
    ensure!(
        manifest.noether_revision == TOPOLOGY_NOETHER_REVISION
            || manifest.noether_revision == COHOMOLOGY_NOETHER_REVISION
            || manifest.noether_revision == NOETHER_REVISION
            || manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "topology Noether revision mismatch"
    );
    ensure!(
        manifest.total_nondegenerate == total_nondegenerate
            && manifest.maximum_dimension == maximum_dimension,
        "topology shell mismatch"
    );
    ensure!(
        identity_manifest.noether_revision == manifest.identity_noether_revision,
        "topology identity revision link mismatch"
    );
    let identity = identity_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    ensure!(
        identity.sha256 == manifest.identity_sha256,
        "topology identity artifact link mismatch"
    );
    ensure!(
        artifact(
            &identity_directory.join("identities.jsonl"),
            identity.row_count
        )? == *identity,
        "identity artifact hash mismatch"
    );
    for expected in &manifest.artifacts {
        ensure!(
            artifact(&directory.join(&expected.path), expected.row_count)? == *expected,
            "topology artifact hash mismatch"
        );
    }
    let expected = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "topology.jsonl")
        .context("topology manifest omits topology.jsonl")?;
    let records = BufReader::new(File::open(directory.join("topology.jsonl"))?)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        records.len() as u64 == expected.row_count,
        "topology row count mismatch"
    );
    Ok(records)
}

pub fn emit_cohomology_shell(
    data_root: &Path,
    census: &CohomologyCensus,
    validation: &CohomologyValidationReport,
) -> Result<PathBuf> {
    ensure!(
        validation.status == "complete" && validation.contract == COHOMOLOGY_CONTRACT,
        "refusing unvalidated output"
    );
    ensure!(
        validation.total_nondegenerate == census.total_nondegenerate
            && validation.maximum_dimension == census.maximum_dimension
            && validation.coefficient == format!("F{}", census.prime),
        "validation shell mismatch"
    );
    let identity_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{}", census.maximum_dimension))
        .join(format!("total-{}", census.total_nondegenerate));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT,
        "identity contract mismatch"
    );
    ensure!(
        accepted_identity_noether_revision(&identity_manifest.noether_revision),
        "identity Noether revision mismatch"
    );
    ensure!(
        identity_manifest.total_nondegenerate == census.total_nondegenerate
            && identity_manifest.maximum_dimension == census.maximum_dimension,
        "identity shell mismatch"
    );
    let identity = identity_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    ensure!(
        artifact(
            &identity_directory.join("identities.jsonl"),
            identity.row_count
        )? == *identity,
        "identity artifact hash mismatch"
    );

    let collection = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join(format!("cohomology-f{}-v1", census.prime))
        .join(format!("dimension-{}", census.maximum_dimension));
    fs::create_dir_all(&collection)?;
    let destination = collection.join(format!("total-{}", census.total_nondegenerate));
    if destination.exists() {
        bail!(
            "immutable cohomology shell already exists at {}; remove it deliberately before rebuilding",
            destination.display()
        );
    }
    let staging = collection.join(format!(
        ".total-{}.tmp-{}",
        census.total_nondegenerate,
        std::process::id()
    ));
    fs::create_dir(&staging)?;
    let records_path = staging.join("cohomology.jsonl");
    {
        let file = File::create(&records_path)?;
        let mut writer = BufWriter::new(file);
        for record in &census.records {
            serde_json::to_writer(&mut writer, record)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }
    let validation_path = staging.join("validation.json");
    write_pretty_json(&validation_path, validation)?;
    let manifest = CohomologyManifest {
        contract: COHOMOLOGY_CONTRACT,
        noether_revision: NOETHER_REVISION,
        identity_noether_revision: identity_manifest.noether_revision,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        coefficient: format!("F{}", census.prime),
        identity_sha256: identity.sha256.clone(),
        artifacts: vec![
            artifact(&records_path, census.records.len() as u64)?,
            artifact(&validation_path, 1)?,
        ],
    };
    write_pretty_json(&staging.join("manifest.json"), &manifest)?;
    fs::rename(&staging, &destination)?;
    Ok(destination)
}

pub fn load_cohomology_shell(
    data_root: &Path,
    total_nondegenerate: usize,
    maximum_dimension: usize,
    prime: u64,
) -> Result<Vec<SimplicialSetCohomologyRecord>> {
    let identity_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT,
        "identity contract mismatch"
    );
    ensure!(
        accepted_identity_noether_revision(&identity_manifest.noether_revision),
        "identity Noether revision mismatch"
    );
    ensure!(
        identity_manifest.total_nondegenerate == total_nondegenerate
            && identity_manifest.maximum_dimension == maximum_dimension,
        "identity shell mismatch"
    );
    let identity = identity_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    ensure!(
        artifact(
            &identity_directory.join("identities.jsonl"),
            identity.row_count
        )? == *identity,
        "identity artifact hash mismatch"
    );

    let directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join(format!("cohomology-f{prime}-v1"))
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let manifest: CohomologyManifestRead =
        serde_json::from_reader(File::open(directory.join("manifest.json"))?)?;
    ensure!(
        manifest.contract == COHOMOLOGY_CONTRACT,
        "cohomology contract mismatch"
    );
    ensure!(
        manifest.noether_revision == COHOMOLOGY_NOETHER_REVISION
            || manifest.noether_revision == NOETHER_REVISION
            || manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "cohomology Noether revision mismatch"
    );
    ensure!(
        manifest.identity_noether_revision == identity_manifest.noether_revision,
        "cohomology identity revision link mismatch"
    );
    ensure!(
        manifest.total_nondegenerate == total_nondegenerate
            && manifest.maximum_dimension == maximum_dimension
            && manifest.coefficient == format!("F{prime}"),
        "cohomology shell mismatch"
    );
    ensure!(
        manifest.identity_sha256 == identity.sha256,
        "cohomology identity artifact link mismatch"
    );
    for expected in &manifest.artifacts {
        ensure!(
            artifact(&directory.join(&expected.path), expected.row_count)? == *expected,
            "cohomology artifact hash mismatch"
        );
    }
    let expected = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "cohomology.jsonl")
        .context("cohomology manifest omits cohomology.jsonl")?;
    let records = BufReader::new(File::open(directory.join("cohomology.jsonl"))?)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<Result<Vec<SimplicialSetCohomologyRecord>>>()?;
    ensure!(
        records.len() as u64 == expected.row_count,
        "cohomology row count mismatch"
    );
    Ok(records)
}

pub fn emit_steenrod_operations_shell(
    data_root: &Path,
    census: &SteenrodOperationsCensus,
    validation: &SteenrodOperationsValidationReport,
) -> Result<PathBuf> {
    ensure!(
        validation.status == "complete" && validation.contract == STEENROD_OPERATIONS_CONTRACT,
        "refusing unvalidated output"
    );
    ensure!(
        validation.total_nondegenerate == census.total_nondegenerate
            && validation.maximum_dimension == census.maximum_dimension
            && validation.coefficient == format!("F{}", census.prime),
        "validation shell mismatch"
    );
    ensure!(census.prime % 2 == 1, "Steenrod sidecar prime is not odd");

    let identity_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{}", census.maximum_dimension))
        .join(format!("total-{}", census.total_nondegenerate));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT,
        "identity contract mismatch"
    );
    ensure!(
        accepted_identity_noether_revision(&identity_manifest.noether_revision),
        "identity Noether revision mismatch"
    );
    ensure!(
        identity_manifest.total_nondegenerate == census.total_nondegenerate
            && identity_manifest.maximum_dimension == census.maximum_dimension,
        "identity shell mismatch"
    );
    let identity = identity_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    ensure!(
        artifact(
            &identity_directory.join("identities.jsonl"),
            identity.row_count
        )? == *identity,
        "identity artifact hash mismatch"
    );

    let cohomology_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join(format!("cohomology-f{}-v1", census.prime))
        .join(format!("dimension-{}", census.maximum_dimension))
        .join(format!("total-{}", census.total_nondegenerate));
    let cohomology_manifest: CohomologyManifestRead =
        serde_json::from_reader(File::open(cohomology_directory.join("manifest.json"))?)?;
    ensure!(
        cohomology_manifest.contract == COHOMOLOGY_CONTRACT,
        "cohomology contract mismatch"
    );
    ensure!(
        cohomology_manifest.noether_revision == COHOMOLOGY_NOETHER_REVISION
            || cohomology_manifest.noether_revision == NOETHER_REVISION
            || cohomology_manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "cohomology Noether revision mismatch"
    );
    ensure!(
        cohomology_manifest.identity_noether_revision == identity_manifest.noether_revision
            && cohomology_manifest.identity_sha256 == identity.sha256,
        "cohomology identity link mismatch"
    );
    ensure!(
        cohomology_manifest.total_nondegenerate == census.total_nondegenerate
            && cohomology_manifest.maximum_dimension == census.maximum_dimension
            && cohomology_manifest.coefficient == format!("F{}", census.prime),
        "cohomology shell mismatch"
    );
    for expected in &cohomology_manifest.artifacts {
        ensure!(
            artifact(
                &cohomology_directory.join(&expected.path),
                expected.row_count
            )? == *expected,
            "cohomology artifact hash mismatch"
        );
    }
    let cohomology = cohomology_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "cohomology.jsonl")
        .context("cohomology manifest omits cohomology.jsonl")?;
    ensure!(
        cohomology.row_count == census.records.len() as u64,
        "Steenrod/cohomology row count mismatch"
    );

    let collection = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join(format!("steenrod-operations-f{}-v1", census.prime))
        .join(format!("dimension-{}", census.maximum_dimension));
    fs::create_dir_all(&collection)?;
    let destination = collection.join(format!("total-{}", census.total_nondegenerate));
    if destination.exists() {
        bail!(
            "immutable Steenrod shell already exists at {}; remove it deliberately before rebuilding",
            destination.display()
        );
    }
    let staging = collection.join(format!(
        ".total-{}.tmp-{}",
        census.total_nondegenerate,
        std::process::id()
    ));
    fs::create_dir(&staging)?;
    let records_path = staging.join("steenrod-operations.jsonl");
    {
        let file = File::create(&records_path)?;
        let mut writer = BufWriter::new(file);
        for record in &census.records {
            serde_json::to_writer(&mut writer, record)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }
    let validation_path = staging.join("validation.json");
    write_pretty_json(&validation_path, validation)?;
    let manifest = SteenrodOperationsManifest {
        contract: STEENROD_OPERATIONS_CONTRACT,
        noether_revision: NOETHER_REVISION,
        identity_noether_revision: identity_manifest.noether_revision,
        cohomology_noether_revision: cohomology_manifest.noether_revision,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        coefficient: format!("F{}", census.prime),
        identity_sha256: identity.sha256.clone(),
        cohomology_sha256: cohomology.sha256.clone(),
        artifacts: vec![
            artifact(&records_path, census.records.len() as u64)?,
            artifact(&validation_path, 1)?,
        ],
    };
    write_pretty_json(&staging.join("manifest.json"), &manifest)?;
    fs::rename(&staging, &destination)?;
    Ok(destination)
}

pub fn load_steenrod_operations_shell(
    data_root: &Path,
    total_nondegenerate: usize,
    maximum_dimension: usize,
    prime: u64,
) -> Result<Vec<SimplicialSetSteenrodOperationsRecord>> {
    ensure!(prime % 2 == 1, "Steenrod sidecar prime is not odd");
    let identity_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT
            && accepted_identity_noether_revision(&identity_manifest.noether_revision),
        "identity manifest mismatch"
    );
    ensure!(
        identity_manifest.total_nondegenerate == total_nondegenerate
            && identity_manifest.maximum_dimension == maximum_dimension,
        "identity shell mismatch"
    );
    let identity = identity_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    ensure!(
        artifact(
            &identity_directory.join("identities.jsonl"),
            identity.row_count
        )? == *identity,
        "identity artifact hash mismatch"
    );

    let cohomology_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join(format!("cohomology-f{prime}-v1"))
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let cohomology_manifest: CohomologyManifestRead =
        serde_json::from_reader(File::open(cohomology_directory.join("manifest.json"))?)?;
    ensure!(
        cohomology_manifest.contract == COHOMOLOGY_CONTRACT
            && (cohomology_manifest.noether_revision == COHOMOLOGY_NOETHER_REVISION
                || cohomology_manifest.noether_revision == NOETHER_REVISION
                || cohomology_manifest.noether_revision == LEGACY_NOETHER_REVISION),
        "cohomology manifest mismatch"
    );
    ensure!(
        cohomology_manifest.identity_noether_revision == identity_manifest.noether_revision
            && cohomology_manifest.identity_sha256 == identity.sha256
            && cohomology_manifest.total_nondegenerate == total_nondegenerate
            && cohomology_manifest.maximum_dimension == maximum_dimension
            && cohomology_manifest.coefficient == format!("F{prime}"),
        "cohomology source link mismatch"
    );
    for expected in &cohomology_manifest.artifacts {
        ensure!(
            artifact(
                &cohomology_directory.join(&expected.path),
                expected.row_count
            )? == *expected,
            "cohomology artifact hash mismatch"
        );
    }
    let cohomology = cohomology_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "cohomology.jsonl")
        .context("cohomology manifest omits cohomology.jsonl")?;

    let directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join(format!("steenrod-operations-f{prime}-v1"))
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let manifest: SteenrodOperationsManifestRead =
        serde_json::from_reader(File::open(directory.join("manifest.json"))?)?;
    ensure!(
        manifest.contract == STEENROD_OPERATIONS_CONTRACT
            && (manifest.noether_revision == NOETHER_REVISION
                || manifest.noether_revision == LEGACY_NOETHER_REVISION),
        "Steenrod manifest mismatch"
    );
    ensure!(
        manifest.identity_noether_revision == identity_manifest.noether_revision
            && manifest.cohomology_noether_revision == cohomology_manifest.noether_revision
            && manifest.identity_sha256 == identity.sha256
            && manifest.cohomology_sha256 == cohomology.sha256
            && manifest.total_nondegenerate == total_nondegenerate
            && manifest.maximum_dimension == maximum_dimension
            && manifest.coefficient == format!("F{prime}"),
        "Steenrod source link mismatch"
    );
    for expected in &manifest.artifacts {
        ensure!(
            artifact(&directory.join(&expected.path), expected.row_count)? == *expected,
            "Steenrod artifact hash mismatch"
        );
    }
    let expected = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "steenrod-operations.jsonl")
        .context("Steenrod manifest omits steenrod-operations.jsonl")?;
    let records = BufReader::new(File::open(directory.join("steenrod-operations.jsonl"))?)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<Result<Vec<SimplicialSetSteenrodOperationsRecord>>>()?;
    ensure!(
        records.len() as u64 == expected.row_count && expected.row_count == cohomology.row_count,
        "Steenrod row count mismatch"
    );
    Ok(records)
}

pub fn emit_homotopy_shell(
    data_root: &Path,
    census: &HomotopyCensus,
    validation: &HomotopyValidationReport,
) -> Result<PathBuf> {
    ensure!(
        validation.status == "complete" && validation.contract == HOMOTOPY_CONTRACT,
        "refusing unvalidated output"
    );
    ensure!(
        validation.total_nondegenerate == census.total_nondegenerate
            && validation.maximum_dimension == census.maximum_dimension,
        "validation shell mismatch"
    );
    let identity_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{}", census.maximum_dimension))
        .join(format!("total-{}", census.total_nondegenerate));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT,
        "identity contract mismatch"
    );
    ensure!(
        accepted_identity_noether_revision(&identity_manifest.noether_revision),
        "identity Noether revision mismatch"
    );
    ensure!(
        identity_manifest.total_nondegenerate == census.total_nondegenerate
            && identity_manifest.maximum_dimension == census.maximum_dimension,
        "identity shell mismatch"
    );
    let identity = identity_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    ensure!(
        artifact(
            &identity_directory.join("identities.jsonl"),
            identity.row_count
        )? == *identity,
        "identity artifact hash mismatch"
    );

    let collection = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join("homotopy-v1")
        .join(format!("dimension-{}", census.maximum_dimension));
    fs::create_dir_all(&collection)?;
    let destination = collection.join(format!("total-{}", census.total_nondegenerate));
    if destination.exists() {
        bail!(
            "immutable homotopy shell already exists at {}; remove it deliberately before rebuilding",
            destination.display()
        );
    }
    let staging = collection.join(format!(
        ".total-{}.tmp-{}",
        census.total_nondegenerate,
        std::process::id()
    ));
    fs::create_dir(&staging)?;
    let records_path = staging.join("homotopy.jsonl");
    {
        let file = File::create(&records_path)?;
        let mut writer = BufWriter::new(file);
        for record in &census.records {
            serde_json::to_writer(&mut writer, record)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }
    let validation_path = staging.join("validation.json");
    write_pretty_json(&validation_path, validation)?;
    let manifest = HomotopyManifest {
        contract: HOMOTOPY_CONTRACT,
        noether_revision: NOETHER_REVISION,
        identity_noether_revision: identity_manifest.noether_revision,
        total_nondegenerate: census.total_nondegenerate,
        maximum_dimension: census.maximum_dimension,
        identity_sha256: identity.sha256.clone(),
        artifacts: vec![
            artifact(&records_path, census.records.len() as u64)?,
            artifact(&validation_path, 1)?,
        ],
    };
    write_pretty_json(&staging.join("manifest.json"), &manifest)?;
    fs::rename(&staging, &destination)?;
    Ok(destination)
}

pub fn load_homotopy_shell(
    data_root: &Path,
    total_nondegenerate: usize,
    maximum_dimension: usize,
) -> Result<Vec<SimplicialSetHomotopyRecord>> {
    let identity_directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT,
        "identity contract mismatch"
    );
    ensure!(
        accepted_identity_noether_revision(&identity_manifest.noether_revision),
        "identity Noether revision mismatch"
    );
    ensure!(
        identity_manifest.total_nondegenerate == total_nondegenerate
            && identity_manifest.maximum_dimension == maximum_dimension,
        "identity shell mismatch"
    );
    let identity = identity_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    ensure!(
        artifact(
            &identity_directory.join("identities.jsonl"),
            identity.row_count
        )? == *identity,
        "identity artifact hash mismatch"
    );

    let directory = data_root
        .join("finite-simplicial-sets")
        .join("v1")
        .join("enrichments")
        .join("homotopy-v1")
        .join(format!("dimension-{maximum_dimension}"))
        .join(format!("total-{total_nondegenerate}"));
    let manifest: HomotopyManifestRead =
        serde_json::from_reader(File::open(directory.join("manifest.json"))?)?;
    ensure!(
        manifest.contract == HOMOTOPY_CONTRACT,
        "homotopy contract mismatch"
    );
    ensure!(
        manifest.noether_revision == HOMOTOPY_NOETHER_REVISION
            || manifest.noether_revision == NOETHER_REVISION
            || manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "homotopy Noether revision mismatch"
    );
    ensure!(
        manifest.identity_noether_revision == identity_manifest.noether_revision,
        "homotopy identity revision link mismatch"
    );
    ensure!(
        manifest.total_nondegenerate == total_nondegenerate
            && manifest.maximum_dimension == maximum_dimension,
        "homotopy shell mismatch"
    );
    ensure!(
        manifest.identity_sha256 == identity.sha256,
        "homotopy identity artifact link mismatch"
    );
    for expected in &manifest.artifacts {
        ensure!(
            artifact(&directory.join(&expected.path), expected.row_count)? == *expected,
            "homotopy artifact hash mismatch"
        );
    }
    let expected = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "homotopy.jsonl")
        .context("homotopy manifest omits homotopy.jsonl")?;
    let records = BufReader::new(File::open(directory.join("homotopy.jsonl"))?)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<Result<Vec<SimplicialSetHomotopyRecord>>>()?;
    ensure!(
        records.len() as u64 == expected.row_count,
        "homotopy row count mismatch"
    );
    Ok(records)
}

/// Whether this known producer uses a supported identity encoding.
pub fn accepted_identity_noether_revision(revision: &str) -> bool {
    revision == IDENTITY_NOETHER_REVISION
        || revision == TOPOLOGY_NOETHER_REVISION
        || revision == COHOMOLOGY_NOETHER_REVISION
        || revision == HOMOTOPY_NOETHER_REVISION
        || revision == NOETHER_REVISION
        || revision == LEGACY_NOETHER_REVISION
}

fn write_pretty_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}

fn artifact(path: &Path, row_count: u64) -> Result<Artifact> {
    let mut file = File::open(path)?;
    let byte_count = file.metadata()?.len();
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(Artifact {
        path: path
            .file_name()
            .context("artifact has no file name")?
            .to_string_lossy()
            .into_owned(),
        row_count,
        byte_count,
        sha256: hex::encode(digest.finalize()),
    })
}
