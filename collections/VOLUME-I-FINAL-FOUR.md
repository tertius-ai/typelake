# TypeBase Volume I: the final four collections

**Status:** design plan and implementation notes; the catalogue is an editorial review candidate. Individual RECORD.md/TYPEBASE.md files and run evidence specify implemented fields, methods, and validation extent.

**Collections:** finite simplicial complexes, finite simplicial sets, finite-group
cohomology, and Eilenberg--Mac Lane anima

**Product rule:** Noether computes mathematics for one supplied object; TypeLake
owns enumeration, census scope, records, and evidence; TypeBase publishes a
frozen catalogue.

These are the last four collections planned for TypeBase Volume I. They should
reuse the Poset and strong-Steiner-category machinery where it is genuinely
shared, but they should not force a premature universal schema. Each collection
gets its own versioned contract and builder.

The most important correction to the initial plan is that **census information
does not belong in Noether**. Noether may return a canonical form, an
isomorphism certificate, an automorphism group, a reduction, or a cohomology
ring for an object supplied by its caller. It must not know which finite objects
TypeLake enumerates, where a complete shell ends, how many classes a shell
contains, or which position an object occupies in a TypeBase label bucket.

The second correction is an upgrade. Modular cohomology of a finite group need
not be published merely as an unexplained truncation. Symonds's regularity
theorem gives finite bounds on the degrees of generators and relations, and
smaller data-dependent completeness criteria are often available. We should
therefore make a **complete, machine-certified presentation** the normal target
for `H^*(BG; F_p)`.

## 1. Product boundary

### 1.1 Noether

Noether owns reusable operations on one supplied mathematical object:

- canonical forms, isomorphisms, and automorphism groups;
- exact chain and cochain complexes over supported coefficient contexts;
- simplicial reductions and explicit equivalence certificates;
- homology, cup products, cohomology operations, and graded-commutative
  presentations;
- resolutions and effective chain models for a supplied finite group or
  symbolic space;
- mathematical completion playbooks and typed evidence for a claimed complete
  ring presentation.

These operations may have generic and specialized implementations. Their
meaning must not depend on a TypeLake edition, a census cutoff, or an LMFDB
label.

Noether should expose no exhaustive census enumerator for either finite
simplicial complexes or finite simplicial sets. These enumerators are
specialized database-building programs, not natural operations on one supplied
mathematical object. Migrate their useful fixtures and reference logic to the
corresponding TypeLake builders. If an enumerator is already public in the 0.1
line, deprecate it there and remove it at the next allowed breaking release; an
unreleased or private enumerator can be removed immediately. Do not extend
either enumerator into the production census.

### 1.2 TypeLake

TypeLake owns all collection and census decisions:

- the orderly generators and canonical-augmentation rules;
- the finite shells selected for a build;
- expected and observed class counts;
- source versions for group lists and external comparison data;
- sharding, checkpoints, rejected candidates, merge rules, and run manifests;
- canonical bytes and content identifiers for accepted records;
- per-invariant coverage and completion status;
- exact evidence connecting every derived value to code and inputs.

In particular, known counts of simplicial complexes, the statement that all
groups of order at most 32 were processed, and the largest computed degree for
an Eilenberg--Mac Lane space are TypeLake facts. They should appear in source
files, manifests, and validation tests in this repository, never as constants
or public vocabulary in Noether.

### 1.3 TypeBase

TypeBase chooses immutable TypeLake batches, states a reader-facing
completeness claim, assigns stable labels, and constructs pages and searches.
The third coordinate of a label such as `6.2.137` depends on the complete
canonical ordering of a TypeBase bucket and therefore cannot be produced by
Noether.

## 2. Shared record vocabulary

The four contracts should share small value shapes only where the mathematics
is already the same. In particular, use one exact graded-commutative
presentation shape for group cohomology and Eilenberg--Mac Lane cohomology.

```text
GradedCommutativePresentation {
  coefficient_context
  generators: [
    { id, degree, parity, canonical_cocycle?, display_name?, name_evidence? }
  ]
  relations: [ homogeneous_polynomial ]
  monomial_order
  normal_form_basis
  hilbert_poincare_series?
  operations: [
    { operation, source_generator, target_normal_form, chain_witness? }
  ]
}
```

