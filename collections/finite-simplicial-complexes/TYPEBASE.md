# TypeBase projection: finite simplicial complexes

Collection path: `/SimplicialComplex/`

Object path: `/SimplicialComplex/<vertices.dimension.index>`

The index is assigned only after the corresponding vertex shell is certified
complete. Pages show a compact geometric plate in low dimension and a
facet-incidence plate otherwise. Search fields include vertex count,
dimension, facets, face vector, Euler characteristic, components, integral
homology, strong core, witnessed simple collapses, and automorphism order.

The collection source page is generated from immutable run manifests. The
completeness page distinguishes complete isomorphism shells, theorem-complete
strong-core data, witnessed simple relations, complete finite-field cohomology
rings, and the exact range of stored cohomology operations.

## Implemented projection

The first static projection contains all 16,352 certified classes through six
vertices. It assigns labels only after loading the full identity frontier,
resolves strong-core and greedy-collapse targets to TypeBase labels, and checks
every selected input against its immutable manifest before publication.

The browser does not download the complete search index on first paint. It
loads a 1,752-byte `catalogue.json` and 15,067-byte, 30-row `landing.json`, then
fetches one of 22 compressed `(vertices, dimension)` search buckets when a
restriction determines a bucket. Paging beyond the landing rows may fetch the
291,819-byte complete set of bucket indexes. Search supports facets, face vector,
Euler characteristic, components, strong-core label and status, automorphism
order, and integral-homology degree/free rank and torsion.

Full identity, topology, and `F2`/`F3` records are split into 82 compressed
detail shards of at most 256 objects. The audited build occupies 4,311,835
bytes in 108 files; no detail shard exceeds 69,301 bytes. Two independent full
projections were recursively byte-identical. These are measured build figures,
not completeness claims or Noether constants.

The implemented reader routes are the catalogue and persistent object paths
above plus `/SimplicialComplex/Source`, `/Completeness`, and `/Glossary`. Source
and Completeness load the full retained manifest on demand; ordinary catalogue
and object pages do not.
