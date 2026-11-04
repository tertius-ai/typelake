use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, bail, ensure};
use noether::{
    groups::{Permutation, PermutationGroup},
    topology::{SimplicialFace, SimplicialSet},
};
use serde::{Deserialize, Serialize};

use crate::records::SimplicialSetKey;

pub const MAX_REFERENCE_TOTAL: usize = 5;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LevelStats {
    pub total_nondegenerate: usize,
    pub parents: usize,
    pub raw_boundaries: u64,
    pub boundary_orbits: u64,
    pub valid_augmentations: u64,
    pub canonical_tests: u64,
    pub duplicate_acceptances: u64,
    pub accepted: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnumerationStats {
    pub maximum_dimension: usize,
    pub levels: Vec<LevelStats>,
    pub raw_boundaries: u64,
    pub boundary_orbits: u64,
    pub valid_augmentations: u64,
    pub canonical_tests: u64,
    pub duplicate_acceptances: u64,
    pub accepted: usize,
}

pub fn orderly_representatives(
    total_nondegenerate: usize,
    maximum_dimension: usize,
    candidate_cap: u64,
) -> Result<(EnumerationStats, BTreeMap<SimplicialSetKey, SimplicialSet>)> {
    let empty = SimplicialSet::new(Vec::new())?;
    let mut parents = BTreeMap::from([(SimplicialSetKey::from_value(&empty), empty)]);
    let mut stats = EnumerationStats {
        maximum_dimension,
        ..EnumerationStats::default()
    };
    if total_nondegenerate == 0 {
        stats.accepted = 1;
        return Ok((stats, parents));
    }

    for total in 1..=total_nondegenerate {
        let mut level = LevelStats {
            total_nondegenerate: total,
            parents: parents.len(),
            ..LevelStats::default()
        };
        let mut children = BTreeMap::new();
        for (parent_key, parent) in &parents {
            visit_valid_augmentations(
                parent,
                maximum_dimension,
                candidate_cap,
                true,
                &mut stats.raw_boundaries,
                &mut level.raw_boundaries,
                &mut stats.boundary_orbits,
                &mut level.boundary_orbits,
                |candidate| {
                    stats.valid_augmentations += 1;
                    level.valid_augmentations += 1;
                    stats.canonical_tests += 1;
                    level.canonical_tests += 1;
                    let canonical = candidate.canonical_form()?;
                    if canonical_parent_key(&canonical)? != *parent_key {
                        return Ok(());
                    }
                    let key = SimplicialSetKey::from_value(&canonical);
                    if children.insert(key, canonical).is_some() {
                        stats.duplicate_acceptances += 1;
                        level.duplicate_acceptances += 1;
                    }
                    Ok(())
                },
            )?;
        }
        level.accepted = children.len();
        stats.levels.push(level);
        parents = children;
    }
    stats.accepted = parents.len();
    Ok((stats, parents))
}

pub fn reference_representatives(
    total_nondegenerate: usize,
    maximum_dimension: usize,
    candidate_cap: u64,
) -> Result<(EnumerationStats, BTreeMap<SimplicialSetKey, SimplicialSet>)> {
    if total_nondegenerate > MAX_REFERENCE_TOTAL {
        bail!(
            "labeled reference enumeration supports total size at most {MAX_REFERENCE_TOTAL}, requested {total_nondegenerate}"
        );
    }
    let empty = SimplicialSet::new(Vec::new())?;
    let empty_key = SimplicialSetKey::from_value(&empty);
    let mut labeled = BTreeMap::from([(empty_key, empty)]);
    let mut stats = EnumerationStats {
        maximum_dimension,
        ..EnumerationStats::default()
    };
    if total_nondegenerate == 0 {
        stats.accepted = 1;
        return Ok((stats, labeled));
    }

    for total in 1..=total_nondegenerate {
        let mut level = LevelStats {
            total_nondegenerate: total,
            parents: labeled.len(),
            ..LevelStats::default()
        };
        let mut children = BTreeMap::new();
        for parent in labeled.values() {
            visit_valid_augmentations(
                parent,
                maximum_dimension,
                candidate_cap,
                false,
                &mut stats.raw_boundaries,
                &mut level.raw_boundaries,
                &mut stats.boundary_orbits,
                &mut level.boundary_orbits,
                |candidate| {
                    stats.valid_augmentations += 1;
                    level.valid_augmentations += 1;
                    children.insert(SimplicialSetKey::from_value(&candidate), candidate);
                    Ok(())
                },
            )?;
        }
        level.accepted = children.len();
        stats.levels.push(level);
        labeled = children;
    }

    let mut canonical = BTreeMap::new();
    for value in labeled.into_values() {
        stats.canonical_tests += 1;
        let value = value.canonical_form()?;
        canonical.insert(SimplicialSetKey::from_value(&value), value);
    }
    stats.accepted = canonical.len();
    Ok((stats, canonical))
}

#[allow(clippy::too_many_arguments)]
fn visit_valid_augmentations(
    parent: &SimplicialSet,
    maximum_dimension: usize,
    candidate_cap: u64,
    quotient_by_automorphisms: bool,
    total_raw: &mut u64,
    level_raw: &mut u64,
    total_orbits: &mut u64,
    level_orbits: &mut u64,
    mut visit: impl FnMut(SimplicialSet) -> Result<()>,
) -> Result<()> {
    let minimum_dimension = if parent.is_empty() {
        0
    } else {
        usize::try_from(parent.dimension()).context("nonempty dimension is nonnegative")?
    };
    let automorphisms = quotient_by_automorphisms
        .then(|| parent.automorphism_group())
        .transpose()?;
    let offsets = dimension_offsets(parent);
    for dimension in minimum_dimension..=maximum_dimension {
        if dimension == 0 {
            add_candidate(total_raw, level_raw, candidate_cap)?;
            *total_orbits += 1;
            *level_orbits += 1;
            let mut faces = defining_faces(parent);
            if faces.is_empty() {
                faces.push(Vec::new());
            }
            faces[0].push(Vec::new());
            visit(SimplicialSet::new(faces)?)?;
            continue;
        }
        let options = face_options(parent, dimension);
        if options.is_empty() {
            continue;
        }
        let mut boundary = Vec::with_capacity(dimension + 1);
        let mut seen = BTreeSet::new();
        visit_boundaries(
            parent,
            dimension,
            &options,
            &mut boundary,
            total_raw,
            level_raw,
            candidate_cap,
            automorphisms.as_ref(),
            &offsets,
            &mut seen,
            total_orbits,
            level_orbits,
            &mut visit,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn visit_boundaries(
    parent: &SimplicialSet,
    dimension: usize,
    options: &[SimplicialFace],
    boundary: &mut Vec<SimplicialFace>,
    total_raw: &mut u64,
    level_raw: &mut u64,
    candidate_cap: u64,
    automorphisms: Option<&PermutationGroup>,
    offsets: &[usize],
    seen: &mut BTreeSet<Vec<crate::records::SimplicialFaceKey>>,
    total_orbits: &mut u64,
    level_orbits: &mut u64,
    visit: &mut impl FnMut(SimplicialSet) -> Result<()>,
) -> Result<()> {
    if boundary.len() == dimension + 1 {
        add_candidate(total_raw, level_raw, candidate_cap)?;
        if let Some(automorphisms) = automorphisms {
            let key = boundary
                .iter()
                .map(crate::records::SimplicialFaceKey::from)
                .collect::<Vec<_>>();
            if seen.contains(&key) {
                return Ok(());
            }
            mark_boundary_orbit(boundary, dimension, automorphisms, offsets, seen)?;
        }
        *total_orbits += 1;
        *level_orbits += 1;
        let mut faces = defining_faces(parent);
        if faces.len() <= dimension {
            faces.resize(dimension + 1, Vec::new());
        }
        faces[dimension].push(boundary.clone());
        if let Ok(candidate) = SimplicialSet::new(faces) {
            visit(candidate)?;
        }
        return Ok(());
    }
    for option in options {
        boundary.push(option.clone());
        visit_boundaries(
            parent,
            dimension,
            options,
            boundary,
            total_raw,
            level_raw,
            candidate_cap,
            automorphisms,
            offsets,
            seen,
            total_orbits,
            level_orbits,
            visit,
        )?;
        boundary.pop();
    }
    Ok(())
}

fn dimension_offsets(value: &SimplicialSet) -> Vec<usize> {
    if value.is_empty() {
        return Vec::new();
    }
    let mut total = 0;
    (0..=value.dimension() as usize)
        .map(|dimension| {
            let offset = total;
            total += value.count(dimension);
            offset
        })
        .collect()
}

fn mark_boundary_orbit(
    boundary: &[SimplicialFace],
    source_dimension: usize,
    automorphisms: &PermutationGroup,
    offsets: &[usize],
    seen: &mut BTreeSet<Vec<crate::records::SimplicialFaceKey>>,
) -> Result<()> {
    let mut queue = VecDeque::from([boundary.to_vec()]);
    while let Some(value) = queue.pop_front() {
        let key = value
            .iter()
            .map(crate::records::SimplicialFaceKey::from)
            .collect::<Vec<_>>();
        if !seen.insert(key) {
            continue;
        }
        for generator in automorphisms.generators() {
            queue.push_back(
                value
                    .iter()
                    .map(|face| relabel_face(face, source_dimension, generator, offsets))
                    .collect::<Result<Vec<_>>>()?,
            );
        }
    }
    Ok(())
}

fn relabel_face(
    face: &SimplicialFace,
    source_dimension: usize,
    permutation: &Permutation,
    offsets: &[usize],
) -> Result<SimplicialFace> {
    let (dimension, simplex, surjection) = match face {
        SimplicialFace::Cell(simplex) => (source_dimension - 1, *simplex, None),
        SimplicialFace::Degenerate {
            simplex_dimension,
            simplex,
            surjection,
        } => (*simplex_dimension, *simplex, Some(surjection.clone())),
    };
    let offset = offsets
        .get(dimension)
        .copied()
        .context("face dimension is absent from automorphism domain")?;
    let image = usize::try_from(permutation.apply((offset + simplex) as u32))
        .context("permutation image does not fit usize")?
        .checked_sub(offset)
        .context("automorphism moved a face outside its dimension")?;
    Ok(match surjection {
        None => SimplicialFace::Cell(image),
        Some(surjection) => SimplicialFace::degenerate(dimension, image, surjection),
    })
}

fn add_candidate(total: &mut u64, level: &mut u64, cap: u64) -> Result<()> {
    *total = total.checked_add(1).context("candidate count overflow")?;
    *level = level.checked_add(1).context("candidate count overflow")?;
    ensure!(*total <= cap, "candidate cap {cap} exceeded");
    Ok(())
}

fn face_options(parent: &SimplicialSet, source_dimension: usize) -> Vec<SimplicialFace> {
    let face_dimension = source_dimension - 1;
    let mut options = (0..parent.count(face_dimension))
        .map(SimplicialFace::Cell)
        .collect::<Vec<_>>();
    for simplex_dimension in 0..face_dimension {
        for simplex in 0..parent.count(simplex_dimension) {
            for surjection in monotone_surjections(source_dimension, simplex_dimension) {
                options.push(SimplicialFace::degenerate(
                    simplex_dimension,
                    simplex,
                    surjection,
                ));
            }
        }
    }
    options
}

fn monotone_surjections(source_vertices: usize, target_dimension: usize) -> Vec<Vec<usize>> {
    if target_dimension == 0 {
        return vec![vec![0; source_vertices]];
    }
    let mut jumps = Vec::with_capacity(target_dimension);
    let mut output = Vec::new();
    visit_jump_sets(
        source_vertices,
        target_dimension,
        1,
        &mut jumps,
        &mut output,
    );
    output
}

fn visit_jump_sets(
    source_vertices: usize,
    jumps_needed: usize,
    start: usize,
    jumps: &mut Vec<usize>,
    output: &mut Vec<Vec<usize>>,
) {
    if jumps.len() == jumps_needed {
        let mut value = 0;
        let mut surjection = Vec::with_capacity(source_vertices);
        for position in 0..source_vertices {
            if jumps.binary_search(&position).is_ok() {
                value += 1;
            }
            surjection.push(value);
        }
        output.push(surjection);
        return;
    }
    let remaining = jumps_needed - jumps.len();
    for jump in start..=source_vertices.saturating_sub(remaining) {
        jumps.push(jump);
        visit_jump_sets(source_vertices, jumps_needed, jump + 1, jumps, output);
        jumps.pop();
    }
}

fn defining_faces(value: &SimplicialSet) -> Vec<Vec<Vec<SimplicialFace>>> {
    if value.is_empty() {
        return Vec::new();
    }
    (0..=value.dimension() as usize)
        .map(|dimension| {
            (0..value.count(dimension))
                .map(|simplex| {
                    value
                        .faces_of(dimension, simplex)
                        .expect("dimension and simplex are in range")
                        .to_vec()
                })
                .collect()
        })
        .collect()
}

fn canonical_parent_key(child: &SimplicialSet) -> Result<SimplicialSetKey> {
    ensure!(!child.is_empty(), "empty set has no parent");
    let top = usize::try_from(child.dimension()).context("nonempty dimension is nonnegative")?;
    (0..child.count(top))
        .map(|simplex| {
            let mut faces = defining_faces(child);
            faces[top].remove(simplex);
            let parent = SimplicialSet::new(faces)?.canonical_form()?;
            Ok(SimplicialSetKey::from_value(&parent))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .min()
        .context("nonempty top level has a deletable cell")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surjections_are_canonical_and_complete() {
        assert_eq!(monotone_surjections(3, 0), vec![vec![0, 0, 0]]);
        assert_eq!(
            monotone_surjections(4, 2),
            vec![vec![0, 1, 2, 2], vec![0, 1, 1, 2], vec![0, 0, 1, 2]]
        );
    }

    #[test]
    fn production_and_labeled_oracle_agree_in_the_small_box() {
        for total in 0..=4 {
            let (_, production) = orderly_representatives(total, 2, 10_000_000).unwrap();
            let (_, reference) = reference_representatives(total, 2, 10_000_000).unwrap();
            assert!(production.keys().eq(reference.keys()), "total={total}");
        }
    }
}
