use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use noether::{
    Integer,
    combin::{BeatPointKind, Poset, PosetCanonicalLabel},
    groups::PermutationGroup,
    homalg::FinitelyGeneratedAbelianGroup,
    topology::SimplicialComplex,
};
use sha2::{Digest, Sha256};

use crate::{
    RECORD_VERSION,
    records::{
        AutomorphismGroupRecord, BeatPointKindRecord, BeatPointRecord, ComplexKey, ComplexRecord,
        CoreRelation, IntegralHomologyRecord, OppositeRelation, OrderComplexRelation,
        PosetInvariants, PosetKey, PosetRecord, PosetRelations,
    },
};

pub fn enrich_poset(poset: &Poset) -> Result<(PosetRecord, ComplexRecord)> {
    let canonical = poset.canonical_label()?;
    if !canonical.verify(poset) {
        bail!("Noether returned unverifiable canonical-label evidence");
    }
    let canonical_poset = canonical.poset();
    let canonical = if canonical_poset == poset {
        canonical
    } else {
        canonical_poset.canonical_label()?
    };
    enrich_poset_with_core_cache(&canonical, &mut BTreeMap::new())
}

pub(crate) fn enrich_poset_with_core_cache(
    canonical: &PosetCanonicalLabel,
    core_homology: &mut BTreeMap<PosetKey, Vec<FinitelyGeneratedAbelianGroup>>,
) -> Result<(PosetRecord, ComplexRecord)> {
    let poset = canonical.poset();
    if !canonical.verify(poset) {
        bail!("enrichment requires evidence computed on the canonical representative");
    }
    let key = PosetKey::from_canonical_label(canonical);

    let covers = poset
        .covers()
        .edges()
        .iter()
        .map(|edge| [edge.source, edge.target])
        .collect::<Vec<_>>();
    let components = poset.connected_components();
    let mut component_sizes = components.iter().map(Vec::len).collect::<Vec<_>>();
    component_sizes.sort_unstable();

    let invariants = PosetInvariants {
        cover_count: covers.len(),
        strict_comparable_pair_count: poset.comparable_pair_count(),
        strict_lower_counts: poset.strict_lower_counts(),
        strict_upper_counts: poset.strict_upper_counts(),
        minimal_count: poset.minimal_elements().len(),
        maximal_count: poset.maximal_elements().len(),
        component_count: components.len(),
        component_sizes,
        is_connected: poset.order() > 0 && components.len() == 1,
        height: poset.height(),
        width: poset.width(),
        is_lattice: poset.is_lattice(),
    };

    let automorphism_group = automorphism_record(canonical.automorphism_group())?;
    let source_complex = poset.order_complex()?;
    let (has_core, homology) = if poset.is_core() {
        let homology = if let Some(groups) = core_homology.get(&key) {
            groups.clone()
        } else {
            let groups = source_complex
                .graded_homology_invariants()
                .context("computing integral homology of a core order complex")?;
            core_homology.insert(key.clone(), groups.clone());
            groups
        };
        let order_u32 = u32::try_from(poset.order()).context("poset order exceeds u32")?;
        (
            CoreRelation {
                target: key.clone(),
                core_size: poset.order(),
                is_core: true,
                deletions: Vec::new(),
                retained_vertices: (0..order_u32).collect(),
                canonical_labeling: (0..order_u32).collect(),
                reduction_verified: true,
            },
            homology,
        )
    } else {
        let core = poset.core()?;
        let core_key = labeled_incidence_key(core.core())?;
        let homology = if let Some(groups) = core_homology.get(&core_key) {
            groups.clone()
        } else {
            let groups = core
                .graded_homology_invariants()
                .context("computing integral homology from the canonical core")?;
            core_homology.insert(core_key.clone(), groups.clone());
            groups
        };
        let relation = CoreRelation {
            target: core_key,
            core_size: core.core().order(),
            is_core: false,
            deletions: core
                .deletions()
                .iter()
                .map(|deletion| BeatPointRecord {
                    element: deletion.element(),
                    kind: match deletion.kind() {
                        BeatPointKind::Down => BeatPointKindRecord::Down,
                        BeatPointKind::Up => BeatPointKindRecord::Up,
                    },
                    witness: deletion.witness(),
                })
                .collect(),
            retained_vertices: core.retained_vertices().to_vec(),
            canonical_labeling: core.canonical_labeling().images().to_vec(),
            reduction_verified: core.verify(poset),
        };
        (relation, homology)
    };

    let opposite_key = PosetKey::from_canonical_label(&poset.opposite()?.canonical_label()?);
    let opposite = OppositeRelation {
        is_self_dual: opposite_key == key,
        target: opposite_key,
    };

    let complex = complex_record_with_homology(poset.order(), &source_complex, &homology)?;
    let record = PosetRecord {
        record_version: RECORD_VERSION,
        key,
        order: poset.order(),
        covers,
        invariants,
        automorphism_group,
        relations: PosetRelations {
            has_core,
            opposite,
            order_complex: OrderComplexRelation {
                target: complex.key.clone(),
            },
        },
    };
    record.check_shape()?;
    Ok((record, complex))
}