The ambient algebra, not an informal convention in the renderer, enforces the
Koszul sign rule. Over `F_p` for odd `p`, the square of an odd-degree element is
therefore zero. Over `Z`, an odd square need only be 2-torsion and must not be
silently replaced by zero.

Every ring-valued record also carries an honest extent:

```text
RingExtent =
  | CompletePresentation {
      certificate
    }
  | ExactThroughDegree {
      degree
      no_claim_above_degree: true
    }
```

A completeness certificate records the theorem or criterion, every numerical
bound used, and hashes of the exact computations that discharge its
hypotheses. A presentation is never marked complete merely because no new
generator appeared for several degrees.

Operation-based names are display metadata, not mathematical identity. A
generator keeps a stable canonical id and cocycle representative. If it can be
written as a Bockstein, Steenrod square, reduced power, or iterated operation on
an earlier named class, the builder selects a shortest deterministic expression
and stores the calculation witnessing it. Unnamed generators remain legitimate.

## 3. Collection 03: finite simplicial complexes

### 3.1 Object and equivalence relations

The stored object is a finite abstract simplicial complex on a canonical vertex
set. The primary catalogue is up to simplicial isomorphism.

Publish the following coarser structures separately:

1. the strong-homotopy core obtained by deleting dominated vertices;
2. explicit elementary-collapse and expansion certificates when found;
3. connected components under a declared, finite simple-move search when that
   search has a complete boundary;
4. homology and other homotopy invariants, which are invariants and not
   equivalence claims.

Strong cores are a particularly good first quotient because the finite core is
unique up to isomorphism. General simple-homotopy equivalence must not be called
complete merely because a bounded search found no connecting zigzag. Each
relation record states whether it is a theorem-complete classification or a
witnessed relation inside a bounded search.

### 3.2 Enumeration

The TypeLake builder uses orderly generation by canonical augmentation:

- represent a complex by its antichain of facets;
- adjoin one new vertex through a downward-closed link in the old complex;
- canonicalize the candidate and retain it only when deletion of the canonical
  last vertex gives the declared parent;
- quotient possible links by the parent's automorphism action before expensive
  canonicalization;
- parent/link sharding is future work; the current simplicial builders are not sharded.

This is the simplicial-complex analogue of adjoining a maximum over a downset
in the Poset builder. A redundant labeled generator remains useful only as a
small-shell oracle in TypeLake validation.

Provisional first edition:

- complete through 6 vertices;
- order 7 as a separately benchmarked expansion target, not a promise in the
  first manifest;
- exact strong cores throughout the complete region;
- complete elementary-collapse closure only for explicitly declared finite
  search regions.

The bounds are builder parameters and edition metadata. They are not Noether
API limits.

### 3.3 Noether work

Within the existing owning domains, add or improve:

- `topology::SimplicialComplex::canonical_form()`;
- a partition-refined automorphism calculation using vertex incidence data;
- domination, elementary collapse, and certificate-returning reduction
  operations;
- cup products and the chain-level diagonals needed by cohomology operations;
- exact JSON serialization of canonical complexes and reduction witnesses.

Canonicalization and automorphisms should share a colored incidence encoding
of vertices and facets. Brute-force vertex permutations remain only a small
input fallback.

### 3.4 Record and label

The TypeBase label is

```text
vertices.dimension.index
```

where `index` is one-based canonical-key order within the first two
coordinates. The object path is `/SimplicialComplex/<label>`.

The record includes facets, all face counts, dimension, components, Euler
characteristic, automorphisms, integral homology, selected field cohomology
rings and operations, the strong core, and every certified collapse relation.
The facet list is the stored object; rendered triangulations are replaceable
presentation assets.

### 3.5 View

Use one compact geometric plate when a low-dimensional realization is useful,
with vertices, edges, and oriented faces packed in the established TypeBase
visual language. Higher-dimensional complexes receive a facet-incidence view
and dimension filters rather than a misleading projection. Search supports
vertex count, dimension, number of facets, core, homology, Euler
characteristic, and automorphism group.

