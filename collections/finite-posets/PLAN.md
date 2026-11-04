# Finite posets: census plan

**Status:** working plan, updated 2026-08-26
**Active product:** a certified finite-Poset census and its first TypeBase
Volume I projection.
**Initial target:** finite posets up to isomorphism, with exact homotopy cores,
automorphism actions, and integral homology.

This census is intended to become the first TypeLake collection, but creating
and validating the mathematics does not depend on TypeLake. Existing storage
and schema prototypes may inform a later adapter; they do not constrain the
census records or the computation.

The mathematical census remains independent of serving infrastructure. The
current effort additionally builds a frozen TypeBase label map, static Parquet
projection, and static browser. Remote publication, MCP, DOI registration, and
long-term operational policy remain later work.

## 1. Outcomes

For every completed order `n`, the census will contain exactly one
canonical representative of every finite poset of order `n`. Every such poset
will have:

- its canonical Hasse diagram;
- its canonical principal-downset encoding;
- its natural automorphism group as a subgroup of `S_n`, with exact generators
  and order;
- its canonical Stong core and a `has-core` relation to that core;
- its complete order complex as a linked simplicial-complex record, carrying
  integral homology including torsion;
- enough inexpensive structural invariants to validate, organize, and later
  query the collection;
- exact run metadata identifying the Noether revision, census-builder revision,
  record version, parameters, and validation results.

The records make the homotopy census a deterministic grouping by canonical
core, not a second mathematical object type. For every isomorphism stratum known
completely through order `N`, grouping by canonical core gives a complete
classification by finite-space homotopy type through order `N`.

The target suggested by the current architecture is completeness through
order 10. This is a milestone, not an assumption built into the code. We will
advance one order at a time only after the preceding order clears its count,
symmetry, core, topology, and reproducibility gates.

### Active five-step plan

1. **Complete for the reference corpus:** finish canonicalization, symmetry,
   cores, order complexes, lean graded homology, and core-first Poset homology
   in Noether. A Poset-specific DMT remains optional and benchmark-driven.
2. **Complete:** build and validate the redundant `poset-census` reference
   generator, enricher, and versioned JSONL emitter through order 4.
3. **Complete through order 7:** build the hereditary orderly generator, prove
   equality with the reference `PosetKey` sets, and add the core-first Noether
   homology playbook.
4. **Complete through order 10:** deterministic sharding, resumable
   checkpoints, merge validation, full per-shard reconstruction, and atomic
   publication produced 2,567,284 certified order-10 rows.
5. **Current:** assign frozen `a.b.c` labels, add independently evidenced
   abstract automorphism-group names, build the static Parquet edition, and
   verify the museum-style browser against bounded exact-label reads.

### Explicit non-goals

This project does not currently include:

- TypeLake ingestion or serving;
- a TypeLake collection contract or physical lake layout;
- an MCP or query API;
- an atomic remote append protocol;
- a mutable server-side search database;
- a generic workflow framework;
- poset enumeration inside Noether;
- executing submitted reconstruction snippets;
- claiming complete homotopy coverage beyond the completed isomorphism range.

## 2. Working decisions

These decisions govern the census implementation. The later TypeLake adapter
must preserve them losslessly but does not participate in defining them.

### 2.1 Mathematical code belongs in Noether

Noether owns reusable mathematics on a finite poset: canonicalization,
automorphisms, cores, order complexes, homology routing, and structural
invariants. Its Rust facade is authoritative and its Python API remains a thin
projection.

The census builder owns enumeration, canonical augmentation, sharding,
checkpoints, record construction, validation against external counts, and
optional GAP identification. Those operations are specific to building this
corpus and do not belong in Noether.

### 2.2 Census code belongs in this repository

The code consuming Noether will live in `typelake`, initially as a Rust crate
under `crates/poset-census/`. The crate has no TypeLake storage or schema
dependency. It owns enumeration, enrichment orchestration, versioned census
records, run metadata, and validation reports. Keeping it beside the plan and
small fixtures makes a census release traceable to one repository revision
without adding corpus-specific machinery to Noether.

### 2.3 The generated corpus contains all isomorphism classes

The earlier architecture proposed storing only connected posets as primary
rows and deriving disconnected posets by multiset composition. For the first
collection we instead begin with every canonical isomorphism class as a real
generated object.

The projected saving from connected-only normalization is small compared with
the additional identity, relation, provenance, and view-building complexity.
It would also make a nominally complete object catalogue depend on a derivation
layer we are intentionally not building yet. Connected-component decomposition
will still be computed, and the generated corpus will let us measure the choice
before freezing larger census orders.

### 2.4 A core is a poset, not a second object type

The core of `P` is canonicalized with the same poset canonical form and has the
same `PosetKey` as the corresponding Poset object in the isomorphism
catalogue. Each `P` has a uniform `P --has-core--> core(P)` relation; a core may
point to itself. The homotopy grouping selects distinct core keys and aggregates the
posets that point to each one.

### 2.5 The natural automorphism action is generally intransitive

`Aut(P)` will be stored faithfully as a permutation group on the vertices of
`P`. An order-preserving automorphism preserves the number of strict
predecessors of every vertex, so a transitive natural action is possible only
for an antichain. Transitive-action labels are consequently not part of the
census record.

Optional external names must say what they identify:

- a `SmallGroup(order, id)` label may identify the abstract group when the GAP
  library covers it;
- an LMFDB abstract-group label may identify that same underlying abstract
  group;
- neither label is part of poset identity.

### 2.6 Conventions are explicit

The phrase “height of a vertex” in the canonicalization work means

```text
strict_lower_count(x) = |{y : y < x}|.
```

The collection field `height` means the number of elements in a longest chain.
These are different invariants and will have different names. Homology means
unreduced simplicial homology of the order complex unless a record declares
another convention.

### 2.7 Dual and opposite

We use both standard terms with distinct API roles. `opposite(P)` constructs the
poset with its order reversed, and `opposite` is the relation from `P` to that
canonical poset. `dual` describes the corresponding equivalence property, as in
`is_self_dual`. Thus `P` is self-dual exactly when it is isomorphic to its
opposite. We do not store separate `dual` and `opposite` relations for the same
mathematical reversal.

## 3. Repository and local data layout

The proposed tracked layout is:

```text
typelake/
  collections/
    finite-posets/
      PLAN.md
      RECORD.md                census record v1 and mathematical conventions
      validation/
        counts.json            accepted aggregate counts and sources
        cases/                 small hand-checkable posets
  crates/
    poset-census/
      Cargo.toml
      src/
        main.rs
        enumerate.rs
        augment.rs
        enrich.rs
        records.rs
        validate.rs
        jsonl.rs
      tests/
```