/// Encode principal downsets in the current labeling.
///
/// The caller supplies a poset already known to be canonical. Keeping this
/// record-layer encoding local avoids rerunning canonical search merely to
/// serialize evidence that Noether's core reduction has already established.
fn labeled_incidence_key(poset: &Poset) -> Result<PosetKey> {
    let powers = (0..poset.order())
        .map(|exponent| {
            u32::try_from(exponent)
                .map(|exponent| Integer::from(1_u8) << exponent)
                .context("poset order exceeds principal-downset exponent range")
        })
        .collect::<Result<Vec<_>>>()?;
    let mut entries = Vec::with_capacity(poset.order());
    for upper in 0..poset.order() {
        let mut downset = Integer::from(0_u8);
        for (lower, power) in powers.iter().enumerate() {
            if poset.leq(lower as u32, upper as u32)? {
                downset += power;
            }
        }
        entries.push(downset.to_string());
    }
    PosetKey::new(entries)
}

fn automorphism_record(group: &PermutationGroup) -> Result<AutomorphismGroupRecord> {
    let generators = group
        .generators()
        .iter()
        .filter(|generator| !generator.is_identity())
        .map(|generator| generator.images().to_vec())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let mut seen = vec![false; group.degree()];
    let mut orbits = Vec::new();
    for point in 0..group.degree() {
        if seen[point] {
            continue;
        }
        let mut orbit = group.orbit(point as u32)?;
        orbit.sort_unstable();
        for &member in &orbit {
            seen[member as usize] = true;
        }
        orbits.push(orbit);
    }
    orbits.sort_by_key(|orbit| orbit.first().copied());

    Ok(AutomorphismGroupRecord {
        degree: group.degree(),
        generators,
        order: group.order().to_decimal(),
        is_transitive: group.degree() > 0 && orbits.len() == 1,
        orbits,
        generators_verified: true,
        abstract_group: None,
    })
}

pub fn complex_record(vertex_count: usize, complex: &SimplicialComplex) -> Result<ComplexRecord> {
    let homology = complex
        .graded_homology_invariants()
        .context("computing integral homology in every degree")?;
    complex_record_with_homology(vertex_count, complex, &homology)
}

fn complex_record_with_homology(
    vertex_count: usize,
    complex: &SimplicialComplex,
    homology: &[FinitelyGeneratedAbelianGroup],
) -> Result<ComplexRecord> {
    let facets = complex
        .facets()
        .into_iter()
        .map(|facet| facet.vertices().to_vec())
        .collect::<Vec<_>>();
    let key = complex_key(vertex_count, &facets)?;
    let dimension = complex.dimension();
    let f_vector = if dimension < 0 {
        Vec::new()
    } else {
        (0..=dimension as usize)
            .map(|degree| complex.count(degree))
            .collect()
    };
    let degree_count = usize::try_from(dimension.saturating_add(1))
        .context("order-complex dimension does not fit usize")?;
    ensure!(
        homology.len() <= degree_count,
        "core homology has degrees above the source order complex"
    );
    let integral_homology = (0..degree_count)
        .map(|degree| {
            homology.get(degree).map_or(
                IntegralHomologyRecord {
                    degree,
                    free_rank: 0,
                    invariant_factors: Vec::new(),
                },
                |group| IntegralHomologyRecord {
                    degree,
                    free_rank: group.free_rank(),
                    invariant_factors: group
                        .torsion_coefficients()
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                },
            )
        })
        .collect();

    Ok(ComplexRecord {
        record_version: RECORD_VERSION,
        key,
        vertex_count,
        facets,
        is_empty: complex.is_empty(),
        dimension,
        f_vector,
        euler_characteristic: complex.euler_characteristic(),
        integral_homology,
    })
}

pub fn complex_key(vertex_count: usize, facets: &[Vec<u32>]) -> Result<ComplexKey> {
    ensure!(
        (vertex_count == 0) == facets.is_empty(),
        "the order-complex facet list is empty exactly at vertex count zero"
    );
    ensure!(
        facets.windows(2).all(|pair| pair[0] < pair[1]),
        "order-complex facets are not uniquely lexicographically sorted"
    );
    let mut represented = vec![false; vertex_count];
    for facet in facets {
        ensure!(!facet.is_empty(), "order-complex facets must be nonempty");
        ensure!(
            facet.windows(2).all(|pair| pair[0] < pair[1]),
            "vertices within an order-complex facet must be strictly increasing"
        );
        for &vertex in facet {
            let slot = represented
                .get_mut(vertex as usize)
                .context("order-complex facet vertex is out of range")?;
            *slot = true;
        }
    }
    ensure!(
        represented.into_iter().all(|present| present),
        "every inherited order-complex vertex must occur in a facet"
    );
    for (index, facet) in facets.iter().enumerate() {
        for other in facets.iter().skip(index + 1) {
            ensure!(
                !facet
                    .iter()
                    .all(|vertex| other.binary_search(vertex).is_ok())
                    && !other
                        .iter()
                        .all(|vertex| facet.binary_search(vertex).is_ok()),
                "an order-complex facet is a proper subset of another facet"
            );
        }
    }

    let vertex_count = u64::try_from(vertex_count).context("complex vertex count exceeds u64")?;
    let facet_count = u64::try_from(facets.len()).context("complex facet count exceeds u64")?;
    let mut digest = Sha256::new();
    digest.update(b"finite-posets/order-complex");
    digest.update([0]);
    digest.update(1_u32.to_be_bytes());
    digest.update(vertex_count.to_be_bytes());
    digest.update(facet_count.to_be_bytes());
    for facet in facets {
        digest.update(
            u64::try_from(facet.len())
                .context("complex facet length exceeds u64")?
                .to_be_bytes(),
        );
        for &vertex in facet {
            digest.update(vertex.to_be_bytes());
        }
    }
    Ok(ComplexKey::from_digest(digest.finalize().into()))
}
