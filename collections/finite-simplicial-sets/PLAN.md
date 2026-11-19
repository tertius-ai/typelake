# Finite simplicial sets

Current record fields and encodings are specified in `RECORD.md`; browser behavior is specified in `TYPEBASE.md` where present. This plan also describes future work. Current release scope is selected in `../VOLUME-I.md`, and checks count as evidence only when an accompanying run report records their execution.

**Status:** complete identity, normalized-topology, `F2`/`F3` cohomology,
odd-primary Steenrod-operation, and replayable horn-collapse builders at the
measured first-edition frontier

This collection contains finite simplicial sets up to simplicial-set
isomorphism. Degenerate simplices are represented by their unique degeneracy
of a nondegenerate simplex and are not counted as stored cells.

## First edition

The census is finite only after bounding both total nondegenerate cells and
dimension. The measured identity frontier is:

- at most 6 nondegenerate simplices in dimension at most 3;
- 7 nondegenerate simplices in dimension at most 2;
- complete counts refined by the nondegenerate dimension vector;
- independent labeled/canonical comparison through total size 5;
- integral normalized homology for every accepted object;
- complete normalized `F2` and `F3` cohomology rings, with all unstable-range
  characteristic-two squares and odd-primary reduced powers and
  beta-reduced powers supported by the first-edition coefficient fields;
- every elementary horn-collapse relation and one deterministic replayable
  greedy horn-collapse sequence per object.

The benchmark found 89,531 classes at `(6, 3)` and 30,808 classes at `(7, 2)`.
The former identity shell builds and fully revalidates in about 66 seconds; the
latter in about 26 seconds on the reference machine. Total size 10 remains an
expansion target after stronger boundary pruning, not a first-edition
completeness claim.

The `topology-v2` enrichment was rebuilt over the whole frontier with Noether
revision `8911e74a15f9df93c65ad0074232db384ef83f91`. It contains 89,531
replayed reduction witnesses at `(6, 3)` (record hash
`d042865eb9489c49f50c79d9682ae2ca3a7682490cf168ae4e698f9f5b87f498`)
and 30,808 at `(7, 2)` (record hash
`8363f29fe9423e7f5e2f219ff2dd86514dc9430adbd8d9526826489fadb14eac`).
The identity artifacts remain linked by their original hashes and exact
Noether revision; changing an enrichment did not relabel the census.
Installed topology shells can be independently hash-checked, reloaded, and
mathematically revalidated with the builder's `validate-topology` command.

The `cohomology-v1` enrichment is installed across the same complete frontier
at both `F2` and `F3`, using Noether revision
`5ab3080d627b7b9633a085df8556c147b652c4ab`. Independent installed-artifact
validation passed all 89,531 `(6,3)` records in 247.9 seconds at `F2` and 172.2
seconds at `F3`, and all 30,808 `(7,2)` records in 60.6 seconds at `F2` and
52.6 seconds at `F3`.

The `homotopy-v1` enrichment is installed across every identity shell at the
same frontier using Noether revision
`3603f10148fc609475c5bdc1641e9f1b88701c2b`. The maximal `(6,3)` shell stores
89,531 records and 21,789 immediate horn-collapse edges; the `(7,2)` shell
stores 30,808 records and 11,717 immediate edges. Independent installed-file
revalidation replayed all relations and greedy sequences in 21.8 and 9.7
seconds respectively. The normalized sidecar occupies approximately 245 MB.

The additive `steenrod-operations-v1` enrichment is installed at `F3` across
the same frontier using Noether revision
`993dd0ff53ca7cf80e52a5d2f96b1659bce0b2a8`. It links the immutable
`cohomology-v1` artifact rather than duplicating its cocycles, products, or
presentations. The maximal `(6,3)` shell contains 668,055 operation values,
including 679 nonzero beta-reduced-power values, and independently revalidated
in 31.8 seconds; `(7,2)` contains 210,826 operation values, including 309
nonzero beta-reduced-power values, and revalidated in 12.1 seconds. The full
sidecar tree occupies approximately 248 MB.

The dimension cap is edition metadata, not a Noether limit. Higher-dimensional
minimal spheres enter later boxes even when their total nondegenerate size is
small.

## Orderly generation

Cells are adjoined in nondecreasing dimension. To add a nondegenerate
`d`-simplex, enumerate its `d+1` faces among all existing `(d-1)`-simplices,
including canonical degeneracies, reject failed simplicial identities, and
canonicalize the result with Noether. Among the top-dimensional cells of a
canonical child, delete each in turn and choose the least canonical parent
key. Retain the augmentation exactly when this canonical parent is the
declared parent. Parent automorphisms may quotient boundary assignments before
canonicalization without changing this rule.

A redundant generator retains every valid labeled augmentation and
canonicalizes only at the end. It is a validation oracle, not a production
path.

## Noether cleanup gate

No exhaustive simplicial-set enumerator belongs in Noether. A full repository
and history audit found that Noether never exposed one: the only related census
API was the bounded simplicial-**complex** function
`enumerate_simplicial_complexes`, removed at revision
`5bdf6ba95ed5c11a2488b7553d21ab13e7970485`. Production and redundant
simplicial-set enumeration have always lived in this TypeLake collection.

The removal inventory covers the Rust implementation and exports, PyO3
registration and wrappers, Python imports and stubs, public-surface metadata,
examples, tests, and documentation. Regenerate the disposable API material and
run Noether's import/discovery tests afterward to prove that no census entry
point remains. Keep only operations on a supplied `SimplicialSet` in Noether:
construction, validation, canonicalization, automorphisms, chain models,
reductions, and invariants.

The audit covered private modules, feature-gated surfaces, PyO3 registration,
Python projections, stubs, discovery metadata, examples, tests, documentation,
and repository history. No `SimplicialSet` census entry point exists in the
current Rust or Python surface. This gate is therefore passed rather than a
destructive no-op. TypeLake's redundant small generator preserves the
independent validation role, and production simplicial-set enumeration remains
in `simplicial-set-census`.

## Build stages

1. `identity`: nondegenerate face data, canonical key, automorphisms, counts;
2. `topology-v2`: normalized boundary certificates, integral homology, and a
   complete replayable log of deterministic universal-unit algebraic Morse
   cancellations on normalized chains (implemented);
3. `cohomology-v1`: canonical normalized cocycles, complete products and
   presentations over `F2` and `F3`, and all `Sq^k` for
   `0 <= k <= degree(x)` at `F2` (implemented);
4. `steenrod-operations-v1`: every `P^i(x)` with `2i <= degree(x)` and every
   `beta P^i(x)` with `2i < degree(x)` at `F3`, linked to the exact immutable
   cohomology source record (implemented);
5. `homotopy-v1`: all immediate elementary horn-collapse edges and one
   deterministic replayable greedy sequence per object (implemented); the
   reverse edge is the corresponding horn attachment/anodyne extension, while
   the greedy terminal target is not claimed to be a canonical homotopy core;
6. `publish`: immutable labels and TypeBase projection.

The stable label is `nondegenerate_simplices.dimension.index`, with the index
one-based in canonical-key order inside a complete bucket.
