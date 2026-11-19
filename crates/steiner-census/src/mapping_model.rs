//! Exact mapping models for the pinned Noether version.
//!
//! A generator is identified by its entire ambient cell, including whiskering.
//! Each successive skeleton is realized in the ambient category and checked
//! bijectively against the actual fillers. Equal cell counts alone do not
//! certify a model. This compatibility implementation also fixes the upstream
//! `FiniteOmegaCategory::strong_steiner_model` method.

use noether::category::{
    CategoryError, FiniteOmegaCategory, SignedBoundary, SteinerCategory, SteinerCell,
};

/// Recover a model together with an exhaustive degreewise boundary check.
/// Unsupported nonfree mapping categories return `None`.
pub fn verified_mapping_model(
    mapping: &FiniteOmegaCategory,
) -> Result<Option<SteinerCategory>, CategoryError> {
    let counts = mapping.exact_cell_counts()?;
    if counts.is_empty() || counts[0] == 0 {
        return Ok(Some(SteinerCategory::empty()));
    }
    let mut generators = vec![mapping.exact_cells(0)?];
    for degree in 1..counts.len() {
        generators.push(
            mapping
                .exact_cells(degree)?
                .into_iter()
                .filter(|cell| cell.top_atoms().len() == 1)
                .collect(),
        );
    }
    let atom_counts: Vec<_> = generators.iter().map(Vec::len).collect();
    let mut boundaries = vec![Vec::new()];
    let mut model = SteinerCategory::from_boundaries(vec![atom_counts[0]], boundaries.clone())?;
    let mut inverse: Vec<(SteinerCell, SteinerCell)> = Vec::new();
    for degree in 0..counts.len() {
        if degree > 0 {
            let mut boundary = Vec::new();
            for generator in &generators[degree] {
                let source = generator.source()?;
                let target = generator.target()?;
                let Some((_, source_model)) =
                    inverse.iter().find(|(ambient, _)| ambient == &source)
                else {
                    return Ok(None);
                };
                let Some((_, target_model)) =
                    inverse.iter().find(|(ambient, _)| ambient == &target)
                else {
                    return Ok(None);
                };
                let source_support = &source_model.negative_supports()[degree - 1];
                let target_support = &target_model.negative_supports()[degree - 1];
                let negative: Vec<_> = source_support
                    .iter()
                    .copied()
                    .filter(|atom| !target_support.contains(atom))
                    .collect();
                let positive: Vec<_> = target_support
                    .iter()
                    .copied()
                    .filter(|atom| !source_support.contains(atom))
                    .collect();
                boundary.push(SignedBoundary::new(&negative, &positive)?);
            }
            boundaries.push(boundary);
            let Ok(next) = SteinerCategory::from_boundaries(
                atom_counts[..=degree].to_vec(),
                boundaries.clone(),
            ) else {
                return Ok(None);
            };
            model = next;
        }
        let expected = mapping.exact_cells(degree)?;
        let mut observed = Vec::new();
        let mut next_inverse = Vec::new();
        for cell in model.cells(degree)? {
            let Some(ambient) = realize(mapping, &generators, &cell)? else {
                return Ok(None);
            };
            if next_inverse
                .iter()
                .any(|(previous, _)| previous == &ambient)
            {
                return Ok(None);
            }
            if degree == 0 || !cell.is_identity() {
                if !expected.contains(&ambient) {
                    return Ok(None);
                }
                observed.push(ambient.clone());
            }
            if degree > 0 {
                for (model_boundary, actual_boundary) in [
                    (cell.source()?, ambient.source()?),
                    (cell.target()?, ambient.target()?),
                ] {
                    let Some((_, prior_model)) = inverse
                        .iter()
                        .find(|(actual, _)| actual == &actual_boundary)
                    else {
                        return Ok(None);
                    };
                    // The earlier skeleton has another category context; compare
                    // double sequences, which retain every whiskering component.
                    if prior_model.negative_supports() != model_boundary.negative_supports()
                        || prior_model.positive_supports() != model_boundary.positive_supports()
                    {
                        return Ok(None);
                    }
                }
            }
            next_inverse.push((ambient, cell));
        }
        if observed.len() != expected.len() {
            return Ok(None);
        }
        inverse = next_inverse;
    }
    Ok(Some(model))
}