## 4. Collection 04: finite simplicial sets

### 4.1 Object

The stored object is a finite simplicial set encoded by its nondegenerate
simplices and the unique degeneracy decompositions of their faces. Degenerate
simplices are derived, not repeated indefinitely in the record.

The primary catalogue is up to simplicial-set isomorphism. The record also
stores explicit anodyne, collapse, subdivision, or other simple-equivalence
witnesses implemented by the accepted playbooks. As for complexes, a relation
found by a bounded move search is not promoted to a complete classification
without a terminating theorem.

### 4.2 Enumeration

Enumerate by adjoining one nondegenerate simplex:

1. choose its dimension;
2. assign its faces to existing simplices or forced degeneracies;
3. reject immediately when simplicial identities fail;
4. quotient assignments by the automorphism group of the parent;
5. keep the augmentation when the least canonical parent over deletions of top-dimensional simplices is the declared parent.

This generator belongs entirely to `simplicial-set-census`. The repository and
history audit found no Noether simplicial-set enumerator to remove; the related
bounded simplicial-complex census function was removed at revision
`5bdf6ba95ed5c11a2488b7553d21ab13e7970485`. Noether retains construction,
verification, canonicalization, automorphisms, and invariant computation for a
supplied `SimplicialSet`; it has no census entry point.

Current diagnostics record aggregate candidate and acceptance counts. Per-reason rejection counts are future instrumentation.

Provisional first edition:

- certify the generator against independent labeled enumeration through total
  nondegenerate size 8;
- target a complete census through total nondegenerate size 10 after the
  branching benchmark;
- refine every coverage statement by nondegenerate dimension vector.

The total number of nondegenerate simplices is the primary size only together
with a dimension cap: total size alone is not a finite shell, since the
minimal `n`-sphere has two nondegenerate simplices for every `n > 0`.
Completeness is therefore stated for finite boxes
`(total nondegenerate simplices <= N, dimension <= D)` and reconciled by the
full nondegenerate dimension vector.

### 4.3 Noether work

- canonical form and automorphisms for `topology::SimplicialSet`;
- normalized chains with certificate-preserving simplicial DMT;
- exact realization/cell models needed for homology and cup products;
- finite simple-map primitives with explicit witnesses;
- chain-level cup-i operations at 2 and May--Steenrod operations at odd primes.

Do not use the finite `SimplicialSet` type to pretend that the generally
infinite simplicial sets `BG` and `K(A,n)` are finite. Those collections use
symbolic spaces with effective chain models and an explicit requested degree.

### 4.4 Record, label, and view

The TypeBase label is

```text
nondegenerate_simplices.dimension.index
```

with path `/SimplicialSet/<label>`. The record stores the nondegenerate
simplices, every face target and degeneracy word, canonical key,
automorphisms, normalized homology, cohomology data, and all certified
equivalence relations.

The main plate shows nondegenerate simplices only. Faces identified with one
another receive matching edge or face marks; degeneracies are available as a
secondary combinatorial panel. This keeps the picture faithful without drawing
an infinite tower of degenerate simplices.

## 5. Collection 05: cohomology of finite groups

### 5.1 Scope and identity

For each selected finite group `G`, publish

```text
H^*(BG; Z)
H^*(BG; F_p) for every prime p dividing |G|.
```

The group identity is its pinned LMFDB abstract-group label when available,
with the exact permutation presentation and upstream SmallGroups identity also
stored. The LMFDB label is an external name, not Noether object identity.

Provisional first edition: every group of order at most 32. The group list,
exceptions, upstream versions, and completion counts live in TypeLake coverage
manifests. Pages use paths such as

```text
/FiniteGroupCohomology/8.3/F2
/FiniteGroupCohomology/8.3/Z
```

and link `8.3` directly to the corresponding LMFDB group page.

### 5.2 Complete modular presentations

For `H^*(BG; F_p)`, a complete presentation is the default target, not a fixed
degree truncation. The safe completion playbook has a family lane and
theorem-bound resolution lanes.

#### Finite abelian classifying spaces

