use steiner_census::{
    Census, DEFAULT_STREET_DEGREE, Generator, enrich_census, enumerate::orderly_representatives,
    validate::validate_census,
};

fn arrow_shell() -> Census {
    let (stats, representatives) = orderly_representatives(3, 20_000_000).unwrap();
    enrich_census(
        Generator::CellAttachment,
        3,
        20_000_000,
        stats.raw_candidates,
        representatives,
        DEFAULT_STREET_DEGREE,
    )
    .unwrap()
}

#[test]
fn a_duplicate_object_pair_cannot_replace_a_missing_pair() {
    let mut census = arrow_shell();
    validate_census(&census).unwrap();
    census.mappings[1] = census.mappings[0].clone();
    assert!(
        validate_census(&census)
            .unwrap_err()
            .to_string()
            .contains("object pair")
    );
}

#[test]
fn enriched_fields_must_reconstruct_from_boundaries() {
    let mut census = arrow_shell();
    let position = census
        .categories
        .iter()
        .position(|row| row.invariants.dimension == 1)
        .unwrap();
    let original = census.categories[position].clone();
    census.categories[position].automorphism_group.order = "99".into();
    assert!(validate_census(&census).is_err());
    census.categories[position] = original.clone();
    census.categories[position]
        .invariants
        .polygraphic_euler_characteristic += 1;
    assert!(validate_census(&census).is_err());
    census.categories[position] = original.clone();
    census.categories[position].recognized_families.clear();
    assert!(validate_census(&census).is_err());
    census.categories[position] = original;
    census.categories[position].dualities.clear();
    assert!(validate_census(&census).is_err());
}