fn realize(
    mapping: &FiniteOmegaCategory,
    generators: &[Vec<SteinerCell>],
    cell: &SteinerCell,
) -> Result<Option<SteinerCell>, CategoryError> {
    let depth = mapping.depth();
    let base = &generators[0][0];
    let mut negative = base.negative_supports()[..depth].to_vec();
    let mut positive = base.positive_supports()[..depth].to_vec();
    for degree in 0..=cell.dimension() {
        for (supports, result) in [
            (cell.negative_supports(), &mut negative),
            (cell.positive_supports(), &mut positive),
        ] {
            let mut chain: Vec<_> = supports[degree]
                .iter()
                .flat_map(|&index| {
                    generators[degree][index].negative_supports()[depth + degree]
                        .iter()
                        .copied()
                })
                .collect();
            chain.sort_unstable();
            if chain.windows(2).any(|pair| pair[0] == pair[1]) {
                return Ok(None);
            }
            result.push(chain);
        }
    }
    Ok(mapping
        .ambient()
        .cell_from_supports(negative, positive)
        .ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use noether::category::AtomId;

    #[test]
    fn whiskered_globes_have_distinct_boundaries() {
        let xy = SignedBoundary::new(&[0], &[1]).unwrap();
        let yz = SignedBoundary::new(&[1], &[2]).unwrap();
        let parallel = SignedBoundary::new(&[0], &[1]).unwrap();
        let category = SteinerCategory::from_boundaries(
            vec![3, 4, 2, 1],
            vec![
                vec![],
                vec![xy.clone(), xy, yz.clone(), yz],
                vec![parallel.clone(); 2],
                vec![parallel],
            ],
        )
        .unwrap();
        let mapping = category
            .mapping_category(
                &category.atom(AtomId::new(0, 0)).unwrap(),
                &category.atom(AtomId::new(0, 2)).unwrap(),
            )
            .unwrap();
        let model = verified_mapping_model(&mapping)
            .unwrap()
            .expect("two disjoint globes are free");
        let globe = SteinerCategory::globe(2).unwrap();
        assert!(
            model
                .is_isomorphic(&globe.disjoint_union(&globe).unwrap())
                .unwrap()
        );
        assert_ne!(
            model.boundary(AtomId::new(2, 0)).unwrap(),
            model.boundary(AtomId::new(2, 1)).unwrap()
        );
    }

    #[test]
    fn whiskered_composite_boundaries_recover_two_orientals() {
        let xy = SignedBoundary::new(&[0], &[1]).unwrap();
        let yz = SignedBoundary::new(&[1], &[2]).unwrap();
        let category = SteinerCategory::from_boundaries(
            vec![3, 5, 3, 1],
            vec![
                vec![],
                vec![xy.clone(), xy.clone(), xy, yz.clone(), yz],
                vec![
                    SignedBoundary::new(&[0], &[1]).unwrap(),
                    SignedBoundary::new(&[1], &[2]).unwrap(),
                    SignedBoundary::new(&[0], &[2]).unwrap(),
                ],
                vec![SignedBoundary::new(&[2], &[0, 1]).unwrap()],
            ],
        )
        .unwrap();
        let mapping = category
            .mapping_category(
                &category.atom(AtomId::new(0, 0)).unwrap(),
                &category.atom(AtomId::new(0, 2)).unwrap(),
            )
            .unwrap();
        let model = verified_mapping_model(&mapping)
            .unwrap()
            .expect("two disjoint orientals are free");
        let oriental = SteinerCategory::oriental(2).unwrap();
        assert!(
            model
                .is_isomorphic(&oriental.disjoint_union(&oriental).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn empty_identity_and_single_globe_models() {
        for dimension in 0..=4 {
            let category = SteinerCategory::globe(dimension).unwrap();
            for source in category.cells(0).unwrap() {
                for target in category.cells(0).unwrap() {
                    let mapping = category.mapping_category(&source, &target).unwrap();
                    let model = verified_mapping_model(&mapping).unwrap().unwrap();
                    assert_eq!(
                        model.exact_cell_counts().unwrap(),
                        mapping.exact_cell_counts().unwrap()
                    );
                }
            }
        }
    }
}
