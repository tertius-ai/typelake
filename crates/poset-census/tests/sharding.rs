use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime},
};

use poset_census::{
    Generator, enrich_census_with_generator,
    enumerate::orderly_representatives,
    jsonl::emit_completed_order,
    shard::{
        CertificationOptions, MergeOptions, PublishOptions, ShardOptions, certify_orderly_shard,
        merge_orderly_shards, publish_certified_merge, run_orderly_shard,
    },
    validate::validate_census,
};

static TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn sharded_output_matches_monolithic_and_resume_is_byte_identical() {
    let uninterrupted = temporary_root("uninterrupted");
    let resumed = temporary_root("resumed");
    prepare_completed_order_four(&uninterrupted);
    copy_completed_order_four(&uninterrupted, &resumed);

    let uninterrupted_merge = run_all_shards(&uninterrupted, None);
    let resumed_merge = run_all_shards(&resumed, Some(2));

    for file in [
        "posets.jsonl",
        "simplicial-complexes.jsonl",
        "merge-validation.json",
    ] {
        assert_eq!(
            fs::read(uninterrupted_merge.join(file)).unwrap(),
            fs::read(resumed_merge.join(file)).unwrap(),
            "resumed {file} differs"
        );
    }

    let direct = prepare_direct_order_five(&uninterrupted);
    for file in ["posets.jsonl", "simplicial-complexes.jsonl"] {
        assert_eq!(
            fs::read(uninterrupted_merge.join(file)).unwrap(),
            fs::read(direct.join(file)).unwrap(),
            "sharded {file} differs from the monolithic reference"
        );
    }

    let published = certify_and_publish(&resumed);
    for file in ["posets.jsonl", "simplicial-complexes.jsonl"] {
        assert_eq!(
            fs::read(published.join(file)).unwrap(),
            fs::read(direct.join(file)).unwrap(),
            "certified publication {file} differs from the monolithic reference"
        );
    }
    let validation = fs::read_to_string(published.join("validation.json")).unwrap();
    assert!(validation.contains("\"full_record_reconstruction\": true"));
    for unrun in [
        "core_replay_and_idempotence",
        "euler_and_integral_homology",
        "core_homology_invariance",
    ] {
        assert!(validation.contains(&format!("\"{unrun}\": true")));
    }

    fs::remove_dir_all(uninterrupted).unwrap();
    fs::remove_dir_all(resumed).unwrap();
}

#[test]
fn publication_rejects_a_shard_manifest_changed_after_the_merge() {
    let root = temporary_root("changed-manifest");
    prepare_completed_order_four(&root);
    run_all_shards(&root, None);
    let manifest = root.join(
        "work/finite-posets/resume-equivalence/shards/order=5-shard=00000-of-00003/manifest.json",
    );
    let mut bytes = fs::read(&manifest).unwrap();
    bytes.push(b'\n');
    fs::write(&manifest, bytes).unwrap();
    for shard_id in 0..3 {
        certify_orderly_shard(&CertificationOptions {
            data_root: root.clone(),
            run_id: "resume-equivalence".to_owned(),
            target_order: 5,
            shard_id,
            shard_count: 3,
        })
        .unwrap();
    }
    let error = publish_certified_merge(&PublishOptions {
        data_root: root.clone(),
        run_id: "resume-equivalence".to_owned(),
        target_order: 5,
        shard_count: 3,
    })
    .unwrap_err();
    assert!(error.to_string().contains("merge consumed"));
    fs::remove_dir_all(root).unwrap();
}

fn certify_and_publish(data_root: &Path) -> PathBuf {
    for shard_id in 0..3 {
        let options = CertificationOptions {
            data_root: data_root.to_owned(),
            run_id: "resume-equivalence".to_owned(),
            target_order: 5,
            shard_id,
            shard_count: 3,
        };
        let first = certify_orderly_shard(&options).unwrap();
        let repeated = certify_orderly_shard(&options).unwrap();
        assert_eq!(first, repeated);
    }
    let outcome = publish_certified_merge(&PublishOptions {
        data_root: data_root.to_owned(),
        run_id: "resume-equivalence".to_owned(),
        target_order: 5,
        shard_count: 3,
    })
    .unwrap();
    assert_eq!(outcome.poset_count, 63);
    outcome.directory
}

fn copy_completed_order_four(source_root: &Path, target_root: &Path) {
    let relative = Path::new("census/record-version=1/order=4");
    let source = source_root.join(relative);
    let target = target_root.join(relative);
    fs::create_dir_all(&target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
    }
}

fn run_all_shards(data_root: &Path, initial_parent_budget: Option<usize>) -> PathBuf {
    for shard_id in 0..3 {
        let mut options = ShardOptions {
            data_root: data_root.to_owned(),
            run_id: "resume-equivalence".to_owned(),
            target_order: 5,
            shard_id,
            shard_count: 3,
            candidate_cap: 100_000,
            checkpoint_every: 2,
            parent_budget: initial_parent_budget,
        };
        let first = run_orderly_shard(&options).unwrap();
        if initial_parent_budget.is_some() {
            assert!(!first.complete);
            seed_uncheckpointed_part(&first.directory, 1);
            options.parent_budget = None;
            assert!(run_orderly_shard(&options).unwrap().complete);
        } else {
            assert!(first.complete);
        }
    }
    let merged = merge_orderly_shards(&MergeOptions {
        data_root: data_root.to_owned(),
        run_id: "resume-equivalence".to_owned(),
        target_order: 5,
        shard_count: 3,
    })
    .unwrap();
    assert_eq!(merged.poset_count, 63);
    merged.directory
}

fn seed_uncheckpointed_part(shard_directory: &Path, part_index: usize) {
    let parts = shard_directory.join("parts");
    for name in [
        format!("{part_index:06}-posets.jsonl"),
        format!("{part_index:06}-simplicial-complexes.jsonl"),
        format!(".{part_index:06}-posets.jsonl.tmp-{}", std::process::id()),
        format!(
            ".{part_index:06}-simplicial-complexes.jsonl.tmp-{}",
            std::process::id()
        ),
    ] {
        fs::write(parts.join(name), b"incomplete\n").unwrap();
    }
}

fn prepare_completed_order_four(data_root: &Path) {
    let (stats, representatives) = orderly_representatives(4, 1_000).unwrap();
    let census = enrich_census_with_generator(
        Generator::HereditaryOrderly,
        4,
        1_000,
        stats.downset_count,
        representatives,
    )
    .unwrap();
    let validation = validate_census(&census).unwrap();
    emit_completed_order(
        data_root,
        &census,
        &validation,
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
        Duration::from_millis(1),
    )
    .unwrap();
}

fn prepare_direct_order_five(data_root: &Path) -> PathBuf {
    let (stats, representatives) = orderly_representatives(5, 10_000).unwrap();
    let census = enrich_census_with_generator(
        Generator::HereditaryOrderly,
        5,
        10_000,
        stats.downset_count,
        representatives,
    )
    .unwrap();
    let validation = validate_census(&census).unwrap();
    emit_completed_order(
        data_root,
        &census,
        &validation,
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_001),
        Duration::from_millis(1),
    )
    .unwrap()
}

fn temporary_root(label: &str) -> PathBuf {
    let id = TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "poset-census-shard-test-{}-{label}-{id}",
        std::process::id()
    ))
}