When the supplied concrete permutation group is finite abelian, Noether first
certifies its canonical invariant factors `A` and then uses `BG = K(A,1)`.
The resulting closed-form graded-commutative presentation, named generators,
primary operations, and reconstruction hash form a complete family
certificate. This lane does not infer completeness from a short resolution
truncation.

#### Universal Symonds bound

Symonds proved that the mod-`p` cohomology ring of a finite group has
Castelnuovo--Mumford regularity zero. A useful consequence is:

- if a non-cyclic finite group has a faithful complex representation that is a
  sum of irreducibles of dimensions `n_i`, set `S = sum_i n_i^2`;
- algebra generators occur in degrees at most `S`;
- defining relations occur in degrees at most `2S`.

Independently, `|G|-1` is always a valid generator-degree bound, with twice
that as a relation-degree bound; this also covers cyclic groups. Thus the
builder should search for a small faithful sum of irreducibles and use the
order bound as a correctness fallback. Once the proposed generators give a
surjection and that surjection is verified to be an isomorphism through degree
`2S`, it is an isomorphism in every degree.

The following universal-bound certificate design is a proposal, not an accepted record variant. Implemented records use `SymondsUniversalBound` or `SymondsParameterCriterion` as specified in the collection RECORD.md.

```text
method = "symonds-generator-relation-bound"
prime
faithful_representation
irreducible_dimensions
faithfulness_witness
generator_bound = S
relation_bound = 2*S
computed_through_degree
presentation_hash
comparison_hashes
```

#### Smaller data-dependent completion

The practical Symonds criterion is often substantially cheaper. Suppose a ring
approximation through degree `n` contains a homogeneous system of parameters
of degrees `d_1,...,d_r` which is certified to lift to a system of parameters
of the true cohomology ring. Let `N` be the largest degree needed to generate
the approximation as a module over those parameters. If

```text
n >= max(N, d_1 + ... + d_r - r + 1),
```

then the approximation is the complete ring. The certificate records the
parameters, their restriction tests on maximal elementary abelian
`p`-subgroups, the module generators, `N`, and the verified inequality.

The implementation may also support the Benson, modified Benson, or
Hilbert--Poincare criteria. These are separate certificate variants with
separate hypotheses, not a generic `complete = true` flag.

### 5.3 Computation and validation

Implementation checkpoint (2026-08-30): Noether revision
`ef61b0e99ba5196a5697bd5573683eca509e46c1` supplies checked sparse minimal
p-group resolutions, a normalized-bar correctness oracle, deterministic Sylow
subgroups, resolution comparison maps, stable-element intersections, canonical
cohomology bases, and cup products through a requested degree. Pinned timings
are 68--77 ms for `C_2^5` through degree 4 and 40--48 ms for `S_4` mod 2 through
degree 5; native `S_3`, `A_4`, and `S_4` dimensions agree with HAP. Batched left
multiplication builds the full degree-4 `C_2^5` product table in about 2.28
seconds. It also extracts minimal homogeneous generators and all relations
visible through an exact bound; the optimized `C_2^5` cohomology-plus-extraction
regression through degree 4 takes 0.98 seconds. TypeLake owns
a pinned 144-group GAP/SmallGrp source artifact and an immutable, fully
recomputed development census through order 32 and degree 4. Its 198 relevant
coefficient rows occupy 1,893,859 bytes and contain 593 minimal generators and
231 relations visible through degree 4. Those statistics describe the
preserved `modular-v2` artifact: all rows are `ExactThroughDegree`, and its
manifest makes zero complete-presentation claims. Noether now also returns a
typed, recomputable Symonds universal-bound certificate when an exact algebra
range is sufficient; the separate `modular-v3` contract stores that certificate
without rewriting v2. The full v2 build and recomputation takes 824.732 seconds
on the pinned benchmark CPU; an independent artifact reload repeats every check
in 413.943 seconds. Complete presentations for the remaining rows, operations,
and their certificates remain release gates; this checkpoint is not described
as a complete ring census.

