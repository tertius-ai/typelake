use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::{
    MODULAR_CONTRACT, ModularCensus,
    enrich::modular_record,
    records::{FiniteGroupModularCohomologyRecord, GroupSourceFile, ModularComputationRecord},
    source::prime_divisors,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModularValidationReport {
    pub contract: String,
    pub maximum_order: usize,
    pub initial_proof_degree: usize,
    pub maximum_proof_degree: usize,
    pub proof_depths: BTreeMap<usize, usize>,
    pub groups: usize,
    pub coefficient_rows: usize,
    pub keys_unique_and_sorted: bool,
    pub source_groups_reconstructed: bool,
    pub finite_abelian_certificates_recomputed: bool,
    pub resolutions_recomputed: bool,
    pub basis_and_products_recomputed: bool,
    pub algebra_recomputed: bool,
    pub extents_honest: bool,
    pub finite_abelian_presentations: usize,
    pub symonds_presentations: usize,
    pub symonds_parameter_presentations: usize,
    pub symonds_universal_presentations: usize,
    pub complete_presentations: usize,
}

pub fn validate_modular_census(
    source: &GroupSourceFile,
    census: &ModularCensus,
) -> Result<ModularValidationReport> {
    let selected_groups = source
        .groups
        .iter()
        .filter(|group| group.small_groups_order <= census.maximum_order)
        .collect::<Vec<_>>();
    let expected_rows = selected_groups
        .iter()
        .map(|group| prime_divisors(group.small_groups_order).len())
        .sum::<usize>();
    ensure!(
        census.records.len() == expected_rows,
        "modular coefficient-row count mismatch"
    );
    let keys = census
        .records
        .iter()
        .map(|record| {
            (
                record.group_label.as_str(),
                record.coefficient_context.characteristic.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    ensure!(keys.len() == census.records.len(), "duplicate modular row");

    for record in &census.records {
        record.check_shape()?;
        let group = selected_groups
            .iter()
            .find(|group| group.lmfdb_label == record.group_label)
            .with_context(|| format!("missing source group {}", record.group_label))?;
        let prime = record.coefficient_context.characteristic.parse::<u64>()?;
        let proof_degree = record.inspected_through_degree();
        ensure!(
            (census.initial_proof_degree..=census.maximum_proof_degree).contains(&proof_degree),
            "record proof degree lies outside the declared adaptive range"
        );
        let expected = modular_record(group, prime, proof_degree)?;
        ensure!(&expected == record, "modular recomputation mismatch");
    }

    let finite_abelian_presentations = census
        .records
        .iter()
        .filter(|record| record.is_finite_abelian_family())
        .count();
    let symonds_presentations = census
        .records
        .iter()
        .filter(|record| {
            matches!(
                &record.computation,
                ModularComputationRecord::Resolution {
                    extent: crate::records::RingExtentRecord::CompletePresentation { .. },
                    ..
                }
            )
        })
        .count();
    let symonds_parameter_presentations = census
        .records
        .iter()
        .filter(|record| {
            matches!(
                &record.computation,
                ModularComputationRecord::Resolution {
                    extent: crate::records::RingExtentRecord::CompletePresentation {
                        certificate: crate::records::CompletionCertificateRecord::SymondsParameterCriterion { .. },
                    },
                    ..
                }
            )
        })
        .count();
    let symonds_universal_presentations = symonds_presentations - symonds_parameter_presentations;
    let complete_presentations = census
        .records
        .iter()
        .filter(|record| record.is_complete())
        .count();
    ensure!(
        complete_presentations == census.records.len(),
        "release census contains an incomplete modular presentation"
    );
    let mut proof_depths = BTreeMap::new();
    for record in &census.records {
        *proof_depths
            .entry(record.inspected_through_degree())
            .or_insert(0) += 1;
    }

    Ok(ModularValidationReport {
        contract: MODULAR_CONTRACT.to_owned(),
        maximum_order: census.maximum_order,
        initial_proof_degree: census.initial_proof_degree,
        maximum_proof_degree: census.maximum_proof_degree,
        proof_depths,
        groups: selected_groups.len(),
        coefficient_rows: census.records.len(),
        keys_unique_and_sorted: true,
        source_groups_reconstructed: true,
        finite_abelian_certificates_recomputed: true,
        resolutions_recomputed: true,
        basis_and_products_recomputed: true,
        algebra_recomputed: true,
        extents_honest: true,
        finite_abelian_presentations,
        symonds_presentations,
        symonds_parameter_presentations,
        symonds_universal_presentations,
        complete_presentations,
    })
}

pub fn validate_records_sorted(records: &[FiniteGroupModularCohomologyRecord]) -> Result<()> {
    ensure!(
        records.windows(2).all(|pair| {
            parse_group_key(
                &pair[0].group_label,
                &pair[0].coefficient_context.characteristic,
            ) < parse_group_key(
                &pair[1].group_label,
                &pair[1].coefficient_context.characteristic,
            )
        }),
        "records are not sorted"
    );
    Ok(())
}

fn parse_group_key(label: &str, prime: &str) -> (usize, usize, u64) {
    let (order, index) = label.split_once('.').unwrap_or(("0", "0"));
    (
        order.parse().unwrap_or(0),
        index.parse().unwrap_or(0),
        prime.parse().unwrap_or(0),
    )
}
