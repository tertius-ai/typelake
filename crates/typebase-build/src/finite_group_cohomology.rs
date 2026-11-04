//! Static TypeBase projection of the complete finite-group modular census.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use anyhow::{Context, Result, ensure};
use finite_group_cohomology_census::{
    INTEGRAL_CONTRACT, INTEGRAL_NOETHER_REVISION, LEGACY_INTEGRAL_NOETHER_REVISION,
    LEGACY_NOETHER_REVISION, MODULAR_CONTRACT, NOETHER_REVISION,
    integral::validate_integral_census,
    integral_jsonl::{IntegralManifest, integral_path, load_integral_census},
    jsonl::{ModularManifest, load_modular_census, modular_path},
    records::{
        CompletionCertificateRecord, FiniteGroupIntegralCohomologyRecord,
        FiniteGroupModularCohomologyRecord, FiniteGroupSourceRecord, GroupSourceFile,
        IntegralCompletionCertificateRecord, IntegralGeneratorOriginRecord,
        IntegralTruncationCertificateRecord, ModularComputationRecord,
    },
    source::load_source,
};
use serde::{Deserialize, Serialize};

use crate::static_component::{
    Artifact, StagingDirectory, inspect_file, write_json, write_zstd_json,
};

const COMPONENT: &str = "typebase/volume-1/finite-group-cohomology";
const COLLECTION: &str = "finite-group-cohomology/modular-v7+integral-v4";
const STANDARD_NAMES_CONTRACT: &str = "finite-group-standard-names/gap-structure-description-v1";
const STANDARD_NAMES_BYTES: &[u8] = include_bytes!(
    "../../../collections/finite-group-cohomology/sources/standard-names-through-32.json"
);