Bulk and intermediate data do not belong in Git. Commands will require an
explicit `POSET_CENSUS_DATA_ROOT`; there will be no hidden large default inside
the repository. The local layout describes the computation itself rather than
anticipating a future lake layout:

```text
$POSET_CENSUS_DATA_ROOT/
  work/finite-posets/<run-id>/
    run.json
    checkpoints/
    shards/
    merged/order=<n>/
    logs/
  census/record-version=1/order=<n>/
    posets.jsonl
    simplicial-complexes.jsonl
    validation.json
    run.json
```

`work/` is replaceable computation state. A completed `census/` order is
immutable once its run and validation reports have been written. The generator
must be able to reproduce those files byte for byte.

The first durable format is sorted JSON Lines: transparent, diffable at small
orders, and independent of a database design. A later TypeLake adapter may
convert the records to Parquet without changing their mathematical contents.

## 4. Identity and logical records

### 4.1 Census identity

For a labeling `0, ..., n-1`, the incidence number of vertex `k` is

```text
sum(2^j for j <=_P k).
```

The canonical form is the relabeling minimizing the sequence of these numbers
lexicographically. Census generation and sorting use the canonical incidence
sequence as `PosetKey`; it determines the complete canonical order relation and
does not depend on a database hash convention.

An enumerator produces canonical presentations directly. Relations in the
census refer to another `PosetKey`. A later TypeLake adapter may assign content
hashes and presentation ids without changing or recomputing the census.

The canonical incidence sequence is useful for deterministic ordering,
debugging, and future TypeBase label assignment. It is not additional defining
data: the canonical Hasse diagram already determines it.

### 4.2 Computation bundle

The census builder will assemble one typed computation bundle per poset before
writing a versioned JSON record. At minimum it contains:

```text
PosetComputation
  census record version
  canonical PosetKey
  canonical covers
  canonical incidence sequence
  structural invariants
  automorphism action and order
  canonical core key and reduction evidence
  order-complex key
  opposite-poset key
  provenance

SimplicialComplexComputation
  census record version
  ComplexKey
  vertex set and lexicographically sorted maximal facets
  dimension, f-vector, and Euler characteristic
  integral homology
  provenance
```

These bundles are internal pipeline values, not Noether or TypeLake public
types. They allow the complete deterministic computation to be checked together
before the census writer emits linked coherent records. Identical exact complex
presentations are written once and may be referenced by multiple Posets.

### 4.3 Relations

The initial relation set is:

- `has-core`: `P -> canonical_core(P)`;
- `opposite`: `P -> canonical_form(P^op)`;
- `order-complex`: `P -> Delta(P)`;
- `canonicalizes-to`: used only when a submitted presentation is not already
  canonical.

Every relation target must exist in the completed census or in the same
candidate order. `has-core`, `opposite`, and `order-complex` are total on
completed Poset records.

### 4.4 Worked complete record: the square face poset

Here is pseudocode for the folded record of an interesting eight-element
example: the face poset of the boundary of a square. Elements `0, 1, 2, 3` are
the vertices and `4, 5, 6, 7` are the edges. Its order complex is a circle, it
is already a Stong core, and its natural automorphism action is the dihedral
symmetry of the square.

This is deliberately pseudocode rather than the final JSON encoding. The
mathematical bundle becomes one coherent census record, while run-wide
provenance and validation are referenced from the accompanying reports. A later
TypeLake adapter must preserve every value without recomputing mathematics.

```yaml
object:
  record_version: 1
  key: &square_key [1, 2, 4, 8, 19, 37, 74, 140]

  defining:
    covers:                                 # the canonical Hasse diagram
      n: 8
      arcs:
        - [0, 4]
        - [0, 5]
        - [1, 4]
        - [1, 6]
        - [2, 5]
        - [2, 7]
        - [3, 6]
        - [3, 7]

  canonicalization:
    method: lexicographic-principal-downsets
    incidence_sequence: [1, 2, 4, 8, 19, 37, 74, 140]
    kind: complete

  values:
    n: 8
    cover_count: 8
    strict_comparable_pairs: 8
    strict_lower_counts: [0, 0, 0, 0, 2, 2, 2, 2]
    strict_upper_counts: [2, 2, 2, 2, 0, 0, 0, 0]
    minimal_count: 4
    maximal_count: 4
    height: 2                              # elements in a longest chain
    width: 4
    connected_component_sizes: [8]
    is_connected: true
    is_lattice: false
    is_self_dual: true

    automorphism_group:
      degree: 8
      generators:                          # one-line notation, original -> image
        - [1, 0, 3, 2, 4, 6, 5, 7]
        - [0, 2, 1, 3, 5, 4, 7, 6]
      order: "8"
      is_transitive: false
      orbits:
        - [0, 1, 2, 3]
        - [4, 5, 6, 7]
      abstract_group:
        structure: D4                      # order-eight dihedral group
        external_labels:
          lmfdb.abstract_group: "8.3"
          gap.SmallGroup: [8, 3]

    core:
      key: *square_key
      size: 8
      is_core: true
      beat_point_deletions: []
      reduction_verified: true

  relations:
    has-core:
      target: *square_key
      value: {reduction_length: 0}
    opposite:
      target: *square_key
      value: {self_duality_verified: true}
    order-complex:
      target: &square_complex_key "<ComplexKey-for-the-eight-cycle>"

  provenance:
    primary:
      source: enumeration
      agent: {name: poset-census, version: "<git-revision>"}
      origin: "complete isomorph-free enumeration at order 8"
      rigor: {level: machine-certified, assumptions: []}
      run: "<immutable-run-id>"
    derived:
      source: computation
      agent: {name: noether, version: "0.1.0+<git-revision>"}
      recreate:
        language: shell
        code: "poset-census enrich --order 8 --run <immutable-run-id>"
      rigor: {level: machine-certified, assumptions: []}
      oracle: {name: GAP, version: "<version>", agreed: true}

  coverage:
    isomorphism_scope: {order: 8, status: complete}
    homotopy_scope: {source_orders_through: 8, status: complete}
    certificate: "<completeness-certificate-id>"

linked_objects:
  simplicial_complexes:
    - record_version: 1
      key: *square_complex_key
      defining:
        vertices: [0, 1, 2, 3, 4, 5, 6, 7]
        facets:                              # maximal chains in the Poset
          - [0, 4]
          - [0, 5]
          - [1, 4]
          - [1, 6]
          - [2, 5]
          - [2, 7]
          - [3, 6]
          - [3, 7]
      values:
        dimension: 1
        f_vector: [8, 8]                    # eight vertices and eight edges
        euler_characteristic: 0
        integral_homology:                  # unreduced homology
          0: {rank: 1, invariant_factors: []} # Z
          1: {rank: 1, invariant_factors: []} # Z
      checks:
        euler_from_faces: 0
        euler_from_homology: 0
```

