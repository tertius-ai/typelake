use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime},
};

use noether::{combin::Poset, topology::SimplicialComplex};
use poset_census::{
    Generator,
    enrich::complex_key,
    enrich_census, enrich_census_with_generator,
    enumerate::{orderly_representatives, reference_representatives},
    jsonl::emit_completed_order,
    records::PosetKey,
    validate::validate_census,
};

static TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

const ORDER_3_POSETS: &str =
    include_str!("../../../collections/finite-posets/validation/cases/order-3/posets.jsonl");
const ORDER_3_COMPLEXES: &str = include_str!(
    "../../../collections/finite-posets/validation/cases/order-3/simplicial-complexes.jsonl"
);
const MINIMAL_CIRCLE_POSET: &str = include_str!(
    "../../../collections/finite-posets/validation/cases/minimal-finite-circle/poset.json"
);
const MINIMAL_CIRCLE_COMPLEX: &str = include_str!(
    "../../../collections/finite-posets/validation/cases/minimal-finite-circle/simplicial-complex.json"
);

#[test]
fn reference_generator_matches_small_unlabeled_counts() {
    let expected = [1, 1, 2, 5, 16];
    for (order, expected_count) in expected.into_iter().enumerate() {
        let (_, representatives) = reference_representatives(order, 1_000).unwrap();
        assert_eq!(representatives.len(), expected_count, "order {order}");
    }
}

#[test]
fn orderly_generator_matches_reference_key_sets_through_order_five() {
    let expected = [1, 1, 2, 5, 16, 63];
    for (order, expected_count) in expected.into_iter().enumerate() {
        let (_, reference) = reference_representatives(order, 2_000).unwrap();
        let (stats, orderly) = orderly_representatives(order, 100_000).unwrap();
        assert_eq!(
            orderly.keys().collect::<Vec<_>>(),
            reference.keys().collect::<Vec<_>>()
        );
        assert_eq!(orderly.len(), expected_count, "order {order}");
        assert_eq!(
            stats.levels.last().map(|level| level.accepted_count),
            (order > 0).then_some(expected_count)
        );
    }
}

#[test]
fn generators_retain_evidence_on_the_canonical_representative() {
    for representatives in [
        reference_representatives(4, 64).unwrap().1,
        orderly_representatives(5, 100_000).unwrap().1,
    ] {
        for (key, canonical) in representatives {
            assert!(canonical.verify(canonical.poset()));
            assert_eq!(
                canonical.poset().canonical_form().unwrap(),
                *canonical.poset()
            );
            assert_eq!(key, PosetKey::from_canonical_label(&canonical));
        }
    }
}