The installed `modular-v3` degree-4 artifact has 198 rows in 1,894,295 bytes
and SHA-256
`dc9b8ff2a5c64b813f1d9ff8a176b05c97cb2e04c754eaab08bca4caea0b7ff1`.
It certifies complete presentations for exactly `2.1` over `F_2` and `3.1`
over `F_3`; the other 196 rows remain exact truncations. Its conservative
build-and-replay pass took 1,418.780 seconds, and a separate disk reload and
full mathematical recomputation took 432.786 seconds with every record and
certificate reproduced.

Noether revision `2841882b7adbaa6519248caf74df0730067c0a53` and TypeLake
contract `finite-group-cohomology/modular-v4` add the finite-abelian
classifying-space lane. The installed order-through-32, degree-4 artifact has
all 144 groups and 198 coefficient rows. It certifies 74 complete rows from 54
nontrivial finite abelian groups; 124 nonabelian rows remain honest degree-4
truncations. Its 1,430,433-byte JSONL has SHA-256
`44a705de8906279537821f270cdf691ace44e473b03478ab229c0da93356d0d2`.
Construction plus in-memory replay took 650.486 seconds, and an independent
installed-file hash check and mathematical recomputation took 349.911 seconds.

Noether revision `106fa9e922334fea5461aa67eb51dea445f70cfc` adds exact
Symonds-parameter certificates. The TypeLake `modular-v5` degree-8 pilot
certifies 196 of 198 rows: 74 by the finite-abelian family theorem and 122 by
the parameter criterion. The remaining rows are exactly `32.8/F_2` and
`32.44/F_2`; targeted computation certifies them at degrees 12 and 10 after
Noether revision `82724ee956faecdac3fdbbec734aae7eb9d5e16c` replaces the
prime-field dense relation reduction with sparse RREF. The release artifact
will record these per-row proof depths rather than impose degree 12 on every
row.

That release artifact is now built under contract `modular-v6`. All 198 rows
are complete: 74 finite-abelian family certificates and 124 Symonds-parameter
certificates. Its proof-depth histogram is `{8: 196, 10: 1, 12: 1}`; it has no
modular truncations. The 15,229,985-byte JSONL has SHA-256
`d7e25caa5a850a2774d1b27f86605b23aee4b4db6b3299c525a3ea9a1092bfd9`.
An independent installed-file reload and full mathematical recomputation took
1,429.168 seconds and reproduced the stored validation report exactly.

Noether revision `76248daf94ce49a9d297fc6e29f5f9cbda775b27` adds typed
admissible primary-operation words and operation-adapted generator witnesses
modulo decomposables, with the witness payload included in each canonical
algebra digest. TypeLake contract `modular-v7` retains all 198 complete rings
and the proof-depth histogram `{8: 196, 10: 1, 12: 1}`. Its
15,273,124-byte JSONL has SHA-256
`91f1b0bddf3ea8d8fb4201912fe14ace5bd2778bfab5fb2f00863b4a0a810058`;
construction plus immediate replay took 2,708.834 seconds. An independent
installed-file reload and complete mathematical recomputation took 1,361.764
seconds and reproduced the stored validation report exactly.

The native correctness lane is:

1. construct a deterministic projective or bar resolution from the supplied
   finite group;
2. reduce it with certified homological algebra;
3. compute cochains and exact cup products;
4. find minimal homogeneous algebra generators and a homogeneous relation
   ideal incrementally;
5. compute Steenrod operations on canonical cocycles;
6. apply a completion criterion;
7. verify the final presentation against independently recomputed dimensions,
   products, restrictions, and its Hilbert--Poincare series.

HAP and `p_group_cohomology` are valuable parity oracles and possible accepted
subprocess accelerators. Their absence must not change the meaning of a result,
and imported presentations are not accepted without translation and exact
validation.

### 5.4 Integral cohomology

Symonds's theorem above is a theorem about modular cohomology. It does **not**
by itself certify a presentation of `H^*(BG; Z)`.

For the integral record, compute exact additive groups, products, restrictions,
transfers, and Bocksteins. Separate the positive-degree ring into its
`p`-primary information and record compatible cocycle lifts. The first edition
has two honest outcomes:

