# Finite-poset census record v1

**Status:** draft for review, 2026-08-26

**Contract name:** `finite-posets/v1`

**Canonical Poset encoding:** `principal-downsets/v1`

**Order-complex encoding:** `order-complex/v1`

This document defines the first durable, database-independent records emitted
by `poset-census`. It fixes the mathematical meanings, normalized JSON shapes,
keys, links, and validation rules needed to create the census. It does not
define a TypeLake physical layout, a TypeBase label, or a public schema
authority. A later TypeLake adapter must preserve these records losslessly.

The normative words **must**, **must not**, **should**, and **may** are used in
their usual specification sense.

## 1. Files and scope

A completed order `n` has four files:

```text
census/record-version=1/order=<n>/
  posets.jsonl
  simplicial-complexes.jsonl
  validation.json
  run.json
```

- `posets.jsonl` contains exactly one record for every canonical finite Poset
  in the completed isomorphism census of order `n`.
- `simplicial-complexes.jsonl` contains each distinct exact labeled
  order-complex presentation referenced by those Posets once.
- `validation.json` records the mathematical and completeness gates for the
  order.
- `run.json` records the computation and file provenance.

The two JSON Lines files are mathematical census output. Given the same pinned
source revisions and parameters, they must be reproduced byte for byte.
Timestamps, elapsed time, host information, and unique execution ids belong in
`run.json`, not in a mathematical row; separate executions may therefore have
different run reports while producing identical record-file hashes.

Every JSON Lines record is one UTF-8 JSON object followed by LF. The writer
uses compact JSON with no insignificant whitespace and emits object members in
the order shown in this contract. Readers must not assign meaning to JSON
object-member order. Arrays whose normalization is specified below are
ordered data and must not be reordered by a reader.

## 2. Integer and identifier conventions

### 2.1 Bounded integers

Orders, vertex ids, degrees, ranks, counts, and permutation images are JSON
integers. They must be nonnegative except for the dimension `-1` of the empty
simplicial complex. Vertex ids are in the standard set `0, ..., n - 1` and
must fit an unsigned 32-bit integer.

### 2.2 Unbounded integers

Mathematically unbounded integers are encoded as decimal JSON strings. A
nonnegative value has the regular expression `0|[1-9][0-9]*`; a positive value
has `[1-9][0-9]*`. In v1 this rule applies to:

- entries of a `PosetKey`;
- automorphism-group orders;
- GAP `SmallGroup` orders; and
- torsion invariant factors.

This avoids making object identity depend on a JSON implementation's integer
range.

### 2.3 Permutations

A permutation of degree `n` is an array of length `n` in one-line notation:

```text
[p(0), p(1), ..., p(n - 1)].
```

It maps the current vertex id to its image. Every entry is distinct and lies
in `0, ..., n - 1`. The empty permutation is the unique permutation of degree
zero.

## 3. Poset identity: `PosetKey`

For a labeling of a finite Poset by `0, ..., n - 1`, define the principal-
downset integer of vertex `k` by

```text
a_k = sum(2^j for j <=_P k).
```

Among all labelings, choose the lexicographically least integer sequence

```text
(a_0, ..., a_(n - 1)).
```

The resulting sequence is the `PosetKey`. This is precisely Noether's
lexicographic principal-downset canonical form. It resolves isomorphism of
finite Posets; a canonicalization permutation maps submitted vertex ids to
the ids in this canonical representative and is not generally an
automorphism.

The JSON representation of a `PosetKey` is an array of unsigned decimal
strings. For example, the square face Poset has

```json
["1","2","4","8","19","37","74","140"]
```

The empty Poset has key `[]`. The order is the key length. Within one order,
Poset records are sorted by numeric lexicographic order of the key entries;
across orders, the total order first compares the order and then that sequence.
Sorting the decimal strings lexicographically is incorrect.

A key is self-validating as a labeled order relation:

```text
j <=_P k  iff  bit j of a_k is set.
```

The decoded relation must be reflexive, antisymmetric, and transitive; bit `k`
of `a_k` must be set; no bit at position at least `n` may be set. The sequence
must also equal the complete canonical minimum computed from that relation.

`PosetKey` is the census identity and relation key. It is not a TypeBase human
label and is not a TypeLake content hash. The record does not repeat the same
sequence under a second `incidence_sequence` field: the key is the canonical
incidence sequence.

## 4. Canonical covers

The defining presentation of a Poset record is its transitively reduced Hasse
diagram. `covers` is an array of pairs `[lower, upper]` such that `lower` is
covered by `upper`.

