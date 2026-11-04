use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    SOURCE_BYTES, SOURCE_CONTRACT,
    records::{AbelianGroupSourceFile, FiniteAbelianGroupSourceRecord},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceValidationReport {
    pub contract: String,
    pub gap_version: String,
    pub smallgrp_version: String,
    pub maximum_finite_order: usize,
    pub finite_groups: usize,
    pub source_sha256: String,
    pub labels_unique_and_sorted: bool,
    pub orders_and_invariants_verified: bool,
    pub mathematical_counts_verified: bool,
    pub includes_infinite_cyclic: bool,
}

pub fn load_source() -> Result<AbelianGroupSourceFile> {
    serde_json::from_slice(SOURCE_BYTES).context("parse finite-abelian source artifact")
}

#[must_use]
pub fn source_sha256() -> String {
    hex::encode(Sha256::digest(SOURCE_BYTES))
}

pub fn validate_source(source: &AbelianGroupSourceFile) -> Result<SourceValidationReport> {
    ensure!(
        source.contract == SOURCE_CONTRACT,
        "source contract mismatch"
    );
    ensure!(
        source.maximum_finite_order == 32,
        "unexpected finite-order frontier"
    );
    ensure!(
        source.include_infinite_cyclic,
        "Z is absent from source scope"
    );
    ensure!(!source.gap_version.is_empty(), "missing GAP version");
    ensure!(
        !source.smallgrp_version.is_empty(),
        "missing SmallGrp version"
    );

    let mut labels = BTreeSet::new();
    let mut observed_by_order = BTreeMap::new();
    for group in &source.finite_groups {
        validate_group(group)?;
        ensure!(labels.insert(&group.lmfdb_label), "duplicate LMFDB label");
        *observed_by_order.entry(group.order).or_insert(0usize) += 1;
    }
    ensure!(
        source.finite_groups.windows(2).all(|pair| {
            (pair[0].order, pair[0].small_groups_index)
                < (pair[1].order, pair[1].small_groups_index)
        }),
        "source groups are not uniquely sorted"
    );

    let expected_by_order = (1..=source.maximum_finite_order)
        .filter_map(|order| {
            let count = finite_abelian_group_count(order);
            (count > 0).then_some((order, count))
        })
        .collect::<BTreeMap<_, _>>();
    ensure!(
        observed_by_order == expected_by_order,
        "finite-abelian per-order counts disagree with the partition formula"
    );
    ensure!(
        source.finite_groups.len() == 55,
        "first parameter box must contain 55 finite abelian groups"
    );

    Ok(SourceValidationReport {
        contract: source.contract.clone(),
        gap_version: source.gap_version.clone(),
        smallgrp_version: source.smallgrp_version.clone(),
        maximum_finite_order: source.maximum_finite_order,
        finite_groups: source.finite_groups.len(),
        source_sha256: source_sha256(),
        labels_unique_and_sorted: true,
        orders_and_invariants_verified: true,
        mathematical_counts_verified: true,
        includes_infinite_cyclic: true,
    })
}

fn validate_group(group: &FiniteAbelianGroupSourceRecord) -> Result<()> {
    ensure!(
        group.lmfdb_label == format!("{}.{}", group.order, group.small_groups_index),
        "LMFDB and SmallGroups labels disagree"
    );
    ensure!(
        !group.structure_description.is_empty(),
        "empty GAP structure description"
    );
    ensure!(
        group
            .gap_abelian_invariants
            .iter()
            .all(|factor| *factor > 1),
        "GAP abelian invariant is not a positive nonunit"
    );
    let reconstructed_order = group
        .gap_abelian_invariants
        .iter()
        .try_fold(1u64, |product, factor| product.checked_mul(*factor))
        .context("GAP abelian invariant product overflows u64")?;
    ensure!(
        reconstructed_order == group.order as u64,
        "GAP abelian invariants have wrong order"
    );
    group.group()?.verify()?;
    ensure!(
        group.lmfdb_url()
            == format!(
                "https://www.lmfdb.org/Groups/Abstract/{}",
                group.lmfdb_label
            ),
        "LMFDB URL construction"
    );
    Ok(())
}

/// Number of finite abelian groups of order `order`.
///
/// If `order = product p^e`, the classification theorem gives the product of
/// the partition numbers `p(e)`.
fn finite_abelian_group_count(mut order: usize) -> usize {
    if order == 1 {
        return 1;
    }
    let mut count = 1usize;
    let mut prime = 2usize;
    while prime * prime <= order {
        if order.is_multiple_of(prime) {
            let mut exponent = 0usize;
            while order.is_multiple_of(prime) {
                order /= prime;
                exponent += 1;
            }
            count *= partition_number(exponent);
        }
        prime += 1;
    }
    if order > 1 {
        count *= partition_number(1);
    }
    count
}

fn partition_number(value: usize) -> usize {
    let mut partitions = vec![0usize; value + 1];
    partitions[0] = 1;
    for part in 1..=value {
        for total in part..=value {
            partitions[total] += partitions[total - part];
        }
    }
    partitions[value]
}

#[must_use]
pub fn prime_divisors(mut order: usize) -> Vec<u64> {
    let mut divisors = Vec::new();
    let mut prime = 2usize;
    while prime * prime <= order {
        if order.is_multiple_of(prime) {
            divisors.push(prime as u64);
            while order.is_multiple_of(prime) {
                order /= prime;
            }
        }
        prime += 1;
    }
    if order > 1 {
        divisors.push(order as u64);
    }
    divisors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_source_is_the_complete_finite_abelian_shell() {
        let source = load_source().unwrap();
        let report = validate_source(&source).unwrap();
        assert_eq!(report.finite_groups, 55);
        assert_eq!(report.maximum_finite_order, 32);
        assert!(report.mathematical_counts_verified);
    }

    #[test]
    fn classification_count_and_prime_helpers_are_hand_checkable() {
        assert_eq!(finite_abelian_group_count(1), 1);
        assert_eq!(finite_abelian_group_count(8), 3);
        assert_eq!(finite_abelian_group_count(16), 5);
        assert_eq!(finite_abelian_group_count(32), 7);
        assert_eq!(prime_divisors(1), Vec::<u64>::new());
        assert_eq!(prime_divisors(24), vec![2, 3]);
    }
}
