# TypeBase projection: finite-group cohomology

Collection path: `/FiniteGroupCohomology/`.

Coefficient paths are:

```text
/FiniteGroupCohomology/<lmfdb-label>/F<prime>
/FiniteGroupCohomology/<lmfdb-label>/Z
```

The group label links to
`https://www.lmfdb.org/Groups/Abstract/<lmfdb-label>`. Every result and record
also gives GAP's standard `StructureDescription`, such as `D8`, `Q8`, or
`C4 x C2`; this reader-facing name is external metadata rather than object
identity. The page opens with the graded-commutative quotient presentation,
generator degrees, and extent. A coefficient switch keeps the reader on the
same group.

The main mathematical panels are:

- a generator/operation graph, including Bocksteins, squares, and reduced
  powers when certified;
- a compact additive-dimension table and Hilbert--Poincare series;
- relations and normal-form basis;
- periodicity, Krull dimension, and depth when known;
- the completion certificate in plain language;
- source identity, exact permutation presentation, Noether revision, and
  artifact hashes.

Search fields include group order and SmallGroups id, coefficient prime,
generator and relation degrees, proof degree, completion method, named
operations, complete/truncated extent, LMFDB label, and standard group name.
Integral rows can also be restricted by cohomological degree, free rank, and
torsion order. Periodicity, Krull dimension, and depth remain future search
fields. The default search results remain compact adjacent rows rather than
cards.

The source page identifies the pinned GAP/SmallGrp and LMFDB sources. The
completeness page reports all 144 groups through order 32, modular coverage by
prime, the per-row proof-depth histogram, integral extent counts, and every
exception directly from the selected manifest. For the selected `modular-v7`
artifact all 198 required modular rows are complete; the depth histogram is
196 rows at 8, one at 10, and one at 12. A development truncation is visibly
marked and is never described as a presentation of the full ring.

The implemented TypeBase builder emits a compact JSON search index and one
Zstandard-compressed detail object per abstract group. A detail object bundles
the pinned permutation source with every available coefficient row, so changing
coefficients does not require downloading unrelated groups. Its manifest
rehashes both selected TypeLake inputs and every output.

The selected `integral-v4` family supplies 144 `/Z` rows. Of these, 46 are
theorem-complete presentations: one
cyclic group in every order through 32 and all 14 groups in the source range
with two invariant factors. Cyclic rows retain their two-periodic resolution
certificate. Rank-two rows display the two degree-two integral Bocksteins, the
degree-three integral Tor class, and the complete parity-sensitive Tor-square
relation. Search exposes `integral Bockstein` and `integral Tor`, as well as
generator and relation degrees. Every remaining row has an explicit
`no_claim_above_degree = true`. Eight now include a complete basis-level ring
truncation from a sparse normalized-bar calculation: seven through degree 3
and `6.1` through degree 4. The other 90 stop at the universal degree-two
range computed from abelianization. The combined search has 342 rows (198
modular and 144 integral), and its completeness table is generated from the
exact-degree histogram rather than assuming one truncation degree.

## Implemented reader

The static reader loads the 186,592-byte search index once and fetches one of
144 compressed group details only after selection; the largest detail is
53,450 bytes. It provides persistent coefficient routes, a coefficient switch,
LMFDB links, quotient presentations, named-operation and completion-certificate
panels, and an exact-extent panel for every bounded integral row. Sparse-bar
details additionally carry canonical cocycles and their full in-range product
table. The reference routes are
`/FiniteGroupCohomology/Source`, `/Completeness`, and `/Glossary`, all rendered
from the selected manifest. The audited component is 992,187 bytes in 146
files.