The pairs must:

- use vertices in `0, ..., n - 1`;
- contain no loops or duplicates;
- be sorted lexicographically by `(lower, upper)`;
- be exactly the transitive reduction of the order decoded from `key`; and
- use the canonical vertex labeling fixed by `key`.

The full order relation is not stored because it is recoverable from either
the key or the covers.

## 5. Poset record

The JSON shape is:

```json
{
  "record_version": 1,
  "key": ["1"],
  "order": 1,
  "covers": [],
  "invariants": {},
  "automorphism_group": {},
  "relations": {
    "has_core": {},
    "opposite": {},
    "order_complex": {}
  }
}
```

No member shown above is optional. Optional nested enrichment is identified
below.

### 5.1 Structural invariants

`invariants` has this complete v1 shape:

```json
{
  "cover_count": 0,
  "strict_comparable_pair_count": 0,
  "strict_lower_counts": [0],
  "strict_upper_counts": [0],
  "minimal_count": 1,
  "maximal_count": 1,
  "component_count": 1,
  "component_sizes": [1],
  "is_connected": true,
  "height": 1,
  "width": 1,
  "is_lattice": true
}
```

The fields mean:

- `cover_count`: number of Hasse edges;
- `strict_comparable_pair_count`: number of pairs `(x, y)` with `x < y`;
- `strict_lower_counts[k]`: `|{j : j < k}|`;
- `strict_upper_counts[k]`: `|{j : k < j}|`;
- `minimal_count` and `maximal_count`: numbers of minimal and maximal vertices;
- `component_count`: number of connected components of the undirected Hasse
  diagram;
- `component_sizes`: component cardinalities sorted nondecreasingly;
- `is_connected`: true exactly when `order > 0` and `component_count == 1`;
- `height`: number of elements in a longest chain;
- `width`: maximum cardinality of an antichain; and
- `is_lattice`: whether every pair has a unique meet and a unique join.

For the empty Poset, lower and upper count arrays and `component_sizes` are
empty; all five scalar counts, `height`, and `width` are zero; `is_connected`
and `is_lattice` are false.

`strict_lower_counts` is the per-vertex quantity previously called the height
of a vertex. It must not be confused with the record's longest-chain `height`.

### 5.2 Natural automorphism action

`automorphism_group` describes the complete faithful action of `Aut(P)` on the
canonical vertices of the record:

```json
{
  "degree": 1,
  "generators": [],
  "order": "1",
  "orbits": [[0]],
  "is_transitive": true,
  "generators_verified": true
}
```

The group is the subgroup of `S_n` generated by `generators`. Before writing,
the builder removes identity generators and duplicates, then sorts the
remaining one-line permutations lexicographically. This normalization does
not claim that different algorithms will choose the same generating set. The
pinned Noether revision and the normalization above together determine the v1
bytes. The generated subgroup, its order, and its action are mathematical; a
particular minimal generator list is not part of Poset identity.

Every generator must preserve the complete order relation. `order` is the
exact group order. Each orbit is sorted increasingly and the orbit list is
sorted by its least member. The orbits partition `0, ..., n - 1`.
`is_transitive` is true exactly when `n > 0` and there is one orbit; it is false
for the empty action.

`generators_verified` must be true in an accepted record. The accompanying
validation report states how completeness of the generated group was checked;
direct generator verification alone does not establish that the full group
was found.

When the abstract group has been independently identified, the following
optional member may be added inside `automorphism_group`:

```json
"abstract_group": {
  "lmfdb_label": "8.3",
  "gap_small_group": {"order": "8", "id": 3}
}
```

Each label identifies the underlying abstract group, not the natural
permutation action. Either nested label may be absent. If no abstract-group
identification has been verified, `abstract_group` is absent rather than null.
There is no transitive-action label in the finite-Poset record.

### 5.3 `has_core`

`relations.has_core` is total and has this shape:

```json
{
  "target": ["1"],
  "core_size": 1,
  "is_core": true,
  "deletions": [],
  "retained_vertices": [0],
  "canonical_labeling": [0],
  "reduction_verified": true
}
```

`target` is the `PosetKey` of the canonical Stong core and must resolve to a
Poset record. `core_size` is its order. `is_core` is true exactly when the
source and target keys agree and the deletion list is empty.

Each entry of `deletions` is

```json
{"element": 4, "kind": "down", "witness": 2}
```

