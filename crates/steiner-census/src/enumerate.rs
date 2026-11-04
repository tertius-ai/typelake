use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, bail};
use noether::{
    category::{AtomId, SignedBoundary, SteinerCanonicalLabel, SteinerCategory},
    groups::{Permutation, PermutationGroup},
};
use serde::{Deserialize, Serialize};

use crate::records::SteinerKey;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnumerationStats {
    pub raw_candidates: u64,
    pub orbit_representatives: u64,
    pub canonical_tests: u64,
    pub accepted: usize,
}

/// Complete redundant signed-boundary-matrix generator for one atomic shell.
pub fn reference_representatives(
    atomic_size: usize,
    candidate_cap: u64,
) -> Result<(
    EnumerationStats,
    BTreeMap<SteinerKey, SteinerCanonicalLabel>,
)> {
    if atomic_size == 0 {
        let label = SteinerCategory::empty().canonical_label()?;
        return Ok((
            EnumerationStats {
                raw_candidates: 1,
                orbit_representatives: 1,
                canonical_tests: 1,
                accepted: 1,
            },
            BTreeMap::from([(SteinerKey::from_canonical_label(&label), label)]),
        ));
    }

    let mut representatives = BTreeMap::new();
    let mut stats = EnumerationStats::default();
    for atom_counts in dimension_vectors(atomic_size) {
        let options = boundary_options_by_degree(&atom_counts)?;
        let mut boundaries = vec![Vec::new(); atom_counts.len()];
        visit_boundary_tables(
            &atom_counts,
            &options,
            1,
            0,
            &mut boundaries,
            candidate_cap,
            &mut stats,
            &mut representatives,
        )?;
    }
    stats.accepted = representatives.len();
    Ok((stats, representatives))
}

#[allow(clippy::too_many_arguments)]
fn visit_boundary_tables(
    atom_counts: &[usize],
    options: &[Vec<SignedBoundary>],
    degree: usize,
    atom: usize,
    boundaries: &mut [Vec<SignedBoundary>],
    candidate_cap: u64,
    stats: &mut EnumerationStats,
    representatives: &mut BTreeMap<SteinerKey, SteinerCanonicalLabel>,
) -> Result<()> {
    if degree == atom_counts.len() {
        stats.raw_candidates = stats
            .raw_candidates
            .checked_add(1)
            .context("reference candidate counter overflow")?;
        if stats.raw_candidates > candidate_cap {
            bail!(
                "reference enumeration exceeded candidate cap {candidate_cap}; raise it explicitly"
            );
        }
        let Ok(candidate) =
            SteinerCategory::from_boundaries(atom_counts.to_vec(), boundaries.to_vec())
        else {
            return Ok(());
        };
        stats.canonical_tests += 1;
        insert_canonical(candidate, representatives)?;
        return Ok(());
    }
    if atom == atom_counts[degree] {
        return visit_boundary_tables(
            atom_counts,
            options,
            degree + 1,
            0,
            boundaries,
            candidate_cap,
            stats,
            representatives,
        );
    }
    for boundary in &options[degree] {
        boundaries[degree].push(boundary.clone());
        visit_boundary_tables(
            atom_counts,
            options,
            degree,
            atom + 1,
            boundaries,
            candidate_cap,
            stats,
            representatives,
        )?;
        boundaries[degree].pop();
    }
    Ok(())
}

fn boundary_options_by_degree(atom_counts: &[usize]) -> Result<Vec<Vec<SignedBoundary>>> {
    let mut output = vec![Vec::new(); atom_counts.len()];
    for degree in 1..atom_counts.len() {
        output[degree] = signed_support_pairs(atom_counts[degree - 1])?;
    }
    Ok(output)
}

fn signed_support_pairs(count: usize) -> Result<Vec<SignedBoundary>> {
    let candidate_count = 3usize
        .checked_pow(u32::try_from(count).context("boundary rank exceeds u32")?)
        .context("ternary boundary space overflows usize")?;
    let mut output = Vec::new();
    for mut code in 0..candidate_count {
        let mut negative = Vec::new();
        let mut positive = Vec::new();
        for atom in 0..count {
            match code % 3 {
                1 => negative.push(atom),
                2 => positive.push(atom),
                _ => {}
            }
            code /= 3;
        }
        if negative.is_empty() || positive.is_empty() {
            continue;
        }
        output.push(SignedBoundary::new(&negative, &positive)?);
    }
    Ok(output)
}

fn dimension_vectors(total: usize) -> Vec<Vec<usize>> {
    fn visit(remaining: usize, prefix: &mut Vec<usize>, output: &mut Vec<Vec<usize>>) {
        if remaining == 0 {
            output.push(prefix.clone());
            return;
        }
        for next in 1..=remaining {
            prefix.push(next);
            visit(remaining - next, prefix, output);
            prefix.pop();
        }
    }
    let mut output = Vec::new();
    visit(total, &mut Vec::new(), &mut output);
    output
}