The self-references are intentional. This poset has no beat points, so it is
its own core; its opposite is isomorphic to itself. The automorphism action has
two orbits because order automorphisms cannot exchange square vertices with
square edges. That is why the record stores the faithful intransitive action
and its orbits. The Poset collection does not assign transitive-action labels.
The order complex is not merely summarized on the Poset. Its maximal facets
and expensive invariants live in the linked simplicial-complex record, so a
later consumer never has to enumerate the Poset's chains merely to recover the
complex.

## 5. Plan item 1 — finish the required Noether Poset operations

**Status: complete on the Noether `noether-poset` branch.** The implementation
has normalized Hasse-diagram construction, the complete order relation, one
exact principal-downset canonical and automorphism search, structural
invariants, verified Stong cores, and bounded order-complex construction. The
Rust facade and Python projection expose the same mathematical results. The
census builder must pin the merged Noether revision that contains this work.

### 5.1 Lock the mathematical surface

The Rust facade is authoritative. Python receives the same operations and
canonical Rust result values through the existing thin projection. The intended
surface is:

```text
Poset
  canonical_label() -> PosetCanonicalLabel
  canonical_form() -> Poset
  automorphism_group() -> PermutationGroup
  strict_lower_counts() -> [usize]
  strict_upper_counts() -> [usize]
  minimal_elements() -> [VertexId]
  maximal_elements() -> [VertexId]
  comparable_pair_count() -> usize
  connected_components() -> [[VertexId]]
  height() -> usize
  width() -> usize
  opposite() -> Poset
  is_lattice() -> bool
  is_self_dual() -> bool
  beat_points() -> [BeatPoint]
  is_core() -> bool
  core() -> CoreReduction
  order_complex() -> SimplicialComplex
```

Fallible signatures retain `Result<..., PosetError>` where canonical search,
group construction, topology construction, or an explicit resource bound can
fail. The schematic list above omits that transport detail for readability.

`PosetCanonicalLabel` is a Poset-owned result containing the canonical Poset,
the original-to-canonical permutation, canonical incidence sequence, and the
complete automorphism group acting on the original vertices. The canonical
permutation is generally not an automorphism. Automorphisms arise by composing
two labelings that reach the same canonical form.

`BeatPoint` records the original vertex, whether it is an up-beat or down-beat
point, and the neighboring witness that certifies the beat-point condition.
`CoreReduction` contains the canonical core, a replayable deletion sequence,
the retained-original-vertex map, and enough final relabeling data to verify the
canonical target. These are mathematical evidence types, not census records.

Noether does **not** identify the abstract automorphism group with GAP or LMFDB,
assign labels, enumerate Posets, write checkpoints, or serialize census rows.
Those remain responsibilities of `poset-census`.

### 5.2 One poset-specific symmetry search

The completed implementation replaces the former generic Hasse-graph
canonicalization prepass with one exact Poset search. Its design is:

1. Compute for every vertex:
   - strict predecessor count;
   - strict successor count;
   - lower-cover count;
   - upper-cover count.
2. Use these invariants as an initial partition for possible automorphisms.
3. Refine each cell by its cover and comparability profiles into the other
   cells until stable.
4. Search for the lexicographically minimum principal-downset sequence.
5. Whenever two complete labelings produce the same sequence, compose them to
   obtain an automorphism candidate and verify it against the complete order
   relation or Hasse diagram.
6. Add independent verified candidates as permutation-group generators.
7. Use the group generated so far and the pointwise stabilizer of the current
   prefix to prune equivalent later branches.
8. Return the canonical Hasse diagram, original-to-canonical permutation, and
   complete automorphism group from the same search.

The search remains exact without group pruning; the group changes work, never
semantics. All work caps are deterministic node or object counts and fail with
typed errors.

#### Symmetry acceptance tests

- Exhaustively compare canonical forms with all vertex permutations for every
  naturally labeled poset through order 5.
- Exhaustively compare automorphism groups with all permutations through order
  5, checking both order and membership.
- Check that each generator preserves every comparison and every cover.
- Check canonical invariance under relabeling.
- Check chains, antichains, diamonds, Boolean lattices, disjoint unions, and
  asymmetric layered examples.
- Differentially check a deterministic sample against GAP.
- Benchmark with and without the predecessor/successor partition; retain
  measurements in the collection validation report rather than claiming a
  speedup from intuition.

### 5.3 Core reduction

Noether should expose beat-point reduction as mathematics on `Poset`.

A down-beat point has a greatest element in its strict lower set; an up-beat
point has a least element in its strict upper set. The extremal element is the
witness stored with the deletion. Empty strict lower or upper sets do not make
a point beat by vacuity.

The result should include:

- the reduced poset;
- the canonical form of the core;
- the original vertices retained at each step or an equivalent deletion
  witness;
- whether each deletion was an up-beat or down-beat deletion;
- a verifier that replays the sequence and checks the beat-point condition.

The canonical core is independent of deletion choices up to isomorphism. Tests
must deliberately use different valid deletion orders and compare their
canonical cores. Required identities include:

```text
core(core(P)) = core(P)
core(P) has no beat points
P is a core iff core(P) is isomorphic to P
core(P^op) = core(P)^op up to canonical form
```

Known minimal finite models of spheres provide topology-bearing fixtures.

### 5.4 Order complex and homology

Noether should expose `Poset::order_complex()` returning the existing
`SimplicialComplex` type. Its vertices are poset elements and its simplices are
nonempty chains. The census builder then uses Noether's exact integral
homology machinery; it does not build boundary matrices independently.

The implementation enumerates maximal chains as facets, preserves the Poset's
vertex ids, and stops with a typed error at an explicit facet or simplex limit.
The complete source order complex is still constructed and stored, but the
census does not normally compute its homology by reducing the source boundary
matrices directly.

Noether should add a Poset-specific integral-homology playbook with this exact
strategy order:

1. compute the verified canonical beat-point core;
2. compute homology on that core, returning immediately from a cached core value
   when the census builder already has one;
3. apply a verified Poset/order-complex discrete Morse reduction when it reduces
   the core further; and
4. use the existing sparse integral Smith/rank path as the correctness floor.

Core inheritance is exact: every beat-point deletion is a strong deformation
retract and induces the same integral homology. The builder owns memoization by
canonical core key; the mathematical core/DMT/SNF playbook and any reduction
certificate belong in Noether. For a source complex of dimension larger than
its core complex, the builder pads the inherited graded result with zero groups
through the source dimension, preserving the v1 complex-record convention.

