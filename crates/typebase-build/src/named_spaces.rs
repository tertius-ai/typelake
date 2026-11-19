//! Standard names certified by exact presentations, never by homology alone.

use std::collections::BTreeMap;

use anyhow::Result;
use noether::topology::{SimplicialFace, SimplicialSet};
use simplicial_set_census::records::SimplicialSetKey;

fn sphere_names(dimension: usize) -> Vec<String> {
    let mut names = vec![
        format!("{dimension}-sphere"),
        format!("S{dimension}"),
        format!("S^{dimension}"),
        "sphere".into(),
    ];
    if dimension == 1 {
        names.push("circle".into());
    }
    names
}

pub(crate) fn simplicial_sets() -> Result<BTreeMap<SimplicialSetKey, Vec<String>>> {
    let mut names = BTreeMap::new();
    names.insert(
        SimplicialSetKey::from_value(&SimplicialSet::new(vec![])?),
        vec!["empty space".into()],
    );
    names.insert(
        SimplicialSetKey::from_value(&SimplicialSet::new(vec![vec![vec![]]])?),
        vec!["point".into(), "*".into()],
    );
    for dimension in 0..=3 {
        names.insert(
            SimplicialSetKey::from_value(&SimplicialSet::sphere(dimension).canonical_form()?),
            sphere_names(dimension),
        );
    }
    let interval = SimplicialSet::new(vec![
        vec![vec![], vec![]],
        vec![vec![SimplicialFace::Cell(1), SimplicialFace::Cell(0)]],
    ])?;
    names.insert(
        SimplicialSetKey::from_value(&interval.canonical_form()?),
        vec!["interval".into(), "1-simplex".into(), "Delta1".into()],
    );
    for loops in 2..=6 {
        let bouquet = SimplicialSet::new(vec![
            vec![vec![]],
            vec![vec![SimplicialFace::Cell(0), SimplicialFace::Cell(0)]; loops],
        ])?;
        names.insert(
            SimplicialSetKey::from_value(&bouquet.canonical_form()?),
            vec![
                format!("wedge of {loops} circles"),
                format!("bouquet of {loops} circles"),
            ],
        );
    }
    Ok(names)
}

pub(crate) fn simplicial_complex(vertices: usize, facets: &[Vec<u32>]) -> Vec<String> {
    if vertices == 0 {
        return vec!["empty space".into()];
    }
    if facets.len() == 1 && facets[0].len() == vertices {
        let mut names = vec![
            format!("{}-simplex", vertices - 1),
            format!("Delta{}", vertices - 1),
        ];
        if vertices == 1 {
            names.push("point".into());
        }
        if vertices == 2 {
            names.push("interval".into());
        }
        return names;
    }
    if vertices >= 2
        && facets.len() == vertices
        && facets.iter().all(|facet| facet.len() == vertices - 1)
    {
        let mut names = sphere_names(vertices - 2);
        names.push(format!("boundary of the {}-simplex", vertices - 1));
        return names;
    }
    // A finite connected simple graph with every vertex of degree two is a
    // polygon, hence a specified triangulation of the circle.
    if facets.len() == vertices && facets.iter().all(|facet| facet.len() == 2) {
        let mut adjacency = vec![Vec::new(); vertices];
        for edge in facets {
            adjacency[edge[0] as usize].push(edge[1] as usize);
            adjacency[edge[1] as usize].push(edge[0] as usize);
        }
        let mut seen = vec![false; vertices];
        let mut stack = vec![0];
        while let Some(vertex) = stack.pop() {
            if !seen[vertex] {
                seen[vertex] = true;
                stack.extend(&adjacency[vertex]);
            }
        }
        if adjacency.iter().all(|neighbors| neighbors.len() == 2)
            && seen.into_iter().all(|value| value)
        {
            return sphere_names(1);
        }
    }
    if facets.len() == vertices && facets.iter().all(|facet| facet.len() == 1) {
        return vec![format!("{vertices} discrete points")];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minimal_circle_is_named_by_exact_key() {
        let names = simplicial_sets().unwrap();
        assert!(
            names
                [&SimplicialSetKey::from_value(&SimplicialSet::circle().canonical_form().unwrap())]
                .contains(&"circle".to_string())
        );
    }
    #[test]
    fn a_disconnected_pair_of_cycles_is_not_a_circle() {
        assert!(
            simplicial_complex(
                6,
                &[
                    vec![0, 1],
                    vec![1, 2],
                    vec![0, 2],
                    vec![3, 4],
                    vec![4, 5],
                    vec![3, 5]
                ]
            )
            .is_empty()
        );
        assert!(
            simplicial_complex(4, &[vec![0, 1], vec![1, 2], vec![2, 3], vec![0, 3]])
                .contains(&"circle".to_string())
        );
    }
}
