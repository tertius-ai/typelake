# TypeBase projection: finite simplicial sets

Collection path: `/SimplicialSet/`

Object path:
`/SimplicialSet/<nondegenerate_simplices.dimension.index>`.

The main plate shows nondegenerate simplices only. Matching face marks show
identifications, while canonical degeneracy words appear in a secondary panel.
Search includes total nondegenerate size, dimension vector, dimension,
normalized homology and cohomology, Steenrod-operation values, Euler
characteristic, automorphism order, greedy horn-collapse target, and witnessed
horn-equivalence component. Source and completeness pages state the finite
`(size, dimension)` box and coefficient coverage explicitly.

At `F3`, TypeBase merges `steenrod-operations-v1` only after its per-record
cohomology digest and manifest-level source hashes match the selected
`cohomology-v1` edition. This adds `P^i` and `beta P^i` values without changing
the immutable ring record or its label.

The topology panel distinguishes a normalized-chain reduction from a
simplicial simple equivalence. For the former it reports source and reduced
cell counts and exposes the replayable universal-unit cancellation log; it
does not label the target as a simple-homotopy representative.

The homotopy panel lists every elementary horn collapse as a link to its
canonical target and exposes the deterministic replay sequence. The reverse
direction is described as a horn attachment/anodyne extension. A terminal
greedy target is presented as a reduction endpoint, never as a canonical
homotopy core. Components shown in the browser are the undirected closure of
the stored witnessed moves within the published complete frontier.

## Implemented projection

The first static projection contains 125,843 certified isomorphism classes:
all 95,035 classes with at most six nondegenerate simplices and dimension at
most three, together with all 30,808 classes having seven nondegenerate
simplices and dimension at most two. Labels are assigned only after the entire
frontier has been loaded. Each shell must agree row-for-row and hash-for-hash
with its manifest.

The projection resolves every elementary and greedy horn-collapse target to a
TypeBase label. Its witnessed-component label is the first member in canonical
catalogue order, not a claim of a canonical homotopy core. Full identity,
topology, `F2`/`F3` cohomology, odd-primary operations, and horn witnesses are
stored in compressed detail shards of at most 128 objects.

The browser first loads a 2,249-byte `catalogue.json` and a 19,568-byte,
30-row `landing.json`. It fetches one of 25 compressed
`(total nondegenerate, dimension)` search buckets when the label or selected
restrictions determine a bucket; paging without a restriction may fetch the
1,567,156-byte complete compressed index. Available restrictions include the
nondegenerate cell vector, Euler characteristic, automorphism order,
integral-homology degree/free rank and torsion, normalized/Morse cell counts,
horn-collapse existence, and witnessed component.

The audited projection occupies 34,841,865 bytes in 1,032 files. It has 1,003
compressed detail shards of at most 128 objects, and the largest is 36,862
bytes. Two independent full projections were recursively byte-identical. The
reader implements the catalogue and persistent object paths above plus
`/SimplicialSet/Source`, `/Completeness`, and `/Glossary`; only the reference
pages load the 217,786-byte full manifest. These measurements describe the
projection process and are not embedded as Noether mathematics.