Noether currently has fast sparse `sparla` dispatch for integral homology
invariants. Revision `253d73734b217fad6fc5839704b2f5688672bb55` puts the
general adjacent-differential/Smith orchestration and exact algebraic Morse
reduction in `homalg`. `ChainComplex` can cancel `1` and `-1` pivots over any
unital coefficient ring, returning a replayable reduction certificate, and
its integral graded-invariant playbook selects direct Smith or
reduction-then-Smith at a benchmarked private crossover. `topology` separately
constructs an exact signed Morse chain model from an acyclic free-face matching
and routes sufficiently large, substantially reduced simplicial complexes into
the same `homalg` invariant extraction. `linalg` continues to own Smith normal
form for an arbitrary integral matrix. Poset core reduction remains the outer
mathematical playbook selecting the smaller homotopy-equivalent input. The
existing `graded_integral_homology()` retains full boundary witnesses and is
not the lean all-degree route needed here.

The census stores the resulting complex as a linked object. Its defining data
are the inherited vertex set and the sorted maximal facets, which are sufficient
to reconstruct the entire abstract complex without enumerating chains in the
Poset again. `ComplexKey` is a versioned content key for that exact labeled
facet presentation. Census v1 does not add simplicial-complex canonicalization
up to isomorphism to this Noether milestone; a later adapter may deduplicate
isomorphic complexes by relabeling the stored facets without rerunning the
Poset computation.

Noether uses the standard signed dimension convention for its public
simplicial-object surfaces: an empty complex has dimension `-1`, while a
nonempty discrete complex has dimension zero. Internal chain and boundary
indices remain nonnegative. The census stores Noether's dimension directly.

We will measure whether reconstructing downward closure from facets is itself a
material cost. Full faces-by-dimension or boundary matrices may be added as an
optional cache only if the census measurements justify their storage cost.

For `n <= 10`, enumerating all chains for the stored facet presentation is
bounded and practical, but limits must still be explicit. Direct source-complex
Smith computations remain validation oracles on hand cases and deterministic
samples rather than the default calculation for every noncore Poset.
Cross-checks include:

```text
Euler(order_complex(P)) = alternating sum of f-vector
                         = alternating sum of integral Betti numbers
homology(P) = homology(core(P))
homology(P^op) = homology(P)
```

Mod-`p` Betti numbers are derivable from integral homology by the universal
coefficient theorem, including the contribution of `p`-torsion one degree
lower. They can later be a derived TypeBase value computed from integral
homology rather than a separately stored census computation.

### 5.5 Structural operations

The first Noether surface needed by the collection is:

- `strict_lower_counts()` and `strict_upper_counts()`;
- minimal and maximal vertices;
- connected components of the undirected Hasse diagram;
- longest-chain height;
- width;
- `opposite()`;
- `is_lattice()`;
- `is_self_dual()` using canonical forms;
- beat points, `is_core()`, and `core()`;
- `order_complex()`.

Operations should return typed exact values. Collection code may organize or
serialize them but must not reimplement them.

Implementation notes:

- strict predecessor and successor counts come directly from the packed order
  relation and also seed symmetry partitioning;
- height is the number of elements in a longest chain;
- width uses an exact Dilworth reduction to bipartite maximum matching, not
  subset enumeration;
- components are those of the undirected Hasse diagram and are returned in a
  deterministic order;
- `opposite()` reverses the order while retaining the current vertex ids;
- `is_lattice()` checks unique meets and joins in the finite order; and
- `is_self_dual()` compares the canonical forms of `P` and `P.opposite()`.

For the empty Poset, height and width are zero, the minimal/maximal and
component lists are empty, and `is_lattice()` is false. Counts use `usize`
inside Noether; the census is responsible for its chosen portable encoding.

### 5.6 Validation and stopping condition

The milestone includes three validation layers:

1. **Hand fixtures:** empty Poset, singleton, chains, antichains, the vee and
   diamond, disconnected examples, Posets with up- and down-beat points, and the
   eight-element square face Poset.
2. **Exhaustive small checks:** canonical forms and full automorphism membership
   against every permutation through order 5; structural invariants against
   simple brute-force definitions; core witnesses replayed step by step; and
   order-complex simplices compared with all nonempty chains.
3. **Independent checks:** deterministic GAP samples for automorphism groups;
   known finite models for cores; and Euler characteristic plus integral
   homology checks for points, discrete spaces, contractible Posets, and the
   square model of the circle.

Every public operation is tested through Rust and Python. Benchmarks cover
chains, antichains, layered symmetric Posets, asymmetric Posets, and selected
orders up to the census target. A skipped GAP oracle is recorded as skipped and
does not count as verification.

Item 1 is complete when:

- canonical form and automorphism group come from one exact Poset search, with
  no generic graph-canonicalization prepass;
- every operation above is available from the public Rust facade and Python
  projection with precise stubs and typed failures;
- the exhaustive, hand, parity, and topology gates pass;
- representative order-10 inputs remain within explicit resource limits; and
- `poset-census` can obtain every mathematical value in its record without
  reimplementing Poset mathematics.

## 6. Enumeration design

Enumeration is census-specific and lives only in `poset-census`.

### 6.1 Correctness floor

Before implementing an optimized generator, build a small reference generator.
It may generate redundantly, canonicalize every candidate with Noether, and
deduplicate by `PosetKey`. It only needs to reach orders where the complete
answer is small enough for exhaustive comparison.

This slow path remains in tests. An optimized generator is not accepted merely
because it matches OEIS totals; it must produce the same `PosetKey` set as
the reference generator at every feasible order.

### 6.2 Orderly generation tree

The production generator follows the natural prefix tree obtained by adjoining
a new maximal element. The principal-downset canonical form was chosen so this
tree is hereditary.

1. Let `P` be a canonical parent on `0, ..., n - 1`.
2. Enumerate order ideals of `P`; an ideal is the strict downset of the new
   maximal element `n`. The empty ideal creates an isolated maximal element.
3. Under the natural action of `Aut(P)` on ideals, retain the ideal whose bitset
   is numerically least in each orbit.
4. Append vertex `n` over that ideal without relabeling the parent.
5. Accept the child exactly when this labeled child is already Noether's
   canonical representative.
6. Emit accepted children in deterministic canonical-code order.

The correctness argument is short. A canonical principal-downset labeling is a
linear extension, so its last vertex is maximal. Deleting that last vertex
leaves the prefix of the incidence sequence. If this prefix were not canonical,
a smaller labeling of the prefix extended by the last maximal vertex would give
a smaller labeling of the child. Thus every canonical child has a canonical
parent obtained by deleting its last vertex. Its last strict downset is also
least in its `Aut(P)` orbit, or an automorphism of the prefix would decrease the
last incidence integer. Conversely, parent plus that ideal reconstructs the
child exactly. Therefore every canonical child is accepted once.

