use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{SOURCE_BYTES, records::GroupSourceFile};

const ACCEPTED_COUNTS: &str =
    include_str!("../../../collections/finite-group-cohomology/validation/counts.json");

#[derive(Debug, Deserialize)]
struct CountFile {
    maximum_order: usize,
    orders: Vec<OrderCount>,
}

#[derive(Debug, Deserialize)]
struct OrderCount {
    order: usize,
    groups: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceValidationReport {
    pub contract: String,
    pub gap_version: String,
    pub smallgrp_version: String,
    pub maximum_order: usize,
    pub groups: usize,
    pub source_sha256: String,
    pub labels_unique_and_sorted: bool,
    pub accepted_counts: bool,
    pub presentations_verified: bool,
}

pub fn load_source() -> Result<GroupSourceFile> {
    serde_json::from_slice(SOURCE_BYTES).context("parse finite-group source artifact")
}

#[must_use]
pub fn source_sha256() -> String {
    hex::encode(Sha256::digest(SOURCE_BYTES))
}

pub fn validate_source(source: &GroupSourceFile) -> Result<SourceValidationReport> {
    source.check_shape()?;
    let counts: CountFile = serde_json::from_str(ACCEPTED_COUNTS)?;
    ensure!(
        counts.maximum_order == source.maximum_order,
        "count frontier mismatch"
    );
    let expected = counts
        .orders
        .iter()
        .map(|entry| (entry.order, entry.groups))
        .collect::<BTreeMap<_, _>>();
    let observed = source
        .groups
        .iter()
        .fold(BTreeMap::new(), |mut counts, group| {
            *counts.entry(group.small_groups_order).or_insert(0usize) += 1;
            counts
        });
    ensure!(
        observed == expected,
        "SmallGroups per-order counts disagree"
    );
    let labels = source
        .groups
        .iter()
        .map(|group| group.lmfdb_label.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(labels.len() == source.groups.len(), "duplicate LMFDB label");
    ensure!(
        source.groups.len() == 144,
        "first edition must contain 144 groups"
    );
    Ok(SourceValidationReport {
        contract: source.contract.clone(),
        gap_version: source.gap_version.clone(),
        smallgrp_version: source.smallgrp_version.clone(),
        maximum_order: source.maximum_order,
        groups: source.groups.len(),
        source_sha256: source_sha256(),
        labels_unique_and_sorted: true,
        accepted_counts: true,
        presentations_verified: true,
    })
}

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
    fn checked_in_source_is_the_complete_smallgroups_shell() {
        let source = load_source().unwrap();
        let report = validate_source(&source).unwrap();
        assert_eq!(report.groups, 144);
        assert_eq!(report.maximum_order, 32);
        assert_eq!(
            report.source_sha256,
            "640135dd932b70ca84ecb5f54815a89bfa3316e677fb586617d85c27fc347e59"
        );
    }

    #[test]
    fn relevant_primes_are_distinct_and_sorted() {
        assert_eq!(prime_divisors(1), Vec::<u64>::new());
        assert_eq!(prime_divisors(24), vec![2, 3]);
        assert_eq!(prime_divisors(25), vec![5]);
    }
}
