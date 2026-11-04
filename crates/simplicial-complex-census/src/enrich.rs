use std::collections::{BTreeSet, VecDeque};

use anyhow::{Result, ensure};
use noether::numthy::FiniteField;
use noether::{
    SemanticType,
    topology::{SimplicialCollapseSequence, SimplicialComplex, StrongCoreReduction},
};
use sha2::{Digest, Sha256};

use crate::{
    CohomologyCensus, RECORD_VERSION, TopologyCensus,
    records::{
        CohomologyBasisRecord, CohomologyOperationRecord, CohomologyProductRecord,
        CohomologyProductTermRecord, DominatedVertexRecord, ElementaryCollapseRecord,
        FiniteFieldContextRecord, FiniteMultiplicationTableCertificate,
        GreedyCollapseRelationRecord, IntegralHomologyRecord, RingExtentRecord,
        SimplicialComplexCohomologyRecord, SimplicialComplexIdentityRecord, SimplicialComplexKey,
        SimplicialComplexTopologyRecord, StrongCoreRelationRecord,
    },
};

pub fn enrich_cohomology(
    vertices: usize,
    prime: u64,
    identities: &[SimplicialComplexIdentityRecord],
    topology: &[SimplicialComplexTopologyRecord],
) -> Result<CohomologyCensus> {
    ensure!(
        identities.len() == topology.len(),
        "identity/topology row mismatch"
    );
    let field = FiniteField::prime(prime)?;
    let records = identities
        .iter()
        .zip(topology)
        .map(|(identity, topology)| {
            ensure!(identity.vertices == vertices, "mixed identity shell");
            ensure!(
                identity.key == topology.complex,
                "identity/topology key mismatch"
            );
            cohomology_record(identity, topology, &field)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(CohomologyCensus {
        vertices,
        prime,
        records,
    })
}

pub fn cohomology_record(
    identity: &SimplicialComplexIdentityRecord,
    topology: &SimplicialComplexTopologyRecord,
    field: &FiniteField,
) -> Result<SimplicialComplexCohomologyRecord> {
    identity.check_shape()?;
    ensure!(
        identity.key == topology.complex,
        "identity/topology key mismatch"
    );
    let core = topology.strong_core.target.reconstruct();
    let algebra = core.cohomology_ring(field.clone())?;
    let algebra_sha256 = hex::encode(Sha256::digest(algebra.canonical_bytes()?));
    let basis = algebra
        .basis()
        .iter()
        .map(|element| CohomologyBasisRecord {
            id: element.id().to_owned(),
            degree: element.degree(),
            cocycle_coefficients: element
                .cocycle()
                .values()
                .map(|value| {
                    value
                        .coefficients()
                        .into_iter()
                        .map(|coefficient| coefficient.to_string())
                        .collect()
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let product_terms = |terms: &[noether::topology::SimplicialCohomologyProductTerm]| {
        terms
            .iter()
            .map(|term| CohomologyProductTermRecord {
                basis_id: basis[term.basis()].id.clone(),
                coefficient: term
                    .coefficient()
                    .coefficients()
                    .into_iter()
                    .map(|coefficient| coefficient.to_string())
                    .collect(),
            })
            .collect::<Vec<_>>()
    };
    let products = algebra
        .products()
        .iter()
        .map(|product| CohomologyProductRecord {
            left_id: basis[product.left()].id.clone(),
            right_id: basis[product.right()].id.clone(),
            terms: product_terms(product.terms()),
        })
        .collect::<Vec<_>>();
    let operations = if field.characteristic().to_string() == "2" {
        algebra
            .basis()
            .iter()
            .enumerate()
            .flat_map(|(index, element)| {
                (0..=element.degree()).map(move |square| (index, element, square))
            })
            .map(|(index, element, square)| {
                let square = usize::try_from(square)?;
                Ok(CohomologyOperationRecord {
                    operation: format!("Sq^{square}"),
                    source_id: element.id().to_owned(),
                    target_terms: product_terms(&algebra.steenrod_square(square, index)?),
                    evidence: if square == 0 {
                        "steenrod-square-zero-identity/v1"
                    } else if u32::try_from(square)? == element.degree() {
                        "unstable-top-square-via-alexander-whitney-cup-product/v1"
                    } else {
                        "medina-mardones-subset-cup-i/v1"
                    }
                    .to_owned(),
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let connected_presentation_json = algebra
        .presentation()
        .map(|presentation| presentation.to_json())
        .transpose()?;
    let presentation_sha256 = algebra
        .presentation()
        .map(|presentation| {
            presentation
                .canonical_bytes()
                .map(|bytes| hex::encode(Sha256::digest(bytes)))
        })
        .transpose()?;
    let maximum_nonzero_degree = basis.iter().map(|basis| basis.degree).max().unwrap_or(0);
    let additive_dimensions = (0..=maximum_nonzero_degree)
        .map(|degree| basis.iter().filter(|basis| basis.degree == degree).count())
        .collect();

    Ok(SimplicialComplexCohomologyRecord {
        record_version: RECORD_VERSION,
        complex: identity.key.clone(),
        coefficient_context: FiniteFieldContextRecord {
            characteristic: field.characteristic().to_string(),
            extension_degree: field.degree(),
            defining_modulus: field.defining_modulus().map(<[u64]>::to_vec),
        },
        computed_on_strong_core: topology.strong_core.target.clone(),
        basis,
        products,
        operations,
        connected_presentation_json,
        extent: RingExtentRecord::CompletePresentation {
            certificate: FiniteMultiplicationTableCertificate {
                method: "finite-cochain-multiplication-table/v1".to_owned(),
                maximum_nonzero_degree,
                additive_dimensions,
                algebra_sha256,
                presentation_sha256,
                verified: true,
            },
        },
    })
}

pub fn enrich_topology(
    vertices: usize,
    identities: &[SimplicialComplexIdentityRecord],
) -> Result<TopologyCensus> {
    let records = identities
        .iter()
        .map(|identity| {
            ensure!(identity.vertices == vertices, "mixed identity shell");
            topology_record(identity)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(TopologyCensus { vertices, records })
}

pub fn topology_record(
    identity: &SimplicialComplexIdentityRecord,
) -> Result<SimplicialComplexTopologyRecord> {
    identity.check_shape()?;
    let complex = identity.key.reconstruct();
    let strong = complex.strong_core_reduction()?;
    strong.verify()?;
    let core = strong.core();
    let integral_homology = (0..degree_count(&complex))
        .map(|degree| {
            let group = core.homology_invariants(degree)?;
            Ok(IntegralHomologyRecord {
                degree,
                free_rank: group.free_rank(),
                torsion_coefficients: group
                    .torsion_coefficients()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let greedy = complex.greedy_collapse_reduction()?;
    greedy.verify()?;

    Ok(SimplicialComplexTopologyRecord {
        record_version: RECORD_VERSION,
        complex: identity.key.clone(),
        f_vector: (0..degree_count(&complex))
            .map(|dimension| complex.count(dimension))
            .collect(),
        component_sizes: component_sizes(&complex),
        euler_characteristic: complex.euler_characteristic(),
        integral_homology,
        strong_core: strong_core_record(&strong)?,
        greedy_collapse: greedy_collapse_record(&greedy)?,
    })
}

fn strong_core_record(reduction: &StrongCoreReduction) -> Result<StrongCoreRelationRecord> {
    let target = reduction.core().canonical_form()?;
    Ok(StrongCoreRelationRecord {
        target: SimplicialComplexKey::from_canonical(&target),
        source_vertices: reduction.source().count(0),
        target_vertices: reduction.core().count(0),
        steps: reduction
            .steps()
            .iter()
            .map(|step| DominatedVertexRecord {
                vertex: step.vertex(),
                dominator: step.dominator(),
            })
            .collect(),
        reduction_json: reduction.to_json()?,
        verified: true,
    })
}

fn greedy_collapse_record(
    sequence: &SimplicialCollapseSequence,
) -> Result<GreedyCollapseRelationRecord> {
    let target = sequence.target().canonical_form()?;
    Ok(GreedyCollapseRelationRecord {
        target: SimplicialComplexKey::from_canonical(&target),
        steps: sequence
            .steps()
            .iter()
            .map(|step| ElementaryCollapseRecord {
                free_face: step.free_face().to_vec(),
                coface: step.coface().to_vec(),
            })
            .collect(),
        sequence_json: sequence.to_json()?,
        terminal: sequence.is_terminal(),
        verified: true,
    })
}

pub(crate) fn component_sizes(complex: &SimplicialComplex) -> Vec<usize> {
    let vertices = complex
        .simplices(0)
        .iter()
        .map(|simplex| simplex.vertices()[0])
        .collect::<Vec<_>>();
    let edges = complex
        .simplices(1)
        .iter()
        .map(|simplex| (simplex.vertices()[0], simplex.vertices()[1]))
        .collect::<Vec<_>>();
    let mut unseen = vertices.iter().copied().collect::<BTreeSet<_>>();
    let mut output = Vec::new();
    while let Some(start) = unseen.first().copied() {
        unseen.remove(&start);
        let mut size = 0_usize;
        let mut queue = VecDeque::from([start]);
        while let Some(vertex) = queue.pop_front() {
            size += 1;
            for &(left, right) in &edges {
                let neighbor = if left == vertex {
                    Some(right)
                } else if right == vertex {
                    Some(left)
                } else {
                    None
                };
                if let Some(neighbor) = neighbor
                    && unseen.remove(&neighbor)
                {
                    queue.push_back(neighbor);
                }
            }
        }
        output.push(size);
    }
    output.sort_unstable();
    output
}

pub(crate) fn degree_count(complex: &SimplicialComplex) -> usize {
    usize::try_from(complex.dimension().saturating_add(1)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::SimplicialComplexIdentityRecord;

    #[test]
    fn topology_record_uses_the_strong_core_for_homology() {
        let disk = SimplicialComplex::from_facets(&[vec![0, 1, 2]])
            .canonical_form()
            .unwrap();
        let identity = SimplicialComplexIdentityRecord::from_canonical(&disk).unwrap();
        let record = topology_record(&identity).unwrap();
        assert_eq!(record.f_vector, vec![3, 3, 1]);
        assert_eq!(record.component_sizes, vec![3]);
        assert_eq!(record.strong_core.target_vertices, 1);
        assert_eq!(record.integral_homology.len(), 3);
        assert_eq!(record.integral_homology[0].free_rank, 1);
        assert!(
            record.integral_homology[1..]
                .iter()
                .all(|group| group.free_rank == 0 && group.torsion_coefficients.is_empty())
        );
        assert!(record.greedy_collapse.terminal);
    }

    #[test]
    fn disconnected_components_are_exact() {
        let value = SimplicialComplex::from_facets(&[vec![0, 1], vec![2], vec![3, 4, 5]]);
        assert_eq!(component_sizes(&value), vec![1, 2, 3]);
    }

    #[test]
    fn empty_complex_has_no_nonnegative_degrees() {
        let empty = SimplicialComplex::from_facets(&[]);
        let identity = SimplicialComplexIdentityRecord::from_canonical(&empty).unwrap();
        let record = topology_record(&identity).unwrap();
        assert!(record.f_vector.is_empty());
        assert!(record.component_sizes.is_empty());
        assert!(record.integral_homology.is_empty());
    }

    #[test]
    fn projective_plane_cohomology_records_all_supported_squares() {
        let projective_plane = SimplicialComplex::from_facets(&[
            vec![0, 1, 2],
            vec![0, 1, 3],
            vec![0, 2, 4],
            vec![0, 3, 5],
            vec![0, 4, 5],
            vec![1, 2, 5],
            vec![1, 3, 4],
            vec![1, 4, 5],
            vec![2, 3, 4],
            vec![2, 3, 5],
        ])
        .canonical_form()
        .unwrap();
        let identity = SimplicialComplexIdentityRecord::from_canonical(&projective_plane).unwrap();
        let topology = topology_record(&identity).unwrap();
        let field = FiniteField::prime(2_u8).unwrap();
        let record = cohomology_record(&identity, &topology, &field).unwrap();
        assert_eq!(
            record
                .basis
                .iter()
                .map(|basis| basis.degree)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(record.operations.len(), 6);
        let square_zero = record
            .operations
            .iter()
            .find(|operation| operation.operation == "Sq^0" && operation.source_id == "h1_0")
            .unwrap();
        assert_eq!(square_zero.target_terms[0].basis_id, "h1_0");
        let square_one = record
            .operations
            .iter()
            .find(|operation| operation.operation == "Sq^1" && operation.source_id == "h1_0")
            .unwrap();
        assert_eq!(square_one.target_terms[0].basis_id, "h2_0");
        assert_eq!(square_zero.evidence, "steenrod-square-zero-identity/v1");
        assert_eq!(
            square_one.evidence,
            "unstable-top-square-via-alexander-whitney-cup-product/v1"
        );
        assert!(record.connected_presentation_json.is_some());
        assert!(matches!(
            record.extent,
            RingExtentRecord::CompletePresentation { .. }
        ));
    }
}