The first implementation may enumerate all ideals and test orbit minimality
directly. A later incremental ideal generator may avoid constructing rejected
ideals, but it must produce the same accepted `(parent, ideal)` pairs. A focused
Noether `is_canonical()` predicate could stop when it finds a smaller labeling
without completing the automorphism group of a rejected child; accepted
children still need the combined canonical-label/automorphism result.

### 6.3 Determinism, sharding, and checkpoints

Generation has no implicit randomness. If a randomized validation is added, it
has an explicit recorded seed and cannot determine accepted content.

Work is sharded by deterministic prefixes of the augmentation tree or by
canonical parent ranges. A checkpoint contains only collection-pipeline state:

- run id and generator version;
- target order;
- shard id and shard count;
- last completed canonical parent or augmentation prefix;
- hashes of completed output parts.

No checkpoint or enumeration type is added to Noether. Resuming a completed
shard must reproduce byte-identical output.

The implemented `hereditary-downset-parent-range/v1` strategy divides the
sorted, certified order-`n-1` corpus into contiguous balanced row ranges. The
canonical-child prefix theorem makes the Poset output of those ranges disjoint
and globally sorted by construction. Each checkpoint names immutable JSONL
parts, their exact parent intervals, row and byte counts, and SHA-256 digests.
An OS-level exclusive lock prevents concurrent writers for one shard; a crash
releases the lock and leaves at worst an unreferenced temporary part. Resumption
verifies the parent census validation/run hashes and every completed part
before proceeding. Linked complexes are sorted within each part and combined
by a bounded k-way merge.

### 6.4 Measured baseline before orderly generation

The first release-mode baseline was recorded on 2026-08-26 with the pinned
Noether revision `e676eb9a591469da2dcc7c442aefab9d84276c84`, FLINT 3.6.0,
Rust 1.95.0, and an AMD Ryzen 9 7950X3D. The harness ran single-threaded with no
custom `RUSTFLAGS`; pipeline figures are one complete run and operation figures
are medians of five warm runs. The reproducible harness is
`crates/poset-census/examples/profile_noether.rs`.

The complete reference pipeline gave:

| order | raw subsets | isomorphism classes | enumerate | enrich | validate |
|---:|---:|---:|---:|---:|---:|
| 5 | 1,024 | 63 | 12.6 ms | 5.6 ms | 10.0 ms |
| 6 | 32,768 | 318 | 615 ms | 50 ms | 88 ms |
| 7 | 2,097,152 | 2,045 | 62.3 s | 652 ms | 1.13 s |

This makes the immediate scaling boundary unambiguous. The redundant generator
is a correctness oracle, not a route to order 8: its candidate space would jump
to 268,435,456 relation subsets. The first production optimization belongs in
the orderly generator, which must avoid creating these duplicates, rather than
inside Noether.

Selected Noether operation medians were:

| input | canonical form plus automorphisms | order complex | integral homology in all degrees |
|---|---:|---:|---:|
| 10-antichain | 14.2 ms | 0.002 ms | <0.001 ms |
| complete height-two, order 10 | 4.69 ms | 0.013 ms | 0.035 ms |
| 10-crown | 0.207 ms | 0.010 ms | 0.025 ms |
| 10-chain, 1,023 simplices | 0.012 ms | 0.769 ms | 44.6 ms |
| 12-chain, 4,095 simplices | 0.024 ms | 3.98 ms | 1.69 s |
| 16-antichain | 418 ms | not run | not run |
| Boolean lattice `B_4`, order 16 | 0.231 ms | not run | not run |

These figures identify two Noether-sensitive regimes without conflating them
with reference-enumerator growth:

1. Canonicalization plus automorphism construction is expensive for very large
   symmetry cells. The antichain and complete height-two family deserve a
   dedicated benchmark and profile before scaling beyond the initial target,
   although generic, crown, chain, and Boolean-lattice examples remain much
   cheaper.
2. The current census calls `homology_invariants(degree)` separately in every
   degree. Adjacent boundary ranks and Smith data are consequently recomputed.
   Core-first Poset homology is the primary fix: the 12-chain reduces to one
   point in about 0.008 ms, while direct all-degree homology of its order complex
   with 4,095 nonempty simplices took 1.69 s. A lean graded invariant-factor
   operation remains the sparse-SNF fallback for cores that survive beat-point
   and DMT reduction.

There is also avoidable orchestration work outside Noether. The initial
enricher can invoke canonical search up to five times for one canonical input:
for the source, again for its canonical action, inside core reduction, again to
obtain the core key, and for the opposite. Before changing Noether's canonical
algorithm, the builder should:

- accept and verify a canonical source once, using that search's automorphism
  group directly;
- obtain the key of an already canonical core without another canonical
  search, preferably through a small reusable labeled-incidence operation in
  Noether;
- handle an already-core canonical source without asking `core()` to
  canonicalize it again; and
- keep the exhaustive recomputation in validation separate from production
  enrichment, with later large-order validation explicitly sampled where the
  completeness proof does not require every expensive invariant twice.

This orchestration work is now implemented. Both generators retain canonical
label and automorphism evidence through the enrichment handoff. Production
enrichment consumes that evidence directly, obtains a canonical core's key
from its already-certified labeled incidence sequence without another search,
and handles an already-core source without invoking core canonicalization. The
deliberately redundant validation path remains independent.

Parallelism should remain at the deterministic parent/shard level. None of the
measurements justify adding nondeterministic parallel machinery inside a
single Noether Poset operation yet.

### 6.5 Orderly and core-first benchmark

The follow-up release-mode run on 2026-08-26 used Noether revision
`e0c74bb95db029b6bc9c571404a3362c3d9f2c52` on the same machine and build
profile. The optimized and reference generators have identical `PosetKey` sets
through order 7; the ordinary test suite checks through order 5 and deliberate
stress tests check orders 6 and 7.

| order | parents | downsets | canonical tests | classes | orderly enumerate | enrich | exhaustive validate |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 5 | 16 | 135 | 101 | 63 | 2.73 ms | 5.71 ms | 11.29 ms |
| 6 | 63 | 766 | 576 | 318 | 27.60 ms | 51.68 ms | 102.94 ms |
| 7 | 318 | 5,439 | 4,162 | 2,045 | 347.67 ms | 599.64 ms | 1.11 s |
| 8 | 2,045 | 48,346 | 38,280 | 16,999 | 5.57 s | 9.84 s | 15.58 s |

The earlier reference-generator measurement was 72.30 seconds at order 7,
versus 0.348 seconds for orderly generation here, approximately a 208-fold
reduction. At order 8, generation is no longer the dominant stage: repeated
enrichment and especially exhaustive reconstruction validation take about 25
seconds combined. This confirms the next work item is builder orchestration,
followed by deterministic sharding, rather than a change to Noether's canonical
search.