where `kind` is `"down"` or `"up"`. A down-beat deletion is certified by a
greatest element in the current strict lower set; an up-beat deletion is
certified by a least element in the current strict upper set. Empty strict
sets do not satisfy either condition by vacuity. Element and witness ids refer
to the original canonical Poset, and the witness must still be active at that
step.

After replaying all deletions, `retained_vertices` lists the active original
ids in increasing order. Reduced-poset vertex `i` corresponds to
`retained_vertices[i]`. `canonical_labeling` is the permutation from those
reduced-poset ids to the canonical ids of `target`.

`reduction_verified` must be true in an accepted record. Verification replays
every deletion, reconstructs the reduced Poset, applies
`canonical_labeling`, checks the target key, and checks that the target has no
beat points.

### 5.4 `opposite`

`relations.opposite` is total:

```json
{
  "target": ["1"],
  "is_self_dual": true
}
```

`target` is the canonical key of the Poset obtained by reversing every
comparison. It must resolve to a Poset record of the same order.
`is_self_dual` is true exactly when `target` equals the source key. The
construction is called `opposite`; `dual` is reserved for the isomorphism
property expressed by `is_self_dual`.

### 5.5 `order_complex`

`relations.order_complex` is total:

```json
{
  "target": "oc1-sha256:5dfae799ce9fc1f16f149145dda56c291dd20fb4c2d57d2f0740556a77611978"
}
```

The target is a `ComplexKey` resolving to a record in
`simplicial-complexes.jsonl`. The linked record contains the complete maximal-
facet presentation and topology; the Poset record does not duplicate its
f-vector, Euler characteristic, or homology.

## 6. Order-complex identity: `ComplexKey`

The order complex has the inherited standard vertex set `0, ..., n - 1` and
one nonempty simplex for every chain in the Poset. Its defining stored data are
`vertex_count` and the complete normalized list of maximal facets. Census v1
does not canonicalize simplicial complexes up to vertex relabeling; the key
identifies this exact labeled facet presentation.

A normalized facet:

- is a nonempty list of distinct vertex ids sorted increasingly;
- contains only ids below `vertex_count`;
- is not a proper subset of another facet; and
- occurs once.

The facet list is sorted lexicographically. It is empty exactly when
`vertex_count` is zero.

`ComplexKey` is written as

```text
oc1-sha256:<64 lowercase hexadecimal digits>
```

The digest is SHA-256 of these bytes, concatenated without padding beyond the
specified integer widths:

```text
UTF-8("finite-posets/order-complex")
00                                      # one domain-separator byte
u32be(1)                                # encoding version
u64be(vertex_count)
u64be(number of facets)
for each normalized facet:
    u64be(number of vertices in the facet)
    for each vertex:
        u32be(vertex)
```

This content key is independent of JSON object-member order and whitespace.
Changing these defining bytes requires a new complex-key prefix and an
explicit equivalence map; adding a derived invariant does not.

## 7. Simplicial-complex record

The complete shape is:

```json
{
  "record_version": 1,
  "key": "oc1-sha256:5dfae799ce9fc1f16f149145dda56c291dd20fb4c2d57d2f0740556a77611978",
  "vertex_count": 0,
  "facets": [],
  "is_empty": true,
  "dimension": -1,
  "f_vector": [],
  "euler_characteristic": 0,
  "integral_homology": []
}
```

The fields mean:

- `vertex_count`: cardinality of the inherited standard vertex set;
- `facets`: complete normalized maximal nonempty simplices;
- `is_empty`: whether there are no nonempty simplices;
- `dimension`: maximum `facet.length - 1`, or `-1` for the empty complex;
- `f_vector[d]`: number of nonempty `d`-simplices;
- `euler_characteristic`: unreduced alternating sum of `f_vector`; and
- `integral_homology`: unreduced integral simplicial homology.

For a nonempty `d`-dimensional complex, `integral_homology` contains exactly
one entry for each degree from zero through `d`, including zero groups:

```json
{
  "degree": 1,
  "free_rank": 1,
  "invariant_factors": []
}
```

This denotes

```text
Z^free_rank direct-sum Z/a_1 direct-sum ... direct-sum Z/a_r.
```

The `invariant_factors` are positive decimal strings greater than one and
satisfy `a_i | a_(i+1)`. The empty complex has no nonnegative chain degree, so
its homology list is empty under this unreduced convention.

The stored facets must reconstruct the complete abstract complex and reproduce
the recorded dimension, f-vector, Euler characteristic, and homology without
consulting the source Poset. Identical exact presentations are written once,
even if several Posets reference them.

