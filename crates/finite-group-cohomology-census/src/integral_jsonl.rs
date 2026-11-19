//! Immutable JSONL artifacts for exact integral group-cohomology extents.

use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    INTEGRAL_CONTRACT, INTEGRAL_NOETHER_REVISION, IntegralCensus, LEGACY_INTEGRAL_NOETHER_REVISION,
    integral::{IntegralValidationReport, validate_records_sorted},
    jsonl::Artifact,
    records::FiniteGroupIntegralCohomologyRecord,
    source::source_sha256,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralManifest {
    pub contract: String,
    pub noether_revision: String,
    pub source_sha256: String,
    pub maximum_order: usize,
    pub additive_through_degree: usize,
    pub bar_entry_limit: usize,
    pub source_groups: usize,
    pub complete_presentations: usize,
    pub complete_cyclic_presentations: usize,
    pub complete_rank_two_abelian_presentations: usize,
    pub exact_low_degree_truncations: usize,
    pub exact_sparse_bar_truncations: usize,
    pub exact_degrees: std::collections::BTreeMap<usize, usize>,
    pub pending_groups: usize,
    pub validation: IntegralValidationReport,
    pub artifacts: Vec<Artifact>,
}

pub fn integral_path(
    root: &Path,
    maximum_order: usize,
    additive_through_degree: usize,
    bar_entry_limit: usize,
) -> PathBuf {
    root.join("finite-group-cohomology")
        .join("integral-v4")
        .join(format!("order-through-{maximum_order}"))
        .join(format!("additive-degree-{additive_through_degree}"))
        .join(format!("bar-entry-limit-{bar_entry_limit}"))
}

pub fn emit_integral_census(
    root: &Path,
    census: &IntegralCensus,
    validation: IntegralValidationReport,
) -> Result<IntegralManifest> {
    validate_records_sorted(&census.records)?;
    ensure!(validation.contract == INTEGRAL_CONTRACT);
    ensure!(validation.maximum_order == census.maximum_order);
    ensure!(validation.additive_through_degree == census.additive_through_degree);
    ensure!(validation.bar_entry_limit == census.bar_entry_limit);
    ensure!(validation.source_groups == census.source_group_count);
    ensure!(
        validation.complete_presentations
            + validation.exact_low_degree_truncations
            + validation.exact_sparse_bar_truncations
            == census.records.len()
    );
    let destination = integral_path(
        root,
        census.maximum_order,
        census.additive_through_degree,
        census.bar_entry_limit,
    );
    ensure!(!destination.exists(), "immutable artifact already exists");
    let parent = destination
        .parent()
        .context("artifact path has no parent")?;
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(
        ".bar-entry-limit-{}.{}.tmp",
        census.bar_entry_limit,
        std::process::id()
    ));
    ensure!(!staging.exists(), "staging path already exists");
    fs::create_dir(&staging)?;
    let records_path = staging.join("records.jsonl");
    let mut writer = BufWriter::new(File::create(&records_path)?);
    let mut digest = Sha256::new();
    let mut byte_count = 0u64;
    for record in &census.records {
        let mut line = serde_json::to_vec(record)?;
        line.push(b'\n');
        writer.write_all(&line)?;
        digest.update(&line);
        byte_count = byte_count
            .checked_add(u64::try_from(line.len())?)
            .context("integral artifact byte count overflow")?;
    }
    writer.flush()?;
    let artifact = Artifact {
        path: "records.jsonl".to_owned(),
        row_count: census.records.len(),
        byte_count,
        sha256: hex::encode(digest.finalize()),
    };
    let manifest = IntegralManifest {
        contract: INTEGRAL_CONTRACT.to_owned(),
        noether_revision: INTEGRAL_NOETHER_REVISION.to_owned(),
        source_sha256: source_sha256(),
        maximum_order: census.maximum_order,
        additive_through_degree: census.additive_through_degree,
        bar_entry_limit: census.bar_entry_limit,
        source_groups: census.source_group_count,
        complete_presentations: validation.complete_presentations,
        complete_cyclic_presentations: validation.complete_cyclic_presentations,
        complete_rank_two_abelian_presentations: validation.complete_rank_two_abelian_presentations,
        exact_low_degree_truncations: validation.exact_low_degree_truncations,
        exact_sparse_bar_truncations: validation.exact_sparse_bar_truncations,
        exact_degrees: validation.exact_degrees.clone(),
        pending_groups: validation.pending_groups,
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

pub fn load_integral_census(
    root: &Path,
    maximum_order: usize,
    additive_through_degree: usize,
    bar_entry_limit: usize,
) -> Result<(IntegralManifest, IntegralCensus)> {
    let directory = integral_path(
        root,
        maximum_order,
        additive_through_degree,
        bar_entry_limit,
    );
    let manifest: IntegralManifest =
        serde_json::from_slice(&fs::read(directory.join("manifest.json"))?)?;
    ensure!(manifest.contract == INTEGRAL_CONTRACT, "integral contract");
    ensure!(
        (manifest.noether_revision == INTEGRAL_NOETHER_REVISION
            || manifest.noether_revision == crate::PRERELEASE_NOETHER_REVISION)
            || manifest.noether_revision == LEGACY_INTEGRAL_NOETHER_REVISION,
        "integral Noether revision"
    );
    ensure!(manifest.source_sha256 == source_sha256(), "source hash");
    ensure!(
        manifest.maximum_order == maximum_order
            && manifest.additive_through_degree == additive_through_degree
            && manifest.bar_entry_limit == bar_entry_limit,
        "integral manifest shell"
    );
    ensure!(manifest.artifacts.len() == 1, "integral artifact inventory");
    let artifact = &manifest.artifacts[0];
    let bytes = fs::read(directory.join(&artifact.path))?;
    ensure!(
        u64::try_from(bytes.len())? == artifact.byte_count,
        "integral artifact bytes"
    );
    ensure!(
        hex::encode(Sha256::digest(&bytes)) == artifact.sha256,
        "integral artifact hash"
    );
    let records = BufReader::new(bytes.as_slice())
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<Result<Vec<FiniteGroupIntegralCohomologyRecord>>>()?;
    ensure!(
        records.len() == artifact.row_count,
        "integral artifact rows"
    );
    ensure!(
        records.len()
            == manifest.complete_presentations
                + manifest.exact_low_degree_truncations
                + manifest.exact_sparse_bar_truncations,
        "integral manifest record count"
    );
    validate_records_sorted(&records)?;
    let source_group_count = manifest.source_groups;
    Ok((
        manifest,
        IntegralCensus {
            maximum_order,
            additive_through_degree,
            bar_entry_limit,
            source_group_count,
            records,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        integral::{integral_census, validate_integral_census},
        source::load_source,
    };

    #[test]
    fn integral_artifact_round_trips_and_recomputes() {
        let source = load_source().unwrap();
        let census = integral_census(&source, 4, 8, 100_000).unwrap();
        let validation = validate_integral_census(&source, &census).unwrap();
        let root = tempfile::tempdir().unwrap();
        let emitted = emit_integral_census(root.path(), &census, validation).unwrap();
        let (loaded_manifest, loaded) = load_integral_census(root.path(), 4, 8, 100_000).unwrap();
        assert_eq!(loaded_manifest, emitted);
        assert_eq!(loaded.records, census.records);
        assert_eq!(
            validate_integral_census(&source, &loaded).unwrap(),
            loaded_manifest.validation
        );
    }
}