- `CompletePresentation`, when an implemented integral completion theorem,
  periodic resolution, or family-specific proof supplies a certificate;
- `ExactThroughDegree`, otherwise, with no implied vanishing or stability
  above the stated degree.

Before freezing this collection, run a focused mathematical work package on a
uniform integral completion criterion. One promising route is a finite
Bockstein tower for each `p` dividing `|G|`, together with chain-level products
and the exponent bound supplied by the finite group. Until that route is
proved sufficient and encoded as checkable evidence, it is a research plan,
not a completeness claim.

This distinction still materially improves the collection: every modular ring
should normally be complete, while integral records state their exact extent
rather than presenting a truncation as a ring.

### 5.5 Names and reader view

Generators receive canonical ids such as `x1`, `x2`, and stable mathematical
display names when justified:

```text
u
beta(u)
Sq^2(u)
P^1(v)
beta P^1(v)
```

The page shows the quotient presentation first, followed by generator degrees,
an operation graph, additive dimensions, and the completion certificate in
plain language. Readers can switch coefficient rings without leaving the group
page. Search supports group order and structure, prime, generator and relation
degrees, periodicity, Krull dimension, depth, and named operations.

## 6. Collection 06: Eilenberg--Mac Lane anima

### 6.1 Scope

This is a parameterized value collection, not a census up to isomorphism.
Begin with:

- finite abelian groups `A` of order at most 32;
- `A = Z` as the first infinite input;
- `1 <= n <= 4`;
- integral homology and cohomology, plus `F_p` coefficients for primes relevant
  to `A`;
- exact data through total degree 16 initially.

Every bound belongs to the TypeLake build manifest and can grow without a
Noether release.

The path is explicit, for example:

```text
/EilenbergMacLane/8.2.2/Z
/EilenbergMacLane/8.2/2/F2
/EilenbergMacLane/Z.3/F3
```

Here the finite abelian group label should agree with the chosen LMFDB group
label and link to it. The page title also gives the invariant-factor
decomposition, so the external label is never the only description of `A`.

### 6.2 Effective computation

Add symbolic `ClassifyingSpace(G)` and `EilenbergMacLaneSpace(A,n)` inputs that
produce effective chain data only through an explicit degree. They are not
subclasses of a finite simplicial-set value.

The playbook should combine:

- normalized bar constructions in the smallest practical cases;
- tensor and twisted tensor products;
- Eilenberg--Zilber reductions;
- certified DMT before Smith or field reduction;
- effective-homology models for iterated classifying-space constructions;
- independent low-degree comparisons with known Cartan--Serre descriptions.

Store integral homology groups, integral cohomology groups and products, mod-`p`
cohomology rings, Bocksteins, Steenrod operations, and the reduction evidence
used to obtain them. Ring extents are normally degree-bounded here unless a
known general presentation theorem is implemented for the particular family.

### 6.3 Naming and display

Eilenberg--Mac Lane cohomology is the natural place to make operation names
central. Begin with the fundamental class `iota_n`; derive names by admissible
operation words and deterministic simplification. Store the actual cohomology
class and operation proof behind each name.

The reader page combines:

- the group and delooping degree;
- homology and cohomology tables by degree;
- a compact generator-and-operation genealogy;
- ring presentations through the certified extent;
- links from a class to every group-cohomology record in which it appears via a
  classifying map, once those maps are computed.

## 7. Builder layout

Use collection-specific crates in TypeLake rather than a universal census
framework:

```text
crates/simplicial-complex-census/
crates/simplicial-set-census/
crates/finite-group-cohomology-census/
crates/eilenberg-maclane-build/
```

They may share private utilities for manifests, content hashing, Parquet
projection, and restartable shards. Mathematical algorithms must still enter
through Noether's public Rust facade.

Collection documents live in the following files where implemented; fixtures live in crate tests and collection validation directories, rather than a common `fixtures/` layout:

```text
PLAN.md
RECORD.md
TYPEBASE.md
```

This document supplies the common plan; the individual documents freeze exact
canonical encodings and release scopes when implementation begins.

## 8. Implementation sequence