/// Complete hereditary cell-attachment generator through one atomic shell.
pub fn orderly_representatives(
    atomic_size: usize,
    candidate_cap: u64,
) -> Result<(
    EnumerationStats,
    BTreeMap<SteinerKey, SteinerCanonicalLabel>,
)> {
    let empty = SteinerCategory::empty().canonical_label()?;
    let mut parents = BTreeMap::from([(SteinerKey::from_canonical_label(&empty), empty)]);
    let mut stats = EnumerationStats::default();
    if atomic_size == 0 {
        stats.accepted = 1;
        return Ok((stats, parents));
    }
    for _child_size in 1..=atomic_size {
        let mut children = BTreeMap::new();
        for (parent_key, parent_label) in &parents {
            extend_parent(
                parent_key,
                parent_label,
                candidate_cap,
                &mut stats,
                &mut children,
            )?;
        }
        parents = children;
    }
    stats.accepted = parents.len();
    Ok((stats, parents))
}

/// Complete recursive reference generator without orbit or canonical-parent pruning.
///
/// Every nonempty finite strong Steiner category is obtained by deleting one
/// top-degree atom and then reattaching its two disjoint parallel boundary
/// cells. Unlike [`orderly_representatives`], this generator tries every such
/// attachment to every canonical parent and deduplicates only after full
/// canonicalization. Its deliberately different acceptance path makes it a
/// practical independent oracle beyond the range of the boundary-matrix scan.
pub fn recursive_reference_representatives(
    atomic_size: usize,
    candidate_cap: u64,
) -> Result<(
    EnumerationStats,
    BTreeMap<SteinerKey, SteinerCanonicalLabel>,
)> {
    let empty = SteinerCategory::empty().canonical_label()?;
    let mut parents = BTreeMap::from([(SteinerKey::from_canonical_label(&empty), empty)]);
    let mut stats = EnumerationStats::default();
    if atomic_size == 0 {
        stats.accepted = 1;
        return Ok((stats, parents));
    }
    for _child_size in 1..=atomic_size {
        let mut children = BTreeMap::new();
        for parent_label in parents.values() {
            extend_parent_without_pruning(
                parent_label.category(),
                candidate_cap,
                &mut stats,
                &mut children,
            )?;
        }
        parents = children;
    }
    stats.accepted = parents.len();
    Ok((stats, parents))
}

fn extend_parent_without_pruning(
    parent: &SteinerCategory,
    candidate_cap: u64,
    stats: &mut EnumerationStats,
    children: &mut BTreeMap<SteinerKey, SteinerCanonicalLabel>,
) -> Result<()> {
    if parent.dimension() <= 0 {
        count_recursive_candidate(candidate_cap, stats)?;
        if let Some(child) = adjoin_object(parent)? {
            stats.canonical_tests += 1;
            insert_canonical(child, children)?;
        }
    }
    let first_degree = if parent.dimension() < 1 {
        1
    } else {
        parent.dimension() as usize
    };
    let last_degree = if parent.dimension() < 1 {
        1
    } else {
        parent.dimension() as usize + 1
    };
    for degree in first_degree..=last_degree {
        let lower_cells = parent.exact_cells(degree - 1)?;
        let mut seen_support_pairs = BTreeSet::new();
        for source in &lower_cells {
            for target in &lower_cells {
                if source == target
                    || !source
                        .top_atoms()
                        .iter()
                        .all(|atom| !target.top_atoms().contains(atom))
                {
                    continue;
                }
                if degree > 1
                    && (source.source()? != target.source()?
                        || source.target()? != target.target()?)
                {
                    continue;
                }
                if parent.cell(degree - 1, source.top_atoms())? != *source
                    || parent.cell(degree - 1, target.top_atoms())? != *target
                {
                    continue;
                }
                let pair = (source.top_atoms().to_vec(), target.top_atoms().to_vec());
                if !seen_support_pairs.insert(pair.clone()) {
                    continue;
                }
                count_recursive_candidate(candidate_cap, stats)?;
                let Some(child) = adjoin_atom(parent, degree, &pair.0, &pair.1)? else {
                    continue;
                };
                stats.canonical_tests += 1;
                insert_canonical(child, children)?;
            }
        }
    }
    Ok(())
}

fn count_recursive_candidate(candidate_cap: u64, stats: &mut EnumerationStats) -> Result<()> {
    stats.raw_candidates = stats
        .raw_candidates
        .checked_add(1)
        .context("recursive-reference candidate counter overflow")?;
    if stats.raw_candidates > candidate_cap {
        bail!(
            "recursive-reference enumeration exceeded candidate cap {candidate_cap}; raise it explicitly"
        );
    }
    Ok(())
}

