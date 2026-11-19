//! Static TypeBase projection of the Eilenberg--Mac Lane invariant census.

use std::{collections::BTreeSet, path::PathBuf};

use anyhow::{Context, Result, ensure};
use eilenberg_maclane_build::{
    INVARIANT_CONTRACT, LEGACY_NOETHER_REVISION, NOETHER_REVISION,
    jsonl::{Manifest as TypeLakeManifest, artifact_path, load_census},
    records::{
        AbelianGroupInvariantsRecord, EilenbergMacLaneRecord, IntegralRingGeneratorOriginRecord,
        InvariantCoverage, ModularGeneratorOriginRecord, ModularPresentationExtentRecord,
    },
};
use serde::Serialize;

use crate::static_component::{
    Artifact, StagingDirectory, inspect_file, write_json, write_zstd_json,
};

const COMPONENT: &str = "typebase/volume-1/eilenberg-maclane";
const COLLECTION: &str = "eilenberg-maclane/invariants-v8";

#[derive(Clone, Debug)]
pub struct EilenbergMacLaneBuildOptions {
    pub data_root: PathBuf,
    pub output_root: PathBuf,
    pub maximum_finite_order: usize,
    pub maximum_delooping_degree: usize,
    pub exact_through_degree: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EilenbergMacLaneBuildOutcome {
    pub directory: PathBuf,
    pub space_count: usize,
    pub coefficient_rows: usize,
}

#[derive(Clone, Debug, Serialize)]
struct SearchRow {
    label: String,
    path: String,
    coefficient: String,
    prime: Option<u64>,
    finite_group: bool,
    group_order: Option<usize>,
    group_label: Option<String>,
    group_description: String,
    delooping_degree: usize,
    homology_available: bool,
    cohomology_available: bool,
    ring_available: bool,
    presentation_extent: String,
    exact_through_degree: Option<usize>,
    additive_exact_through_degree: Option<usize>,
    no_claim_above_degree: bool,
    generator_count: usize,
    generator_degrees: Vec<usize>,
    relation_count: usize,
    relation_degrees: Vec<usize>,
    named_operations: Vec<String>,
    has_integral_torsion: bool,
    integral_homology_signature: Option<String>,
    integral_cohomology_signature: Option<String>,
}

#[derive(Serialize)]
struct SpaceDetail<'a> {
    label: &'a str,
    coefficient_paths: Vec<String>,
    record: &'a EilenbergMacLaneRecord,
}

#[derive(Serialize)]
struct ComponentChecks {
    source_manifest_verified: bool,
    object_labels_unique: bool,
    coefficient_paths_unique: bool,
    all_modular_rows_available: bool,
    complete_integral_rings_visible: bool,
    pending_integral_cells_visible: bool,
    search_and_detail_counts_match: bool,
}

#[derive(Serialize)]
struct ComponentManifest {
    component: &'static str,
    builder_source: poset_census::SourceIdentity,
    status: &'static str,
    volume: u32,
    collection: &'static str,
    maximum_finite_order: usize,
    maximum_delooping_degree: usize,
    exact_through_degree: usize,
    space_count: usize,
    coefficient_rows: usize,
    integral_homology_available: usize,
    integral_homology_pending: usize,
    integral_cohomology_available: usize,
    integral_cohomology_pending: usize,
    integral_cohomology_ring_available: usize,
    integral_cohomology_ring_pending: usize,
    modular_cohomology_available: usize,
    modular_cohomology_pending: usize,
    label_scheme: &'static str,
    coefficient_path_scheme: &'static str,
    noether_revision: String,
    builder_noether_revision: &'static str,
    type_lake_manifest: TypeLakeManifest,
    inputs: Vec<Artifact>,
    outputs: Vec<Artifact>,
    checks: ComponentChecks,
}

