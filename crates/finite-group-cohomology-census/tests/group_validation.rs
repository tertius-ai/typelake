use std::{fs, process::Command};

use finite_group_cohomology_census::{enrich::modular_record, source::load_source};

fn validate(label: &str, root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_finite-group-cohomology-census"))
        .args([
            "validate-modular-group",
            "--group-label",
            label,
            "--records-root",
        ])
        .arg(root)
        .output()
        .unwrap()
}

#[test]
fn group_replay_reports_its_scope_and_rejects_changed_identity() {
    let root = tempfile::tempdir().unwrap();
    let source = load_source().unwrap();
    let group = source
        .groups
        .iter()
        .find(|group| group.lmfdb_label == "4.2")
        .unwrap();
    let record = modular_record(group, 2, 8).unwrap();
    let path = root.path().join("4.2-F2.json");
    fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    let result = validate("4.2", root.path());
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["group_label"], "4.2");
    assert_eq!(report["noether_version"], "0.3.2");
    assert_eq!(report["validation"]["groups"], 1);
    assert_eq!(report["validation"]["coefficient_rows"], 1);
    let mut changed = serde_json::to_value(&record).unwrap();
    changed["group_canonical_sha256"] = "0".repeat(64).into();
    fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(!validate("4.2", root.path()).status.success());
}

#[test]
fn trivial_group_has_no_modular_rows_and_missing_coefficients_fail() {
    let root = tempfile::tempdir().unwrap();
    let result = validate("1.1", root.path());
    assert!(result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["validation"]["groups"], 1);
    assert_eq!(report["validation"]["coefficient_rows"], 0);
    assert!(!validate("6.1", root.path()).status.success());
    assert!(!validate("999.1", root.path()).status.success());
}