#[derive(Clone, Debug)]
pub struct FiniteGroupCohomologyBuildOptions {
    pub data_root: PathBuf,
    pub output_root: PathBuf,
    pub maximum_order: usize,
    pub initial_proof_degree: usize,
    pub maximum_proof_degree: usize,
    pub integral_additive_through_degree: usize,
    pub integral_bar_entry_limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteGroupCohomologyBuildOutcome {
    pub directory: PathBuf,
    pub group_count: usize,
    pub coefficient_rows: usize,
}

#[derive(Clone, Debug, Serialize)]
struct SearchRow {
    label: String,
    standard_name: String,
    path: String,
    group_order: usize,
    small_groups_index: usize,
    coefficient: String,
    prime: Option<u64>,
    complete: bool,
    computation: String,
    completion_method: String,
    proof_degree: Option<usize>,
    additive_preview_degree: Option<usize>,
    generator_count: usize,
    generator_degrees: Vec<usize>,
    relation_count: usize,
    relation_degrees: Vec<usize>,
    named_operations: Vec<String>,
    integral_cohomology_signature: Option<String>,
    lmfdb_url: String,
}

#[derive(Serialize)]
struct GroupDetail<'a> {
    label: &'a str,
    standard_name: &'a str,
    lmfdb_url: String,
    source: &'a FiniteGroupSourceRecord,
    canonical_group_sha256: String,
    modular_cohomology: Vec<&'a FiniteGroupModularCohomologyRecord>,
    integral_cohomology: IntegralCoverage<'a>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum IntegralCoverage<'a> {
    Available {
        record: &'a FiniteGroupIntegralCohomologyRecord,
    },
}

#[derive(Serialize)]
struct ComponentChecks {
    source_manifest_verified: bool,
    standard_names_complete: bool,
    all_groups_resolved: bool,
    coefficient_paths_unique: bool,
    every_modular_presentation_complete: bool,
    search_and_detail_counts_match: bool,
    integral_coverage_not_overstated: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StandardNameCatalogue {
    contract: String,
    gap_version: String,
    smallgrp_version: String,
    maximum_order: usize,
    names: Vec<StandardNameRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StandardNameRow {
    lmfdb_label: String,
    standard_name: String,
}

#[derive(Serialize)]
struct ComponentManifest {
    component: &'static str,
    status: &'static str,
    volume: u32,
    collection: &'static str,
    maximum_order: usize,
    group_count: usize,
    coefficient_rows: usize,
    complete_modular_presentations: usize,
    integral_records: usize,
    integral_status: &'static str,
    label_scheme: &'static str,
    coefficient_path_scheme: &'static str,
    noether_revision: String,
    builder_noether_revision: &'static str,
    integral_noether_revision: String,
    type_lake_manifest: ModularManifest,
    integral_type_lake_manifest: IntegralManifest,
    inputs: Vec<Artifact>,
    outputs: Vec<Artifact>,
    checks: ComponentChecks,
}

pub fn build_finite_group_cohomology_component(
    options: &FiniteGroupCohomologyBuildOptions,
) -> Result<FiniteGroupCohomologyBuildOutcome> {
    let source = load_source()?;
    let standard_names = load_standard_names(&source)?;
    let (source_manifest, census) = load_modular_census(
        &options.data_root,
        options.maximum_order,
        options.initial_proof_degree,
        options.maximum_proof_degree,
    )?;
    let (integral_manifest, integral_census) = load_integral_census(
        &options.data_root,
        options.maximum_order,
        options.integral_additive_through_degree,
        options.integral_bar_entry_limit,
    )?;
    ensure!(integral_manifest.contract == INTEGRAL_CONTRACT);
    ensure!(
        integral_manifest.noether_revision == INTEGRAL_NOETHER_REVISION
            || integral_manifest.noether_revision == LEGACY_INTEGRAL_NOETHER_REVISION
    );
    ensure!(
        validate_integral_census(&source, &integral_census)? == integral_manifest.validation,
        "integral TypeLake validation report differs on replay"
    );
    let selected_groups = source
        .groups
        .iter()
        .filter(|group| group.small_groups_order <= options.maximum_order)
        .collect::<Vec<_>>();
    ensure!(source_manifest.contract == MODULAR_CONTRACT);
    ensure!(
        source_manifest.noether_revision == NOETHER_REVISION
            || source_manifest.noether_revision == LEGACY_NOETHER_REVISION
    );
    ensure!(source_manifest.groups == selected_groups.len());
    ensure!(source_manifest.complete_presentations == census.records.len());
    ensure!(census.records.iter().all(|record| record.is_complete()));

    let mut by_group = BTreeMap::<&str, Vec<&FiniteGroupModularCohomologyRecord>>::new();
    for record in &census.records {
        by_group
            .entry(&record.group_label)
            .or_default()
            .push(record);
    }
    let mut integral_by_group = integral_census
        .records
        .iter()
        .map(|record| (record.group_label.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let staging = StagingDirectory::new(&options.output_root)?;
    let mut outputs = Vec::new();
    let mut search_rows = Vec::with_capacity(census.records.len());
    let mut coefficient_paths = BTreeSet::new();
    let mut detail_coefficient_rows = 0usize;

    for group in &selected_groups {
        let standard_name = standard_names
            .get(&group.lmfdb_label)
            .context("finite group has no standard name")?;
        let modular = by_group
            .remove(group.lmfdb_label.as_str())
            .unwrap_or_default();
        let canonical_group_sha256 = group.canonical_group_sha256()?;
        for record in &modular {
            ensure!(record.group_canonical_sha256 == canonical_group_sha256);
            let row = search_row(group, standard_name, record)?;
            ensure!(
                coefficient_paths.insert(row.path.clone()),
                "duplicate finite-group cohomology coefficient path"
            );
            search_rows.push(row);
        }
        let integral = integral_by_group.remove(group.lmfdb_label.as_str());
        if let Some(record) = integral {
            ensure!(record.group_canonical_sha256 == canonical_group_sha256);
            let row = integral_search_row(
                group,
                standard_name,
                record,
                options.integral_additive_through_degree,
            )?;
            ensure!(
                coefficient_paths.insert(row.path.clone()),
                "duplicate finite-group cohomology coefficient path"
            );
            search_rows.push(row);
        }
        detail_coefficient_rows += modular.len();
        let detail = GroupDetail {
            label: &group.lmfdb_label,
            standard_name,
            lmfdb_url: group.lmfdb_url(),
            source: group,
            canonical_group_sha256,
            modular_cohomology: modular,
            integral_cohomology: IntegralCoverage::Available {
                record: integral.context("finite group has no integral cohomology record")?,
            },
        };
        let path = format!("details/{}.json.zst", group.lmfdb_label);
        outputs.push(write_zstd_json(
            &staging.path,
            &path,
            "details",
            1,
            &detail,
        )?);
    }
    ensure!(by_group.is_empty(), "cohomology row has no source group");
    ensure!(
        integral_by_group.is_empty(),
        "integral cohomology row has no source group"
    );
    ensure!(detail_coefficient_rows == census.records.len());
    search_rows.sort_by_key(|row| {
        (
            row.group_order,
            row.small_groups_index,
            row.coefficient.clone(),
        )
    });
    outputs.push(write_json(
        &staging.path,
        "search.json",
        "search",
        u64::try_from(search_rows.len())?,
        &search_rows,
    )?);

    let source_directory = modular_path(
        &options.data_root,
        options.maximum_order,
        options.initial_proof_degree,
        options.maximum_proof_degree,
    );
    let integral_source_directory = integral_path(
        &options.data_root,
        options.maximum_order,
        options.integral_additive_through_degree,
        options.integral_bar_entry_limit,
    );
    let inputs = vec![
        inspect_file(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../../collections/finite-group-cohomology/sources/standard-names-through-32.json",
            ),
            "finite-group-cohomology/sources/standard-names-through-32.json",
            "external_names",
            u64::try_from(standard_names.len())?,
        )?,
        inspect_file(
            &source_directory.join("manifest.json"),
            &format!(
                "finite-group-cohomology/modular-v7/order-through-{}/proof-degree-{}-through-{}/manifest.json",
                options.maximum_order, options.initial_proof_degree, options.maximum_proof_degree
            ),
            "typelake_manifest",
            1,
        )?,
        inspect_file(
            &source_directory.join("records.jsonl"),
            &format!(
                "finite-group-cohomology/modular-v7/order-through-{}/proof-degree-{}-through-{}/records.jsonl",
                options.maximum_order, options.initial_proof_degree, options.maximum_proof_degree
            ),
            "typelake_records",
            u64::try_from(census.records.len())?,
        )?,
        inspect_file(
            &integral_source_directory.join("manifest.json"),
            &format!(
                "finite-group-cohomology/integral-v4/order-through-{}/additive-degree-{}/bar-entry-limit-{}/manifest.json",
                options.maximum_order,
                options.integral_additive_through_degree,
                options.integral_bar_entry_limit
            ),
            "integral_typelake_manifest",
            1,
        )?,
        inspect_file(
            &integral_source_directory.join("records.jsonl"),
            &format!(
                "finite-group-cohomology/integral-v4/order-through-{}/additive-degree-{}/bar-entry-limit-{}/records.jsonl",
                options.maximum_order,
                options.integral_additive_through_degree,
                options.integral_bar_entry_limit
            ),
            "integral_typelake_records",
            u64::try_from(integral_census.records.len())?,
        )?,
    ];
    let manifest = ComponentManifest {
        component: COMPONENT,
        status: "passed",
        volume: 1,
        collection: COLLECTION,
        maximum_order: options.maximum_order,
        group_count: selected_groups.len(),
        coefficient_rows: search_rows.len(),
        complete_modular_presentations: census.records.len(),
        integral_records: integral_census.records.len(),
        integral_status: "complete_abelian_families_and_budgeted_exact_sparse_bar_truncations",
        label_scheme: "LMFDB abstract-group order.index",
        coefficient_path_scheme: "/FiniteGroupCohomology/<order.index>/{Z|F<prime>}",
        noether_revision: source_manifest.noether_revision.clone(),
        builder_noether_revision: NOETHER_REVISION,
        integral_noether_revision: integral_manifest.noether_revision.clone(),
        type_lake_manifest: source_manifest,
        integral_type_lake_manifest: integral_manifest,
        inputs,
        outputs,
        checks: ComponentChecks {
            source_manifest_verified: true,
            standard_names_complete: true,
            all_groups_resolved: true,
            coefficient_paths_unique: true,
            every_modular_presentation_complete: true,
            search_and_detail_counts_match: true,
            integral_coverage_not_overstated: true,
        },
    };
    write_json(&staging.path, "manifest.json", "manifest", 1, &manifest)?;
    staging.install()?;
    Ok(FiniteGroupCohomologyBuildOutcome {
        directory: options.output_root.clone(),
        group_count: selected_groups.len(),
        coefficient_rows: search_rows.len(),
    })
}

fn load_standard_names(source: &GroupSourceFile) -> Result<BTreeMap<String, String>> {
    let catalogue: StandardNameCatalogue = serde_json::from_slice(STANDARD_NAMES_BYTES)?;
    ensure!(catalogue.contract == STANDARD_NAMES_CONTRACT);
    ensure!(catalogue.gap_version == source.gap_version);
    ensure!(catalogue.smallgrp_version == source.smallgrp_version);
    ensure!(catalogue.maximum_order == source.maximum_order);
    let names = catalogue
        .names
        .into_iter()
        .map(|row| {
            ensure!(
                !row.standard_name.trim().is_empty(),
                "empty standard group name"
            );
            Ok((row.lmfdb_label, row.standard_name))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    ensure!(names.len() == source.groups.len());
    ensure!(
        source
            .groups
            .iter()
            .all(|group| names.contains_key(&group.lmfdb_label)),
        "standard-name catalogue does not cover the finite-group source"
    );
    Ok(names)
}

fn search_row(
    group: &FiniteGroupSourceRecord,
    standard_name: &str,
    record: &FiniteGroupModularCohomologyRecord,
) -> Result<SearchRow> {
    let prime = record.coefficient_context.characteristic.parse::<u64>()?;
    let coefficient = format!("F{prime}");
    let path = format!(
        "/FiniteGroupCohomology/{}/{}",
        group.lmfdb_label, coefficient
    );
    let (computation, completion_method, generator_degrees, relation_degrees, named_operations) =
        match &record.computation {
            ModularComputationRecord::FiniteAbelianClassifyingSpace { certificate } => (
                "finite_abelian_classifying_space".to_owned(),
                certificate.method.clone(),
                certificate
                    .generators
                    .iter()
                    .map(|generator| usize::try_from(generator.degree))
                    .collect::<Result<Vec<_>, _>>()?,
                presentation_relation_degrees(&certificate.presentation)?,
                certificate
                    .operations
                    .iter()
                    .map(|operation| operation.operation.display_name.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            ),
            ModularComputationRecord::Resolution {
                algebra, extent, ..
            } => {
                let method = match extent {
                finite_group_cohomology_census::records::RingExtentRecord::CompletePresentation {
                    certificate,
                } => match certificate {
                    CompletionCertificateRecord::SymondsUniversalBound { .. } => {
                        "symonds_universal_bound"
                    }
                    CompletionCertificateRecord::SymondsParameterCriterion { .. } => {
                        "symonds_parameter_criterion"
                    }
                },
                finite_group_cohomology_census::records::RingExtentRecord::ExactThroughDegree {
                    ..
                } => "exact_through_degree",
            };
                (
                    "resolution".to_owned(),
                    method.to_owned(),
                    algebra
                        .generators
                        .iter()
                        .map(|generator| generator.degree)
                        .collect(),
                    algebra
                        .relations
                        .iter()
                        .map(|relation| relation.degree)
                        .collect(),
                    algebra
                        .generators
                        .iter()
                        .filter_map(|generator| {
                            generator
                                .operation_witness
                                .as_ref()
                                .map(|witness| witness.operation.display_name.clone())
                        })
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                )
            }
        };
    Ok(SearchRow {
        label: group.lmfdb_label.clone(),
        standard_name: standard_name.to_owned(),
        path,
        group_order: group.small_groups_order,
        small_groups_index: group.small_groups_index,
        coefficient,
        prime: Some(prime),
        complete: record.is_complete(),
        computation,
        completion_method,
        proof_degree: Some(record.inspected_through_degree()),
        additive_preview_degree: None,
        generator_count: generator_degrees.len(),
        generator_degrees,
        relation_count: relation_degrees.len(),
        relation_degrees,
        named_operations,
        integral_cohomology_signature: None,
        lmfdb_url: group.lmfdb_url(),
    })
}

fn integral_search_row(
    group: &FiniteGroupSourceRecord,
    standard_name: &str,
    record: &FiniteGroupIntegralCohomologyRecord,
    additive_preview_degree: usize,
) -> Result<SearchRow> {
    let presentation = record.presentation.as_ref();
    let generators = presentation.map_or(&[][..], |value| value.generators.as_slice());
    let relations = presentation.map_or(&[][..], |value| value.relations.as_slice());
    let generator_degrees = generators
        .iter()
        .map(|generator| generator.degree)
        .collect::<Vec<_>>();
    let relation_degrees = relations
        .iter()
        .map(|relation| {
            relation
                .terms
                .first()
                .context("integral relation is empty")?
                .generator_exponents
                .iter()
                .zip(&generator_degrees)
                .try_fold(0usize, |degree, (exponent, generator_degree)| {
                    let exponent = usize::try_from(*exponent)?;
                    degree
                        .checked_add(exponent.checked_mul(*generator_degree).context(
                            "integral relation degree overflow while building TypeBase projection",
                        )?)
                        .context(
                            "integral relation degree overflow while building TypeBase projection",
                        )
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let named_operations = generators
        .iter()
        .map(|generator| match &generator.origin {
            IntegralGeneratorOriginRecord::IntegralBockstein { .. } => {
                "integral Bockstein".to_owned()
            }
            IntegralGeneratorOriginRecord::CyclicTorClass { .. } => "integral Tor".to_owned(),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let (computation, completion_method, complete, proof_degree) = match &record.extent {
        finite_group_cohomology_census::records::IntegralRingExtentRecord::CompletePresentation {
            certificate,
        } => match certificate {
            IntegralCompletionCertificateRecord::CyclicPeriodicResolution { .. } => (
                "cyclic_periodic_resolution".to_owned(),
                "cyclic_periodic_resolution".to_owned(),
                true,
                None,
            ),
            IntegralCompletionCertificateRecord::FiniteAbelianClassifyingSpace {
                certificate,
            } => (
                "finite_abelian_classifying_space".to_owned(),
                certificate.method.clone(),
                true,
                None,
            ),
        },
        finite_group_cohomology_census::records::IntegralRingExtentRecord::ExactThroughDegree {
            degree,
            certificate,
            ..
        } => match certificate {
            IntegralTruncationCertificateRecord::UniversalLowDegree { method, .. } => (
                "universal_low_degree".to_owned(),
                method.clone(),
                false,
                Some(*degree),
            ),
            IntegralTruncationCertificateRecord::SparseNormalizedBar { method, .. } => (
                "sparse_normalized_bar".to_owned(),
                method.clone(),
                false,
                Some(*degree),
            ),
        },
    };
    Ok(SearchRow {
        label: group.lmfdb_label.clone(),
        standard_name: standard_name.to_owned(),
        path: format!("/FiniteGroupCohomology/{}/Z", group.lmfdb_label),
        group_order: group.small_groups_order,
        small_groups_index: group.small_groups_index,
        coefficient: "Z".to_owned(),
        prime: None,
        complete,
        computation,
        completion_method,
        proof_degree,
        additive_preview_degree: Some(
            record
                .additive_groups_through_degree
                .last()
                .map_or(additive_preview_degree, |group| group.degree),
        ),
        generator_count: generator_degrees.len(),
        generator_degrees,
        relation_count: relation_degrees.len(),
        relation_degrees,
        named_operations,
        integral_cohomology_signature: Some(
            record
                .additive_groups_through_degree
                .iter()
                .map(|group| {
                    format!(
                        "{}:{}:{}",
                        group.degree,
                        group.free_rank,
                        group.torsion_coefficients.join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(";"),
        ),
        lmfdb_url: group.lmfdb_url(),
    })
}

fn presentation_relation_degrees(presentation: &serde_json::Value) -> Result<Vec<usize>> {
    let generator_degrees = presentation["ring"]["generators"]
        .as_array()
        .context("graded presentation has no generator array")?
        .iter()
        .map(|generator| {
            generator["degree"]
                .as_u64()
                .context("graded generator has no degree")
                .and_then(|degree| usize::try_from(degree).map_err(Into::into))
        })
        .collect::<Result<Vec<_>>>()?;
    presentation["relations"]
        .as_array()
        .context("graded presentation has no relation array")?
        .iter()
        .map(|relation| {
            let terms = relation["terms"]
                .as_array()
                .context("graded relation has no terms")?;
            let first = terms.first().context("graded relation is empty")?;
            first["exponents"]
                .as_array()
                .context("graded relation term has no exponents")?
                .iter()
                .zip(&generator_degrees)
                .try_fold(0usize, |degree, (exponent, generator_degree)| {
                    let exponent = usize::try_from(
                        exponent
                            .as_u64()
                            .context("graded exponent is not an integer")?,
                    )?;
                    degree
                        .checked_add(exponent.checked_mul(*generator_degree).context(
                            "graded relation degree overflow while building TypeBase projection",
                        )?)
                        .context(
                            "graded relation degree overflow while building TypeBase projection",
                        )
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use finite_group_cohomology_census::{
        enrich::{modular_census, modular_record},
        integral::{integral_census, integral_record, validate_integral_census},
        integral_jsonl::emit_integral_census,
        jsonl::emit_modular_census,
        validate::validate_modular_census,
    };

    use super::*;

    #[test]
    fn presentation_relation_degree_is_derived_from_the_ambient_grading() {
        let presentation = serde_json::json!({
            "ring": {"generators": [{"degree": 2}, {"degree": 3}]},
            "relations": [{"terms": [{"exponents": [3, 2]}]}]
        });
        assert_eq!(presentation_relation_degrees(&presentation).unwrap(), [12]);
    }

    #[test]
    fn pilot_projection_resolves_groups_and_coefficient_paths() {
        let source = load_source().unwrap();
        let census = modular_census(&source, 4, 3).unwrap();
        let validation = validate_modular_census(&source, &census).unwrap();
        let temporary = tempfile::tempdir().unwrap();
        emit_modular_census(temporary.path(), &census, validation).unwrap();
        let integral = integral_census(&source, 4, 8, 100_000).unwrap();
        let integral_validation = validate_integral_census(&source, &integral).unwrap();
        emit_integral_census(temporary.path(), &integral, integral_validation).unwrap();
        let output = temporary.path().join("typebase");
        let outcome = build_finite_group_cohomology_component(&FiniteGroupCohomologyBuildOptions {
            data_root: temporary.path().to_owned(),
            output_root: output.clone(),
            maximum_order: 4,
            initial_proof_degree: 3,
            maximum_proof_degree: 3,
            integral_additive_through_degree: 8,
            integral_bar_entry_limit: 100_000,
        })
        .unwrap();
        assert_eq!(outcome.group_count, 5);
        assert_eq!(outcome.coefficient_rows, 9);
        assert!(output.join("details/1.1.json.zst").is_file());
        let search: Vec<serde_json::Value> =
            serde_json::from_slice(&std::fs::read(output.join("search.json")).unwrap()).unwrap();
        assert_eq!(search.len(), 9);
        assert_eq!(search[0]["path"], "/FiniteGroupCohomology/1.1/Z");
        assert_eq!(search[0]["standard_name"], "1");
        assert_eq!(search[1]["path"], "/FiniteGroupCohomology/2.1/F2");
        assert_eq!(search[2]["path"], "/FiniteGroupCohomology/2.1/Z");
        let klein_four_integral = search
            .iter()
            .find(|row| row["path"] == "/FiniteGroupCohomology/4.2/Z")
            .unwrap();
        assert_eq!(
            klein_four_integral["computation"],
            "finite_abelian_classifying_space"
        );
        assert_eq!(
            klein_four_integral["relation_degrees"],
            serde_json::json!([2, 2, 3, 6])
        );
        assert_eq!(
            klein_four_integral["named_operations"],
            serde_json::json!(["integral Bockstein", "integral Tor"])
        );
        assert_eq!(klein_four_integral["standard_name"], "C2 x C2");
    }

    #[test]
    fn nonabelian_integral_row_advertises_only_its_exact_extent() {
        let source = load_source().unwrap();
        let group = source
            .groups
            .iter()
            .find(|group| group.lmfdb_label == "6.1")
            .unwrap();
        let record = integral_record(group, 16, 1).unwrap();
        let row = integral_search_row(group, "S3", &record, 16).unwrap();
        assert!(!row.complete);
        assert_eq!(row.computation, "universal_low_degree");
        assert_eq!(row.proof_degree, Some(2));
        assert_eq!(row.additive_preview_degree, Some(2));
        assert_eq!(
            row.integral_cohomology_signature.as_deref(),
            Some("0:1:;1:0:;2:0:2")
        );
        assert!(row.generator_degrees.is_empty());
        assert!(row.relation_degrees.is_empty());

        let sparse_record = integral_record(group, 16, 100_000).unwrap();
        let sparse_row = integral_search_row(group, "S3", &sparse_record, 16).unwrap();
        assert!(!sparse_row.complete);
        assert_eq!(sparse_row.computation, "sparse_normalized_bar");
        assert_eq!(sparse_row.proof_degree, Some(4));
        assert_eq!(sparse_row.additive_preview_degree, Some(4));
    }

    #[test]
    fn operation_adapted_resolution_names_are_searchable() {
        let source = load_source().unwrap();
        let group = source
            .groups
            .iter()
            .find(|group| group.lmfdb_label == "6.1")
            .unwrap();
        let record = modular_record(group, 3, 5).unwrap();
        let row = search_row(group, "S3", &record).unwrap();
        assert_eq!(row.named_operations, ["beta"]);
    }
}