The builder may obtain `integral_homology` through the source Poset's verified
`has_core` reduction rather than reducing the source complex's boundary
matrices. It computes each canonical core's homology once, using Noether's
Poset-specific core/DMT/sparse-SNF playbook, and reuses that exact graded group
for every Poset in the core fiber. If the source complex has larger dimension
than the core complex, zero groups are appended through the source dimension.
This changes the computation route, not the value or the standalone meaning of
the complex record: direct boundary computation from `facets` must still give
the same groups.

## 8. Complete square-face example

The following illustrates the canonical eight-vertex square-boundary face Poset with the optional abstract-group enrichment attached. Census rows omit `abstract_group`; its labels are supplied by a separate enrichment. Both displayed group labels identify the order-eight dihedral group.

```json
{
  "record_version": 1,
  "key": ["1","2","4","8","19","37","74","140"],
  "order": 8,
  "covers": [[0,4],[0,5],[1,4],[1,6],[2,5],[2,7],[3,6],[3,7]],
  "invariants": {
    "cover_count": 8,
    "strict_comparable_pair_count": 8,
    "strict_lower_counts": [0,0,0,0,2,2,2,2],
    "strict_upper_counts": [2,2,2,2,0,0,0,0],
    "minimal_count": 4,
    "maximal_count": 4,
    "component_count": 1,
    "component_sizes": [8],
    "is_connected": true,
    "height": 2,
    "width": 4,
    "is_lattice": false
  },
  "automorphism_group": {
    "degree": 8,
    "generators": [[0,2,1,3,5,4,7,6],[1,0,3,2,4,6,5,7]],
    "order": "8",
    "orbits": [[0,1,2,3],[4,5,6,7]],
    "is_transitive": false,
    "generators_verified": true,
    "abstract_group": {
      "lmfdb_label": "8.3",
      "gap_small_group": {"order": "8", "id": 3}
    }
  },
  "relations": {
    "has_core": {
      "target": ["1","2","4","8","19","37","74","140"],
      "core_size": 8,
      "is_core": true,
      "deletions": [],
      "retained_vertices": [0,1,2,3,4,5,6,7],
      "canonical_labeling": [0,1,2,3,4,5,6,7],
      "reduction_verified": true
    },
    "opposite": {
      "target": ["1","2","4","8","19","37","74","140"],
      "is_self_dual": true
    },
    "order_complex": {
      "target": "oc1-sha256:d3e6ea646cd7e937974e402bf72f0c937bc44db87773c37b02dc64812e7bb43f"
    }
  }
}
```

Its linked complex record is:

```json
{
  "record_version": 1,
  "key": "oc1-sha256:d3e6ea646cd7e937974e402bf72f0c937bc44db87773c37b02dc64812e7bb43f",
  "vertex_count": 8,
  "facets": [[0,4],[0,5],[1,4],[1,6],[2,5],[2,7],[3,6],[3,7]],
  "is_empty": false,
  "dimension": 1,
  "f_vector": [8,8],
  "euler_characteristic": 0,
  "integral_homology": [
    {"degree": 0, "free_rank": 1, "invariant_factors": []},
    {"degree": 1, "free_rank": 1, "invariant_factors": []}
  ]
}
```

## 9. Ordering and deduplication

Before emission:

1. Each Poset computation must pass the record checks in section 10.
2. Posets are deduplicated by exact `PosetKey` equality.
3. Poset rows are sorted by numeric lexicographic `PosetKey` order.
4. Complexes are deduplicated by `ComplexKey`; a digest collision is a hard
   failure unless the defining presentations are byte-for-byte equal.
5. Complex rows are sorted by their complete ASCII key.

Shards may be emitted in any deterministic work partition, but the completed
files use the global ordering above. Discovery order must never affect output
order.

## 10. Per-record validation

An accepted Poset row must pass all of these checks:

1. `record_version == 1` and `order == key.length`.
2. The key decodes to a finite order relation and is its canonical minimum.
3. `covers` is the sorted transitive reduction of that relation.
4. Every structural invariant recomputes from the relation.
5. Every automorphism generator is a valid permutation preserving the order.
6. The generators have the stated group order and orbits.
7. The core reduction replays, ends in a beat-point-free Poset, and reaches the
   target key.
8. The opposite recomputes to its target key.
9. The order-complex target resolves and its facets are exactly the maximal
   chains of the Poset.

An accepted complex row must pass all of these checks:

