# Finite simplicial complexes

**Status:** complete identity, topology, and `F2`/`F3` cohomology builders
through 6 vertices

This collection contains finite abstract simplicial complexes up to simplicial
isomorphism. Census scope and completeness evidence belong here in TypeLake;
Noether supplies operations on one complex.

## First edition

- complete identity shells through 6 vertices;
- order 7 is an expansion target gated by a measured branching run;
- exact strong-homotopy cores throughout every complete identity shell;
- replayable elementary-collapse relations, without claiming that a greedy
  terminal target is a canonical simple-homotopy representative;
- integral homology computed from the strong core;
- complete finite-dimensional `F2` and `F3` cohomology algebras, canonical
  cocycles, products, and every characteristic-two Steenrod square supported
  by the class degree in versioned enrichment passes.

The production generator adjoins a new vertex whose link is a subcomplex of a
canonical parent. It enumerates link antichains, quotients links by the parent
automorphism action, canonicalizes the child with Noether, and retains it only
when deleting the last vertex of the canonical child recovers the declared
canonical parent. A redundant labeled generator through four vertices is an
independent local oracle.

Known unlabeled and labeled shell totals are source data under `validation/`,
not Noether constants. The initial accepted counts are OEIS A006602 and
A006126, respectively.

Noether's former bounded `enumerate_simplicial_complexes` census API shipped in
v0.1 but crossed this ownership boundary. It was removed from Rust, PyO3,
Python, stubs, discovery metadata, and Noether-owned exhaustive tests at
Noether revision `5bdf6ba95ed5c11a2488b7553d21ab13e7970485`. The TypeLake reference
generator preserves the useful small-shell oracle: it agrees with the orderly
generator and the accepted counts `1, 1, 2, 5, 20` through four vertices.

## Build stages

1. `identity`: canonical facets, key, automorphisms, and complete shell proof;
2. `topology`: strong core, collapse witnesses, integral homology, and Euler
   checks (implemented and replay-validated);
3. `cohomology`: canonical cocycles, products, supported operations, and exact
   extent (implemented for `F2` and `F3`; the `F2` v2 pass stores all
   `Sq^k`, `0 <= k <= |x|`);
4. `publish`: immutable labels and TypeBase projections.

The stable TypeBase label is `vertices.dimension.index`, where `index` is
one-based canonical-key order inside a complete `(vertices, dimension)`
bucket. Noether and the identity builder do not assign this label.
