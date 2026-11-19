//! Replay validation into an explicit copied successor root, retaining upstream reports.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
struct Options {
    #[arg(long)]
    data_root: PathBuf,
}
fn install(directory: &Path, report: &impl Serialize) -> Result<()> {
    ensure!(
        !directory.join("upstream-manifest.json").exists(),
        "validation successor already exists"
    );
    let path = directory.join("manifest.json");
    let bytes = fs::read(&path)?;
    let mut manifest: Value = serde_json::from_slice(&bytes)?;
    fs::write(directory.join("upstream-manifest.json"), &bytes)?;
    fs::copy(
        directory.join("validation.json"),
        directory.join("upstream-validation.json"),
    )?;
    let bytes = serde_json::to_vec_pretty(report)?;
    fs::write(directory.join("validation.json"), &bytes)?;
    for artifact in manifest["artifacts"].as_array_mut().unwrap() {
        if artifact["path"] == "validation.json" {
            artifact["sha256"] = json!(hex::encode(Sha256::digest(&bytes)));
            artifact["byte_count"] = json!(bytes.len());
        }
    }
    manifest["revalidation"] = json!({"producer":poset_census::source_identity(), "upstream_manifest_sha256":hex::encode(Sha256::digest(fs::read(directory.join("upstream-manifest.json"))?))});
    fs::write(path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}
fn main() -> Result<()> {
    let root = Options::parse().data_root;
    for vertices in 0..=6 {
        use simplicial_complex_census::{
            CohomologyCensus, Generator, IdentityCensus, jsonl, validate,
        };
        let directory = root.join(format!(
            "finite-simplicial-complexes/v1/vertices-{vertices}"
        ));
        let manifest: Value = serde_json::from_slice(&fs::read(directory.join("manifest.json"))?)?;
        let records = jsonl::load_identity_shell(&root, vertices)?;
        let census = IdentityCensus {
            generator: Generator::CanonicalVertexLink,
            vertices,
            candidate_cap: manifest["candidate_cap"].as_u64().unwrap(),
            stats: serde_json::from_value(manifest["stats"].clone())?,
            records,
        };
        install(&directory, &validate::validate_identity_census(&census)?)?;
        let topology = jsonl::load_topology_shell(&root, vertices)?;
        for prime in [2, 3] {
            let records = jsonl::load_cohomology_shell_version(&root, vertices, prime, 2)?;
            let cohomology = CohomologyCensus {
                vertices,
                prime,
                records,
            };
            let validation =
                validate::validate_cohomology_census(&cohomology, &census.records, &topology)?;
            install(&root.join(format!("finite-simplicial-complexes/v1/enrichments/cohomology-f{prime}-v2/vertices-{vertices}")), &validation)?;
        }
        eprintln!("revalidated complex identity/cohomology shell {vertices}");
    }
    for (total, dimension) in (0..=6).map(|total| (total, 3)).chain([(7, 2)]) {
        use simplicial_set_census::{IdentityCensus, jsonl, validate};
        let directory = root.join(format!(
            "finite-simplicial-sets/v1/dimension-{dimension}/total-{total}"
        ));
        let manifest: Value = serde_json::from_slice(&fs::read(directory.join("manifest.json"))?)?;
        let records = jsonl::load_identity_shell(&root, total, dimension)?;
        let census = IdentityCensus {
            total_nondegenerate: total,
            maximum_dimension: dimension,
            candidate_cap: manifest["candidate_cap"].as_u64().unwrap(),
            stats: serde_json::from_value(manifest["stats"].clone())?,
            records,
        };
        install(&directory, &validate::validate_identity_census(&census)?)?;
        eprintln!("revalidated simplicial-set identity shell {total}/{dimension}");
    }
    Ok(())
}