fn extend_parent(
    parent_key: &SteinerKey,
    parent_label: &SteinerCanonicalLabel,
    candidate_cap: u64,
    stats: &mut EnumerationStats,
    children: &mut BTreeMap<SteinerKey, SteinerCanonicalLabel>,
) -> Result<()> {
    let parent = parent_label.category();
    if parent.dimension() <= 0 {
        consider_child(
            parent_key,
            adjoin_object(parent)?,
            candidate_cap,
            true,
            stats,
            children,
        )?;
    }
    let first_positive = 1usize;
    let last_positive = if parent.dimension() < 1 {
        1
    } else {
        parent.dimension() as usize + 1
    };
    let first_positive = if parent.dimension() < 1 {
        first_positive
    } else {
        parent.dimension() as usize
    };
    for degree in first_positive..=last_positive {
        let lower_cells = parent.exact_cells(degree - 1)?;
        let mut seen_support_pairs = BTreeSet::new();
        for source in &lower_cells {
            for target in &lower_cells {
                if source == target
                    || !source
                        .top_atoms()
                        .iter()
                        .all(|atom| !target.top_atoms().contains(atom))
                {
                    continue;
                }
                if degree > 1
                    && (source.source()? != target.source()?
                        || source.target()? != target.target()?)
                {
                    continue;
                }
                if parent.cell(degree - 1, source.top_atoms())? != *source
                    || parent.cell(degree - 1, target.top_atoms())? != *target
                {
                    continue;
                }
                let pair = (source.top_atoms().to_vec(), target.top_atoms().to_vec());
                if !seen_support_pairs.insert(pair.clone()) {
                    continue;
                }
                stats.raw_candidates = stats
                    .raw_candidates
                    .checked_add(1)
                    .context("orderly candidate counter overflow")?;
                if stats.raw_candidates > candidate_cap {
                    bail!(
                        "orderly enumeration exceeded candidate cap {candidate_cap}; raise it explicitly"
                    );
                }
                if !is_boundary_orbit_minimum(
                    parent,
                    parent_label.automorphism_group(),
                    degree,
                    &pair,
                )? {
                    continue;
                }
                stats.orbit_representatives = stats
                    .orbit_representatives
                    .checked_add(1)
                    .context("orderly orbit-representative counter overflow")?;
                let child = adjoin_atom(parent, degree, &pair.0, &pair.1)?;
                consider_child(parent_key, child, candidate_cap, false, stats, children)?;
            }
        }
    }
    Ok(())
}

fn consider_child(
    parent_key: &SteinerKey,
    candidate: Option<SteinerCategory>,
    candidate_cap: u64,
    count_candidate: bool,
    stats: &mut EnumerationStats,
    children: &mut BTreeMap<SteinerKey, SteinerCanonicalLabel>,
) -> Result<()> {
    if count_candidate {
        stats.raw_candidates = stats
            .raw_candidates
            .checked_add(1)
            .context("orderly candidate counter overflow")?;
        if stats.raw_candidates > candidate_cap {
            bail!(
                "orderly enumeration exceeded candidate cap {candidate_cap}; raise it explicitly"
            );
        }
        stats.orbit_representatives = stats
            .orbit_representatives
            .checked_add(1)
            .context("orderly orbit-representative counter overflow")?;
    }
    let Some(candidate) = candidate else {
        return Ok(());
    };
    stats.canonical_tests += 1;
    let label = candidate.canonical_label()?;
    let canonical = label.category();
    let canonical_parent = delete_distinguished_atom(canonical)?
        .canonical_label()
        .context("canonical parent failed to canonicalize")?;
    if SteinerKey::from_canonical_label(&canonical_parent) != *parent_key {
        return Ok(());
    }
    let canonical_label = canonical.canonical_label()?;
    let key = SteinerKey::from_canonical_label(&canonical_label);
    if let Some(existing) = children.insert(key.clone(), canonical_label)
        && existing.category() != canonical
    {
        bail!("canonical-key collision at {key:?}");
    }
    Ok(())
}

fn is_boundary_orbit_minimum(
    category: &SteinerCategory,
    group: &PermutationGroup,
    degree: usize,
    pair: &(Vec<usize>, Vec<usize>),
) -> Result<bool> {
    let mut generators = group.generators().to_vec();
    generators.extend(group.generators().iter().map(Permutation::inverse));
    let mut seen = BTreeSet::from([pair.clone()]);
    let mut queue = VecDeque::from([pair.clone()]);
    while let Some(current) = queue.pop_front() {
        for generator in &generators {
            let image = transform_boundary_pair(category, degree, &current, generator)?;
            if image < *pair {
                return Ok(false);
            }
            if seen.insert(image.clone()) {
                queue.push_back(image);
            }
        }
    }
    Ok(true)
}

