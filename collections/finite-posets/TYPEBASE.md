# TypeBase Volume I — finite Posets

**Status:** working contract for the first Volume I component

This document defines the editorial finite-Poset view selected from the
certified `finite-posets/v1` census. It does not change census identity. A
`PosetKey` remains the mathematical join key; the labels below are stable names
inside TypeBase Volume I.

## Scope

The isomorphism catalogue contains one row for every finite Poset through order
10. The homotopy catalogue contains the canonical cores reached from those
rows, with complete source fibers through order 10. A core is an ordinary
catalogued Poset and therefore has an isomorphism label of its own.

The catalogue does not contain one row for every labeling of a Poset. The
labeled-poset totals reconstructed from `n! / |Aut(P)|` are validation evidence,
not stored objects.

## Labels

Every Poset label has the form

```text
a.b.c
```

where:

- `a` is the number of vertices;
- `b` is the dimension of the order complex, equivalently the number of
  vertices in a longest chain minus one; and
- `c` is the one-based position in canonical `PosetKey` order among Posets with
  the same `(a, b)`.

Thus labels are deterministic consequences of a completed isomorphism census.
They are assigned only after the complete bucket has passed certification. The
empty Poset has order-complex dimension `-1` and label `0.-1.1`. This is the
only negative label component.

Labels sort numerically by the tuple `(a, b, c)`, not as text. The label map is
an immutable artifact of Volume I. Later TypeLake enrichments do not renumber
it. A future TypeBase volume may define another namespace, but it must not
silently change Volume I labels.

`has_core` and `opposite` are rendered as links to the target's label in this
same catalogue. The homotopy view may assign additional catalogue labels to
core classes later; those names do not replace the isomorphism labels.

## Automorphism groups

Each row retains the faithful natural permutation action on the canonical
vertices. When the underlying abstract group has been independently identified,
the view also carries its LMFDB abstract-group label, such as `8.3`, and links
to

```text
https://www.lmfdb.org/Groups/Abstract/<label>
```

The LMFDB label names the abstract group, not the natural action. Labels of
trivial and prime-order groups follow because the order determines the
isomorphism class. All other published labels are produced by GAP `IdGroup`
where `IdGroupsAvailable` is true. Unidentified groups have no external label;
the browser never guesses one from the group order alone. The GAP version,
Small Groups id, source key, output hash, and coverage totals are stored in the
separate immutable enrichment.

## Browser projection

The public browser is a static application over immutable, Zstandard-compressed
Parquet. It uses two projections:

1. a narrow search table containing labels, scalar facets, relation labels,
   automorphism-group order and optional LMFDB label; and
2. label-range detail parts containing canonical keys, covers, per-vertex
   counts, full natural actions, core evidence, and linked order-complex data.

Detail parts contain at most 10,000 consecutive rows from one `(order,
dimension)` bucket. A label therefore determines its detail part without a
server-side index. The TypeLake object, relation, and order-complex batches
remain independently addressable even if this browser projection denormalizes
some fields for a single efficient read.

## Hasse diagrams

The browser constructs a bounded SVG from the exact canonical covers. The
horizontal coordinate is determined by `strict_lower_counts`, so every cover
edge points from left to right. Vertices with the same count form a discrete
column and are ordered deterministically by canonical vertex id, with a bounded
barycentric refinement used only for presentation. The SVG is a rendition and
does not participate in Poset identity.

Client-side rendering keeps the distribution small and gives every Poset a
diagram without storing millions of nearly identical SVG wrappers. A future
rendered-SVG batch may be added for citation, accessibility, or print without
changing the object or label.

## Publication rule

The finite-Poset component may enter Volume I only when:

- every source order has a passed immutable census report;
- every order-10 shard has passed full independent record reconstruction;
- all core, opposite, and order-complex targets resolve;
- the label map is complete and unique;
- the Parquet projection reproduces source row counts and selected values; and
- a cold browser test proves that an exact-label lookup reads only bounded
  one bounded detail part rather than downloading an entire order.
