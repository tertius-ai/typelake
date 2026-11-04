use std::{
    collections::HashSet,
    fs::{self, File},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime},
};

use arrow_array::{Array, StringArray};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use poset_census::{
    Generator, enrich_census_with_generator, enumerate::orderly_representatives,
    jsonl::emit_completed_order, validate::validate_census,
};
use typebase_build::posets::{PosetBuildOptions, build_poset_component};

static TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn projection_assigns_resolvable_labels_and_portable_provenance() {
    let root = temporary_root();
    let source = root.join("source");
    for order in 0..=4 {
        let (stats, representatives) = orderly_representatives(order, 10_000).unwrap();
        let census = enrich_census_with_generator(
            Generator::HereditaryOrderly,
            order,
            10_000,
            stats.downset_count,
            representatives,
        )
        .unwrap();
        let validation = validate_census(&census).unwrap();
        emit_completed_order(
            &source,
            &census,
            &validation,
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000 + order as u64),
            Duration::from_millis(1),
        )
        .unwrap();
    }

    let output = root.join("volume-1");
    let outcome = build_poset_component(&PosetBuildOptions {
        data_root: source,
        output_root: output.clone(),
        maximum_order: 4,
        rows_per_detail_part: 3,
        group_identifications: None,
    })
    .unwrap();
    assert_eq!(outcome.poset_count, 25);

    let manifest = fs::read_to_string(output.join("manifest.json")).unwrap();
    assert!(!manifest.contains(root.to_string_lossy().as_ref()));
    assert!(manifest.contains("census/record-version=1/order=4/posets.jsonl"));
    assert!(manifest.contains("\"stores_labeled_posets\": false"));
    assert!(manifest.contains("\"landing_path\": \"landing.json\""));

    let landing: Vec<serde_json::Value> =
        serde_json::from_reader(File::open(output.join("landing.json")).unwrap()).unwrap();
    assert_eq!(landing.len(), 25);
    assert_eq!(landing[0]["label"], "0.-1.1");
    assert_eq!(landing[24]["label"], "4.3.1");

    let search = parquet_rows(output.join("search/order=4/posets.parquet"));
    assert_eq!(search.len(), 16);
    assert_eq!(search.iter().collect::<HashSet<_>>().len(), 16);
    assert!(search.iter().all(|label| label.starts_with("4.")));

    let detail_path = output.join("details/order=4/dimension=1/part-00001.parquet");
    assert!(detail_path.is_file());
    let details = parquet_rows(detail_path);
    assert!(details.iter().any(|label| label == "4.1.4"));

    fs::remove_dir_all(root).unwrap();
}

fn parquet_rows(path: PathBuf) -> Vec<String> {
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path).unwrap())
        .unwrap()
        .build()
        .unwrap();
    reader
        .flat_map(|batch| {
            let batch = batch.unwrap();
            let labels = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            (0..labels.len())
                .map(|index| labels.value(index).to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn temporary_root() -> PathBuf {
    let id = TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "typebase-poset-projection-test-{}-{id}",
        std::process::id()
    ))
}