On the 12-chain, direct integral homology in every source-complex degree took
1.688 seconds, the `homalg`-owned one-Smith-reduction-per-boundary route took
1.653 seconds, and the Poset core-first route took 0.011 milliseconds. Thus
repeated adjacent-boundary work is real but not the main cost; Smith reduction
itself dominates the unreduced source complex. Full enrichment takes 6.78
milliseconds for that chain because it computes homology on the one-point core
while still constructing and storing the 4,095-simplex source order complex.
The builder also memoizes homology by canonical core key and pads inherited zero
groups through the source dimension.

### 6.6 Exact Morse follow-up

The 2026-08-26 follow-up uses Noether revision
`253d73734b217fad6fc5839704b2f5688672bb55`. The reproducible Noether benchmark
is `cargo bench -p noether --bench homology_playbooks`; it compares direct
Smith reduction, the generic `homalg` algebraic-Morse playbook, and the
topology-aware signed Morse chain model before the same exact invariant
extraction. Representative release-mode measurements on the same workstation
were:

| input | direct Smith | algebraic Morse + Smith | simplicial Morse + Smith |
|---|---:|---:|---:|
| 4-simplex, 31 cells | 12.9 µs | 21.9 µs | 18.3 µs |
| 6-simplex, 127 cells | 218 µs | 729 µs | 256 µs |
| 8-simplex, 511 cells | 102 ms | 19.9 ms | 0.115 ms |
| path complex, 128 vertices | 13.2 ms | 1.21 ms | not applicable |
| path complex, 256 vertices | 110 ms | 5.33 ms | not applicable |
| triangulated 8-by-8 torus, 384 cells | 181 ms | 10.0 ms | 0.095 ms |

These data justify two different private decisions rather than a universal
"always reduce" rule. General based complexes use direct Smith below 8,192
dense matrix cells and for unit-free or monomial-support differentials;
otherwise `homalg` cancels universal-unit pairs first. Simplicial complexes use
ordinary sparse boundaries below 128 nonempty cells, then select the Morse
model only when the resulting critical-boundary work is at least four times
smaller. The user-facing operations do not expose strategy names or a planning
API.

The TypeLake poset harness, run from the pinned git dependency with five warm
repetitions, now reports 3.684 ms for graded homology of the 4,095-cell
12-chain order complex, versus 1.653 seconds before the Morse work (about
449-fold faster). Calling the single-degree operation separately in every
degree takes 46.3 ms because it reconstructs the model each time; the builder
uses the graded operation. Core-first homology remains faster at 0.016 ms and
therefore stays the primary Poset strategy, with simplicial Morse providing the
exact fallback for nontrivial cores.

### 6.7 Canonical-evidence handoff and order-9 sizing

The release-mode follow-up on 2026-08-26 used Noether revision
`253d73734b217fad6fc5839704b2f5688672bb55`. The complete order-3 fixtures
remained byte-identical after the handoff change, and complete order-4
validation still passes. At order 8, production enrichment fell from the
earlier 9.84 seconds to 6.17 seconds.
Absolute generation and exhaustive-validation timings varied upward in this
run, so the enrichment comparison is the useful orchestration signal rather
than a claim that unrelated phases improved.

| order | parents | downsets | canonical tests | classes | generate only | peak RSS |
|---:|---:|---:|---:|---:|---:|---:|
| 8 | 2,045 | 48,346 | 38,280 | 16,999 | 9.43 s | 224 MB |
| 9 | 16,999 | 550,338 | 451,411 | 183,231 | 188.93 s | 577 MB |

The order-9 class count agrees with the accepted 183,231. A complete emitted
order-8 slice occupies 27 MB uncompressed: 19.7 MB of Poset records and 8.0 MB
of linked order-complex records. Order 10 has 2,567,284 accepted classes.
Serial generation therefore remains plausible, but retaining all canonical
labels and all enriched records in one process is the wrong execution model:
runtime is likely measured in hours and uncompressed output in several
gigabytes. Order 10 should be produced from the certified order-9 parents as
deterministic parent-range shards with streaming output and checkpoint/resume.

### 6.8 Sharded execution benchmark

The completed sharded implementation was exercised at order 8 from the
validated order-7 corpus. Four sequential shards used at most 41 MB each and
accepted `5,751`, `4,191`, `3,989`, and `3,068` children respectively. Their
bounded merge took 0.43 seconds and about 40 MB, recovered all 16,999 Posets,
and cleared the accepted unlabeled, connected, and labeled-from-automorphisms
count identities. An independent eight-shard run produced byte-identical
Poset and linked-complex JSONL, and both files were byte-identical to the fully
validated monolithic order-8 output. The common SHA-256 digests were
`485a92023d45020e0f07cbd261f1414f0973af84b7ace5e784edc1957de152c3`
for `posets.jsonl` and
`8893bbc017f7ef6d30276974044755ddf96b7ef2a00576b02228780a685b8db3`
for `simplicial-complexes.jsonl`.

The unequal four-shard timings confirm that equal parent counts do not imply
equal work. Large runs should therefore create many more deterministic ranges
than concurrent workers and schedule those ranges dynamically. This preserves
simple contiguous-prefix merging while bounding the tail from unusually
expensive parents.

The ordinary test suite also interrupts every shard at a fixed immutable-part
boundary, plants uncheckpointed partial files, resumes, and proves that the
final shard merge and validation report are byte-identical to an uninterrupted
run and to the monolithic order-5 mathematical output.

## 7. Invariants and priorities

The corpus should distinguish what must be present before an order is accepted
from useful later enrichment.

### 7.1 Required for the first accepted order

#### Defining and identity

- canonical covers;
- order `n`;
- canonical `PosetKey`;
- canonical incidence sequence for deterministic sorting and audit.

#### Order-theoretic

- number of covers;
- number of strict comparable pairs;
- strict predecessor and successor counts by vertex;
- numbers of minimal and maximal elements;
- longest-chain height;
- width;
- connected-component ids or sizes;
- `is_connected`;
- `is_lattice`;
- opposite-poset key;
- `is_self_dual`.

#### Symmetry

- canonical generators of the natural permutation action where a stable
  generator normalization is available;
- otherwise exact verified generators with the generation convention recorded;
- automorphism-group order;
- vertex orbits;
- optional GAP validation result and external abstract-group label.

#### Homotopy and topology

- `is_core`;
- canonical core key;
- core size;
- verified beat-point deletion evidence in the census record;
- linked order-complex key.

#### Linked simplicial complex

