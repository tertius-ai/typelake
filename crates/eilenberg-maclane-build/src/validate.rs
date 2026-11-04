use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use crate::{
    EilenbergMacLaneCensus, INVARIANT_CONTRACT,
    enrich::build_census,
    records::{AbelianGroupSourceFile, EilenbergMacLaneRecord},
    source::validate_source,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    pub contract: String,
    pub symbolic_spaces: usize,
    pub integral_homology_available: usize,
    pub integral_homology_pending: usize,
    pub integral_cohomology_available: usize,
    pub integral_cohomology_pending: usize,
    pub integral_cohomology_ring_available: usize,
    pub integral_cohomology_ring_pending: usize,
    pub modular_coefficient_rows: usize,
    pub modular_cohomology_available: usize,
    pub modular_cohomology_pending: usize,
    pub source_verified: bool,
    pub keys_unique_and_sorted: bool,
    pub symbolic_hashes_recomputed: bool,
    pub invariant_coverage_recomputed: bool,
    pub no_implicit_claims: bool,
}

pub fn validate_census(
    source: &AbelianGroupSourceFile,
    census: &EilenbergMacLaneCensus,
) -> Result<ValidationReport> {
    validate_source(source)?;
    validate_records_sorted(&census.records)?;
    let expected = build_census(
        source,
        census.maximum_finite_order,
        census.maximum_delooping_degree,
        census.exact_through_degree,
    )?;
    ensure!(
        census.records == expected.records,
        "full Noether recomputation differs from records"
    );
    let selected_finite_groups = source
        .finite_groups
        .iter()
        .filter(|group| group.order <= census.maximum_finite_order)
        .count();
    let selected_groups = selected_finite_groups + usize::from(source.include_infinite_cyclic);
    ensure!(
        census.records.len() == selected_groups * census.maximum_delooping_degree,
        "parameter-box row count mismatch"
    );

    for record in &census.records {
        record.check_shape()?;
        ensure!(
            record.space_hash()? == record.space_canonical_sha256,
            "symbolic space hash mismatch"
        );
    }

    let integral_homology_available = census
        .records
        .iter()
        .filter(|record| record.integral_homology.is_available())
        .count();
    let integral_cohomology_available = census
        .records
        .iter()
        .filter(|record| record.integral_cohomology.is_available())
        .count();
    let integral_cohomology_ring_available = census
        .records
        .iter()
        .filter(|record| record.integral_cohomology_ring.is_available())
        .count();
    let modular_coefficient_rows = census
        .records
        .iter()
        .map(|record| record.modular_cohomology.len())
        .sum::<usize>();
    let modular_cohomology_available = census
        .records
        .iter()
        .flat_map(|record| &record.modular_cohomology)
        .filter(|record| record.coverage.is_available())
        .count();

    Ok(ValidationReport {
        contract: INVARIANT_CONTRACT.to_owned(),
        symbolic_spaces: census.records.len(),
        integral_homology_available,
        integral_homology_pending: census.records.len() - integral_homology_available,
        integral_cohomology_available,
        integral_cohomology_pending: census.records.len() - integral_cohomology_available,
        integral_cohomology_ring_available,
        integral_cohomology_ring_pending: census.records.len() - integral_cohomology_ring_available,
        modular_coefficient_rows,
        modular_cohomology_available,
        modular_cohomology_pending: modular_coefficient_rows - modular_cohomology_available,
        source_verified: true,
        keys_unique_and_sorted: true,
        symbolic_hashes_recomputed: true,
        invariant_coverage_recomputed: true,
        no_implicit_claims: true,
    })
}

pub fn validate_records_sorted(records: &[EilenbergMacLaneRecord]) -> Result<()> {
    let labels = records
        .iter()
        .map(|record| record.label.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(labels.len() == records.len(), "duplicate object label");
    ensure!(
        records
            .windows(2)
            .all(|pair| record_key(&pair[0]) < record_key(&pair[1])),
        "records are not in canonical parameter order"
    );
    Ok(())
}

fn record_key(record: &EilenbergMacLaneRecord) -> (u8, usize, usize, usize) {
    if let Some(label) = &record.group.lmfdb_label {
        let (order, index) = label.split_once('.').unwrap_or(("0", "0"));
        (
            0,
            order.parse().unwrap_or(0),
            index.parse().unwrap_or(0),
            record.delooping_degree,
        )
    } else {
        (1, 0, 0, record.delooping_degree)
    }
}

#[cfg(test)]
mod tests {
    use crate::{enrich::build_census, source::load_source};

    use super::*;

    #[test]
    fn pilot_validation_recomputes_every_record() {
        let source = load_source().unwrap();
        let census = build_census(&source, 3, 4, 8).unwrap();
        let report = validate_census(&source, &census).unwrap();
        assert_eq!(report.symbolic_spaces, 16);
        assert_eq!(report.integral_homology_available, 8);
        assert_eq!(report.integral_homology_pending, 8);
        assert_eq!(report.integral_cohomology_available, 8);
        assert_eq!(report.integral_cohomology_ring_available, 8);
        assert_eq!(report.modular_coefficient_rows, 16);
        assert_eq!(report.modular_cohomology_available, 16);
        assert_eq!(report.modular_cohomology_pending, 0);
        assert!(report.no_implicit_claims);
    }
}