pub fn build_eilenberg_maclane_component(
    options: &EilenbergMacLaneBuildOptions,
) -> Result<EilenbergMacLaneBuildOutcome> {
    let (source_manifest, census) = load_census(
        &options.data_root,
        options.maximum_finite_order,
        options.maximum_delooping_degree,
        options.exact_through_degree,
    )?;
    ensure!(source_manifest.contract == INVARIANT_CONTRACT);
    ensure!(
        (source_manifest.noether_revision == NOETHER_REVISION
            || source_manifest.noether_revision
                == eilenberg_maclane_build::PRERELEASE_NOETHER_REVISION)
            || source_manifest.noether_revision == LEGACY_NOETHER_REVISION
    );
    ensure!(source_manifest.validation.modular_cohomology_pending == 0);

    let staging = StagingDirectory::new(&options.output_root)?;
    let mut outputs = Vec::new();
    let mut search_rows = Vec::new();
    let mut labels = BTreeSet::new();
    let mut coefficient_paths = BTreeSet::new();

    for record in &census.records {
        ensure!(
            labels.insert(record.label.clone()),
            "duplicate Eilenberg--Mac Lane label"
        );
        let integral = integral_search_row(record)?;
        ensure!(
            coefficient_paths.insert(integral.path.clone()),
            "duplicate Eilenberg--Mac Lane integral path"
        );
        search_rows.push(integral);
        for modular in &record.modular_cohomology {
            let row = modular_search_row(record, modular)?;
            ensure!(
                coefficient_paths.insert(row.path.clone()),
                "duplicate Eilenberg--Mac Lane modular path"
            );
            search_rows.push(row);
        }
        let detail = SpaceDetail {
            label: &record.label,
            coefficient_paths: std::iter::once(format!("/EilenbergMacLane/{}/Z", record.label))
                .chain(record.modular_cohomology.iter().map(|modular| {
                    format!("/EilenbergMacLane/{}/F{}", record.label, modular.prime)
                }))
                .collect(),
            record,
        };
        outputs.push(write_zstd_json(
            &staging.path,
            &format!("details/{}.json.zst", record.label),
            "details",
            1,
            &detail,
        )?);
    }
    outputs.push(write_zstd_json(
        &staging.path,
        "search.json.zst",
        "search",
        u64::try_from(search_rows.len())?,
        &search_rows,
    )?);

    let source_directory = artifact_path(
        &options.data_root,
        options.maximum_finite_order,
        options.maximum_delooping_degree,
        options.exact_through_degree,
    );
    let inputs = vec![
        inspect_file(
            &source_directory.join("manifest.json"),
            &format!(
                "eilenberg-maclane/invariants-v8/order-through-{}-plus-Z/delooping-through-{}/degree-{}/manifest.json",
                options.maximum_finite_order,
                options.maximum_delooping_degree,
                options.exact_through_degree
            ),
            "typelake_manifest",
            1,
        )?,
        inspect_file(
            &source_directory.join("records.jsonl"),
            &format!(
                "eilenberg-maclane/invariants-v8/order-through-{}-plus-Z/delooping-through-{}/degree-{}/records.jsonl",
                options.maximum_finite_order,
                options.maximum_delooping_degree,
                options.exact_through_degree
            ),
            "typelake_records",
            u64::try_from(census.records.len())?,
        )?,
    ];
    let validation = &source_manifest.validation;
    let complete_integral_rings_visible = validation.integral_cohomology_ring_available > 0;
    let manifest = ComponentManifest {
        component: COMPONENT,
        builder_source: poset_census::source_identity(),
        status: "modular_complete_integral_partial",
        volume: 1,
        collection: COLLECTION,
        maximum_finite_order: options.maximum_finite_order,
        maximum_delooping_degree: options.maximum_delooping_degree,
        exact_through_degree: options.exact_through_degree,
        space_count: census.records.len(),
        coefficient_rows: search_rows.len(),
        integral_homology_available: validation.integral_homology_available,
        integral_homology_pending: validation.integral_homology_pending,
        integral_cohomology_available: validation.integral_cohomology_available,
        integral_cohomology_pending: validation.integral_cohomology_pending,
        integral_cohomology_ring_available: validation.integral_cohomology_ring_available,
        integral_cohomology_ring_pending: validation.integral_cohomology_ring_pending,
        modular_cohomology_available: validation.modular_cohomology_available,
        modular_cohomology_pending: validation.modular_cohomology_pending,
        label_scheme: "<LMFDB abelian-group label>.<delooping degree>, or Z.<delooping degree>",
        coefficient_path_scheme: "/EilenbergMacLane/<label>/{Z|F<prime>}",
        noether_revision: source_manifest.noether_revision.clone(),
        builder_noether_revision: NOETHER_REVISION,
        type_lake_manifest: source_manifest,
        inputs,
        outputs,
        checks: ComponentChecks {
            source_manifest_verified: true,
            object_labels_unique: true,
            coefficient_paths_unique: true,
            all_modular_rows_available: true,
            complete_integral_rings_visible,
            pending_integral_cells_visible: true,
            search_and_detail_counts_match: true,
        },
    };
    write_json(&staging.path, "manifest.json", "manifest", 1, &manifest)?;
    staging.install()?;
    Ok(EilenbergMacLaneBuildOutcome {
        directory: options.output_root.clone(),
        space_count: census.records.len(),
        coefficient_rows: search_rows.len(),
    })
}

