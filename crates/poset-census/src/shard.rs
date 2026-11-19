//! Restartable deterministic shards for large orderly census levels.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use noether::{
    combin::Poset,
    groups::{Permutation, PermutationGroup},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use crate::{
    CONTRACT, NOETHER_REVISION, RECORD_VERSION,
    enrich::enrich_poset_with_core_cache,
    enumerate::extend_orderly_parent,
    records::{ComplexKey, ComplexRecord, PosetKey, PosetRecord},
    validate::accepted_order_counts,
};

/// Versioned execution strategy for canonical-parent shards.
pub const SHARD_GENERATOR: &str = "hereditary-downset-parent-range/v1";
const MAX_MERGE_FAN_IN: usize = 64;

/// One invocation of a deterministic parent-range shard.
#[derive(Clone, Debug)]
pub struct ShardOptions {
    pub data_root: PathBuf,
    pub run_id: String,
    pub target_order: usize,
    pub shard_id: usize,
    pub shard_count: usize,
    pub candidate_cap: u64,
    pub checkpoint_every: usize,
    /// Optional lease bound. A later invocation resumes from the checkpoint.
    pub parent_budget: Option<usize>,
}

/// Result of one shard invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShardOutcome {
    pub directory: PathBuf,
    pub complete: bool,
    pub parent_start: usize,
    pub parent_end: usize,
    pub next_parent_index: usize,
    pub accepted_count: u64,
}

/// Merge all completed shards for one target order.
#[derive(Clone, Debug)]
pub struct MergeOptions {
    pub data_root: PathBuf,
    pub run_id: String,
    pub target_order: usize,
    pub shard_count: usize,
}

/// Completed merge and its integrity/count report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeOutcome {
    pub directory: PathBuf,
    pub poset_count: u64,
    pub complex_count: u64,
}

/// Independently reconstruct every mathematical row in one completed shard.
#[derive(Clone, Debug)]
pub struct CertificationOptions {
    pub data_root: PathBuf,
    pub run_id: String,
    pub target_order: usize,
    pub shard_id: usize,
    pub shard_count: usize,
}

/// Completed full-row certification for one shard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertificationOutcome {
    pub directory: PathBuf,
    pub poset_count: u64,
    pub complex_count: u64,
}

/// Publish a merge only after every source shard has a full-row certificate.
#[derive(Clone, Debug)]
pub struct PublishOptions {
    pub data_root: PathBuf,
    pub run_id: String,
    pub target_order: usize,
    pub shard_count: usize,
}