1. **Shared algebra.** Implement exact graded-commutative presentations,
   homogeneous ideals and normal forms, canonical cocycles, cup products, and
   operation records in Noether.
2. **Simplicial complexes.** Add canonicalization, automorphisms, and certified
   reductions; build and independently certify the first complete TypeLake
   shells.
3. **Simplicial sets and Noether cleanup.** Audit the Rust facade, PyO3
   transport, Python projection and stubs, public-surface metadata, examples,
   tests, documentation, and repository history for a Noether-owned
   simplicial-set census entry point. The completed audit found none; the
   related bounded simplicial-complex enumerator was removed at revision
   `5bdf6ba95ed5c11a2488b7553d21ab13e7970485`. Do not introduce a replacement.
   Keep Noether's reusable operations on a supplied `SimplicialSet`.
   Validate TypeLake's canonical augmentation against its redundant small
   enumerator. Store odd-primary `P^i` and `beta P^i` as an additive sidecar
   linked to the immutable cohomology record and complete throughout the
   unstable range, rather than rewriting the installed ring census.
4. **Modular group cohomology.** Implement deterministic resolutions and the
   Symonds completion-certificate variants; complete all selected `(G,p)`
   records before optimizing with external accelerators. The release parity
   gate now checks one deterministic group in every one of the 45
   `(order, prime)` buckets through order 32 against GAP/HAP through degree 3.
5. **Integral group cohomology.** Compute exact rings to a declared bound and
   investigate a uniform completion theorem without delaying honest modular
   publication. The preserved TypeLake contract `integral-v2` stores 46
   complete rings through order 32: all 32 cyclic groups via periodic
   resolutions and all 14 rank-two finite abelian groups via the
   classifying-space theorem. Contract `integral-v3`, pinned to Noether
   revision `76248daf94ce49a9d297fc6e29f5f9cbda775b27`, retains those presentations
   and adds the universal exact degree-two truncation for each of the other 98
   groups. Its 144-row JSONL is 157,588 bytes with SHA-256
   `49e06b66a2481089c71e2c7b8e5ac4d1bdcfba2f88737e8b828a1fabe09de7c8`;
   no integral group is now pending, and every incomplete ring explicitly
   states `no_claim_above_degree = true`.
   Contract `integral-v4`, pinned to Noether revision
   `2eb46dcf7f6b7e35f28d7a356e67755a001e6880`, retains those immutable
   checkpoints and applies sparse normalized-bar computation under a declared
   100,000-contribution census budget. It gives seven non-family groups exact
   rings through degree 3 and `6.1` through degree 4; the remaining 90 retain
   the universal exact degree-two extent. Canonical cocycles and every ordered
   basis product in range are stored and replayed.
6. **Eilenberg--Mac Lane models.** Reuse the chain, ring, and operation layer to
   build the initial parameter box. The first native checkpoint is pinned at
   Noether revision `2d6c5dc084ad8ac0b43a39bf3f83460028da0e23`: symbolic
   `K(A,n)` values, exact Kunneth homology for every finitely generated
   abelian `K(A,1)`, the `K(Z^r,2)` closed form, and typed failure outside that
   regime. TypeLake contract details and the honest partial-coverage build are
   owned by `collections/eilenberg-maclane/`. The next bounded checkpoint is
   pinned at Noether revision
   `4d5987a63ff4238939647b895d1f2104a7bd2500`: Cartan--Serre admissible-square
   presentations over `F_2`, exact through the requested degree for free and
   elementary 2-primary coefficient groups. TypeLake contract `invariants-v4`
   records 78 complete closed forms and 53 additional exact-through-degree
   presentations without promoting either kind into a census concern in
   Noether. The odd-primary checkpoint at Noether revision
   `ad7a6281f1f3deb1f4c289ff1a8c5e2fe93435b9` adds admissible
   Bockstein/reduced-power words for free and elementary `p`-primary groups.
   TypeLake contract `invariants-v5` records 235 of 304 modular rows; its 69
   explicit pending rows are exactly the higher-primary-exponent cases. The
   higher-primary checkpoint at Noether revision
   `6bd44e51f639b35a1352c25fb67b2de79c6aeb40` adds the fundamental and
   higher-Bockstein Cartan seeds for every cyclic primary exponent. TypeLake
   contract `invariants-v6` consequently records all 304 requested modular
   rows with zero pending modular cells. The integral-ring checkpoint at
   Noether revision `ceca3fa021a9313181b48b00ccedf5d9645e614f` adds complete
   graded-commutative `Z`-presentations for points, the free/cyclic
   classifying-space family with at most one cyclic torsion factor, and free
   second deloopings. TypeLake contract `invariants-v7` records 37 complete
   integral rings and leaves the other 187 ring cells explicitly pending.
   The rank-two checkpoint at Noether revision
   `cb2619170e52a11a6791b4d44df5f74771b7268d` adds the degree-three Tor class
   and the parity-sensitive Tor-square relation for classifying spaces of
   finite abelian groups with two invariant factors. TypeLake contract
   `invariants-v8` records 51 complete integral rings and leaves 173 ring cells
   explicitly pending.
