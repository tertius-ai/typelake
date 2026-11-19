use serde_json::Value;
use simplicial_complex_census::{Generator, enrich, enumerate, identity_census, jsonl, validate};
use std::fs;
use typebase_build::simplicial_complexes::{
    SimplicialComplexBuildOptions, build_simplicial_complex_component,
};

#[test]
fn source_built_complexes_project_an_explicit_lane_and_reject_corruption() {
    let root = tempfile::tempdir().unwrap();
    for vertices in 0..=3 {
        let (stats, representatives) =
            enumerate::orderly_representatives(vertices, 100_000).unwrap();
        let identity = identity_census(
            Generator::CanonicalVertexLink,
            vertices,
            100_000,
            stats,
            representatives,
        )
        .unwrap();
        jsonl::emit_identity_shell(
            root.path(),
            &identity,
            &validate::validate_identity_census(&identity).unwrap(),
        )
        .unwrap();
        let topology = enrich::enrich_topology(vertices, &identity.records).unwrap();
        jsonl::emit_topology_shell(
            root.path(),
            &topology,
            &validate::validate_topology_census(&topology).unwrap(),
        )
        .unwrap();
        for prime in [2, 3] {
            let cohomology =
                enrich::enrich_cohomology(vertices, prime, &identity.records, &topology.records)
                    .unwrap();
            jsonl::emit_cohomology_shell(
                root.path(),
                &cohomology,
                &validate::validate_cohomology_census(
                    &cohomology,
                    &identity.records,
                    &topology.records,
                )
                .unwrap(),
            )
            .unwrap();
        }
    }
    let options = SimplicialComplexBuildOptions {
        data_root: root.path().to_owned(),
        output_root: root.path().join("projection"),
        maximum_vertices: 3,
        f3_cohomology_version: 2,
        rows_per_detail_part: 2,
    };
    assert_eq!(
        build_simplicial_complex_component(&options)
            .unwrap()
            .complex_count,
        9
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(options.output_root.join("manifest.json")).unwrap())
            .unwrap();
    assert!(
        manifest["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str().unwrap().contains("cohomology-f3-v2/"))
    );
    let directory = root
        .path()
        .join("finite-simplicial-complexes/v1/enrichments/cohomology-f3-v2/vertices-3");
    let data = directory.join("cohomology.jsonl");
    let mut bytes = fs::read(&data).unwrap();
    bytes.push(b'\n');
    fs::write(data, bytes).unwrap();
    let failed = SimplicialComplexBuildOptions {
        output_root: root.path().join("failed"),
        ..options
    };
    assert!(build_simplicial_complex_component(&failed).is_err());
    assert!(!failed.output_root.exists());
    assert!(!root.path().read_dir().unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp-")
    }));
}