- `ComplexKey` for the exact labeled facet presentation;
- inherited vertex set;
- complete lexicographically sorted list of maximal facets;
- dimension and f-vector;
- unreduced Euler characteristic;
- integral homology by degree, including torsion;
- provenance tying construction to Noether and the source Poset run.

### 7.2 Useful second-pass invariants

- rank function and rank sizes when graded;
- number of order ideals and filters;
- number of linear extensions;
- Möbius function summaries;
- numbers of maximal chains;
- bounded, complemented, modular, and distributive lattice flags;
- comparability and incomparability graph invariants;
- abstract automorphism-group invariants and `SmallGroup` labels;
- counts of isomorphism classes mapping to each core by source order.

An invariant enters the census record only if its meaning, convention, value
shape, and source are declared in `RECORD.md`. Useful exploratory values may
remain in run reports until then.

## 8. Census record v1

The defining field is a transitively reduced canonical Hasse diagram. `RECORD.md`
will define the small versioned JSON record, every mathematical convention, and
the `PosetKey` encoding. A record-version change does not alter `PosetKey` unless
the mathematical canonical encoding itself changes.

Before completing the first census order, settle these record choices:

1. Name the canonicalization method precisely as lexicographic
   principal-downset canonicalization rather than generic canonical labeling.
2. Define the natural `automorphism_group` as a census-owned structured
   value, while retaining `aut_order` as an inexpensive scalar.
3. Add unambiguous per-vertex strict predecessor/successor counts if they are to
   be published rather than used only internally.
4. Add `is_core` and retain `core_size` plus the `has-core` relation.
5. Omit `homology_mod_p`; it is derivable from integral homology and is not
   needed in the first census record.
6. Decide whether beat-point reduction evidence is provenance/evidence payload
   or a separately typed certificate. It is not part of poset identity.
7. Attach LMFDB abstract-group and GAP `SmallGroup` labels to the underlying
   abstract group. Transitive-action labels are outside the Poset collection
   record.
8. Revisit the connected-only normalization using measured files from the pilot
   rather than the projected size argument in the architecture document.
9. Define `ComplexKey`, the `order-complex` link, and a separate
   simplicial-complex record whose defining data are the complete maximal
   facets.

The census should establish one coherent Poset record plus separately linked
durable mathematical objects. If the natural computation bundles are awkward
or wasteful to encode, fix the records rather than splitting them into generic
scalar facts.

## 9. Validation gates

No order is marked complete because a program terminated successfully. It must
clear all applicable gates below.

### 9.1 Enumeration and completeness

- Match the accepted number of unlabeled posets at that order.
- Match the accepted number of connected unlabeled posets.
- Reconstruct the all-poset count from connected-component multisets as an
  independent check.
- Compare the complete `PosetKey` set with the reference generator wherever
  feasible.
- Assert that canonical keys and canonical encodings are unique.
- Write a completeness report scoped to the exact completed order.

### 9.2 Automorphisms

- Verify every generator directly against the order relation.
- Verify the group order by exhaustive permutation checks at small orders and
  deterministic GAP samples at larger orders.
- Use orbit-stabilizer checks internally.
- Check the labeled/unlabeled identity

  ```text
  sum over [P] of n! / |Aut(P)| = number of labeled posets on n vertices.
  ```

  This jointly tests completeness, deduplication, and automorphism orders.

### 9.3 Core and homotopy

- Replay every stored reduction witness sampled for validation.
- Check core idempotence for every record.
- Check that every target core has no beat points.
- Check invariance under opposite and random relabeling.
- Check known minimal finite models of spheres and other hand-computable spaces.
- Check that every `has-core` target exists.

### 9.4 Homology

- Check that every `order-complex` target resolves to a stored
  simplicial-complex record.
- Exhaustively check at small orders that the stored maximal facets are exactly
  the maximal chains of the source Poset.
- Reconstruct each sampled complex from its stored facets and reproduce its
  f-vector, Euler characteristic, and integral homology without consulting the
  source Poset.
- Compare the Euler characteristic obtained from the order-complex f-vector
  with the alternating sum of integral Betti numbers.
- Check homology invariance under core reduction and opposite.
- Check selected cases against an independent CAS implementation.
- Derive mod-`p` dimensions from integral homology and compare a sample with a
  direct finite-field boundary computation.

### 9.5 Reproducibility and record integrity

- A repeated shard run produces byte-identical sorted output and identical
  hashes.
- A resumed shard equals an uninterrupted shard.
- Every row validates under its pinned census-record version.
- Every relation endpoint resolves.
- Every completed-order file hash appears in its immutable run report.
- Every run report records all code revisions, build profiles, parameters, and
  external oracle versions used for validation.
- Every published Poset label has the structural prefix dictated by its record.
- Within each label bucket, indices are contiguous, one-based, unique, and
  agree with canonical-incidence order.

## 10. Future TypeBase logical views

Serving is out of scope, but the data must be shaped so the intended views can
be built without recomputing mathematics.

### 10.1 Isomorphism view

One row per canonical poset, partitioned by order. For each completed order the
view is complete. It includes the defining Hasse diagram, core id, automorphism
order and action, a link to the stored order complex, its integral homology, and
selected structural facets.

The view contains disconnected as well as connected posets. Connectivity is a
facet, not an existence condition.

### 10.2 Homotopy view

One row per canonical core. It includes:

- the core poset and core size;
- the linked stored order complex, integral homology, and complex invariants;
- the number of known isomorphism classes reducing to it, grouped by source
  order;
- the largest source order for which that count is certified complete;
- separate sampled/construction coverage beyond the complete range.

The view describes finite-space homotopy equivalence, not weak homotopy
equivalence.

### 10.3 Labels

The proposed version-one human label is

```text
n.c.h.w.k
```

where:

- `n` is the number of vertices;
- `c` is the number of connected components of the undirected Hasse diagram;
- `h` is the number of elements in a longest chain;
- `w` is the width; and
- `k` is the one-based position in lexicographic canonical-incidence order
  among all isomorphism classes with that same `(n, c, h, w)` prefix.

All five parts are unpadded decimal integers. For the empty poset, `c`, `h`,
and `w` are all zero. The square face poset in the worked record is
`8.1.2.4.91`: it is connected, has height two and width four, and is the 91st
of the 131 posets in that bucket.

This split has mathematical and operational value. The prefix exposes standard
facets and produces substantially smaller indexing buckets than `n.k` alone.
Its invariants are unchanged by taking the opposite, so a dual pair always
lives in the same bucket. The final index still distinguishes nonisomorphic
duals according to their canonical incidence sequences.

The label is a stable TypeBase citation name, not canonical identity.
Generation, object rows, enrichments, and relations key on canonical hash ids
and never on labels. A label is published only after completeness of its whole
bucket is certified; provisional labels are forbidden because inserting a
missed canonical form would shift later indices. Implementation changes cannot
reorder labels because the order is defined mathematically rather than by
discovery time.