/// Immutable completed-order output installed by certified publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishOutcome {
    pub directory: PathBuf,
    pub poset_count: u64,
    pub complex_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Artifact {
    path: String,
    row_count: u64,
    byte_count: u64,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ParentCorpus {
    posets: Artifact,
    validation: Artifact,
    run: Artifact,
}

#[derive(Debug, Deserialize)]
struct ParentValidationEvidence {
    contract: String,
    order: usize,
    status: String,
    isomorphism_scope: ParentScopeEvidence,
}

#[derive(Debug, Deserialize)]
struct ParentScopeEvidence {
    order: usize,
    status: String,
}

#[derive(Debug, Deserialize)]
struct ParentRunEvidence {
    contract: String,
    parameters: ParentRunParameters,
    outputs: Vec<Artifact>,
    validation_sha256: String,
}

#[derive(Debug, Deserialize)]
struct ParentRunParameters {
    order: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct WorkStats {
    parent_count: u64,
    downset_count: u64,
    orbit_representative_count: u64,
    canonical_test_count: u64,
    accepted_count: u64,
}

impl WorkStats {
    fn checked_add(&mut self, other: &Self) -> Result<()> {
        self.parent_count = checked_sum(self.parent_count, other.parent_count, "parent")?;
        self.downset_count = checked_sum(self.downset_count, other.downset_count, "downset")?;
        self.orbit_representative_count = checked_sum(
            self.orbit_representative_count,
            other.orbit_representative_count,
            "orbit-representative",
        )?;
        self.canonical_test_count = checked_sum(
            self.canonical_test_count,
            other.canonical_test_count,
            "canonical-test",
        )?;
        self.accepted_count =
            checked_sum(self.accepted_count, other.accepted_count, "accepted-child")?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ParentRange {
    start: usize,
    end: usize,
    total: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PartReport {
    index: usize,
    parent_start: usize,
    parent_end: usize,
    last_parent_key: PosetKey,
    posets: Artifact,
    complexes: Artifact,
    stats: WorkStats,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ShardCheckpoint {
    contract: String,
    record_version: u32,
    generator: String,
    noether_revision: String,
    run_id: String,
    target_order: usize,
    shard_id: usize,
    shard_count: usize,
    candidate_cap: u64,
    checkpoint_every: usize,
    parent_input: ParentCorpus,
    parent_range: ParentRange,
    next_parent_index: usize,
    last_parent_key: Option<PosetKey>,
    last_child_key: Option<PosetKey>,
    parts: Vec<PartReport>,
    stats: WorkStats,
    complete: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ShardManifest {
    contract: String,
    record_version: u32,
    generator: String,
    noether_revision: String,
    run_id: String,
    target_order: usize,
    shard_id: usize,
    shard_count: usize,
    candidate_cap: u64,
    checkpoint_every: usize,
    parent_input: ParentCorpus,
    parent_range: ParentRange,
    parts: Vec<PartReport>,
    stats: WorkStats,
    posets: Artifact,
    complexes: Artifact,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MergeChecks {
    shard_ranges_complete: bool,
    shard_hashes_verified: bool,
    poset_keys_unique_and_sorted: bool,
    complex_keys_unique_and_sorted: bool,
    unlabeled_count: CountResult,
    connected_unlabeled_count: CountResult,
    labeled_count_from_automorphisms: CountResult,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CountResult {
    expected: String,
    observed: String,
    passed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MergeReport {
    contract: String,
    record_version: u32,
    operation: String,
    generator: String,
    noether_revision: String,
    run_id: String,
    target_order: usize,
    shard_count: usize,
    parent_input: ParentCorpus,
    shard_manifests: Vec<Artifact>,
    outputs: Vec<Artifact>,
    checks: MergeChecks,
    status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CertificationChecks {
    source_hashes_verified: bool,
    keys_unique_and_sorted: bool,
    canonical_records_reconstructed: bool,
    automorphism_actions_reconstructed: bool,
    core_reductions_replayed: bool,
    opposite_relations_reconstructed: bool,
    order_complexes_reconstructed: bool,
    integral_homology_reconstructed: bool,
    every_complex_referenced: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CertificationReport {
    #[serde(default)]
    validation_version: u32,
    contract: String,
    record_version: u32,
    operation: String,
    generator: String,
    noether_revision: String,
    run_id: String,
    target_order: usize,
    shard_id: usize,
    shard_count: usize,
    source_manifest: Artifact,
    posets: Artifact,
    complexes: Artifact,
    checks: CertificationChecks,
    status: String,
}

#[derive(Debug, Serialize)]
struct PublishedScope {
    order: usize,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct PublishedHomotopyScope {
    source_orders_through: usize,
    status: &'static str,
}

/// Gates run by version-two shard certification before publication.
#[derive(Debug, Serialize)]
struct PublishedChecks {
    shard_ranges_complete: bool,
    shard_hashes_verified: bool,
    poset_keys_unique_and_sorted: bool,
    complex_keys_unique_and_sorted: bool,
    unlabeled_posets: CountResult,
    connected_unlabeled_posets: CountResult,
    labeled_posets_from_automorphisms: CountResult,
    full_record_reconstruction: bool,
    canonical_keys_and_covers: bool,
    automorphism_generators_and_orbits: bool,
    core_replay_and_idempotence: bool,
    opposite_relations: bool,
    order_complex_reconstruction: bool,
    euler_and_integral_homology: bool,
    core_homology_invariance: bool,
}

#[derive(Debug, Serialize)]
struct PublishedValidationReport {
    contract: &'static str,
    order: usize,
    status: &'static str,
    generator: &'static str,
    raw_candidate_count: u64,
    accepted_count_sources: PublishedCountSources,
    isomorphism_scope: PublishedScope,
    homotopy_scope: PublishedHomotopyScope,
    certification_shard_count: usize,
    certification_root_sha256: String,
    checks: PublishedChecks,
    external_oracles: Vec<PublishedOracle>,
}

#[derive(Debug, Serialize)]
struct PublishedCountSources {
    unlabeled_posets: &'static str,
    connected_unlabeled_posets: &'static str,
    labeled_posets: &'static str,
}

#[derive(Debug, Serialize)]
struct PublishedOracle {
    name: &'static str,
    status: &'static str,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct PublishedRunParameters {
    order: usize,
    shard_count: usize,
    raw_candidate_count: u64,
    generator: &'static str,
    seed: Option<u64>,
}

#[derive(Debug, Serialize)]
struct PublishedRevision {
    revision: &'static str,
    dirty: bool,
}

#[derive(Debug, Serialize)]
struct PublishedRevisions {
    noether: PublishedRevision,
    poset_census: PublishedRevision,
}

#[derive(Debug, Serialize)]
struct PublishedRunReport {
    contract: &'static str,
    run_id: String,
    operation: &'static str,
    published_unix_milliseconds: u128,
    revisions: PublishedRevisions,
    parameters: PublishedRunParameters,
    inputs: Vec<Artifact>,
    outputs: Vec<Artifact>,
    validation_sha256: String,
    certification_root_sha256: String,
    reconstruction_command: String,
}

/// Run or resume one deterministic canonical-parent shard.
pub fn run_orderly_shard(options: &ShardOptions) -> Result<ShardOutcome> {
    validate_shard_options(options)?;
    let parent_path = parent_posets_path(&options.data_root, options.target_order - 1);
    let parent_input = inspect_parent_corpus(&options.data_root, options.target_order - 1)?;
    let expected_parent_count =
        usize::try_from(accepted_order_counts(options.target_order - 1)?.unlabeled)
            .context("accepted parent count does not fit usize")?;
    ensure!(
        parent_input.posets.row_count == expected_parent_count as u64,
        "parent corpus has {} rows; accepted count is {expected_parent_count}",
        parent_input.posets.row_count
    );
    let parent_range =
        balanced_range(expected_parent_count, options.shard_id, options.shard_count)?;
    let run_root = work_run_root(&options.data_root, &options.run_id);
    let shard_directory = shard_directory(
        &run_root,
        options.target_order,
        options.shard_id,
        options.shard_count,
    );
    let checkpoint_path = checkpoint_path(
        &run_root,
        options.target_order,
        options.shard_id,
        options.shard_count,
    );
    fs::create_dir_all(shard_directory.join("parts"))
        .with_context(|| format!("creating {}", shard_directory.display()))?;
    fs::create_dir_all(
        checkpoint_path
            .parent()
            .context("checkpoint path has no parent")?,
    )?;
    let lock_path = shard_directory.join("invocation.lock");
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)?;
    lock.try_lock_exclusive().with_context(|| {
        format!(
            "shard {} is already being updated by another invocation",
            shard_directory.display()
        )
    })?;

    let manifest_path = shard_directory.join("manifest.json");
    if manifest_path.exists() {
        let manifest: ShardManifest = read_json(&manifest_path)?;
        verify_manifest_identity(&manifest, options, &parent_input, &parent_range)?;
        verify_manifest_structure(&manifest)?;
        verify_artifact(&shard_directory, &manifest.posets)?;
        verify_artifact(&shard_directory, &manifest.complexes)?;
        return Ok(ShardOutcome {
            directory: shard_directory,
            complete: true,
            parent_start: parent_range.start,
            parent_end: parent_range.end,
            next_parent_index: parent_range.end,
            accepted_count: manifest.stats.accepted_count,
        });
    }

    let mut checkpoint = if checkpoint_path.exists() {
        let checkpoint: ShardCheckpoint = read_json(&checkpoint_path)?;
        verify_checkpoint_identity(&checkpoint, options, &parent_input, &parent_range)?;
        verify_checkpoint_structure(&checkpoint)?;
        for part in &checkpoint.parts {
            verify_artifact(&shard_directory, &part.posets)?;
            verify_artifact(&shard_directory, &part.complexes)?;
        }
        ensure!(
            checkpoint.last_child_key == last_child_key(&shard_directory, &checkpoint.parts)?,
            "checkpoint last child key does not match its completed parts"
        );
        checkpoint
    } else {
        let checkpoint = initial_checkpoint(options, parent_input, parent_range.clone());
        write_json_atomic(&checkpoint_path, &checkpoint)?;
        checkpoint
    };

    let processed = process_parent_range(
        options,
        &parent_path,
        &shard_directory,
        &checkpoint_path,
        &mut checkpoint,
    )?;
    if checkpoint.next_parent_index < checkpoint.parent_range.end {
        return Ok(ShardOutcome {
            directory: shard_directory,
            complete: false,
            parent_start: checkpoint.parent_range.start,
            parent_end: checkpoint.parent_range.end,
            next_parent_index: checkpoint.next_parent_index,
            accepted_count: checkpoint.stats.accepted_count,
        });
    }
    debug_assert!(processed || checkpoint.parent_range.start == checkpoint.parent_range.end);

    let manifest = finalize_shard(&shard_directory, &checkpoint)?;
    write_json_atomic(&manifest_path, &manifest)?;
    checkpoint.complete = true;
    write_json_atomic(&checkpoint_path, &checkpoint)?;
    Ok(ShardOutcome {
        directory: shard_directory,
        complete: true,
        parent_start: checkpoint.parent_range.start,
        parent_end: checkpoint.parent_range.end,
        next_parent_index: checkpoint.parent_range.end,
        accepted_count: checkpoint.stats.accepted_count,
    })
}

/// Merge completed shards into a deterministic candidate census slice.
///
/// This clears artifact, sorting, uniqueness, and accepted-count gates. It
/// deliberately writes beneath `work/`; independent mathematical validation
/// must still run before publishing the immutable `census/` directory.
pub fn merge_orderly_shards(options: &MergeOptions) -> Result<MergeOutcome> {
    validate_merge_options(options)?;
    let run_root = work_run_root(&options.data_root, &options.run_id);
    fs::create_dir_all(&run_root)?;
    let merge_lock_path = run_root.join(format!("merge-order={}.lock", options.target_order));
    let merge_lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&merge_lock_path)?;
    merge_lock.try_lock_exclusive().with_context(|| {
        format!(
            "order {} is already being merged by another invocation",
            options.target_order
        )
    })?;
    let final_directory = run_root
        .join("merged")
        .join(format!("order={}", options.target_order));
    let report_path = final_directory.join("merge-validation.json");
    if report_path.exists() {
        let report: MergeReport = read_json(&report_path)?;
        verify_merge_identity(&report, options)?;
        for artifact in &report.outputs {
            verify_artifact(&final_directory, artifact)?;
        }
        return Ok(MergeOutcome {
            directory: final_directory,
            poset_count: report.outputs[0].row_count,
            complex_count: report.outputs[1].row_count,
        });
    }
    ensure!(
        !final_directory.exists(),
        "partial merged directory {} already exists without a report",
        final_directory.display()
    );

    let mut manifests = Vec::with_capacity(options.shard_count);
    let mut manifest_artifacts = Vec::with_capacity(options.shard_count);
    for shard_id in 0..options.shard_count {
        let directory = shard_directory(
            &run_root,
            options.target_order,
            shard_id,
            options.shard_count,
        );
        let path = directory.join("manifest.json");
        let manifest: ShardManifest = read_json(&path)
            .with_context(|| format!("reading completed manifest for shard {shard_id}"))?;
        verify_manifest_identity_for_merge(&manifest, options, shard_id)?;
        verify_manifest_structure(&manifest)?;
        verify_artifact(&directory, &manifest.posets)?;
        verify_artifact(&directory, &manifest.complexes)?;
        manifest_artifacts.push(artifact_for_path(
            &path,
            directory
                .strip_prefix(&run_root)?
                .join("manifest.json")
                .to_string_lossy()
                .into_owned(),
            1,
        )?);
        manifests.push((directory, manifest));
    }
    verify_shard_cover(&manifests)?;

    let staging = run_root.join("merged").join(format!(
        ".order={}.tmp-{}",
        options.target_order,
        process::id()
    ));
    ensure!(
        !staging.exists(),
        "merge staging directory {} already exists",
        staging.display()
    );
    fs::create_dir_all(&staging)?;

    let (posets, connected_count, labeled_count) =
        merge_posets(&staging, options.target_order, &manifests)?;
    let complexes = merge_complexes(&staging, &manifests)?;
    let expected = accepted_order_counts(options.target_order)?;
    let checks = MergeChecks {
        shard_ranges_complete: true,
        shard_hashes_verified: true,
        poset_keys_unique_and_sorted: true,
        complex_keys_unique_and_sorted: true,
        unlabeled_count: count_result(expected.unlabeled, u128::from(posets.row_count)),
        connected_unlabeled_count: count_result(expected.connected_unlabeled, connected_count),
        labeled_count_from_automorphisms: count_result(expected.labeled, labeled_count),
    };
    ensure!(
        checks.unlabeled_count.passed
            && checks.connected_unlabeled_count.passed
            && checks.labeled_count_from_automorphisms.passed,
        "merged census failed accepted count checks"
    );
    let report = MergeReport {
        contract: CONTRACT.to_owned(),
        record_version: RECORD_VERSION,
        operation: "merge-orderly-shards".to_owned(),
        generator: SHARD_GENERATOR.to_owned(),
        noether_revision: NOETHER_REVISION.to_owned(),
        run_id: options.run_id.clone(),
        target_order: options.target_order,
        shard_count: options.shard_count,
        parent_input: manifests[0].1.parent_input.clone(),
        shard_manifests: manifest_artifacts,
        outputs: vec![posets.clone(), complexes.clone()],
        checks,
        status: "passed".to_owned(),
    };
    write_json_atomic(&staging.join("merge-validation.json"), &report)?;
    fs::create_dir_all(
        final_directory
            .parent()
            .context("merged directory has no parent")?,
    )?;
    fs::rename(&staging, &final_directory).with_context(|| {
        format!(
            "publishing merged work {} as {}",
            staging.display(),
            final_directory.display()
        )
    })?;
    sync_directory(
        final_directory
            .parent()
            .context("merged directory has no parent")?,
    )?;
    Ok(MergeOutcome {
        directory: final_directory,
        poset_count: posets.row_count,
        complex_count: complexes.row_count,
    })
}

/// Recompute every mathematical value in one serialized shard.
///
/// Certification is deliberately separate from generation. It reconstructs
/// canonical labels, automorphism groups, cores, opposites, order complexes,
/// and integral homology from the defining covers and compares the resulting
/// records byte-for-byte at the typed value level.
pub fn certify_orderly_shard(options: &CertificationOptions) -> Result<CertificationOutcome> {
    validate_certification_options(options)?;
    let merge_options = MergeOptions {
        data_root: options.data_root.clone(),
        run_id: options.run_id.clone(),
        target_order: options.target_order,
        shard_count: options.shard_count,
    };
    let run_root = work_run_root(&options.data_root, &options.run_id);
    let directory = shard_directory(
        &run_root,
        options.target_order,
        options.shard_id,
        options.shard_count,
    );
    let lock_path = directory.join("certification.lock");
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)?;
    lock.try_lock_exclusive().with_context(|| {
        format!(
            "shard {} is already being certified by another invocation",
            options.shard_id
        )
    })?;

    let manifest_path = directory.join("manifest.json");
    let manifest: ShardManifest = read_json(&manifest_path)?;
    verify_manifest_identity_for_merge(&manifest, &merge_options, options.shard_id)?;
    verify_manifest_structure(&manifest)?;
    verify_artifact(&directory, &manifest.posets)?;
    verify_artifact(&directory, &manifest.complexes)?;
    let source_manifest = artifact_for_path(
        &manifest_path,
        directory
            .strip_prefix(&run_root)?
            .join("manifest.json")
            .to_string_lossy()
            .into_owned(),
        1,
    )?;

    let report_path = directory.join("certification.json");
    if report_path.exists() {
        let report: CertificationReport = read_json(&report_path)?;
        verify_certification_report(
            &report,
            options,
            &source_manifest,
            &manifest.posets,
            &manifest.complexes,
        )?;
        return Ok(CertificationOutcome {
            directory,
            poset_count: report.posets.row_count,
            complex_count: report.complexes.row_count,
        });
    }

    let complexes = read_complex_map(&directory.join(&manifest.complexes.path))?;
    ensure!(
        complexes.len() as u64 == manifest.complexes.row_count,
        "complex map row count changed during certification"
    );
    let mut referenced_complexes = BTreeSet::<ComplexKey>::new();
    let mut core_homology = BTreeMap::new();
    let mut reader = BufReader::new(File::open(directory.join(&manifest.posets.path))?);
    let mut line = String::new();
    let mut previous_key = None;
    let mut poset_count = 0_u64;
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        ensure!(
            line.ends_with('\n'),
            "certified Poset JSONL lacks final newline"
        );
        let record: PosetRecord = serde_json::from_str(&line)?;
        record.check_shape()?;
        ensure!(
            record.order == options.target_order,
            "mixed certified order"
        );
        if let Some(previous) = previous_key.as_ref() {
            ensure!(
                previous < &record.key,
                "certified Poset keys are not strictly sorted"
            );
        }
        let poset = poset_from_record(&record)?;
        let canonical = poset.canonical_label()?;
        ensure!(
            canonical.verify(&poset),
            "canonical evidence did not verify during certification"
        );
        ensure!(
            canonical.poset() == &poset,
            "serialized Poset is not the canonical representative"
        );
        let (recomputed_record, recomputed_complex) =
            enrich_poset_with_core_cache(&canonical, &mut core_homology)?;
        ensure!(
            recomputed_record == record,
            "Poset record {:?} did not reconstruct during certification",
            record.key
        );
        let stored_complex = complexes
            .get(&record.relations.order_complex.target)
            .context("certified order-complex target does not resolve")?;
        ensure!(
            stored_complex == &recomputed_complex,
            "order complex for {:?} did not reconstruct during certification",
            record.key
        );
        crate::validate::check_complex(stored_complex)?;
        let core = poset.core()?;
        ensure!(core.verify(&poset), "core reduction did not replay");
        ensure!(
            core.core().is_core() && core.core().core()?.core() == core.core(),
            "core reduction is not idempotent"
        );
        let core_complex =
            crate::enrich::complex_record(core.core().order(), &core.core().order_complex()?)?;
        ensure!(
            crate::validate::homology_signature(&stored_complex.integral_homology)
                == crate::validate::homology_signature(&core_complex.integral_homology),
            "beat-point reduction changed integral homology"
        );
        referenced_complexes.insert(record.relations.order_complex.target.clone());
        previous_key = Some(record.key);
        poset_count = poset_count
            .checked_add(1)
            .context("certified Poset row count overflow")?;
    }
    ensure!(
        poset_count == manifest.posets.row_count,
        "certified Poset row count disagrees with shard manifest"
    );
    ensure!(
        referenced_complexes.len() == complexes.len(),
        "shard contains an unreferenced order complex"
    );

    let report = CertificationReport {
        validation_version: 2,
        contract: CONTRACT.to_owned(),
        record_version: RECORD_VERSION,
        operation: "certify-orderly-shard".to_owned(),
        generator: SHARD_GENERATOR.to_owned(),
        noether_revision: NOETHER_REVISION.to_owned(),
        run_id: options.run_id.clone(),
        target_order: options.target_order,
        shard_id: options.shard_id,
        shard_count: options.shard_count,
        source_manifest,
        posets: manifest.posets.clone(),
        complexes: manifest.complexes.clone(),
        checks: CertificationChecks {
            source_hashes_verified: true,
            keys_unique_and_sorted: true,
            canonical_records_reconstructed: true,
            automorphism_actions_reconstructed: true,
            core_reductions_replayed: true,
            opposite_relations_reconstructed: true,
            order_complexes_reconstructed: true,
            integral_homology_reconstructed: true,
            every_complex_referenced: true,
        },
        status: "passed".to_owned(),
    };
    write_json_atomic(&report_path, &report)?;
    Ok(CertificationOutcome {
        directory,
        poset_count,
        complex_count: complexes.len() as u64,
    })
}

/// Install a completed immutable census order after all certification gates.
pub fn publish_certified_merge(options: &PublishOptions) -> Result<PublishOutcome> {
    validate_publish_options(options)?;
    let merge_options = MergeOptions {
        data_root: options.data_root.clone(),
        run_id: options.run_id.clone(),
        target_order: options.target_order,
        shard_count: options.shard_count,
    };
    let merged = merge_orderly_shards(&merge_options)?;
    let merge_report_path = merged.directory.join("merge-validation.json");
    let merge_report: MergeReport = read_json(&merge_report_path)?;
    verify_merge_identity(&merge_report, &merge_options)?;
    ensure!(merge_report.status == "passed", "merge report did not pass");
    ensure!(
        merge_report.checks.shard_ranges_complete
            && merge_report.checks.shard_hashes_verified
            && merge_report.checks.poset_keys_unique_and_sorted
            && merge_report.checks.complex_keys_unique_and_sorted
            && merge_report.checks.unlabeled_count.passed
            && merge_report.checks.connected_unlabeled_count.passed
            && merge_report.checks.labeled_count_from_automorphisms.passed,
        "merge report contains a failed publication gate"
    );
    ensure!(
        merge_report.shard_manifests.len() == options.shard_count,
        "merge report does not identify every shard manifest"
    );

    let run_root = work_run_root(&options.data_root, &options.run_id);
    let publish_lock_path = run_root.join(format!("publish-order={}.lock", options.target_order));
    let publish_lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&publish_lock_path)?;
    publish_lock.try_lock_exclusive().with_context(|| {
        format!(
            "order {} is already being published by another invocation",
            options.target_order
        )
    })?;

    let mut certification_digest = Sha256::new();
    let mut raw_candidate_count = 0_u64;
    for shard_id in 0..options.shard_count {
        let directory = shard_directory(
            &run_root,
            options.target_order,
            shard_id,
            options.shard_count,
        );
        let manifest_path = directory.join("manifest.json");
        let manifest: ShardManifest = read_json(&manifest_path)?;
        verify_manifest_identity_for_merge(&manifest, &merge_options, shard_id)?;
        let source_manifest = artifact_for_path(
            &manifest_path,
            directory
                .strip_prefix(&run_root)?
                .join("manifest.json")
                .to_string_lossy()
                .into_owned(),
            1,
        )?;
        ensure!(
            merge_report.shard_manifests.get(shard_id) == Some(&source_manifest),
            "shard {shard_id} manifest differs from the one the merge consumed"
        );
        let certificate_path = directory.join("certification.json");
        let certificate_bytes = fs::read(&certificate_path)
            .with_context(|| format!("reading certification for completed shard {shard_id}"))?;
        let certificate: CertificationReport = serde_json::from_slice(&certificate_bytes)?;
        let certification_options = CertificationOptions {
            data_root: options.data_root.clone(),
            run_id: options.run_id.clone(),
            target_order: options.target_order,
            shard_id,
            shard_count: options.shard_count,
        };
        verify_certification_report(
            &certificate,
            &certification_options,
            &source_manifest,
            &manifest.posets,
            &manifest.complexes,
        )?;
        certification_digest.update((shard_id as u64).to_le_bytes());
        certification_digest.update((certificate_bytes.len() as u64).to_le_bytes());
        certification_digest.update(&certificate_bytes);
        raw_candidate_count = raw_candidate_count
            .checked_add(manifest.stats.downset_count)
            .context("published raw candidate count overflow")?;
    }
    let certification_root_sha256 = hex::encode(certification_digest.finalize());

    let version_root = options
        .data_root
        .join("census")
        .join(format!("record-version={RECORD_VERSION}"));
    let final_directory = version_root.join(format!("order={}", options.target_order));
    ensure!(
        !final_directory.exists(),
        "completed census directory {} already exists; completed orders are immutable",
        final_directory.display()
    );
    fs::create_dir_all(&version_root)?;
    let staging = version_root.join(format!(
        ".order={}.tmp-{}-{}",
        options.target_order,
        process::id(),
        unix_milliseconds(SystemTime::now())?
    ));
    ensure!(
        !staging.exists(),
        "publication staging directory already exists"
    );
    fs::create_dir(&staging)?;

    copy_and_sync(
        &merged.directory.join("posets.jsonl"),
        &staging.join("posets.jsonl"),
    )?;
    copy_and_sync(
        &merged.directory.join("simplicial-complexes.jsonl"),
        &staging.join("simplicial-complexes.jsonl"),
    )?;
    let posets = artifact_for_jsonl(&staging.join("posets.jsonl"), "posets.jsonl".to_owned())?;
    let complexes = artifact_for_jsonl(
        &staging.join("simplicial-complexes.jsonl"),
        "simplicial-complexes.jsonl".to_owned(),
    )?;
    ensure!(
        posets.row_count == merge_report.outputs[0].row_count
            && posets.byte_count == merge_report.outputs[0].byte_count
            && posets.sha256 == merge_report.outputs[0].sha256,
        "published Poset artifact differs from certified merge"
    );
    ensure!(
        complexes.row_count == merge_report.outputs[1].row_count
            && complexes.byte_count == merge_report.outputs[1].byte_count
            && complexes.sha256 == merge_report.outputs[1].sha256,
        "published complex artifact differs from certified merge"
    );

    let validation = PublishedValidationReport {
        contract: CONTRACT,
        order: options.target_order,
        status: "passed",
        generator: SHARD_GENERATOR,
        raw_candidate_count,
        accepted_count_sources: PublishedCountSources {
            unlabeled_posets: "OEIS A000112",
            connected_unlabeled_posets: "OEIS A000608",
            labeled_posets: "OEIS A001035",
        },
        isomorphism_scope: PublishedScope {
            order: options.target_order,
            status: "complete",
        },
        homotopy_scope: PublishedHomotopyScope {
            source_orders_through: options.target_order,
            status: "complete",
        },
        certification_shard_count: options.shard_count,
        certification_root_sha256: certification_root_sha256.clone(),
        checks: PublishedChecks {
            shard_ranges_complete: true,
            shard_hashes_verified: true,
            poset_keys_unique_and_sorted: true,
            complex_keys_unique_and_sorted: true,
            unlabeled_posets: merge_report.checks.unlabeled_count.clone(),
            connected_unlabeled_posets: merge_report.checks.connected_unlabeled_count.clone(),
            labeled_posets_from_automorphisms: merge_report
                .checks
                .labeled_count_from_automorphisms
                .clone(),
            full_record_reconstruction: true,
            canonical_keys_and_covers: true,
            automorphism_generators_and_orbits: true,
            core_replay_and_idempotence: true,
            opposite_relations: true,
            order_complex_reconstruction: true,
            euler_and_integral_homology: true,
            core_homology_invariance: true,
        },
        external_oracles: vec![PublishedOracle {
            name: "GAP",
            status: "not-applicable",
            reason: "abstract-group labels are a separate optional enrichment",
        }],
    };
    write_json_atomic(&staging.join("validation.json"), &validation)?;
    let validation_artifact = artifact_for_path(
        &staging.join("validation.json"),
        "validation.json".to_owned(),
        1,
    )?;
    let merge_report_artifact = artifact_for_path(
        &merge_report_path,
        format!(
            "work/finite-posets/{}/merged/order={}/merge-validation.json",
            options.run_id, options.target_order
        ),
        1,
    )?;
    let run_report = PublishedRunReport {
        contract: CONTRACT,
        run_id: options.run_id.clone(),
        operation: "publish-certified-orderly-merge",
        published_unix_milliseconds: unix_milliseconds(SystemTime::now())?,
        revisions: PublishedRevisions {
            noether: PublishedRevision {
                revision: NOETHER_REVISION,
                dirty: false,
            },
            poset_census: PublishedRevision {
                revision: env!("POSET_CENSUS_REVISION"),
                dirty: env!("POSET_CENSUS_DIRTY") == "true",
            },
        },
        parameters: PublishedRunParameters {
            order: options.target_order,
            shard_count: options.shard_count,
            raw_candidate_count,
            generator: SHARD_GENERATOR,
            seed: None,
        },
        inputs: vec![merge_report_artifact],
        outputs: vec![
            posets.clone(),
            complexes.clone(),
            validation_artifact.clone(),
        ],
        validation_sha256: validation_artifact.sha256,
        certification_root_sha256,
        reconstruction_command: format!(
            "poset-census publish-certified-merge --order {} --run-id {} --shard-count {} --data-root <explicit-data-root>",
            options.target_order, options.run_id, options.shard_count
        ),
    };
    write_json_atomic(&staging.join("run.json"), &run_report)?;
    sync_directory(&staging)?;
    fs::rename(&staging, &final_directory)?;
    sync_directory(&version_root)?;
    Ok(PublishOutcome {
        directory: final_directory,
        poset_count: posets.row_count,
        complex_count: complexes.row_count,
    })
}

fn process_parent_range(
    options: &ShardOptions,
    parent_path: &Path,
    shard_directory: &Path,
    checkpoint_path: &Path,
    checkpoint: &mut ShardCheckpoint,
) -> Result<bool> {
    let mut reader = BufReader::new(
        File::open(parent_path)
            .with_context(|| format!("opening parent corpus {}", parent_path.display()))?,
    );
    skip_jsonl_rows(&mut reader, checkpoint.next_parent_index)?;
    let mut processed_this_invocation = 0_usize;
    let mut processed_any = false;
    let mut core_homology = BTreeMap::new();
    let mut previous_child_key = checkpoint.last_child_key.clone();

    while checkpoint.next_parent_index < checkpoint.parent_range.end {
        let remaining_lease = options
            .parent_budget
            .map(|budget| budget.saturating_sub(processed_this_invocation));
        if remaining_lease == Some(0) {
            break;
        }
        let batch_size = remaining_lease.map_or(options.checkpoint_every, |remaining| {
            options.checkpoint_every.min(remaining)
        });
        let parent_start = checkpoint.next_parent_index;
        let parent_end = parent_start
            .checked_add(batch_size)
            .context("checkpoint parent range overflow")?
            .min(checkpoint.parent_range.end);
        let report = process_parent_batch(
            &mut reader,
            parent_start,
            parent_end,
            checkpoint.parts.len(),
            shard_directory,
            checkpoint.stats.downset_count,
            options.candidate_cap,
            &mut previous_child_key,
            &mut core_homology,
        )?;
        checkpoint.stats.checked_add(&report.stats)?;
        checkpoint.next_parent_index = parent_end;
        checkpoint.last_parent_key = Some(report.last_parent_key.clone());
        checkpoint.last_child_key = previous_child_key.clone();
        checkpoint.parts.push(report);
        write_json_atomic(checkpoint_path, checkpoint)?;
        processed_this_invocation = processed_this_invocation
            .checked_add(parent_end - parent_start)
            .context("invocation parent counter overflow")?;
        processed_any = true;
    }
    Ok(processed_any)
}

#[allow(clippy::too_many_arguments)]
fn process_parent_batch(
    reader: &mut BufReader<File>,
    parent_start: usize,
    parent_end: usize,
    part_index: usize,
    shard_directory: &Path,
    prior_downsets: u64,
    candidate_cap: u64,
    previous_child_key: &mut Option<PosetKey>,
    core_homology: &mut BTreeMap<PosetKey, Vec<noether::homalg::FinitelyGeneratedAbelianGroup>>,
) -> Result<PartReport> {
    ensure!(
        parent_start < parent_end,
        "cannot commit an empty parent batch"
    );
    let parts_directory = shard_directory.join("parts");
    let poset_name = format!("{part_index:06}-posets.jsonl");
    let complex_name = format!("{part_index:06}-simplicial-complexes.jsonl");
    let poset_path = parts_directory.join(&poset_name);
    let complex_path = parts_directory.join(&complex_name);
    let poset_temporary = parts_directory.join(format!(".{poset_name}.tmp-{}", process::id()));
    let complex_temporary = parts_directory.join(format!(".{complex_name}.tmp-{}", process::id()));
    remove_file_if_exists(&poset_temporary)?;
    remove_file_if_exists(&complex_temporary)?;
    remove_uncheckpointed_file(&poset_path)?;
    remove_uncheckpointed_file(&complex_path)?;

    let mut poset_writer = BufWriter::new(
        File::create(&poset_temporary)
            .with_context(|| format!("creating {}", poset_temporary.display()))?,
    );
    let mut complexes = BTreeMap::new();
    let mut stats = WorkStats::default();
    let mut line = String::new();
    let mut last_parent_key = None;

    for parent_index in parent_start..parent_end {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        ensure!(bytes != 0, "parent corpus ended before row {parent_index}");
        ensure!(
            line.ends_with('\n'),
            "parent row {parent_index} lacks newline"
        );
        let parent_record: PosetRecord = serde_json::from_str(&line)
            .with_context(|| format!("decoding parent row {parent_index}"))?;
        parent_record.check_shape()?;
        let parent = poset_from_record(&parent_record)?;
        let automorphism_group = automorphism_group_from_record(&parent_record)?;
        let output = extend_orderly_parent(&parent, &automorphism_group)?;
        let prospective_downsets = prior_downsets
            .checked_add(stats.downset_count)
            .and_then(|count| count.checked_add(output.downset_count))
            .context("shard downset counter overflow")?;
        ensure!(
            prospective_downsets <= candidate_cap,
            "shard would visit more than {candidate_cap} downsets; raise --candidate-cap explicitly"
        );

        stats.checked_add(&WorkStats {
            parent_count: 1,
            downset_count: output.downset_count,
            orbit_representative_count: output.orbit_representative_count,
            canonical_test_count: output.canonical_test_count,
            accepted_count: output.children.len() as u64,
        })?;
        for (key, canonical) in output.children {
            ensure!(
                key.entries().starts_with(parent_record.key.entries()),
                "canonical child does not extend its canonical parent key"
            );
            if let Some(previous) = previous_child_key.as_ref() {
                ensure!(previous < &key, "shard Poset keys are not strictly sorted");
            }
            let (record, complex) = enrich_poset_with_core_cache(&canonical, core_homology)?;
            ensure!(
                record.key == key,
                "enrichment changed an accepted child key"
            );
            serde_json::to_writer(&mut poset_writer, &record)?;
            poset_writer.write_all(b"\n")?;
            if let Some(existing) = complexes.insert(complex.key.clone(), complex.clone()) {
                ensure!(
                    existing == complex,
                    "order-complex digest collision in shard part"
                );
            }
            *previous_child_key = Some(key);
        }
        last_parent_key = Some(parent_record.key);
    }
    poset_writer.flush()?;
    poset_writer.get_ref().sync_all()?;
    drop(poset_writer);

    let mut complex_writer = BufWriter::new(File::create(&complex_temporary)?);
    for complex in complexes.values() {
        serde_json::to_writer(&mut complex_writer, complex)?;
        complex_writer.write_all(b"\n")?;
    }
    complex_writer.flush()?;
    complex_writer.get_ref().sync_all()?;
    drop(complex_writer);

    fs::rename(&poset_temporary, &poset_path)?;
    fs::rename(&complex_temporary, &complex_path)?;
    sync_directory(&parts_directory)?;
    let posets = artifact_for_jsonl(&poset_path, format!("parts/{poset_name}"))?;
    ensure!(posets.row_count == stats.accepted_count);
    let complex_count = complexes.len() as u64;
    let complexes = artifact_for_jsonl(&complex_path, format!("parts/{complex_name}"))?;
    ensure!(complexes.row_count == complex_count);
    Ok(PartReport {
        index: part_index,
        parent_start,
        parent_end,
        last_parent_key: last_parent_key.context("parent batch has no final key")?,
        posets,
        complexes,
        stats,
    })
}

fn finalize_shard(directory: &Path, checkpoint: &ShardCheckpoint) -> Result<ShardManifest> {
    let poset_paths = checkpoint
        .parts
        .iter()
        .map(|part| directory.join(&part.posets.path))
        .collect::<Vec<_>>();
    let complex_paths = checkpoint
        .parts
        .iter()
        .map(|part| directory.join(&part.complexes.path))
        .collect::<Vec<_>>();
    let poset_path = directory.join("posets.jsonl");
    concatenate_files(&poset_path, &poset_paths)?;
    let posets = artifact_for_jsonl(&poset_path, "posets.jsonl".to_owned())?;
    ensure!(posets.row_count == checkpoint.stats.accepted_count);
    let complex_path = directory.join("simplicial-complexes.jsonl");
    let complexes = merge_complex_files(
        &complex_path,
        "simplicial-complexes.jsonl".to_owned(),
        &complex_paths,
    )?;
    Ok(ShardManifest {
        contract: checkpoint.contract.clone(),
        record_version: checkpoint.record_version,
        generator: checkpoint.generator.clone(),
        noether_revision: checkpoint.noether_revision.clone(),
        run_id: checkpoint.run_id.clone(),
        target_order: checkpoint.target_order,
        shard_id: checkpoint.shard_id,
        shard_count: checkpoint.shard_count,
        candidate_cap: checkpoint.candidate_cap,
        checkpoint_every: checkpoint.checkpoint_every,
        parent_input: checkpoint.parent_input.clone(),
        parent_range: checkpoint.parent_range.clone(),
        parts: checkpoint.parts.clone(),
        stats: checkpoint.stats.clone(),
        posets,
        complexes,
    })
}

fn merge_posets(
    staging: &Path,
    target_order: usize,
    manifests: &[(PathBuf, ShardManifest)],
) -> Result<(Artifact, u128, u128)> {
    let path = staging.join("posets.jsonl");
    let mut writer = BufWriter::new(File::create(&path)?);
    let mut previous_key = None;
    let mut row_count = 0_u64;
    let mut connected_count = 0_u128;
    let mut labeled_count = 0_u128;
    let factorial = factorial(target_order)?;
    for (directory, manifest) in manifests {
        let input_path = directory.join(&manifest.posets.path);
        let mut reader = BufReader::new(File::open(&input_path)?);
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            let read = reader.read_until(b'\n', &mut bytes)?;
            if read == 0 {
                break;
            }
            ensure!(
                bytes.ends_with(b"\n"),
                "shard Poset JSONL lacks final newline"
            );
            let record: PosetRecord = serde_json::from_slice(&bytes)?;
            record.check_shape()?;
            ensure!(
                record.order == target_order,
                "merged shard has wrong Poset order"
            );
            if let Some(previous) = previous_key.as_ref() {
                ensure!(
                    previous < &record.key,
                    "merged Poset keys are not strictly sorted"
                );
            }
            connected_count += u128::from(record.invariants.is_connected);
            let group_order = record
                .automorphism_group
                .order
                .parse::<u128>()
                .context("automorphism-group order exceeds u128")?;
            ensure!(group_order != 0 && factorial % group_order == 0);
            labeled_count = labeled_count
                .checked_add(factorial / group_order)
                .context("labeled Poset count overflow")?;
            writer.write_all(&bytes)?;
            previous_key = Some(record.key);
            row_count = row_count
                .checked_add(1)
                .context("Poset row count overflow")?;
        }
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    Ok((
        {
            let artifact = artifact_for_jsonl(&path, "posets.jsonl".to_owned())?;
            ensure!(artifact.row_count == row_count);
            artifact
        },
        connected_count,
        labeled_count,
    ))
}

fn merge_complexes(staging: &Path, manifests: &[(PathBuf, ShardManifest)]) -> Result<Artifact> {
    let paths = manifests
        .iter()
        .map(|(directory, manifest)| directory.join(&manifest.complexes.path))
        .collect::<Vec<_>>();
    merge_complex_files(
        &staging.join("simplicial-complexes.jsonl"),
        "simplicial-complexes.jsonl".to_owned(),
        &paths,
    )
}

fn merge_complex_files(path: &Path, relative: String, inputs: &[PathBuf]) -> Result<Artifact> {
    if inputs.len() <= MAX_MERGE_FAN_IN {
        return merge_complex_files_once(path, relative, inputs);
    }

    let temporary_directory = path
        .parent()
        .context("complex output has no parent")?
        .join(format!(".complex-merge-runs-{}", process::id()));
    ensure!(
        !temporary_directory.exists(),
        "complex merge staging directory {} already exists",
        temporary_directory.display()
    );
    fs::create_dir(&temporary_directory)?;
    let mut current = inputs.to_vec();
    let mut generation = 0_usize;
    while current.len() > MAX_MERGE_FAN_IN {
        let mut next = Vec::new();
        for (run, chunk) in current.chunks(MAX_MERGE_FAN_IN).enumerate() {
            let run_path =
                temporary_directory.join(format!("generation={generation:03}-run={run:05}.jsonl"));
            merge_complex_files_once(&run_path, run_path.display().to_string(), chunk)?;
            next.push(run_path);
        }
        if generation > 0 {
            for old_run in &current {
                remove_file_if_exists(old_run)?;
            }
        }
        current = next;
        generation = generation
            .checked_add(1)
            .context("complex merge generation overflow")?;
    }
    let artifact = merge_complex_files_once(path, relative, &current)?;
    for run in current {
        remove_file_if_exists(&run)?;
    }
    fs::remove_dir(&temporary_directory)?;
    Ok(artifact)
}

fn merge_complex_files_once(path: &Path, relative: String, inputs: &[PathBuf]) -> Result<Artifact> {
    let mut cursors = inputs
        .iter()
        .map(|input| ComplexCursor::open(input))
        .collect::<Result<Vec<_>>>()?;
    let mut writer = BufWriter::new(File::create(path)?);
    let mut row_count = 0_u64;
    let mut previous = None;
    while let Some(minimum) = cursors
        .iter()
        .filter_map(|cursor| cursor.current.as_ref().map(|record| record.key.clone()))
        .min()
    {
        let mut selected = None;
        for cursor in &mut cursors {
            if cursor.current.as_ref().map(|record| &record.key) == Some(&minimum) {
                let record = cursor
                    .current
                    .take()
                    .context("complex cursor lost its row")?;
                if let Some(existing) = selected.as_ref() {
                    ensure!(
                        existing == &record,
                        "order-complex digest collision across parts"
                    );
                } else {
                    selected = Some(record);
                }
                cursor.advance()?;
            }
        }
        let record: ComplexRecord = selected.context("minimum complex key has no record")?;
        if let Some(previous) = previous.as_ref() {
            ensure!(
                previous < &record.key,
                "merged complex keys are not strictly sorted"
            );
        }
        serde_json::to_writer(&mut writer, &record)?;
        writer.write_all(b"\n")?;
        previous = Some(record.key);
        row_count = row_count
            .checked_add(1)
            .context("complex row count overflow")?;
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    let artifact = artifact_for_jsonl(path, relative)?;
    ensure!(artifact.row_count == row_count);
    Ok(artifact)
}

struct ComplexCursor {
    reader: BufReader<File>,
    buffer: String,
    current: Option<ComplexRecord>,
}

impl ComplexCursor {
    fn open(path: &Path) -> Result<Self> {
        let mut cursor = Self {
            reader: BufReader::new(File::open(path)?),
            buffer: String::new(),
            current: None,
        };
        cursor.advance()?;
        Ok(cursor)
    }

    fn advance(&mut self) -> Result<()> {
        self.buffer.clear();
        let bytes = self.reader.read_line(&mut self.buffer)?;
        if bytes == 0 {
            self.current = None;
        } else {
            ensure!(
                self.buffer.ends_with('\n'),
                "complex JSONL lacks final newline"
            );
            self.current = Some(serde_json::from_str(&self.buffer)?);
        }
        Ok(())
    }
}

fn inspect_parent_corpus(data_root: &Path, expected_order: usize) -> Result<ParentCorpus> {
    let directory = data_root
        .join("census")
        .join(format!("record-version={RECORD_VERSION}"))
        .join(format!("order={expected_order}"));
    let posets = inspect_parent_posets(&directory.join("posets.jsonl"), expected_order)?;
    let validation_path = directory.join("validation.json");
    let validation = artifact_for_path(
        &validation_path,
        format!("census/record-version={RECORD_VERSION}/order={expected_order}/validation.json"),
        1,
    )?;
    let validation_evidence: ParentValidationEvidence = read_json(&validation_path)?;
    ensure!(validation_evidence.contract == CONTRACT);
    ensure!(validation_evidence.order == expected_order);
    ensure!(validation_evidence.status == "passed");
    ensure!(validation_evidence.isomorphism_scope.order == expected_order);
    ensure!(validation_evidence.isomorphism_scope.status == "complete");

    let run_path = directory.join("run.json");
    let run = artifact_for_path(
        &run_path,
        format!("census/record-version={RECORD_VERSION}/order={expected_order}/run.json"),
        1,
    )?;
    let run_evidence: ParentRunEvidence = read_json(&run_path)?;
    ensure!(run_evidence.contract == CONTRACT);
    ensure!(run_evidence.parameters.order == expected_order);
    let reported_posets = run_evidence
        .outputs
        .iter()
        .find(|artifact| artifact.path == "posets.jsonl")
        .context("parent run report does not identify posets.jsonl")?;
    ensure!(
        reported_posets.row_count == posets.row_count
            && reported_posets.byte_count == posets.byte_count
            && reported_posets.sha256 == posets.sha256,
        "parent Poset artifact disagrees with its run report"
    );
    ensure!(
        run_evidence.validation_sha256 == validation.sha256,
        "parent validation artifact disagrees with its run report"
    );
    Ok(ParentCorpus {
        posets,
        validation,
        run,
    })
}

fn inspect_parent_posets(path: &Path, expected_order: usize) -> Result<Artifact> {
    let file = File::open(path)
        .with_context(|| format!("opening completed parent corpus {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut byte_count = 0_u64;
    let mut row_count = 0_u64;
    let mut previous_key = None;
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        let read = reader.read_until(b'\n', &mut bytes)?;
        if read == 0 {
            break;
        }
        ensure!(bytes.ends_with(b"\n"), "parent JSONL lacks final newline");
        digest.update(&bytes);
        byte_count = byte_count
            .checked_add(read as u64)
            .context("parent input byte count overflow")?;
        let record: PosetRecord = serde_json::from_slice(&bytes)?;
        record.check_shape()?;
        ensure!(
            record.order == expected_order,
            "parent corpus has mixed order"
        );
        if let Some(previous) = previous_key.as_ref() {
            ensure!(
                previous < &record.key,
                "parent corpus keys are not strictly sorted"
            );
        }
        previous_key = Some(record.key);
        row_count = row_count
            .checked_add(1)
            .context("parent row count overflow")?;
    }
    Ok(Artifact {
        path: format!("census/record-version={RECORD_VERSION}/order={expected_order}/posets.jsonl"),
        row_count,
        byte_count,
        sha256: hex::encode(digest.finalize()),
    })
}

fn poset_from_record(record: &PosetRecord) -> Result<Poset> {
    Ok(Poset::new(
        record.order,
        &record
            .covers
            .iter()
            .map(|edge| (edge[0], edge[1]))
            .collect::<Vec<_>>(),
    )?)
}

fn automorphism_group_from_record(record: &PosetRecord) -> Result<PermutationGroup> {
    let generators = record
        .automorphism_group
        .generators
        .iter()
        .map(|images| Permutation::from_images(images).map_err(anyhow::Error::from))
        .collect::<Result<Vec<_>>>()?;
    let group = PermutationGroup::from_generators(record.order, &generators)?;
    ensure!(
        group.order().to_decimal() == record.automorphism_group.order,
        "reconstructed parent automorphism-group order changed"
    );
    Ok(group)
}

fn initial_checkpoint(
    options: &ShardOptions,
    parent_input: ParentCorpus,
    parent_range: ParentRange,
) -> ShardCheckpoint {
    ShardCheckpoint {
        contract: CONTRACT.to_owned(),
        record_version: RECORD_VERSION,
        generator: SHARD_GENERATOR.to_owned(),
        noether_revision: NOETHER_REVISION.to_owned(),
        run_id: options.run_id.clone(),
        target_order: options.target_order,
        shard_id: options.shard_id,
        shard_count: options.shard_count,
        candidate_cap: options.candidate_cap,
        checkpoint_every: options.checkpoint_every,
        next_parent_index: parent_range.start,
        last_parent_key: None,
        last_child_key: None,
        parent_input,
        parent_range,
        parts: Vec::new(),
        stats: WorkStats::default(),
        complete: false,
    }
}

fn validate_shard_options(options: &ShardOptions) -> Result<()> {
    validate_run_id(&options.run_id)?;
    ensure!(
        options.target_order > 0,
        "a sharded level needs a parent order"
    );
    ensure!(options.shard_count > 0, "shard count must be positive");
    ensure!(
        options.shard_id < options.shard_count,
        "shard id is out of range"
    );
    ensure!(
        options.checkpoint_every > 0,
        "checkpoint interval must be positive"
    );
    ensure!(
        options.parent_budget != Some(0),
        "parent budget must be positive when supplied"
    );
    ensure!(
        options
            .parent_budget
            .is_none_or(|budget| budget.is_multiple_of(options.checkpoint_every)),
        "parent budget must be a multiple of the checkpoint interval so part boundaries remain reproducible"
    );
    Ok(())
}

fn validate_merge_options(options: &MergeOptions) -> Result<()> {
    validate_run_id(&options.run_id)?;
    ensure!(
        options.target_order > 0,
        "a sharded level needs a parent order"
    );
    ensure!(options.shard_count > 0, "shard count must be positive");
    Ok(())
}

fn validate_certification_options(options: &CertificationOptions) -> Result<()> {
    validate_run_id(&options.run_id)?;
    ensure!(
        options.target_order > 0,
        "a certified shard needs a parent order"
    );
    ensure!(options.shard_count > 0, "shard count must be positive");
    ensure!(
        options.shard_id < options.shard_count,
        "shard id is out of range"
    );
    Ok(())
}

fn validate_publish_options(options: &PublishOptions) -> Result<()> {
    validate_run_id(&options.run_id)?;
    ensure!(
        options.target_order > 0,
        "a certified merge needs a parent order"
    );
    ensure!(options.shard_count > 0, "shard count must be positive");
    Ok(())
}

fn validate_run_id(run_id: &str) -> Result<()> {
    ensure!(!run_id.is_empty(), "run id must not be empty");
    ensure!(run_id != "." && run_id != "..", "invalid run id");
    ensure!(
        run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')),
        "run id may contain only ASCII letters, digits, '.', '-', and '_'"
    );
    Ok(())
}

fn verify_checkpoint_identity(
    checkpoint: &ShardCheckpoint,
    options: &ShardOptions,
    parent_input: &ParentCorpus,
    parent_range: &ParentRange,
) -> Result<()> {
    ensure!(checkpoint.contract == CONTRACT);
    ensure!(checkpoint.record_version == RECORD_VERSION);
    ensure!(checkpoint.generator == SHARD_GENERATOR);
    ensure!(
        (checkpoint.noether_revision == NOETHER_REVISION
            || checkpoint.noether_revision == crate::PRERELEASE_NOETHER_REVISION)
    );
    ensure!(checkpoint.run_id == options.run_id);
    ensure!(checkpoint.target_order == options.target_order);
    ensure!(checkpoint.shard_id == options.shard_id);
    ensure!(checkpoint.shard_count == options.shard_count);
    ensure!(checkpoint.candidate_cap == options.candidate_cap);
    ensure!(checkpoint.checkpoint_every == options.checkpoint_every);
    ensure!(&checkpoint.parent_input == parent_input);
    ensure!(&checkpoint.parent_range == parent_range);
    ensure!(
        checkpoint.next_parent_index >= parent_range.start
            && checkpoint.next_parent_index <= parent_range.end
    );
    ensure!(checkpoint.parts.len() <= parent_range.end - parent_range.start);
    Ok(())
}

fn verify_checkpoint_structure(checkpoint: &ShardCheckpoint) -> Result<()> {
    let mut next_parent = checkpoint.parent_range.start;
    let mut stats = WorkStats::default();
    for (index, part) in checkpoint.parts.iter().enumerate() {
        ensure!(
            part.index == index,
            "checkpoint part indices are not contiguous"
        );
        ensure!(
            part.parent_start == next_parent,
            "checkpoint parent ranges have a gap"
        );
        ensure!(
            part.parent_end > part.parent_start && part.parent_end <= checkpoint.parent_range.end,
            "checkpoint part has an invalid parent range"
        );
        ensure!(part.stats.parent_count == (part.parent_end - part.parent_start) as u64);
        ensure!(part.posets.row_count == part.stats.accepted_count);
        ensure!(part.complexes.row_count <= part.posets.row_count);
        stats.checked_add(&part.stats)?;
        next_parent = part.parent_end;
    }
    ensure!(next_parent == checkpoint.next_parent_index);
    ensure!(
        stats == checkpoint.stats,
        "checkpoint counters do not equal part totals"
    );
    ensure!(
        checkpoint.last_parent_key
            == checkpoint
                .parts
                .last()
                .map(|part| part.last_parent_key.clone())
    );
    ensure!(
        !checkpoint.complete || checkpoint.next_parent_index == checkpoint.parent_range.end,
        "complete checkpoint does not cover its parent range"
    );
    Ok(())
}

fn verify_manifest_structure(manifest: &ShardManifest) -> Result<()> {
    let synthetic = ShardCheckpoint {
        contract: manifest.contract.clone(),
        record_version: manifest.record_version,
        generator: manifest.generator.clone(),
        noether_revision: manifest.noether_revision.clone(),
        run_id: manifest.run_id.clone(),
        target_order: manifest.target_order,
        shard_id: manifest.shard_id,
        shard_count: manifest.shard_count,
        candidate_cap: manifest.candidate_cap,
        checkpoint_every: manifest.checkpoint_every,
        parent_input: manifest.parent_input.clone(),
        parent_range: manifest.parent_range.clone(),
        next_parent_index: manifest.parent_range.end,
        last_parent_key: manifest
            .parts
            .last()
            .map(|part| part.last_parent_key.clone()),
        last_child_key: None,
        parts: manifest.parts.clone(),
        stats: manifest.stats.clone(),
        complete: true,
    };
    verify_checkpoint_structure(&synthetic)?;
    ensure!(manifest.posets.row_count == manifest.stats.accepted_count);
    ensure!(manifest.complexes.row_count <= manifest.posets.row_count);
    Ok(())
}

fn verify_manifest_identity(
    manifest: &ShardManifest,
    options: &ShardOptions,
    parent_input: &ParentCorpus,
    parent_range: &ParentRange,
) -> Result<()> {
    ensure!(manifest.contract == CONTRACT);
    ensure!(manifest.record_version == RECORD_VERSION);
    ensure!(manifest.generator == SHARD_GENERATOR);
    ensure!(
        (manifest.noether_revision == NOETHER_REVISION
            || manifest.noether_revision == crate::PRERELEASE_NOETHER_REVISION)
    );
    ensure!(manifest.run_id == options.run_id);
    ensure!(manifest.target_order == options.target_order);
    ensure!(manifest.shard_id == options.shard_id);
    ensure!(manifest.shard_count == options.shard_count);
    ensure!(manifest.candidate_cap == options.candidate_cap);
    ensure!(manifest.checkpoint_every == options.checkpoint_every);
    ensure!(&manifest.parent_input == parent_input);
    ensure!(&manifest.parent_range == parent_range);
    Ok(())
}

fn verify_manifest_identity_for_merge(
    manifest: &ShardManifest,
    options: &MergeOptions,
    shard_id: usize,
) -> Result<()> {
    ensure!(manifest.contract == CONTRACT);
    ensure!(manifest.record_version == RECORD_VERSION);
    ensure!(manifest.generator == SHARD_GENERATOR);
    ensure!(
        (manifest.noether_revision == NOETHER_REVISION
            || manifest.noether_revision == crate::PRERELEASE_NOETHER_REVISION)
    );
    ensure!(manifest.run_id == options.run_id);
    ensure!(manifest.target_order == options.target_order);
    ensure!(manifest.shard_id == shard_id);
    ensure!(manifest.shard_count == options.shard_count);
    Ok(())
}

fn verify_certification_report(
    report: &CertificationReport,
    options: &CertificationOptions,
    source_manifest: &Artifact,
    posets: &Artifact,
    complexes: &Artifact,
) -> Result<()> {
    ensure!(
        report.validation_version == 2,
        "certificate predates full mathematical validation; recertify in a fresh work run"
    );
    ensure!(report.contract == CONTRACT);
    ensure!(report.record_version == RECORD_VERSION);
    ensure!(report.operation == "certify-orderly-shard");
    ensure!(report.generator == SHARD_GENERATOR);
    ensure!(
        (report.noether_revision == NOETHER_REVISION
            || report.noether_revision == crate::PRERELEASE_NOETHER_REVISION)
    );
    ensure!(report.run_id == options.run_id);
    ensure!(report.target_order == options.target_order);
    ensure!(report.shard_id == options.shard_id);
    ensure!(report.shard_count == options.shard_count);
    ensure!(&report.source_manifest == source_manifest);
    ensure!(&report.posets == posets);
    ensure!(&report.complexes == complexes);
    ensure!(report.status == "passed");
    ensure!(
        report.checks.source_hashes_verified
            && report.checks.keys_unique_and_sorted
            && report.checks.canonical_records_reconstructed
            && report.checks.automorphism_actions_reconstructed
            && report.checks.core_reductions_replayed
            && report.checks.opposite_relations_reconstructed
            && report.checks.order_complexes_reconstructed
            && report.checks.integral_homology_reconstructed
            && report.checks.every_complex_referenced,
        "shard certification contains a failed gate"
    );
    Ok(())
}

fn verify_merge_identity(report: &MergeReport, options: &MergeOptions) -> Result<()> {
    ensure!(report.contract == CONTRACT);
    ensure!(report.record_version == RECORD_VERSION);
    ensure!(report.operation == "merge-orderly-shards");
    ensure!(report.generator == SHARD_GENERATOR);
    ensure!(
        (report.noether_revision == NOETHER_REVISION
            || report.noether_revision == crate::PRERELEASE_NOETHER_REVISION)
    );
    ensure!(report.run_id == options.run_id);
    ensure!(report.target_order == options.target_order);
    ensure!(report.shard_count == options.shard_count);
    ensure!(report.status == "passed");
    ensure!(report.outputs.len() == 2);
    Ok(())
}

fn verify_shard_cover(manifests: &[(PathBuf, ShardManifest)]) -> Result<()> {
    let first = manifests.first().context("no shard manifests")?;
    let parent_input = &first.1.parent_input;
    let total = first.1.parent_range.total;
    let mut next = 0_usize;
    for (shard_id, (_, manifest)) in manifests.iter().enumerate() {
        ensure!(
            &manifest.parent_input == parent_input,
            "shards use different parent inputs"
        );
        ensure!(manifest.candidate_cap == first.1.candidate_cap);
        ensure!(manifest.checkpoint_every == first.1.checkpoint_every);
        ensure!(manifest.parent_range.total == total);
        ensure!(
            manifest.parent_range == balanced_range(total, shard_id, manifests.len())?,
            "shard does not own its deterministic balanced parent range"
        );
        ensure!(
            manifest.parent_range.start == next,
            "shard parent ranges have a gap"
        );
        ensure!(manifest.parent_range.end >= manifest.parent_range.start);
        next = manifest.parent_range.end;
    }
    ensure!(
        next == total,
        "shard parent ranges do not cover the parent corpus"
    );
    Ok(())
}

fn balanced_range(total: usize, shard_id: usize, shard_count: usize) -> Result<ParentRange> {
    ensure!(shard_count > 0 && shard_id < shard_count);
    let total_u128 = total as u128;
    let count_u128 = shard_count as u128;
    let start = usize::try_from(total_u128 * shard_id as u128 / count_u128)?;
    let end = usize::try_from(total_u128 * (shard_id + 1) as u128 / count_u128)?;
    Ok(ParentRange { start, end, total })
}

fn parent_posets_path(data_root: &Path, order: usize) -> PathBuf {
    data_root
        .join("census")
        .join(format!("record-version={RECORD_VERSION}"))
        .join(format!("order={order}"))
        .join("posets.jsonl")
}

fn work_run_root(data_root: &Path, run_id: &str) -> PathBuf {
    data_root.join("work/finite-posets").join(run_id)
}

fn shard_directory(run_root: &Path, order: usize, shard_id: usize, shard_count: usize) -> PathBuf {
    run_root.join("shards").join(format!(
        "order={order}-shard={shard_id:05}-of-{shard_count:05}"
    ))
}

fn checkpoint_path(run_root: &Path, order: usize, shard_id: usize, shard_count: usize) -> PathBuf {
    run_root.join("checkpoints").join(format!(
        "order={order}-shard={shard_id:05}-of-{shard_count:05}.json"
    ))
}

fn read_complex_map(path: &Path) -> Result<BTreeMap<ComplexKey, ComplexRecord>> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = String::new();
    let mut previous_key = None;
    let mut complexes = BTreeMap::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        ensure!(
            line.ends_with('\n'),
            "certified complex JSONL lacks final newline"
        );
        let record: ComplexRecord = serde_json::from_str(&line)?;
        if let Some(previous) = previous_key.as_ref() {
            ensure!(
                previous < &record.key,
                "certified complex keys are not strictly sorted"
            );
        }
        previous_key = Some(record.key.clone());
        ensure!(
            complexes.insert(record.key.clone(), record).is_none(),
            "duplicate complex key during certification"
        );
    }
    Ok(complexes)
}

fn copy_and_sync(source: &Path, target: &Path) -> Result<()> {
    fs::copy(source, target).with_context(|| {
        format!(
            "copying certified artifact {} to {}",
            source.display(),
            target.display()
        )
    })?;
    File::open(target)?.sync_all()?;
    Ok(())
}

fn unix_milliseconds(time: SystemTime) -> Result<u128> {
    Ok(time
        .duration_since(UNIX_EPOCH)
        .context("system clock precedes Unix epoch")?
        .as_millis())
}

fn artifact_for_path(path: &Path, relative: String, row_count: u64) -> Result<Artifact> {
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
        byte_count = byte_count
            .checked_add(read as u64)
            .context("artifact byte count overflow")?;
    }
    Ok(Artifact {
        path: relative,
        row_count,
        byte_count,
        sha256: hex::encode(digest.finalize()),
    })
}

fn artifact_for_jsonl(path: &Path, relative: String) -> Result<Artifact> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut byte_count = 0_u64;
    let mut row_count = 0_u64;
    let mut last_byte = None;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let bytes = &buffer[..read];
        digest.update(bytes);
        byte_count = byte_count
            .checked_add(read as u64)
            .context("artifact byte count overflow")?;
        row_count = row_count
            .checked_add(bytes.iter().filter(|&&byte| byte == b'\n').count() as u64)
            .context("artifact row count overflow")?;
        last_byte = bytes.last().copied();
    }
    ensure!(
        last_byte.is_none() || last_byte == Some(b'\n'),
        "JSONL artifact lacks final newline"
    );
    Ok(Artifact {
        path: relative,
        row_count,
        byte_count,
        sha256: hex::encode(digest.finalize()),
    })
}

fn verify_artifact(base: &Path, artifact: &Artifact) -> Result<()> {
    let observed = if artifact.path.ends_with(".jsonl") {
        artifact_for_jsonl(&base.join(&artifact.path), artifact.path.clone())?
    } else {
        artifact_for_path(
            &base.join(&artifact.path),
            artifact.path.clone(),
            artifact.row_count,
        )?
    };
    ensure!(
        &observed == artifact,
        "artifact {} failed hash/size verification",
        artifact.path
    );
    Ok(())
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    fs::create_dir_all(path.parent().context("JSON path has no parent")?)?;
    let temporary = path.with_extension(format!("tmp-{}", process::id()));
    remove_file_if_exists(&temporary)?;
    let mut writer = BufWriter::new(File::create(&temporary)?);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    fs::rename(&temporary, path)?;
    sync_directory(path.parent().context("JSON path has no parent")?)?;
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_reader(BufReader::new(File::open(path)?)).map_err(Into::into)
}

fn concatenate_files(path: &Path, inputs: &[PathBuf]) -> Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    for input in inputs {
        let mut reader = BufReader::new(File::open(input)?);
        std::io::copy(&mut reader, &mut writer)?;
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}

fn skip_jsonl_rows(reader: &mut BufReader<File>, count: usize) -> Result<()> {
    let mut line = String::new();
    for index in 0..count {
        line.clear();
        ensure!(
            reader.read_line(&mut line)? != 0,
            "JSONL ended before row {index}"
        );
        ensure!(line.ends_with('\n'), "JSONL row {index} lacks newline");
    }
    Ok(())
}

fn last_child_key(base: &Path, parts: &[PartReport]) -> Result<Option<PosetKey>> {
    for part in parts.iter().rev() {
        let reader = BufReader::new(File::open(base.join(&part.posets.path))?);
        let mut last = None;
        for line in reader.lines() {
            let record: PosetRecord = serde_json::from_str(&line?)?;
            last = Some(record.key);
        }
        if last.is_some() {
            return Ok(last);
        }
    }
    Ok(None)
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("removing {}", path.display())),
    }
}

fn remove_uncheckpointed_file(path: &Path) -> Result<()> {
    remove_file_if_exists(path)
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

fn checked_sum(left: u64, right: u64, name: &str) -> Result<u64> {
    left.checked_add(right)
        .with_context(|| format!("{name} counter overflow"))
}

fn count_result(expected: u128, observed: u128) -> CountResult {
    CountResult {
        expected: expected.to_string(),
        observed: observed.to_string(),
        passed: expected == observed,
    }
}

fn factorial(order: usize) -> Result<u128> {
    (1..=order).try_fold(1_u128, |value, factor| {
        value
            .checked_mul(factor as u128)
            .context("factorial exceeds u128")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex_merge_bounds_open_file_fan_in() {
        let root =
            std::env::temp_dir().join(format!("poset-census-merge-fan-in-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let inputs = (0..=MAX_MERGE_FAN_IN)
            .map(|index| {
                let path = root.join(format!("input-{index:03}.jsonl"));
                File::create(&path).unwrap();
                path
            })
            .collect::<Vec<_>>();
        let output = root.join("output.jsonl");
        let artifact = merge_complex_files(&output, "output.jsonl".to_owned(), &inputs).unwrap();
        assert_eq!(artifact.row_count, 0);
        assert_eq!(artifact.byte_count, 0);
        fs::remove_dir_all(root).unwrap();
    }
}
