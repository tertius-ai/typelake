//! A current projection must keep the producer identity of historical inputs.

use std::fs;

use eilenberg_maclane_build::{
    LEGACY_NOETHER_REVISION, NOETHER_REVISION,
    enrich::build_census,
    jsonl::{artifact_path, emit_census, load_census},
    source::{load_source, validate_source},
    validate::validate_census,
};
use serde_json::Value;
use typebase_build::eilenberg_maclane::{
    EilenbergMacLaneBuildOptions, build_eilenberg_maclane_component,
};

#[test]
fn projection_retains_producer_revision_and_rejects_unknown_producers() {
    let temp = tempfile::tempdir().unwrap();
    let source = load_source().unwrap();
    let census = build_census(&source, 2, 1, 4).unwrap();
    emit_census(
        temp.path(),
        &census,
        validate_source(&source).unwrap(),
        validate_census(&source, &census).unwrap(),
    )
    .unwrap();
    let path = artifact_path(temp.path(), 2, 1, 4).join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["noether_revision"] = LEGACY_NOETHER_REVISION.into();
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let output = temp.path().join("projection");
    build_eilenberg_maclane_component(&EilenbergMacLaneBuildOptions {
        data_root: temp.path().to_owned(),
        output_root: output.clone(),
        maximum_finite_order: 2,
        maximum_delooping_degree: 1,
        exact_through_degree: 4,
    })
    .unwrap();
    let projection: Value =
        serde_json::from_slice(&fs::read(output.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(projection["noether_revision"], LEGACY_NOETHER_REVISION);
    assert_eq!(projection["builder_noether_revision"], NOETHER_REVISION);
    assert_eq!(
        projection["type_lake_manifest"]["noether_revision"],
        LEGACY_NOETHER_REVISION
    );

    manifest["noether_revision"] = "0000000000000000000000000000000000000000".into();
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(load_census(temp.path(), 2, 1, 4).is_err());
}

#[test]
fn legacy_steenrod_revision_does_not_bypass_contract_validation() {
    use simplicial_set_census::{enrich, enumerate, identity_census, jsonl, validate};

    let temp = tempfile::tempdir().unwrap();
    let (stats, representatives) = enumerate::orderly_representatives(1, 3, 100).unwrap();
    let identity = identity_census(1, 3, 100, stats, representatives).unwrap();
    jsonl::emit_identity_shell(
        temp.path(),
        &identity,
        &validate::validate_identity_census(&identity).unwrap(),
    )
    .unwrap();
    let cohomology = enrich::enrich_cohomology(1, 3, 3, &identity.records).unwrap();
    jsonl::emit_cohomology_shell(
        temp.path(),
        &cohomology,
        &validate::validate_cohomology_census(&cohomology, &identity.records).unwrap(),
    )
    .unwrap();
    let operations =
        enrich::enrich_steenrod_operations(1, 3, 3, &identity.records, &cohomology.records)
            .unwrap();
    let path = jsonl::emit_steenrod_operations_shell(
        temp.path(),
        &operations,
        &validate::validate_steenrod_operations_census(
            &operations,
            &identity.records,
            &cohomology.records,
        )
        .unwrap(),
    )
    .unwrap()
    .join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["noether_revision"] = simplicial_set_census::LEGACY_NOETHER_REVISION.into();
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert_eq!(
        jsonl::load_steenrod_operations_shell(temp.path(), 1, 3, 3)
            .unwrap()
            .len(),
        1
    );
    manifest["contract"] = "unrelated-contract/v1".into();
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(jsonl::load_steenrod_operations_shell(temp.path(), 1, 3, 3).is_err());
}