The homotopy view reuses a core's Poset label when that label is available. If
cores beyond the complete isomorphism range need labels, a separately
namespaced homotopy-type scheme can be introduced later; it must not overload
the meaning of `typebase.poset`.

## 11. Run reports and provenance

Every run report records:

- a unique run id;
- Noether version and Git revision;
- `poset-census` version and Git revision;
- census-record and `PosetKey` encoding versions;
- Rust toolchain and relevant native-library versions;
- operation (`enumerate`, `enrich`, `validate`, or `emit`);
- order range, shard parameters, and any explicit validation seed;
- input file ids and hashes;
- output files, row counts, and hashes;
- elapsed time and peak-memory measurements;
- validation gates attempted and their results;
- whether GAP or another external oracle was available.

The reconstruction command is part of the report and is a specification for a
reader to run independently. The census tool records commands; it does not
execute commands supplied by data files.

## 12. Implementation phases

### Phase A — lock the small mathematical record

**Status: complete.** `RECORD.md` v1 is implemented by checked Rust record
types. The complete order-3 JSONL fixtures exercise all five classes, including
beat-point reductions, and the selected order-4 fixture is the four-point
minimal finite circle with integral `H_1 = Z`.

Deliverables:

- this plan reviewed;
- version-one census record and mathematical conventions;
- hand-checkable fixtures for all five posets of order 3, selected order-4
  posets, a beat-point reduction, and a minimal finite circle;
- expected canonical forms, automorphism groups, cores, and homology for those
  fixtures.

Exit criterion: the fields and conventions required for a `PosetComputation`
are unambiguous.

### Phase B — finish Noether symmetry

**Status: complete on the Noether `noether-poset` branch, including the
core-first homology follow-up.**

Deliverables:

- combined canonical-form/automorphism search;
- strict predecessor/successor partitioning;
- exhaustive small-order tests;
- GAP parity harness;
- representative benchmarks.

Exit criterion: canonical forms and automorphism groups agree with exhaustive
permutation checks through order 5, and no generic Hasse canonicalization is
performed before the poset search.

### Phase C — finish Noether core and topology operations

**Status: complete on the Noether `noether-poset` branch, including exact
algebraic and simplicial discrete-Morse reduction.**

Deliverables:

- verified beat-point reduction and canonical core;
- order-complex construction;
- structural methods required by the initial record;
- integral homology path and cross-checks.

The follow-up added Poset-level core-first homology without changing the stored
order-complex relation, a general `homalg` algebraic-Morse reduction using
`+1` and `-1` units over any compatible coefficient ring, and a topology-owned
simplicial DMT route before sparse Smith reduction.

Exit criterion: all hand fixtures and core/topology identities pass through both
Rust and Python public surfaces.

### Phase D — build `poset-census` and the reference corpus

**Status: complete.** The crate, CLI, explicit data-root handling, pinned
Noether dependency, redundant fixed-linear-extension generator, deterministic
JSONL emission, validation report, and run report are implemented. The complete
order-3 pilot is a checked fixture, and complete order 4 clears all local count,
symmetry, core, opposite, and topology gates. Tests also establish byte-identical
mathematical output across repeated runs and reject overwriting a completed
order. The orderly generator is Phase E.

Deliverables:

- `poset-census` crate and CLI with no TypeLake dependency;
- explicit data-root handling;
- run reports and deterministic JSON Lines output;
- redundant reference enumeration through a small order;
- complete version-one census records.

Exit criterion: one command creates, enriches, validates, and emits a complete
small-order census from an empty work directory.

### Phase E — validate orderly generation

**Status: complete.** The generator is reference-validated through order 7 and
matched to the accepted class count through order 9. Deterministic sharding,
checkpoint/resume, bounded merging, cross-shard-count reproducibility, and
monolithic/sharded equality are implemented and exercised through order 8.
Exact key-set equality is checked against the reference generator through every
currently feasible reference order, not only against published totals.

Deliverables:

- automorphism-orbit reduction of order ideals;
- orbit-minimal ideal selection and hereditary canonical-child acceptance;
- deterministic sharding and checkpoint/resume;
- equality with the reference `PosetKey` sets at every feasible order.

Exit criterion: the optimized and reference generators give identical objects,
and resumed output is byte-identical to uninterrupted output.

### Phase F — first complete census slice

Generate all posets through order 6, including every required invariant and
relation. Produce:

- immutable, sorted JSON Lines records through order 6;
- complete isomorphism and homotopy groupings;
- count, symmetry, core, homology, and reproducibility reports;
- measured evidence for revisiting connected-only normalization and file shape.

Exit criterion: every validation gate is green and the entire result can be
regenerated from an empty work directory using only pinned source revisions and
declared native dependencies.

### Phase G — scale one order at a time

For each `n = 7, 8, 9, 10`:

1. benchmark and choose shard size;
2. enumerate canonical objects;
3. clear enumeration counts before expensive enrichment;
4. compute cores, symmetry data, and topology in restartable passes;
5. clear all validation gates;
6. freeze the immutable census files and validation report;
7. record performance data before proceeding to `n + 1`.

Failure at one order does not weaken completed lower-order certificates.

## 13. Immediate next work

The next implementation sequence is:

1. **Complete:** translate `RECORD.md` v1 into checked Rust record types and
   complete order-3 JSON conformance cases.
2. **Complete:** scaffold `crates/poset-census` with an explicit data root and
   pinned Noether revision, without a TypeLake storage dependency.
3. **Complete for order 3:** implement redundant enumeration, enrichment,
   sorted JSONL emission, run reporting, and the local validation gates.
4. **Complete:** add a selected order-4 minimal finite-circle fixture and
   generate and validate the complete order-4 reference corpus.
5. **Complete:** add an independent repeated-run byte comparison for the two
   mathematical JSONL files and exercise immutable-output failure behavior.
6. **Complete:** implement the orderly generator and establish equality with
   the reference key sets through order 7.
7. **Complete:** route Poset homology through its core and add general
   homological and simplicial DMT reductions in Noether.
8. **Complete:** preserve canonical-label evidence through enrichment, remove
   redundant source/core searches, and size generation through order 9.
9. **Complete:** implement deterministic canonical-parent shards with durable
   sorted parts and checkpoint/resume; prove resumed, differently sharded, and
   monolithic output are byte-identical.
10. **Current:** generate, validate, and freeze orders 6 through 9, then produce order 10
    from the certified order-9 parent corpus without retaining the complete
    order-10 level in memory.

The guiding rule is that the census record follows the mathematics. TypeLake
must later adapt to the validated record rather than making Poset computation
depend on an untested storage abstraction.
