# Eilenberg--Mac Lane record

## Object identity

```text
EilenbergMacLaneRecord {
  record_version = 8
  label                         # `8.2.2` or `Z.3`
  group {
    kind                        # finite | infinite_cyclic
    invariant_factors           # canonical d1 | ... | dr
    free_rank
    order?                      # finite only
    lmfdb_label?
    lmfdb_url?
    gap_structure_description?
  }
  delooping_degree
  space_canonical_sha256
  integral_homology: InvariantCoverage<IntegralHomologyRecord>
  integral_cohomology: InvariantCoverage<IntegralCohomologyRecord>
  integral_cohomology_ring: InvariantCoverage<IntegralCohomologyRingRecord>
  modular_cohomology: [{ prime, coverage }]
}
```

The stored identity is the canonical finitely generated abelian group and
positive delooping degree. GAP descriptions and LMFDB labels are external
metadata. The Noether semantic hash binds the record to the exact symbolic
space without making a finite simplicial presentation.

## Coverage

```text
InvariantCoverage<T> =
  | Available { value: T }
  | Pending {
      operation
      context
      detail
      no_claim: true
    }
```

`Pending` is a first-class development state, never an empty group, zero ring,
or implied vanishing statement. Release completeness is computed from the
records and cannot be hand-entered into the manifest.

## Additive integral homology

```text
IntegralHomologyRecord {
  exact_through_degree
  groups: [
    { degree, free_rank, torsion_coefficients }
  ]
  strategy
  noether_result_sha256
  verified = true
}
```

There is exactly one canonical group row in every degree from zero through the
inclusive bound. The first native strategies are
`classifying_space_kunneth`, `free_abelian_second_delooping`, and
`trivial_space`. Later effective-homology strategies extend this enum by a new
record contract rather than changing old rows.

## Additive integral cohomology

Version 2 separates additive integral cohomology from a future ring record:

```text
IntegralCohomologyRecord {
  exact_through_degree
  groups: [{ degree, free_rank, torsion_coefficients }]
  method = "integral_universal_coefficient"
  source_homology_sha256
  noether_result_sha256
  verified = true
}
```

The preserved `invariants-v1` contract has this cell pending. Version 2 adds
the additive result but still makes no cup-product claim.

## Complete integral ring records

Version 7 adds a separate cell so an additive universal-coefficient result is
never mistaken for a cup-product computation; Version 8 extends the same
record shape with the rank-two finite-abelian closed form:

```text
IntegralCohomologyRingRecord {
  coefficient = "Z"
  graded_commutative = true
  presentation_extent = "complete_closed_form"
  generators: [{ id, degree, display_name, origin }]
  relations: [{ terms: [{ coefficient, generator_exponents }] }]
  additive_groups_through_degree
  additive_groups: [{ degree, free_rank, torsion_coefficients }]
  strategy
  noether_result_sha256
  verified = true
}
```

The closed-form strategies cover a point, `K(Z^r,1)`,
`K(Z^r + C_m,1)`, `K(Z^r + C_m + C_n,1)` for `m | n`, and `K(Z^r,2)`.
Origins distinguish free fundamental classes, integral Bocksteins,
rank-two cyclic Tor classes, and second-delooping fundamental classes.
Relations are sparse homogeneous integer polynomials in generator order.
Degree-one generators have explicit square-zero relations: graded
commutativity over `Z` by itself would imply only `2 x^2 = 0`. The rank-two
Tor-square relation likewise retains its exceptional decomposable terms when
both invariant factors are even and their quotient is odd. The additive
preview is independently compared with the bounded universal-coefficient
record during replay.

## Prime-field ring records

Version 3 fills a modular cell whenever Noether has a complete closed-form
presentation:

```text
ModularCohomologyRecord {
  coefficient
  presentation_extent = complete_closed_form
  presentation                    # canonical Noether graded presentation JSON
  generators: [
    { id, degree, display_name, origin }
  ]
  operations_scope = primary_operations_on_presentation_generators
  operations: [
    { operation, source_generator, target }
  ]
  hilbert_function_through_degree
  hilbert_function
  strategy
  noether_result_sha256
  verified = true
}
```

Origins distinguish fundamental classes, Bocksteins, and periodic cyclic
classes. Operation targets are canonical normal forms in the stored
presentation. `complete_closed_form` qualifies the ring presentation; the
separate operation scope deliberately does not claim that the short generator
table lists every unstable operation on every algebra element.

Version 4 also fills a modular cell when Noether has a theorem presentation
that is exact only through the collection degree:

```text
ModularCohomologyRecord {
  coefficient
  presentation_extent = ExactThroughDegree {
    degree
    no_claim_above_degree = true
  }
  presentation
  generators: [
    {
      id
      degree
      display_name
      origin = AdmissibleSteenrodSquare {
        component
        sequence
        fundamental_class_lifts_integrally
      }
    }
  ]
  operations_scope = generator_origins_only
  operations = []
  hilbert_function_through_degree
  hilbert_function
  strategy = cartan_serre_admissible_squares
  noether_result_sha256
  verified = true
}
```

Version 5 adds the odd-primary origin

```text
AdmissibleOddPrimaryOperation {
  component
  bocksteins                    # e_0,...,e_k in {0,1}
  reduced_powers                # s_1,...,s_k
  fundamental_class_lifts_integrally
}
```

and the bounded strategy
`cartan_serre_admissible_odd_primary_operations`. The paired arrays encode the
word `beta^e_0 P^s_1 beta^e_1 ... P^s_k beta^e_k`; replay verifies
admissibility and excludes a terminal Bockstein when the fundamental class
lifts integrally.

Version 6 adds higher-primary cyclic summands. Their additional origins are

```text
HigherBockstein { component, primary_exponent }

AdmissibleSteenrodSquareOnHigherPrimaryClass {
  component
  primary_exponent
  source_is_higher_bockstein
  sequence
}

AdmissibleOddPrimaryOperationOnHigherPrimaryClass {
  component
  primary_exponent
  source_is_higher_bockstein
  bocksteins
  reduced_powers
}
```

Here `primary_exponent` is the exponent `r > 1` in `p^r`, and the source flag
distinguishes operations on the fundamental class from operations on its
higher Bockstein. Both families exclude a terminal primary Bockstein.

The stored polynomial presentation contains exactly the algebra generators
that can affect degrees through the bound. Products in those degrees and the
recorded Hilbert function are exact. It is not presented as a finite
description of the generally infinitely generated full ring. The admissible
word in each generator origin is mathematical naming evidence, not a claim to
a separately evaluated table of all Steenrod operations.

## Future effective-model ring records

Integral and prime-field cohomology reuse the graded-commutative presentation
shape from the finite-group-cohomology collection when chain-level effective
models become available:

```text
CohomologyRecord {
  coefficient_context
  exact_through_degree
  additive_groups_or_dimensions
  canonical_basis_and_cocycles
  products
  algebra_generators
  homogeneous_relations
  operations
  names
  extent: ExactThroughDegree { degree, no_claim_above_degree: true }
  noether_result_sha256
  verified = true
}
```

The fundamental class has stable display name `iota_n`. A derived name stores
the normalized operation word, target canonical class, and evidence hash.
Names are metadata and never replace canonical class ids.

## Manifest

The immutable artifact contains canonical JSONL records and a manifest with:

- collection and source contracts;
- exact Noether revision and source SHA256;
- group, delooping, and total-degree bounds;
- object count and per-invariant available/pending counts;
- artifact byte count, row count, and SHA256;
- independent validation booleans and parity-oracle versions.