1. The facets are normalized and reproduce `key` under `order-complex/v1`.
2. Their downward closure reproduces `is_empty`, `dimension`, and `f_vector`.
3. The alternating sum of `f_vector` equals `euler_characteristic`.
4. The verified core reduction and the core's exact integral homology reproduce
   every homology group; direct integral boundary computations from the source
   facets agree on the declared validation cases and deterministic audit sample.
5. The alternating sum of integral Betti numbers equals the Euler
   characteristic.

These are record-integrity gates. Completeness of an order, independent oracle
agreement, and reproducibility are batch-level gates recorded separately.

## 11. Run provenance

`run.json` must identify at least:

- `contract`: exactly `finite-posets/v1`;
- a unique `run_id`;
- the operation: `enumerate`, `enrich`, `validate`, or `emit`;
- full Git revisions and dirty-state flags for Noether and `poset-census`;
- Rust toolchain, build profile, target, and relevant native-library versions;
- generator/enricher versions and all order, shard, and resource parameters;
- explicit seeds for any randomized validation;
- input paths and SHA-256 hashes;
- output kind, order, path, row count, byte count, and SHA-256 hash;
- start time, elapsed time, and peak memory when measured;
- external oracle names, versions, availability, and invocation status; and
- the associated `validation.json` hash.

The run report may record shell reconstruction commands written by the trusted
builder. Data files must never supply commands for the builder to execute.

The initial census has exact deterministic rigor with no mathematical
assumptions. A skipped optional oracle is recorded as skipped and does not count
as independent verification.

## 12. Coverage and validation report

`validation.json` must state an overall `passed` or `failed` status and record,
for its exact order:

- expected and observed counts of unlabeled Posets;
- expected and observed counts of connected unlabeled Posets;
- the independent connected-component multiset reconstruction of the all-Poset
  count;
- comparison with the reference `PosetKey` set wherever feasible;
- uniqueness of Poset and complex keys;
- the labeled/unlabeled identity using automorphism orders;
- core replay, idempotence, opposite, and homology-invariance checks;
- order-complex reconstruction and Euler/homology checks;
- GAP or other oracle results, including sample selection and seed; and
- repeated/resumed output hash comparisons.

Coverage claims are separate from successful validation execution. At minimum
the report carries:

```json
{
  "isomorphism_scope": {"order": 4, "status": "complete"},
  "homotopy_scope": {"source_orders_through": 4, "status": "complete"}
}
```

Allowed status values are `complete`, `sampled`, and `constructed`. A complete
homotopy scope means that grouping the complete isomorphism records through
the stated source order by `has_core.target` gives every finite-space homotopy
fiber in that source range. It does not claim completeness for larger source
orders or classify weak homotopy types.

## 13. Deliberate omissions

Version 1 does not store:

- a TypeLake content id or physical Parquet layout;
- a TypeBase label such as `n.c.h.w.k`;
- a full order-relation matrix;
- a duplicate incidence-sequence field;
- order-complex summaries on the Poset row;
- boundary matrices or every face of the linked complex;
- reduced or mod-`p` homology;
- a transitive-action label;
- unverified abstract-group labels;
- generic fact-per-scalar envelopes; or
- record-level copies of batch provenance.

Mod-`p` Betti numbers are derivable from integral homology using the universal
coefficient theorem. Full faces or boundary matrices may be introduced later
as explicitly versioned caches if measured reconstruction cost justifies them.

## 14. Contract evolution

Adding a derived field requires a new record-contract version but does not by
itself change `PosetKey` or `ComplexKey`. Changing the principal-downset
canonicalization changes Poset identity and requires a new Poset encoding plus
an explicit equivalence map. Changing the complex defining bytes changes the
complex-key prefix and likewise requires a migration.

No accepted field may silently change convention. In particular, reduced
homology must not replace unreduced homology, edge-height must not replace
element-height, and an abstract automorphism group must not replace the stored
natural permutation action under the same field names.

## 15. Review points

The choices most worth reviewing before implementation are:

1. `PosetKey` entries use decimal strings, while bounded counts remain JSON
   integers.
2. `ComplexKey` hashes a small explicit binary facet encoding rather than
   canonical JSON.
3. Complex dimension is `-1` for the empty complex, matching Noether's signed
   public dimension convention; the builder performs no translation.
4. Generator arrays are normalized output evidence, but their particular
   choice is revision-dependent and is not Poset identity.
5. Mathematical JSONL rows omit run ids so that independent executions can
   reproduce identical bytes; file hashes in `run.json` bind rows to
   provenance.
