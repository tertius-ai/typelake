# Finite-group cohomology record

## Source identity

```text
GroupSourceFile {
  contract
  gap_version
  smallgrp_version
  maximum_order
  groups: [FiniteGroupSourceRecord]
}

FiniteGroupSourceRecord {
  lmfdb_label
  small_groups_order
  small_groups_index
  permutation_degree
  generators
}
```

The LMFDB label is an external name; its URL is derived from the label rather
than stored. Mathematical reconstruction uses the stored permutation
presentation. For the first source edition, the label and SmallGroups pair
agree as `order.index`. The builder derives `group_canonical_sha256` after
reconstruction and stores it in each cohomology record; it is deliberately
absent from the GAP-owned raw source row.

TypeBase joins a second checked GAP artifact,
`sources/standard-names-through-32.json`, keyed by the same LMFDB label. It
stores `StructureDescription(SmallGroup(order,index))` for display and search.
These familiar names are not promoted to mathematical identity and do not
change the immutable cohomology source hash.

## Modular record

The current release contract is `finite-group-cohomology/modular-v7`. The immutable
development contracts remain readable in place: `modular-v1` predates algebra
extraction, `modular-v2` adds exact algebra truncations but makes no
completeness claims, and `modular-v3` adds universal-Symonds completion
certificates to the resolution lane. Version 4 adds a separate complete
finite-abelian classifying-space lane. Version 5 adds the data-dependent
Symonds parameter certificate; each is a new contract rather than an in-place
schema mutation.

Version 6 retains the version-5 mathematical record shape and adds adaptive
proof-depth metadata to the artifact contract. The manifest gives the initial
and maximum permitted degrees plus the exact histogram of retained row depths.
Every record's basis, products, algebra, and certificate are recomputed at that
row's retained depth. A version-6 artifact cannot be emitted if even one row
remains `ExactThroughDegree` at the declared ceiling.

Version 7 retains those exact extents and adds typed primary-operation words
to the data boundary. Each resolution-lane algebra generator has a stable
presentation name, a mathematical display name, and—when available—a witness
that it is a nonzero scalar multiple of an admissible operation on an earlier
generator modulo a stored decomposable correction. The witness is replayable;
the display name is never inferred from typography alone.

```text
FiniteGroupModularCohomologyRecord {
  record_version
  group_label
  group_canonical_sha256
  coefficient_context: { characteristic, extension_degree = 1 }
  computation:
    | FiniteAbelianClassifyingSpace {
        certificate {
          method = "finite_abelian_classifying_space"
          invariant_factors
          presentation
          generators: [{ id, degree, display_name, origin }]
          operations_scope
          operations
          hilbert_function_through_degree
          hilbert_function
          noether_result_sha256
          verified = true
        }
      }
    | Resolution {
        resolution {
          model
          strategy
          computed_through_degree
          noether_result_sha256
          verified
        }
        basis: [{ id, degree, canonical_model_cocycle }]
        products: [
          { left_id, right_id, target_terms: [{ basis_id, coefficient }] }
        ]
        algebra {
          exact_through_degree
          generators: [{
            name
            display_name
            degree
            basis_id
            operation_witness?: {
              source_generator_index
              operation: {
                prime
                display_name
                square_sequence? | { bocksteins, reduced_powers }?
              }
              operation_coefficient
              decomposable_correction
            }
          }]
          relations: [{ degree, terms: [{ coefficient, exponents }] }]
          noether_result_sha256
          verified
        }
        extent: RingExtent
      }
}
```

For the first lane, Noether proves that the concrete permutation group is
abelian, reconstructs its canonical invariant factors, and computes the
complete `K(A,1)` presentation. The record stores the resulting
graded-commutative presentation directly, with generator origins and primary
operations. Its Hilbert function is an inspected prefix of a complete ring,
not the ring's extent. Replaying the certificate repeats both the concrete
group decomposition and the symbolic classifying-space calculation.