7. **TypeBase projection.** Freeze labels, definitions, source pages,
   completeness pages, searches, and compact collection-specific galleries.
   The static data projection is implemented for all four collections. The
   simplicial-complex edition contains 16,352 classes through six vertices;
   the simplicial-set edition contains 125,843 classes at the accepted mixed
   frontier. Both use small compressed detail shards and resolve stored
   reduction targets to stable TypeBase labels. The finite-group and
   Eilenberg--Mac Lane projections retain their explicit pending integral
   coverage rather than turning it into a release claim. Browser integration
   is complete: all four collections have compact adjacent-row searches,
   persistent object/coefficient routes, collection-specific detail plates,
   LMFDB links where applicable, and manifest-backed Source, Completeness, and
   Glossary pages. Simplicial catalogue startup uses 30-row landing files and
   label-bucket shards; their full release manifests are fetched only by the
   reference pages. The production build emits an exact static `404.html`
   route loader, so no application server is required.

The order is intentional: the complexes exercise canonicalization and
reductions; simplicial sets exercise identities and normalized chains; modular
group cohomology forces complete graded algebra; Eilenberg--Mac Lane anima then
reuse all three layers.

## 9. Release gates

No collection enters Volume I until all applicable gates pass:

- canonical round-trip and independent small-shell enumeration;
- deterministic output across shard counts and worker ordering;
- exact count reconciliation for every shell claimed complete;
- automorphism orbit--stabilizer checks;
- chain identities and explicit reduction-witness verification;
- homology Euler-characteristic and universal-coefficient checks;
- graded-commutativity, associativity, and relation-ideal checks;
- operation degree, instability, Adem, Cartan, and Bockstein checks in the
  supported range;
- independent CAS parity on a declared sample;
- machine-verifiable completion evidence for every ring marked complete;
- source and completeness pages generated from the pinned manifests.

## 10. Mathematical anchors

- Peter Symonds, [*On the Castelnuovo--Mumford regularity of the cohomology
  ring of a group*](https://personalpages.manchester.ac.uk/staff/peter.symonds/preprints/coreg.pdf),
  especially Proposition 10.2 for generator and relation bounds.
- The `p_group_cohomology` documentation on the
  [Symonds and related completion criteria](https://users.fmi.uni-jena.de/cohomology/documentation/pGroupCohomology.html).
- HAP's [computational group cohomology examples](https://gap-packages.github.io/hap/tutorial/comp.pdf),
  including the direct generator/relation-bound verification strategy.
- Jonathan Barmak and Elias Minian,
  [*Strong homotopy types, nerves and collapses*](https://arxiv.org/abs/0907.2954).
- Anibal Medina-Mardones, [*New formulas for cup-i products and fast
  computation of Steenrod squares*](https://arxiv.org/abs/2105.08025), and
  [*May--Steenrod structures*](https://arxiv.org/abs/2010.02571).
- Ana Romero and Julio Rubio,
  [*Computing the homology of Eilenberg--Mac Lane spaces*](https://arxiv.org/abs/1107.3396).

These references guide mathematical implementations and certificates. Exact
software versions and imported datasets belong in the corresponding TypeLake
run manifests.