fn transform_boundary_pair(
    category: &SteinerCategory,
    degree: usize,
    pair: &(Vec<usize>, Vec<usize>),
    permutation: &Permutation,
) -> Result<(Vec<usize>, Vec<usize>)> {
    let lower_degree = degree - 1;
    let offset = category.atom_counts()[..lower_degree].iter().sum::<usize>();
    let transform = |support: &[usize]| -> Result<Vec<usize>> {
        let mut image = support
            .iter()
            .map(|&atom| {
                let flattened = offset
                    .checked_add(atom)
                    .context("flattened atom overflow")?;
                let image = permutation.apply(
                    u32::try_from(flattened).context("flattened atom exceeds permutation point")?,
                ) as usize;
                image
                    .checked_sub(offset)
                    .context("automorphism moved an atom across degree blocks")
            })
            .collect::<Result<Vec<_>>>()?;
        image.sort_unstable();
        Ok(image)
    };
    Ok((transform(&pair.0)?, transform(&pair.1)?))
}

fn insert_canonical(
    candidate: SteinerCategory,
    representatives: &mut BTreeMap<SteinerKey, SteinerCanonicalLabel>,
) -> Result<()> {
    let label = candidate.canonical_label()?;
    let canonical = label.category().canonical_label()?;
    let key = SteinerKey::from_canonical_label(&canonical);
    representatives.entry(key).or_insert(canonical);
    Ok(())
}

fn adjoin_object(parent: &SteinerCategory) -> Result<Option<SteinerCategory>> {
    if parent.dimension() > 0 {
        return Ok(None);
    }
    let count = parent.atom_count_in_degree(0) + 1;
    Ok(Some(SteinerCategory::from_boundaries(
        vec![count],
        vec![Vec::new()],
    )?))
}

fn adjoin_atom(
    parent: &SteinerCategory,
    degree: usize,
    negative: &[usize],
    positive: &[usize],
) -> Result<Option<SteinerCategory>> {
    let mut atom_counts = parent.atom_counts().to_vec();
    let mut boundaries = copy_boundaries(parent)?;
    if degree > atom_counts.len() {
        return Ok(None);
    }
    if degree == atom_counts.len() {
        atom_counts.push(1);
        boundaries.push(vec![SignedBoundary::new(negative, positive)?]);
    } else {
        atom_counts[degree] += 1;
        boundaries[degree].push(SignedBoundary::new(negative, positive)?);
    }
    Ok(SteinerCategory::from_boundaries(atom_counts, boundaries).ok())
}

fn delete_distinguished_atom(category: &SteinerCategory) -> Result<SteinerCategory> {
    if category.atom_count() == 0 {
        bail!("the empty category has no canonical parent");
    }
    let mut atom_counts = category.atom_counts().to_vec();
    let mut boundaries = copy_boundaries(category)?;
    let degree = atom_counts.len() - 1;
    atom_counts[degree] -= 1;
    if degree > 0 {
        boundaries[degree].pop();
    }
    if atom_counts[degree] == 0 {
        atom_counts.pop();
        boundaries.pop();
    }
    Ok(SteinerCategory::from_boundaries(atom_counts, boundaries)?)
}

fn copy_boundaries(category: &SteinerCategory) -> Result<Vec<Vec<SignedBoundary>>> {
    (0..category.atom_counts().len())
        .map(|degree| {
            if degree == 0 {
                return Ok(Vec::new());
            }
            (0..category.atom_count_in_degree(degree))
                .map(|atom| Ok(category.boundary(AtomId::new(degree, atom))?.clone()))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_vectors_are_compositions() {
        assert_eq!(
            dimension_vectors(3),
            vec![vec![1, 1, 1], vec![1, 2], vec![2, 1], vec![3]]
        );
    }

    #[test]
    fn all_generators_agree_through_size_seven() {
        for size in 0..=7 {
            let (_, reference) = reference_representatives(size, 2_000_000).unwrap();
            let (_, recursive) = recursive_reference_representatives(size, 2_000_000).unwrap();
            let (_, orderly) = orderly_representatives(size, 2_000_000).unwrap();
            assert_eq!(
                reference.keys().collect::<Vec<_>>(),
                orderly.keys().collect::<Vec<_>>(),
                "atomic size {size}"
            );
            assert_eq!(
                recursive.keys().collect::<Vec<_>>(),
                orderly.keys().collect::<Vec<_>>(),
                "atomic size {size}"
            );
        }
    }
}