#[test]
#[ignore = "small-order oracle comparison used by deliberate stress runs"]
fn orderly_generator_matches_reference_through_order_six() {
    let (_, reference) = reference_representatives(6, 40_000).unwrap();
    let (_, orderly) = orderly_representatives(6, 1_000_000).unwrap();
    assert_eq!(
        orderly.keys().collect::<Vec<_>>(),
        reference.keys().collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "order-seven exhaustive key-set oracle used by deliberate release stress runs"]
fn orderly_generator_matches_reference_at_order_seven() {
    let (_, reference) = reference_representatives(7, 2_100_000).unwrap();
    let (_, orderly) = orderly_representatives(7, 10_000).unwrap();
    assert_eq!(
        orderly.keys().collect::<Vec<_>>(),
        reference.keys().collect::<Vec<_>>()
    );
}

#[test]
fn order_three_records_match_complete_fixtures() {
    let (candidate_count, representatives) = reference_representatives(3, 8).unwrap();
    let census = enrich_census(3, 8, candidate_count, representatives).unwrap();
    let report = validate_census(&census).unwrap();
    assert_eq!(report.status, "passed");

    let posets = census
        .posets
        .iter()
        .map(|record| format!("{}\n", serde_json::to_string(record).unwrap()))
        .collect::<String>();
    let complexes = census
        .complexes
        .iter()
        .map(|record| format!("{}\n", serde_json::to_string(record).unwrap()))
        .collect::<String>();
    assert_eq!(posets, ORDER_3_POSETS);
    assert_eq!(complexes, ORDER_3_COMPLEXES);
}

#[test]
fn order_four_clears_counts_and_contains_the_minimal_finite_circle() {
    let (candidate_count, representatives) = reference_representatives(4, 64).unwrap();
    let census = enrich_census(4, 64, candidate_count, representatives).unwrap();
    assert_eq!(validate_census(&census).unwrap().status, "passed");

    let expected_poset = serde_json::from_str(MINIMAL_CIRCLE_POSET).unwrap();
    let expected_complex = serde_json::from_str(MINIMAL_CIRCLE_COMPLEX).unwrap();
    assert!(census.posets.contains(&expected_poset));
    assert!(census.complexes.contains(&expected_complex));
}

#[test]
fn reference_key_flag_records_whether_validation_ran_the_oracle() {
    let (candidate_count, representatives) = reference_representatives(5, 1_024).unwrap();
    let census = enrich_census(5, 1_024, candidate_count, representatives).unwrap();
    let report = validate_census(&census).unwrap();
    assert!(report.checks.reference_key_set_complete);

    let (stats, representatives) = orderly_representatives(6, 1_000_000).unwrap();
    let census = enrich_census_with_generator(
        Generator::HereditaryOrderly,
        6,
        1_000_000,
        stats.downset_count,
        representatives,
    )
    .unwrap();
    let report = validate_census(&census).unwrap();
    assert!(!report.checks.reference_key_set_complete);
}

#[test]
fn complex_key_matches_the_normative_square_example() {
    let square = Poset::new(
        8,
        &[
            (0, 4),
            (0, 5),
            (1, 4),
            (1, 6),
            (2, 5),
            (2, 7),
            (3, 6),
            (3, 7),
        ],
    )
    .unwrap();
    let facets = square
        .order_complex()
        .unwrap()
        .facets()
        .into_iter()
        .map(|facet| facet.vertices().to_vec())
        .collect::<Vec<_>>();
    assert_eq!(
        complex_key(8, &facets).unwrap().as_str(),
        "oc1-sha256:d3e6ea646cd7e937974e402bf72f0c937bc44db87773c37b02dc64812e7bb43f"
    );
}

#[test]
fn empty_complex_uses_signed_dimension() {
    let complex = SimplicialComplex::from_facets(&[]);
    assert_eq!(complex.dimension(), -1);
}

#[test]
fn poset_keys_sort_decimal_entries_numerically() {
    let two = PosetKey::new(vec!["2".to_owned()]).unwrap();
    let ten = PosetKey::new(vec!["10".to_owned()]).unwrap();
    assert!(two < ten);
}

#[test]
fn key_deserialization_rejects_noncanonical_decimal_text() {
    assert!(serde_json::from_str::<PosetKey>(r#"["01"]"#).is_err());
    assert!(
        serde_json::from_str::<poset_census::records::ComplexKey>(r#""oc1-sha256:ABC""#).is_err()
    );
}

#[test]
fn emission_is_reproducible_and_completed_orders_are_immutable() {
    let (candidate_count, representatives) = reference_representatives(3, 8).unwrap();
    let census = enrich_census(3, 8, candidate_count, representatives).unwrap();
    let validation = validate_census(&census).unwrap();
    let first_root = temporary_root("first");
    let second_root = temporary_root("second");
    let started = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let elapsed = Duration::from_millis(10);

    let first = emit_completed_order(&first_root, &census, &validation, started, elapsed).unwrap();
    let second =
        emit_completed_order(&second_root, &census, &validation, started, elapsed).unwrap();
    for file in [
        "posets.jsonl",
        "simplicial-complexes.jsonl",
        "validation.json",
    ] {
        assert_eq!(
            fs::read(first.join(file)).unwrap(),
            fs::read(second.join(file)).unwrap()
        );
    }
    assert!(emit_completed_order(&first_root, &census, &validation, started, elapsed).is_err());

    fs::remove_dir_all(first_root).unwrap();
    fs::remove_dir_all(second_root).unwrap();
}

fn temporary_root(label: &str) -> std::path::PathBuf {
    let id = TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "poset-census-test-{}-{label}-{id}",
        std::process::id()
    ))
}