Development records store canonical cocycle coordinates relative to the
versioned Noether model: a minimal or bar resolution, or the stable subspace in
the Sylow-resolution cohomology. They bind those coordinates to the complete
result hash. The record stores the resolution model, strategy, computed degree, result hash, and verification flag. It does not contain serialized resolution certificates; replay requires the named producer.

In the resolution lane, `products` contains every ordered basis pair whose
target degree lies in the exact extent, including zero products. In the
finite-abelian lane, multiplication is carried by the verified complete
presentation and its canonical normal-form algorithm; the preserved
`modular-v3` artifact remains available when explicit degree-4 model cocycles
are wanted.

`algebra` records a deterministic minimal homogeneous generator
family and generators for every relation forced in the exact range. Its
quotient has the recorded Hilbert function only through
`exact_through_degree`; the record makes no assertion about higher degrees.
When the extent is `CompletePresentation`, the same quotient is a complete
presentation because the separate certificate has succeeded. The builder does
not duplicate the presentation under a second field.

## Extent

```text
ResolutionRingExtent =
  | CompletePresentation {
      certificate:
        | SymondsUniversal {
            method = "symonds_universal_bound"
            group_order
            generator_degree_bound = group_order - 1
            relation_degree_bound = 2 * generator_degree_bound
            computed_through_degree >= relation_degree_bound
            noether_result_sha256
            verified = true
          }
        | SymondsParameters {
            method = "symonds_parameter_criterion"
            group_order
            generator_degree_bound
            relation_degree_bound
            computed_through_degree
            parameter_generator_indices
            elementary_abelian_restrictions: [
              {
                permutation_degree
                generators
                rank
                restricted_parameters
                quotient_generator_power_bounds
              }
            ]
            quotient_generator_power_bounds
            maximum_module_generator_degree = N
            parameter_degree_bound = 1 + sum_i (degree(parameter_i) - 1)
            completion_degree_bound = max(N, parameter_degree_bound)
            noether_result_sha256
            verified = true
          }
    }
  | ExactThroughDegree {
      degree
      no_claim_above_degree: true
    }
```

The `FiniteAbelianClassifyingSpace` computation variant is itself a complete
presentation certificate and therefore does not carry a second `RingExtent`.
The resolution variant retains the extent union above.

Every certificate records numerical bounds, theorem hypotheses, and hashes of
the computations discharging them. A build cannot infer completeness from a
run of empty generator degrees.

For `SymondsParameters`, exact restrictions to every maximal elementary
abelian subgroup of a retained Sylow subgroup prove parameterhood in the true
cohomology ring. Finite-quotient witnesses prove that the approximation is
module-finite over those parameters. The verifier then checks the Symonds
stopping inequality against the exact computed range. Merely finding plausible
parameters in the truncated presentation is not sufficient.

## Integral record

The current contract is `finite-group-cohomology/integral-v4`. The immutable
`integral-v1`, `integral-v2`, and `integral-v3` checkpoints remain readable in
place.

```text
FiniteGroupIntegralCohomologyRecord {
  record_version
  group_label
  group_canonical_sha256
  presentation?: {
    coefficient_context = Z
    generators: [{ id, degree, display_name, origin }]
    relations: [{ terms: [{ coefficient, generator_exponents }] }]
  }
  truncation?: {
    resolution_strategy = "sparse_normalized_bar"
    computed_through_degree
    basis: [{
      id
      degree
      order?: integer
      canonical_model_cocycle: [{ coordinate, coefficient }]
    }]
    products: [{
      left_id
      right_id
      target_terms: [{ basis_id, coefficient }]
    }]
    noether_result_sha256
    verified = true
  }
  additive_groups_through_degree
  extent:
    | CompletePresentation { certificate }
    | ExactThroughDegree {
        degree
        no_claim_above_degree = true
        certificate:
          | UniversalLowDegree {
              method = "universal_low_degree_via_abelianization_ext"
              abelianization_invariant_factors
              noether_result_sha256
              verified = true
            }
          | SparseNormalizedBar {
              method = "sparse_normalized_bar_with_universal_unit_dmt"
              census_sparse_entry_limit
              noether_result_sha256
              verified = true
            }
      }
}
```

