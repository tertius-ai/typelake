# Finite simplicial-complex record

The collection contract is `finite-simplicial-complexes/v1`. Records are
staged so an expensive invariant can be added without rebuilding identities.

## Identity record

```text
SimplicialComplexIdentityRecord {
  record_version
  key: { facets }
  vertices
  dimension
  facets
  canonical_bytes_sha256
  automorphism_group: {
    degree
    order
    generators
    generators_verified
  }
}
```

`facets` is the defining object. It is lexicographically ordered, uses the
canonical vertex set `0,...,vertices-1`, and contains no redundant facet. The
key is the same canonical facet antichain, represented as structured integers
rather than a display string.

## Topology enrichment

```text
SimplicialComplexTopologyRecord {
  record_version
  complex
  f_vector
  component_sizes
  euler_characteristic
  integral_homology
  strong_core: {
    target
    source_vertices
    target_vertices
    steps
    reduction_json
    verified
  }
  greedy_collapse: {
    target
    steps
    sequence_json
    terminal
    verified
  }
}
```

The greedy-collapse target is evidence for one witnessed relation only. It is
not called a core and is not used as object identity.

## Cohomology enrichment

Field cohomology is computed on the stored strong core using Noether's exact
finite-dimensional cohomology algebra. A record contains:

```text
SimplicialComplexCohomologyRecord {
  record_version
  complex
  coefficient_context
  computed_on_strong_core
  basis: [{ id, degree, cocycle_coefficients }]
  products: [{ left_id, right_id, terms }]
  operations: [{ operation, source_id, target_terms, evidence }]
  connected_presentation_json?
  extent
}
```

Finite-field coefficients use power-basis coefficient vectors relative to the
stored field context. They are exact data, not display strings. The first
edition stores complete `F2` and `F3` multiplication tables. The version-two
characteristic-two enrichment stores every `Sq^k(x)` for
`0 <= k <= degree(x)`, including zero values, computed on the canonical
cocycle. The evidence distinguishes the square-zero identity, the unstable top
square computed as a cup product, and genuinely lower squares computed by
Noether's normalized Medina--Mardones cup-i formula. Squares beyond the
unstable range vanish formally and are not redundantly stored.

For a connected complex, `connected_presentation_json` is Noether's exact
graded-commutative quotient presentation. For a disconnected complex, the
complete degree-zero multiplication table is retained instead of pretending
that `H^0` is the coefficient field. Every record has a
`CompletePresentation` extent with a finite-cochain multiplication-table
certificate and canonical algebra/presentation digests. Enrichment records
are keyed by `(complex, coefficient_context)`.

The installed `F2` v2 batch is complete through six vertices. Its order-six
artifact contains 16,143 records (95,241,294 bytes) with SHA-256
`22ac47fac3ad2031799296ad7f73aa226bd15f83fdbecfd6b4daaa99de5ea031`.
The manifest links the first-edition topology artifact revision and the exact
Noether cup-i revision separately. `validate-cohomology` reloads those hashes
and recomputes every stored ring and operation value.
