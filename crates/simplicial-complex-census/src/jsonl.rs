use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    COHOMOLOGY_CONTRACT, CONTRACT, CohomologyCensus, FIRST_EDITION_NOETHER_REVISION,
    IdentityCensus, LEGACY_NOETHER_REVISION, NOETHER_REVISION, TopologyCensus,
    records::{
        SimplicialComplexCohomologyRecord, SimplicialComplexIdentityRecord,
        SimplicialComplexTopologyRecord,
    },
    validate::{CohomologyValidationReport, TopologyValidationReport, ValidationReport},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Artifact {
    pub path: String,
    pub row_count: u64,
    pub byte_count: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IdentityManifest {
    pub contract: &'static str,
    pub noether_revision: &'static str,
    pub vertices: usize,
    pub generator: &'static str,
    pub candidate_cap: u64,
    pub stats: crate::enumerate::EnumerationStats,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
struct IdentityManifestRead {
    contract: String,
    noether_revision: String,
    vertices: usize,
    artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TopologyManifest {
    pub contract: &'static str,
    pub noether_revision: &'static str,
    pub vertices: usize,
    pub identity_sha256: String,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
struct TopologyManifestRead {
    contract: String,
    noether_revision: String,
    vertices: usize,
    identity_sha256: String,
    artifacts: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
struct CohomologyManifestRead {
    contract: String,
    noether_revision: String,
    #[serde(default)]
    topology_noether_revision: String,
    vertices: usize,
    coefficient: String,
    topology_sha256: String,
    artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CohomologyManifest {
    pub contract: &'static str,
    pub noether_revision: &'static str,
    pub topology_noether_revision: String,
    pub vertices: usize,
    pub coefficient: String,
    pub topology_sha256: String,
    pub artifacts: Vec<Artifact>,
}

/// Atomically install one immutable, fully validated identity shell.
pub fn emit_identity_shell(
    data_root: &Path,
    census: &IdentityCensus,
    validation: &ValidationReport,
) -> Result<PathBuf> {
    ensure!(
        validation.status == "complete",
        "refusing unvalidated output"
    );
    ensure!(
        validation.vertices == census.vertices,
        "validation shell mismatch"
    );
    let collection = data_root.join("finite-simplicial-complexes").join("v1");
    fs::create_dir_all(&collection)?;
    let destination = collection.join(format!("vertices-{}", census.vertices));
    if destination.exists() {
        bail!(
            "immutable identity shell already exists at {}; choose a new data root or remove it deliberately",
            destination.display()
        );
    }
    let staging = collection.join(format!(
        ".vertices-{}.tmp-{}",
        census.vertices,
        std::process::id()
    ));
    fs::create_dir(&staging).with_context(|| {
        format!(
            "could not create unique staging directory {}",
            staging.display()
        )
    })?;

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

    let artifacts = vec![
        artifact(&records_path, census.records.len() as u64)?,
        artifact(&validation_path, 1)?,
    ];
    let manifest = IdentityManifest {
        contract: CONTRACT,
        noether_revision: NOETHER_REVISION,
        vertices: census.vertices,
        generator: census.generator.id(),
        candidate_cap: census.candidate_cap,
        stats: census.stats.clone(),
        artifacts,
    };
    write_pretty_json(&staging.join("manifest.json"), &manifest)?;
    fs::rename(&staging, &destination)?;
    Ok(destination)
}

/// Load and hash-check one immutable identity shell.
pub fn load_identity_shell(
    data_root: &Path,
    vertices: usize,
) -> Result<Vec<SimplicialComplexIdentityRecord>> {
    let directory = data_root
        .join("finite-simplicial-complexes")
        .join("v1")
        .join(format!("vertices-{vertices}"));
    let manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(directory.join("manifest.json"))?)?;
    ensure!(manifest.contract == CONTRACT, "identity contract mismatch");
    ensure!(
        manifest.noether_revision == FIRST_EDITION_NOETHER_REVISION
            || manifest.noether_revision == NOETHER_REVISION
            || manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "identity Noether revision mismatch"
    );
    ensure!(manifest.vertices == vertices, "identity shell mismatch");
    let expected = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    let path = directory.join("identities.jsonl");
    let observed = artifact(&path, expected.row_count)?;
    ensure!(observed == *expected, "identity artifact hash mismatch");

    let reader = BufReader::new(File::open(path)?);
    let records = reader
        .lines()
        .map(|line| {
            let record: SimplicialComplexIdentityRecord = serde_json::from_str(&line?)?;
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

/// Atomically install a complete topology enrichment sidecar.
pub fn emit_topology_shell(
    data_root: &Path,
    census: &TopologyCensus,
    validation: &TopologyValidationReport,
) -> Result<PathBuf> {
    ensure!(
        validation.status == "complete",
        "refusing unvalidated output"
    );
    ensure!(
        validation.vertices == census.vertices,
        "validation shell mismatch"
    );
    let identity_directory = data_root
        .join("finite-simplicial-complexes")
        .join("v1")
        .join(format!("vertices-{}", census.vertices));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT,
        "identity contract mismatch"
    );
    ensure!(
        identity_manifest.noether_revision == FIRST_EDITION_NOETHER_REVISION
            || identity_manifest.noether_revision == NOETHER_REVISION
            || identity_manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "identity Noether revision mismatch"
    );
    ensure!(
        identity_manifest.vertices == census.vertices,
        "identity shell mismatch"
    );
    let identity = identity_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "identities.jsonl")
        .context("identity manifest omits identities.jsonl")?;
    let observed_identity = artifact(
        &identity_directory.join("identities.jsonl"),
        identity.row_count,
    )?;
    ensure!(
        observed_identity == *identity,
        "identity artifact hash mismatch"
    );

    let collection = data_root
        .join("finite-simplicial-complexes")
        .join("v1")
        .join("enrichments")
        .join("topology-v1");
    fs::create_dir_all(&collection)?;
    let destination = collection.join(format!("vertices-{}", census.vertices));
    if destination.exists() {
        bail!(
            "immutable topology shell already exists at {}; remove it deliberately before rebuilding",
            destination.display()
        );
    }
    let staging = collection.join(format!(
        ".vertices-{}.tmp-{}",
        census.vertices,
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
    let artifacts = vec![
        artifact(&records_path, census.records.len() as u64)?,
        artifact(&validation_path, 1)?,
    ];
    let manifest = TopologyManifest {
        contract: CONTRACT,
        noether_revision: NOETHER_REVISION,
        vertices: census.vertices,
        identity_sha256: identity.sha256.clone(),
        artifacts,
    };
    write_pretty_json(&staging.join("manifest.json"), &manifest)?;
    fs::rename(&staging, &destination)?;
    Ok(destination)
}

/// Load and hash-check one topology sidecar.
pub fn load_topology_shell(
    data_root: &Path,
    vertices: usize,
) -> Result<Vec<SimplicialComplexTopologyRecord>> {
    let version = data_root.join("finite-simplicial-complexes").join("v1");
    let identity_directory = version.join(format!("vertices-{vertices}"));
    let identity_manifest: IdentityManifestRead =
        serde_json::from_reader(File::open(identity_directory.join("manifest.json"))?)?;
    ensure!(
        identity_manifest.contract == CONTRACT,
        "identity contract mismatch"
    );
    ensure!(
        identity_manifest.noether_revision == FIRST_EDITION_NOETHER_REVISION
            || identity_manifest.noether_revision == NOETHER_REVISION
            || identity_manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "identity Noether revision mismatch"
    );
    ensure!(
        identity_manifest.vertices == vertices,
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

    let directory = version
        .join("enrichments")
        .join("topology-v1")
        .join(format!("vertices-{vertices}"));
    let manifest: TopologyManifestRead =
        serde_json::from_reader(File::open(directory.join("manifest.json"))?)?;
    ensure!(manifest.contract == CONTRACT, "topology contract mismatch");
    ensure!(
        manifest.noether_revision == FIRST_EDITION_NOETHER_REVISION
            || manifest.noether_revision == NOETHER_REVISION
            || manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "topology Noether revision mismatch"
    );
    ensure!(manifest.vertices == vertices, "topology shell mismatch");
    ensure!(
        manifest.identity_sha256 == identity.sha256,
        "topology identity-link mismatch"
    );
    let expected = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "topology.jsonl")
        .context("topology manifest omits topology.jsonl")?;
    let path = directory.join("topology.jsonl");
    ensure!(
        artifact(&path, expected.row_count)? == *expected,
        "topology artifact hash mismatch"
    );
    let records = BufReader::new(File::open(path)?)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<Result<Vec<SimplicialComplexTopologyRecord>>>()?;
    ensure!(
        records.len() as u64 == expected.row_count,
        "topology row count mismatch"
    );
    Ok(records)
}

/// Atomically install one complete finite-field cohomology sidecar.
pub fn emit_cohomology_shell(
    data_root: &Path,
    census: &CohomologyCensus,
    validation: &CohomologyValidationReport,
) -> Result<PathBuf> {
    ensure!(
        validation.status == "complete",
        "refusing unvalidated output"
    );
    ensure!(
        validation.vertices == census.vertices
            && validation.coefficient == format!("F{}", census.prime),
        "validation shell mismatch"
    );
    let topology_directory = data_root
        .join("finite-simplicial-complexes")
        .join("v1")
        .join("enrichments")
        .join("topology-v1")
        .join(format!("vertices-{}", census.vertices));
    let topology_manifest: TopologyManifestRead =
        serde_json::from_reader(File::open(topology_directory.join("manifest.json"))?)?;
    ensure!(
        topology_manifest.contract == CONTRACT,
        "topology contract mismatch"
    );
    ensure!(
        topology_manifest.noether_revision == FIRST_EDITION_NOETHER_REVISION
            || topology_manifest.noether_revision == NOETHER_REVISION
            || topology_manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "topology Noether revision mismatch"
    );
    ensure!(
        topology_manifest.vertices == census.vertices,
        "topology shell mismatch"
    );
    let topology = topology_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "topology.jsonl")
        .context("topology manifest omits topology.jsonl")?;
    ensure!(
        artifact(
            &topology_directory.join("topology.jsonl"),
            topology.row_count
        )? == *topology,
        "topology artifact hash mismatch"
    );

    let collection = data_root
        .join("finite-simplicial-complexes")
        .join("v1")
        .join("enrichments")
        .join(format!("cohomology-f{}-v2", census.prime));
    fs::create_dir_all(&collection)?;
    let destination = collection.join(format!("vertices-{}", census.vertices));
    if destination.exists() {
        bail!(
            "immutable cohomology shell already exists at {}; remove it deliberately before rebuilding",
            destination.display()
        );
    }
    let staging = collection.join(format!(
        ".vertices-{}.tmp-{}",
        census.vertices,
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
    let artifacts = vec![
        artifact(&records_path, census.records.len() as u64)?,
        artifact(&validation_path, 1)?,
    ];
    let manifest = CohomologyManifest {
        contract: COHOMOLOGY_CONTRACT,
        noether_revision: NOETHER_REVISION,
        topology_noether_revision: topology_manifest.noether_revision,
        vertices: census.vertices,
        coefficient: format!("F{}", census.prime),
        topology_sha256: topology.sha256.clone(),
        artifacts,
    };
    write_pretty_json(&staging.join("manifest.json"), &manifest)?;
    fs::rename(&staging, &destination)?;
    Ok(destination)
}

/// Load and hash-check the selected finite-field cohomology sidecar.
///
/// The first edition retains its immutable `F3` version-one artifact. `F2`
/// selects version two, which adds the complete unstable square table.
pub fn load_cohomology_shell(
    data_root: &Path,
    vertices: usize,
    prime: u64,
) -> Result<Vec<SimplicialComplexCohomologyRecord>> {
    let topology_directory = data_root
        .join("finite-simplicial-complexes")
        .join("v1")
        .join("enrichments")
        .join("topology-v1")
        .join(format!("vertices-{vertices}"));
    let topology_manifest: TopologyManifestRead =
        serde_json::from_reader(File::open(topology_directory.join("manifest.json"))?)?;
    ensure!(
        topology_manifest.contract == CONTRACT,
        "topology contract mismatch"
    );
    ensure!(
        topology_manifest.noether_revision == FIRST_EDITION_NOETHER_REVISION
            || topology_manifest.noether_revision == NOETHER_REVISION
            || topology_manifest.noether_revision == LEGACY_NOETHER_REVISION,
        "topology Noether revision mismatch"
    );
    ensure!(
        topology_manifest.vertices == vertices,
        "topology shell mismatch"
    );
    let topology = topology_manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "topology.jsonl")
        .context("topology manifest omits topology.jsonl")?;
    ensure!(
        artifact(
            &topology_directory.join("topology.jsonl"),
            topology.row_count
        )? == *topology,
        "topology artifact hash mismatch"
    );

    let enrichment_root = data_root
        .join("finite-simplicial-complexes")
        .join("v1")
        .join("enrichments");
    let version_two = enrichment_root
        .join(format!("cohomology-f{prime}-v2"))
        .join(format!("vertices-{vertices}"));
    let (directory, legacy) = if version_two.exists() {
        (version_two, false)
    } else {
        ensure!(
            prime == 3,
            "missing selected version-two cohomology artifact"
        );
        (
            enrichment_root
                .join("cohomology-f3-v1")
                .join(format!("vertices-{vertices}")),
            true,
        )
    };
    let manifest: CohomologyManifestRead =
        serde_json::from_reader(File::open(directory.join("manifest.json"))?)?;
    if legacy {
        ensure!(manifest.contract == CONTRACT, "legacy cohomology contract");
        ensure!(
            manifest.noether_revision == FIRST_EDITION_NOETHER_REVISION,
            "legacy cohomology Noether revision"
        );
        ensure!(
            manifest.topology_noether_revision.is_empty(),
            "legacy cohomology unexpectedly names a topology revision"
        );
    } else {
        ensure!(
            manifest.contract == COHOMOLOGY_CONTRACT,
            "cohomology contract mismatch"
        );
        ensure!(
            manifest.noether_revision == NOETHER_REVISION
                || manifest.noether_revision == LEGACY_NOETHER_REVISION,
            "cohomology Noether revision mismatch"
        );
        ensure!(
            manifest.topology_noether_revision == topology_manifest.noether_revision,
            "cohomology topology-revision link mismatch"
        );
    }
    ensure!(
        manifest.vertices == vertices && manifest.coefficient == format!("F{prime}"),
        "cohomology shell mismatch"
    );
    ensure!(
        manifest.topology_sha256 == topology.sha256,
        "cohomology topology-link mismatch"
    );
    let expected = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "cohomology.jsonl")
        .context("cohomology manifest omits cohomology.jsonl")?;
    let path = directory.join("cohomology.jsonl");
    ensure!(
        artifact(&path, expected.row_count)? == *expected,
        "cohomology artifact hash mismatch"
    );
    let records = BufReader::new(File::open(path)?)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<Result<Vec<SimplicialComplexCohomologyRecord>>>()?;
    ensure!(
        records.len() as u64 == expected.row_count,
        "cohomology row count mismatch"
    );
    Ok(records)
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