The cyclic-family presentation remains:

```text
presentation = {
  coefficient_context = Z
  generators = [{ id = u2_1, degree = 2,
                  display_name = beta_m(iota_1) }]
  relations = [m * u2_1]
}
extent = CompletePresentation {
  certificate = CyclicPeriodicResolution {
    period = 2
    odd_differential = g - 1
    even_differential = 1 + g + ... + g^(m-1)
    noether_result_sha256
    verified = true
  }
}
```

Version 2 also stores the complete rank-two finite-abelian family. If
`G = C_(n_1) x C_(n_2)` with `n_1 | n_2`, its generators have degrees
`|a|=|b|=2`, `|c|=3`: `a` and `b` are integral Bocksteins and `c` is the
integral Tor class. The relations include `n_1 a`, `n_2 b`, `n_1 c`, and the
homogeneous degree-six Tor-square relation. In the exceptional even case that
last relation is a sum of three monomials, which is why a relation contains a
list of terms rather than one coefficient/exponent pair. The certificate names
the finite-abelian classifying-space method, invariant factors, presentation
strategy, reconstruction hash, and [Roberts 2013, Theorem
6.3.1](https://uwspace.uwaterloo.ca/items/abb76e24-301d-45c4-9c0e-d78b8b92e471).

For the trivial group the generator and relation lists are empty. Every
complete record materializes the additive groups through the reader-facing
preview degree; that finite table is derived from the complete presentation
and is not its extent.

The integral manifest's `additive_through_degree` field describes that preview
bound for complete presentations. An `ExactThroughDegree` record stops at its
own certified degree, even when the manifest preview bound is larger.

Version 3 adds a record for every remaining finite group. For finite `G`, the
universal identities `H^0(BG; Z) = Z`, `H^1(BG; Z) = 0`, and
`H^2(BG; Z) = Ext(G_ab, Z)` determine the ring truncation through degree 2.
Noether computes `G_ab` exactly, stores its invariant factors and the resulting
additive groups, and hashes the verified truncation. Such a row has no
presentation and makes no claim in degree 3 or above. Thus absence and zero
are never conflated, while all 144 first-edition groups have a meaningful
integral route.

Version 4 computes beyond degree 2 when the normalized bar construction fits
the collection's declared sparse-work budget. In degree `d`, cocycle coordinate
`i` means the basis word `[g_1|...|g_d]` obtained by writing `i` in base
`|G|-1`. The alphabet consists of the nonidentity permutations in lexicographic
order of their zero-based image vectors. Zero coefficients are omitted and
the remaining integer coefficients are canonical decimal strings. Basis ids
are `h<degree>.<one-based-index>` in Noether's deterministic invariant-factor
basis; `order` is absent exactly for a free summand.

The product array contains every ordered pair of basis classes whose degree
sum lies in the exact range, including zero products and multiplication by the
degree-zero unit. A zero product has an empty `target_terms` array. Torsion
coordinates use the canonical interval `[0, order)`; free coordinates are
integers. The Noether result hash binds the group, sparse bar differentials,
lifted cocycles, and coordinate maps; the TypeLake artifact hash also binds
the derived product table. Validation independently recomputes that result and
requires the stored table to contain each eligible ordered pair exactly once.

The `integral-v4` manifest records `bar_entry_limit = 100000`; this is a
reproducible TypeLake census policy, not Noether's implementation ceiling. It
selects degree 4 for `6.1`, degree 3 for seven further groups, and degree 2 for
the other 90 non-family rows. The requested degree 16 remains the additive
preview range only for the 46 complete presentations.

Odd-degree integral squares obey graded commutativity and may be nonzero
2-torsion. The validator must not impose the odd-characteristic exterior rule
on integral records.
