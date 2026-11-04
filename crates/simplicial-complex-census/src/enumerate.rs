use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, bail, ensure};
use noether::{
    groups::{Permutation, PermutationGroup},
    topology::SimplicialComplex,
};
use serde::{Deserialize, Serialize};

use crate::records::SimplicialComplexKey;

/// The production representation uses one bit for each nonempty face of a
/// parent. A target shell of seven vertices therefore has at most 63 link bits.
pub const MAX_ORDERLY_VERTICES: usize = 7;
pub const MAX_REFERENCE_VERTICES: usize = 4;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LevelStats {
    pub vertices: usize,
    pub parents: usize,
    pub raw_candidates: u64,
    pub orbit_representatives: u64,
    pub canonical_tests: u64,
    pub duplicate_acceptances: u64,
    pub accepted: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnumerationStats {
    pub levels: Vec<LevelStats>,
    pub raw_candidates: u64,
    pub orbit_representatives: u64,
    pub canonical_tests: u64,
    pub duplicate_acceptances: u64,
    pub accepted: usize,
}

/// Redundant labeled enumeration retained only as a small-shell oracle.
pub fn reference_representatives(
    vertices: usize,
    candidate_cap: u64,
) -> Result<(
    EnumerationStats,
    BTreeMap<SimplicialComplexKey, SimplicialComplex>,
)> {
    if vertices > MAX_REFERENCE_VERTICES {
        bail!(
            "reference enumeration supports at most {MAX_REFERENCE_VERTICES} vertices, requested {vertices}"
        );
    }
    let face_slots = (1_usize << vertices).saturating_sub(1);
    let candidate_count = 1_u64
        .checked_shl(u32::try_from(face_slots).context("face-slot count exceeds u32")?)
        .context("reference candidate count exceeds u64")?;
    if candidate_count > candidate_cap {
        bail!(
            "reference enumeration needs {candidate_count} candidates, exceeding cap {candidate_cap}"
        );
    }

    let required_vertices = (0..vertices)
        .map(|vertex| 1_u64 << ((1_usize << vertex) - 1))
        .fold(0_u64, |all, vertex| all | vertex);
    let mut representatives = BTreeMap::new();
    let mut canonical_tests = 0_u64;
    for selection in 0..candidate_count {
        if selection & required_vertices != required_vertices
            || !is_downward_closed(selection, vertices)
        {
            continue;
        }
        canonical_tests += 1;
        let candidate = complex_from_face_mask(selection, vertices);
        insert_canonical(candidate, &mut representatives)?;
    }
    let accepted = representatives.len();
    let stats = EnumerationStats {
        levels: vec![LevelStats {
            vertices,
            parents: 0,
            raw_candidates: candidate_count,
            orbit_representatives: canonical_tests,
            canonical_tests,
            duplicate_acceptances: canonical_tests.saturating_sub(accepted as u64),
            accepted,
        }],
        raw_candidates: candidate_count,
        orbit_representatives: canonical_tests,
        canonical_tests,
        duplicate_acceptances: canonical_tests.saturating_sub(accepted as u64),
        accepted,
    };
    Ok((stats, representatives))
}

/// Complete canonical vertex-link augmentation through `vertices`.
pub fn orderly_representatives(
    vertices: usize,
    candidate_cap: u64,
) -> Result<(
    EnumerationStats,
    BTreeMap<SimplicialComplexKey, SimplicialComplex>,
)> {
    if vertices > MAX_ORDERLY_VERTICES {
        bail!(
            "canonical vertex-link enumeration supports at most {MAX_ORDERLY_VERTICES} vertices, requested {vertices}"
        );
    }
    let empty = SimplicialComplex::from_facets(&[]).canonical_form()?;
    let mut parents = BTreeMap::from([(SimplicialComplexKey::from_canonical(&empty), empty)]);
    let mut stats = EnumerationStats::default();
    if vertices == 0 {
        stats.accepted = 1;
        return Ok((stats, parents));
    }

    for child_vertices in 1..=vertices {
        let mut level = LevelStats {
            vertices: child_vertices,
            parents: parents.len(),
            ..LevelStats::default()
        };
        let mut children = BTreeMap::new();
        for parent in parents.values() {
            extend_parent(
                parent,
                child_vertices,
                candidate_cap,
                &mut stats,
                &mut level,
                &mut children,
            )?;
        }
        level.accepted = children.len();
        stats.levels.push(level);
        parents = children;
    }
    stats.accepted = parents.len();
    Ok((stats, parents))
}

fn extend_parent(
    parent: &SimplicialComplex,
    child_vertices: usize,
    candidate_cap: u64,
    stats: &mut EnumerationStats,
    level: &mut LevelStats,
    children: &mut BTreeMap<SimplicialComplexKey, SimplicialComplex>,
) -> Result<()> {
    ensure!(
        parent.count(0) + 1 == child_vertices,
        "parent/child shell mismatch"
    );
    let parent_vertices = child_vertices - 1;
    let allowed_faces = face_mask(parent, parent_vertices)?;
    let links = subcomplex_masks(allowed_faces, parent_vertices)?;
    add_count(
        &mut stats.raw_candidates,
        links.len(),
        candidate_cap,
        "link candidates",
    )?;
    add_count(
        &mut level.raw_candidates,
        links.len(),
        candidate_cap,
        "level link candidates",
    )?;

    let automorphisms = parent.automorphism_group()?;
    let link_representatives = link_orbit_representatives(&links, &automorphisms, parent_vertices)?;
    add_count(
        &mut stats.orbit_representatives,
        link_representatives.len(),
        u64::MAX,
        "link orbit representatives",
    )?;
    add_count(
        &mut level.orbit_representatives,
        link_representatives.len(),
        u64::MAX,
        "level link orbit representatives",
    )?;

    for link in link_representatives {
        stats.canonical_tests += 1;
        level.canonical_tests += 1;
        let candidate = attach_vertex(parent, link, parent_vertices)?;
        let canonical = candidate.canonical_form()?;
        let canonical_parent = delete_vertex(&canonical, parent_vertices).canonical_form()?;
        if &canonical_parent != parent {
            continue;
        }
        let key = SimplicialComplexKey::from_canonical(&canonical);
        if children.insert(key, canonical).is_some() {
            stats.duplicate_acceptances += 1;
            level.duplicate_acceptances += 1;
        }
    }
    Ok(())
}

fn subcomplex_masks(allowed: u64, vertices: usize) -> Result<BTreeSet<u64>> {
    let limit = 1_usize << vertices;
    let faces = (1..limit)
        .filter(|face| allowed & (1_u64 << (face - 1)) != 0)
        .collect::<Vec<_>>();
    let mut output = BTreeSet::new();
    let mut antichain = Vec::new();
    visit_antichains(&faces, 0, &mut antichain, &mut output)?;
    Ok(output)
}

fn visit_antichains(
    faces: &[usize],
    start: usize,
    antichain: &mut Vec<usize>,
    output: &mut BTreeSet<u64>,
) -> Result<()> {
    output.insert(downward_closure(antichain)?);
    for index in start..faces.len() {
        let face = faces[index];
        if antichain
            .iter()
            .all(|selected| face & selected != face && face & selected != *selected)
        {
            antichain.push(face);
            visit_antichains(faces, index + 1, antichain, output)?;
            antichain.pop();
        }
    }
    Ok(())
}

fn downward_closure(antichain: &[usize]) -> Result<u64> {
    let mut output = 0_u64;
    for face in antichain {
        let mut subset = *face;
        while subset != 0 {
            output |= 1_u64
                .checked_shl(u32::try_from(subset - 1).context("face index exceeds u32")?)
                .context("link mask exceeds u64")?;
            subset = (subset - 1) & face;
        }
    }
    Ok(output)
}

fn link_orbit_representatives(
    links: &BTreeSet<u64>,
    group: &PermutationGroup,
    vertices: usize,
) -> Result<Vec<u64>> {
    let mut seen = BTreeSet::<u64>::new();
    let mut output = Vec::new();
    for &link in links {
        if seen.contains(&link) {
            continue;
        }
        let mut orbit = BTreeSet::from([link]);
        let mut queue = VecDeque::from([link]);
        while let Some(current) = queue.pop_front() {
            for generator in group.generators() {
                let image = relabel_face_mask(current, generator, vertices)?;
                if orbit.insert(image) {
                    queue.push_back(image);
                }
            }
        }
        ensure!(
            orbit.iter().all(|image| links.contains(image)),
            "parent automorphism sent a link outside the parent"
        );
        seen.extend(orbit.iter().copied());
        output.push(*orbit.first().context("a link orbit is nonempty")?);
    }
    Ok(output)
}

fn relabel_face_mask(mask: u64, permutation: &Permutation, vertices: usize) -> Result<u64> {
    let mut output = 0_u64;
    for face in 1..(1_usize << vertices) {
        if mask & (1_u64 << (face - 1)) == 0 {
            continue;
        }
        let mut image = 0_usize;
        for vertex in 0..vertices {
            if face & (1_usize << vertex) != 0 {
                image |= 1_usize << permutation.apply(vertex as u32);
            }
        }
        output |= 1_u64 << (image - 1);
    }
    Ok(output)
}

fn attach_vertex(
    parent: &SimplicialComplex,
    link: u64,
    new_vertex: usize,
) -> Result<SimplicialComplex> {
    let new_vertex = u32::try_from(new_vertex).context("vertex id exceeds u32")?;
    let mut facets = parent
        .facets()
        .into_iter()
        .map(|facet| facet.vertices().to_vec())
        .collect::<Vec<_>>();
    let link_facets = maximal_faces(link, new_vertex as usize);
    if link_facets.is_empty() {
        facets.push(vec![new_vertex]);
    } else {
        facets.extend(link_facets.into_iter().map(|face| {
            let mut vertices = vertices_from_face(face, new_vertex as usize);
            vertices.push(new_vertex);
            vertices
        }));
    }
    Ok(SimplicialComplex::from_facets(&facets))
}

fn delete_vertex(complex: &SimplicialComplex, vertex: usize) -> SimplicialComplex {
    let vertex = vertex as u32;
    let facets = complex
        .facets()
        .into_iter()
        .map(|facet| {
            facet
                .vertices()
                .iter()
                .copied()
                .filter(|candidate| *candidate != vertex)
                .collect::<Vec<_>>()
        })
        .filter(|facet| !facet.is_empty())
        .collect::<Vec<_>>();
    SimplicialComplex::from_facets(&facets)
}

fn face_mask(complex: &SimplicialComplex, vertices: usize) -> Result<u64> {
    let mut output = 0_u64;
    for dimension in 0..=complex.dimension().max(0) as usize {
        for simplex in complex.simplices(dimension) {
            let mut face = 0_usize;
            for vertex in simplex.vertices() {
                ensure!((*vertex as usize) < vertices, "noncanonical vertex label");
                face |= 1_usize << *vertex;
            }
            output |= 1_u64
                .checked_shl(u32::try_from(face - 1).context("face index exceeds u32")?)
                .context("face mask exceeds u64")?;
        }
    }
    Ok(output)
}

fn complex_from_face_mask(mask: u64, vertices: usize) -> SimplicialComplex {
    let facets = maximal_faces(mask, vertices)
        .into_iter()
        .map(|face| vertices_from_face(face, vertices))
        .collect::<Vec<_>>();
    SimplicialComplex::from_facets(&facets)
}

fn maximal_faces(mask: u64, vertices: usize) -> Vec<usize> {
    (1..(1_usize << vertices))
        .filter(|face| {
            mask & (1_u64 << (face - 1)) != 0
                && (1..(1_usize << vertices)).all(|candidate| {
                    candidate & face != *face
                        || candidate == *face
                        || mask & (1_u64 << (candidate - 1)) == 0
                })
        })
        .collect()
}

fn vertices_from_face(face: usize, vertices: usize) -> Vec<u32> {
    (0..vertices)
        .filter(|vertex| face & (1_usize << vertex) != 0)
        .map(|vertex| vertex as u32)
        .collect()
}

fn is_downward_closed(mask: u64, vertices: usize) -> bool {
    for face in 1..(1_usize << vertices) {
        if mask & (1_u64 << (face - 1)) == 0 {
            continue;
        }
        let mut subset = (face - 1) & face;
        while subset != 0 {
            if mask & (1_u64 << (subset - 1)) == 0 {
                return false;
            }
            subset = (subset - 1) & face;
        }
    }
    true
}

fn insert_canonical(
    candidate: SimplicialComplex,
    representatives: &mut BTreeMap<SimplicialComplexKey, SimplicialComplex>,
) -> Result<()> {
    let canonical = candidate.canonical_form()?;
    let key = SimplicialComplexKey::from_canonical(&canonical);
    representatives.entry(key).or_insert(canonical);
    Ok(())
}

fn add_count(target: &mut u64, increment: usize, cap: u64, name: &'static str) -> Result<()> {
    *target = target
        .checked_add(u64::try_from(increment).context("counter increment exceeds u64")?)
        .with_context(|| format!("{name} counter overflow"))?;
    if *target > cap {
        bail!("{name} exceeded candidate cap {cap}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_and_reference_generators_agree_through_four_vertices() {
        let expected = [1, 1, 2, 5, 20];
        for (vertices, expected) in expected.into_iter().enumerate() {
            let (_, reference) = reference_representatives(vertices, 100_000).unwrap();
            let (_, orderly) = orderly_representatives(vertices, 100_000).unwrap();
            assert_eq!(reference.len(), expected);
            assert_eq!(orderly.len(), expected);
            assert!(reference.keys().eq(orderly.keys()));
        }
    }

    #[test]
    fn vertex_links_are_enumerated_as_subcomplexes() {
        let edge = SimplicialComplex::from_facets(&[vec![0, 1]]);
        let links = subcomplex_masks(face_mask(&edge, 2).unwrap(), 2).unwrap();
        assert_eq!(links.len(), 5);
        assert!(links.contains(&0));
    }

    #[test]
    fn fifth_shell_has_the_accepted_count() {
        let (_, representatives) = orderly_representatives(5, 1_000_000).unwrap();
        assert_eq!(representatives.len(), 180);
    }
}