fn integral_search_row(record: &EilenbergMacLaneRecord) -> Result<SearchRow> {
    let homology_available = record.integral_homology.is_available();
    let cohomology_available = record.integral_cohomology.is_available();
    let additive_exact_through_degree = match &record.integral_cohomology {
        InvariantCoverage::Available { value } => Some(value.exact_through_degree),
        InvariantCoverage::Pending { .. } => None,
    };
    let ring = match &record.integral_cohomology_ring {
        InvariantCoverage::Available { value } => Some(value),
        InvariantCoverage::Pending { .. } => None,
    };
    let has_integral_torsion = match &record.integral_cohomology {
        InvariantCoverage::Available { value } => value
            .groups
            .iter()
            .any(|group| !group.torsion_coefficients.is_empty()),
        InvariantCoverage::Pending { .. } => false,
    };
    let generator_degrees = ring
        .map(|value| {
            value
                .generators
                .iter()
                .map(|generator| usize::try_from(generator.degree).map_err(Into::into))
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let relation_degrees = ring
        .map(integral_relation_degrees)
        .transpose()?
        .unwrap_or_default();
    let integral_homology_signature = match &record.integral_homology {
        InvariantCoverage::Available { value } => Some(additive_signature(&value.groups)),
        InvariantCoverage::Pending { .. } => None,
    };
    let integral_cohomology_signature = match &record.integral_cohomology {
        InvariantCoverage::Available { value } => Some(additive_signature(&value.groups)),
        InvariantCoverage::Pending { .. } => None,
    };
    Ok(SearchRow {
        label: record.label.clone(),
        path: format!("/EilenbergMacLane/{}/Z", record.label),
        coefficient: "Z".to_owned(),
        prime: None,
        finite_group: record.group.order.is_some(),
        group_order: record.group.order,
        group_label: record.group.lmfdb_label.clone(),
        group_description: record.group.structure_description.clone(),
        delooping_degree: record.delooping_degree,
        homology_available,
        cohomology_available,
        ring_available: ring.is_some(),
        presentation_extent: if ring.is_some() {
            "complete_closed_form".to_owned()
        } else if cohomology_available {
            "additive_exact_through_degree".to_owned()
        } else {
            "pending".to_owned()
        },
        exact_through_degree: None,
        additive_exact_through_degree,
        no_claim_above_degree: ring.is_none() && cohomology_available,
        generator_count: ring.map_or(0, |value| value.generators.len()),
        generator_degrees,
        relation_count: ring.map_or(0, |value| value.relations.len()),
        relation_degrees,
        named_operations: ring.map_or_else(Vec::new, |value| {
            value
                .generators
                .iter()
                .filter_map(|generator| match &generator.origin {
                    IntegralRingGeneratorOriginRecord::IntegralBockstein { .. } => {
                        Some("integral Bockstein".to_owned())
                    }
                    IntegralRingGeneratorOriginRecord::CyclicTorClass { .. } => {
                        Some("integral Tor".to_owned())
                    }
                    _ => None,
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        }),
        has_integral_torsion,
        integral_homology_signature,
        integral_cohomology_signature,
    })
}

fn modular_search_row(
    record: &EilenbergMacLaneRecord,
    modular: &eilenberg_maclane_build::records::ModularCohomologyCoverageRecord,
) -> Result<SearchRow> {
    let InvariantCoverage::Available { value } = &modular.coverage else {
        anyhow::bail!("the selected Eilenberg--Mac Lane artifact has a pending modular row")
    };
    let (presentation_extent, exact_through_degree, no_claim_above_degree) =
        match value.presentation_extent {
            ModularPresentationExtentRecord::CompleteClosedForm => {
                ("complete_closed_form".to_owned(), None, false)
            }
            ModularPresentationExtentRecord::ExactThroughDegree {
                degree,
                no_claim_above_degree,
            } => (
                "exact_through_degree".to_owned(),
                Some(degree),
                no_claim_above_degree,
            ),
        };
    let named_operations = value
        .generators
        .iter()
        .filter_map(|generator| operation_family(&generator.origin))
        .chain(
            value
                .operations
                .iter()
                .map(|operation| operation.operation.as_str()),
        )
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let relation_degrees = presentation_relation_degrees(&value.presentation)?;
    Ok(SearchRow {
        label: record.label.clone(),
        path: format!("/EilenbergMacLane/{}/F{}", record.label, modular.prime),
        coefficient: format!("F{}", modular.prime),
        prime: Some(modular.prime),
        finite_group: record.group.order.is_some(),
        group_order: record.group.order,
        group_label: record.group.lmfdb_label.clone(),
        group_description: record.group.structure_description.clone(),
        delooping_degree: record.delooping_degree,
        homology_available: false,
        cohomology_available: true,
        ring_available: true,
        presentation_extent,
        exact_through_degree,
        additive_exact_through_degree: None,
        no_claim_above_degree,
        generator_count: value.generators.len(),
        generator_degrees: value
            .generators
            .iter()
            .map(|generator| usize::try_from(generator.degree))
            .collect::<Result<Vec<_>, _>>()?,
        relation_count: relation_degrees.len(),
        relation_degrees,
        named_operations,
        has_integral_torsion: false,
        integral_homology_signature: None,
        integral_cohomology_signature: None,
    })
}

fn additive_signature(groups: &[AbelianGroupInvariantsRecord]) -> String {
    groups
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
        .join(";")
}

fn integral_relation_degrees(
    value: &eilenberg_maclane_build::records::IntegralCohomologyRingRecord,
) -> Result<Vec<usize>> {
    value
        .relations
        .iter()
        .map(|relation| {
            let term = relation
                .terms
                .first()
                .context("integral relation is empty")?;
            term.generator_exponents
                .iter()
                .zip(&value.generators)
                .try_fold(0usize, |degree, (exponent, generator)| {
                    let exponent = usize::try_from(*exponent)?;
                    let generator_degree = usize::try_from(generator.degree)?;
                    degree
                        .checked_add(exponent.checked_mul(generator_degree).context(
                            "integral relation degree overflow while building TypeBase projection",
                        )?)
                        .context(
                            "integral relation degree overflow while building TypeBase projection",
                        )
                })
        })
        .collect()
}

fn operation_family(origin: &ModularGeneratorOriginRecord) -> Option<&'static str> {
    match origin {
        ModularGeneratorOriginRecord::FundamentalClass { .. } => None,
        ModularGeneratorOriginRecord::Bockstein { .. } => Some("beta"),
        ModularGeneratorOriginRecord::PeriodicCyclicClass { .. } => Some("periodic class"),
        ModularGeneratorOriginRecord::AdmissibleSteenrodSquare { .. }
        | ModularGeneratorOriginRecord::AdmissibleSteenrodSquareOnHigherPrimaryClass { .. } => {
            Some("Sq")
        }
        ModularGeneratorOriginRecord::AdmissibleOddPrimaryOperation { bocksteins, .. }
        | ModularGeneratorOriginRecord::AdmissibleOddPrimaryOperationOnHigherPrimaryClass {
            bocksteins,
            ..
        } => Some(if bocksteins.iter().any(|value| *value) {
            "beta P"
        } else {
            "P"
        }),
        ModularGeneratorOriginRecord::HigherBockstein { .. } => Some("higher Bockstein"),
    }
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
            let term = relation["terms"]
                .as_array()
                .and_then(|terms| terms.first())
                .context("graded relation is empty")?;
            term["exponents"]
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
    use eilenberg_maclane_build::{
        enrich::build_census,
        jsonl::emit_census,
        source::{load_source, validate_source},
        validate::validate_census,
    };

    use super::*;

    #[test]
    fn operation_families_are_reader_facing_and_stable() {
        assert_eq!(
            operation_family(&ModularGeneratorOriginRecord::HigherBockstein {
                component: 1,
                primary_exponent: 2,
            }),
            Some("higher Bockstein")
        );
        assert_eq!(
            operation_family(
                &ModularGeneratorOriginRecord::AdmissibleOddPrimaryOperationOnHigherPrimaryClass {
                    component: 1,
                    primary_exponent: 2,
                    source_is_higher_bockstein: true,
                    bocksteins: vec![true, false],
                    reduced_powers: vec![1],
                }
            ),
            Some("beta P")
        );
    }

    #[test]
    fn pilot_projection_keeps_pending_integral_cells_visible() {
        let source = load_source().unwrap();
        let source_validation = validate_source(&source).unwrap();
        let census = build_census(&source, 3, 2, 8).unwrap();
        let validation = validate_census(&source, &census).unwrap();
        let temporary = tempfile::tempdir().unwrap();
        emit_census(temporary.path(), &census, source_validation, validation).unwrap();
        let output = temporary.path().join("typebase");
        let outcome = build_eilenberg_maclane_component(&EilenbergMacLaneBuildOptions {
            data_root: temporary.path().to_owned(),
            output_root: output.clone(),
            maximum_finite_order: 3,
            maximum_delooping_degree: 2,
            exact_through_degree: 8,
        })
        .unwrap();
        assert_eq!(outcome.space_count, 8);
        assert_eq!(outcome.coefficient_rows, 16);
        let search_bytes = zstd::stream::decode_all(
            std::fs::read(output.join("search.json.zst"))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        let search: Vec<serde_json::Value> = serde_json::from_slice(&search_bytes).unwrap();
        assert_eq!(search.len(), 16);
        assert!(search.iter().any(|row| {
            row["path"] == "/EilenbergMacLane/2.1.2/Z" && row["presentation_extent"] == "pending"
        }));
        assert!(search.iter().any(|row| {
            row["path"] == "/EilenbergMacLane/2.1.1/Z"
                && row["presentation_extent"] == "complete_closed_form"
                && row["generator_degrees"] == serde_json::json!([2])
                && row["relation_degrees"] == serde_json::json!([2])
                && row["named_operations"] == serde_json::json!(["integral Bockstein"])
                && row["integral_homology_signature"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("0:1:;1:0:2"))
                && row["integral_cohomology_signature"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("0:1:;1:0:;2:0:2"))
        }));
        assert!(search.iter().any(|row| {
            row["path"] == "/EilenbergMacLane/2.1.2/F2" && row["cohomology_available"] == true
        }));
    }
}
