# Finite-group cohomology collection plan

Current record fields and encodings are specified in `RECORD.md`; browser behavior is specified in `TYPEBASE.md` where present. This plan also describes future work. Current release scope is selected in `../VOLUME-I.md`, and checks count as evidence only when an accompanying run report records their execution.

**Collection:** TypeBase Volume I, collection 05

**Contracts:** `finite-group-cohomology/modular-v7` and
`finite-group-cohomology/integral-v4`

The first edition covers every SmallGroups class of order at most 32. TypeLake
owns that cutoff, the pinned source list, build manifests, completeness counts,
and exceptions. Noether receives one concrete finite group at a time and owns
resolutions, cochains, products, operations, and completion certificates.

## 1. Source groups

The checked-in source artifact is generated from GAP's SmallGrp catalogue and
contains exactly one deterministic regular permutation presentation for each
`SmallGroup(n,k)`, `n <= 32`. In this range the LMFDB abstract-group label is
`n.k`; the record stores both names rather than identifying Noether objects by
either external catalogue.

This convention is pinned to the [LMFDB abstract-group label
specification](https://www.lmfdb.org/Groups/Abstract/Labels), not inferred from
page order.

Each source identity contains:

- the LMFDB label and URL;
- the SmallGroups pair;
- an exact zero-based permutation presentation;
- GAP and SmallGrp versions;
- a SHA-256 digest of the canonical Noether group bytes after reconstruction.

The raw GAP artifact stores the label, pair, and presentation; the builder
derives the LMFDB URL and Noether digest so the source file has no
Noether-version-dependent field.

Validation proves 144 unique labels, the accepted per-order SmallGrp counts,
the requested group order, and generator reconstruction. Regenerating with a
new catalogue version creates a new immutable source artifact.

## 2. Modular computation

For each group and each prime dividing its order:

1. first test the concrete permutation group for finite-abelian invariant
   factors;
2. for a finite abelian group, identify `BG = K(G,1)` and store Noether's
   complete closed-form presentation, named generators, primary operations,
   and decomposition certificate;
3. otherwise construct a deterministic free resolution through the current
   proof bound;
4. compute canonical cohomology bases and exact cup products;
5. select minimal homogeneous algebra generators incrementally;
6. compute the homogeneous relation ideal and normal forms;
7. compute typed Bocksteins and Steenrod operations on canonical cochains and
   retain operation-adapted generator witnesses modulo decomposables;
8. apply a typed completion criterion;
9. independently recheck the applicable family certificate or the dimensions,
   products, operations, and Hilbert--Poincare data.

Noether's initial native playbook uses a minimal resolution over `F_p[G]` when
`G` is a p-group and the normalized bar resolution as the general correctness
floor. The production path for non-p-groups will compute the stable elements
in a Sylow p-subgroup. The bar path remains the small exact oracle and is never
treated as an acceptable order-32 scaling strategy.

Every intermediate build is honestly `ExactThroughDegree`. Promotion to
`CompletePresentation` requires one of the following replayable certificates:

- the universal Symonds generator/relation bound;
- the data-dependent Symonds parameter criterion;
- a separately implemented Benson or Hilbert--Poincare criterion;
- a family-specific periodic-resolution proof.

The first release gate is a complete modular presentation for all 144 groups
and every prime divisor of their orders. A row that reaches only a truncation
may remain in a development artifact but cannot satisfy the release manifest.

## 3. Integral computation

Integral records store exact additive groups and products. Complete rows use a
periodic or family-specific finite-presentation certificate. A feasible
arbitrary-group row stores canonical normalized-bar cocycles and the complete
ordered basis product table in its exact range. All other rows use the
universal identities through degree 2. Every incomplete row has
`ExactThroughDegree { no_claim_above_degree: true }`.

The collection policy chooses the largest requested degree whose final
normalized-bar differential has at most the manifest's `bar_entry_limit`
estimated sparse contributions. This is a deterministic census budget,
separate from Noether's larger implementation safety ceiling. The budget,
selected per-group degrees, and their histogram are immutable manifest data.
Restrictions, transfers, and compatible Bockstein lifts remain future
enrichments and must not be inferred from the current records.

## 4. Build artifacts

The builder emits one immutable JSONL artifact ordered by `(group order, group id,
coefficient)`. A manifest records:

- source-artifact SHA-256;
- exact Noether revision;
- coefficient and degree/proof bounds;
- total group and coefficient-row counts;
- certificate-method counts;
- complete versus truncated extent counts;
- artifact byte counts and SHA-256 digests.

Reload validation reconstructs every group, recomputes every retained value,
checks all hashes and completion hypotheses, and rejects unknown fields. HAP is
an environment-gated parity oracle. Its absence never changes the meaning of a
native result. The opt-in HAP oracle reads the selected modular-v7 artifact. Its sampling
includes the first family and first resolution computation per `(order,
prime)` bucket, plus C2, C4, V4, D8 and Q8 hand examples. Results and GAP/HAP
versions belong in the particular oracle run's evidence, not in a blanket
release-audit claim.

## 5. Scaling gates

1. Native bar oracle: cyclic groups and `S_3` through small degrees, with HAP
   parity.
2. Minimal p-group lane: cyclic and elementary abelian groups at `p=2,3`, then
   every p-group through order 16.
3. Sparse resolution storage and lifting: all 51 groups of order 32 through
   the selected completion bounds.
4. Sylow/stable-elements lane: every non-p-group of order at most 32 at each
   relevant prime.
5. Presentation and completion: no modular `ExactThroughDegree` rows remain in
   the release candidate.
6. Integral lane: publish exact extents without delaying the complete modular
   collection.

The benchmark corpus includes `C_2^5` (large resolution ranks), cyclic and
generalized quaternion periodic groups, dihedral groups, and mixed-order
groups with nonnormal Sylow subgroups.

## 6. Noether work remaining

- a serialized free-resolution value with contracting/lifting evidence;
- transfers and their compatibility with the retained restriction maps;
- checked comparison maps from every reduced resolution lane to the normalized
  bar resolution, so lower Steenrod operations do not require the bar lane;
- Steenrod-algebra linear combinations, composition, and Adem normal forms on
  top of the typed admissible words;
- Benson or Hilbert--Poincare completion-certificate variants and verifiers;
- reduced or periodic integral resolutions that extend the practical range
  beyond the sparse normalized-bar lane, including Bockstein towers.

None of these APIs contains an LMFDB label, an order cutoff, or a TypeLake
record type.

## 6.1 Implemented checkpoint

The current native checkpoint is pinned to Noether
`ef61b0e99ba5196a5697bd5573683eca509e46c1`. It provides checked sparse minimal
p-group resolutions, a normalized-bar oracle, deterministic Sylow subgroups,
resolution comparison maps, certified stable-element intersections, canonical
cohomology bases, and exact cup products through a caller-supplied degree. On
the pinned benchmark CPU, `C_2^5` through degree 4 takes 68--77 ms and the
mixed-order `S_4` mod-2 lane through degree 5 takes 40--48 ms. Native dimensions
for `S_3`, `A_4`, and `S_4` agree with HAP. Batched left multiplication builds
the full degree-4 product table for `C_2^5` in about 2.28 seconds. Noether also
extracts deterministic minimal homogeneous generators and all
relations visible through the exact bound. Caching generator multiplication
maps reduces the order-32 `C_2^5` cohomology-plus-extraction regression through
degree 4 to 0.98 seconds; it finds five degree-1 generators and no relations in
that range. It now promotes an algebra truncation only when the exact range
reaches Symonds's universal generator and relation bounds, returning a typed,
recomputable completion certificate. TypeLake's `modular-v3` contract stores
that evidence and otherwise retains `ExactThroughDegree`.

The checked-in GAP 4.12.1 / SmallGrp 1.5.3 source artifact contains 144 regular
permutation presentations and has SHA-256
`640135dd932b70ca84ecb5f54815a89bfa3316e677fb586617d85c27fc347e59`.
Its generator is byte-deterministic. The source validator reconstructs every
group and checks the independent per-order count table. The development census
through order 32 and degree 4 contains all 144 groups and 198 relevant
coefficient rows in a 1,893,859-byte `modular-v2` JSONL artifact with SHA-256
`3813e27f572c7485e98396f48486e8360f7b795ce2642363c53a697d13534b20`.
The records contain 593 minimal generators and 231 relations visible through
degree 4; 89 rows have at least one such relation. The strategy split is 92
minimal p-group, 4 normalized-bar, and 102 Sylow stable-elements rows. The
artifact was built and fully recomputed in 824.732 seconds on the pinned
benchmark CPU. Those figures describe the preserved `modular-v2` artifact: all
198 rows are `ExactThroughDegree`, and its manifest makes zero
complete-presentation claims. An independent reload from that emitted manifest
and JSONL repeated every validation in 413.943 seconds. Version 3 is built
separately and never mutates the v2 bytes.

The installed degree-4 `modular-v3` artifact contains the same 144 groups and
198 coefficient rows in 1,894,295 bytes, with SHA-256
`dc9b8ff2a5c64b813f1d9ff8a176b05c97cb2e04c754eaab08bca4caea0b7ff1`.
The universal Symonds bound certifies exactly two complete presentations:
`2.1` over `F_2` and `3.1` over `F_3`. The other 196 rows remain honest
degree-4 truncations. The initial conservative build and in-memory replay took
1,418.780 seconds; an independent reload, hash check, and full mathematical
recomputation took 432.786 seconds and reproduced every record and certificate.

Noether revision `2841882b7adbaa6519248caf74df0730067c0a53` adds exact
invariant-factor reconstruction for a supplied finite abelian permutation
group and a family-theorem complete `H^*(BG; F_p)` result owned by `homalg`.
TypeLake's `modular-v4` contract tries this lane before constructing a
resolution. The installed order-through-32, degree-4 artifact still contains
all 144 groups and 198 coefficient rows. Its 54 nontrivial finite abelian
groups contribute 74 complete `(G,p)` presentations; 124 nonabelian rows
remain honest exact truncations. The 1,430,433-byte JSONL artifact has SHA-256
`44a705de8906279537821f270cdf691ace44e473b03478ab229c0da93356d0d2`.
Construction plus in-memory mathematical replay took 650.486 seconds; an
independent installed-file hash check and full recomputation took 349.911
seconds and reproduced every record. This is a development checkpoint, not the
complete modular release candidate.

Noether revision `106fa9e922334fea5461aa67eb51dea445f70cfc` implements the
data-dependent Symonds parameter criterion with exact restriction evidence for
every maximal elementary abelian subgroup of a retained Sylow subgroup and a
finite-quotient module witness. TypeLake contract `modular-v5` stores that
certificate without changing the older artifacts. The installed order-through-
32, degree-8 development artifact has all 144 groups and 198 coefficient rows.
It contains 74 finite-abelian family presentations and 122 parameter-certified
resolution presentations; 196 rows are complete. The two honest exceptions
are `32.8/F_2` and `32.44/F_2`, both of which first acquire a generator in
degree 8. The 14,993,053-byte JSONL has SHA-256
`f1a8c7339ffe487599963f38ca8f9a6d93deef14ac5a6dc9a9dbbe39e09e40dd`.
Construction plus the immediate full mathematical replay took 2,895.968
seconds and used 303,760 KiB peak resident memory.

That pilot exposed a dense graded-reduction ceiling. Noether revision
`82724ee956faecdac3fdbbec734aae7eb9d5e16c` routes prime-field homogeneous
relation reduction through the existing sparse RREF kernel. On the same
`32.8/F_2` degree-12 record, the old path failed after 129.65 seconds at its
4,000,000-cell limit; the sparse path completed in 14.86 seconds using 97,972
KiB. Targeted computations certify `32.44/F_2` at degree 10 and `32.8/F_2` at
degree 12. The release contract will therefore retain per-row proof depths
instead of recomputing every already-complete row through degree 12.

TypeLake contract `modular-v6`, pinned to that sparse Noether revision, is the
complete modular release artifact. It contains all 144 groups and all 198
required `(G,p)` rows: 74 finite-abelian family presentations and 124
parameter-certified resolution presentations, with no truncations. The exact
proof-depth histogram is 196 rows at degree 8, `32.44/F_2` at degree 10, and
`32.8/F_2` at degree 12. The 15,229,985-byte JSONL has SHA-256
`d7e25caa5a850a2774d1b27f86605b23aee4b4db6b3299c525a3ea9a1092bfd9`.
Construction plus the immediate full replay took 2,767.538 seconds and used
288,304 KiB peak resident memory. The manifest and records are installed at
`data/finite-group-cohomology/modular-v6/order-through-32/proof-degree-8-through-12/`.
An independent disk reload, byte-hash check, and complete mathematical replay
took 1,429.168 seconds and 270,900 KiB peak resident memory. It reproduced the
stored validation report exactly.

Noether revision `76248daf94ce49a9d297fc6e29f5f9cbda775b27` makes primary
operations typed, records operation-adapted generator witnesses modulo
decomposables, and binds that evidence into the canonical algebra digest.
TypeLake contract `modular-v7` retains the same 198 complete rings and proof
depths while replacing display strings with replayable operation data. Its
15,273,124-byte JSONL has SHA-256
`91f1b0bddf3ea8d8fb4201912fe14ace5bd2778bfab5fb2f00863b4a0a810058`.
Construction plus immediate replay took 2,708.834 seconds. An independent
disk reload, byte-hash check, and full mathematical replay took 1,361.764
seconds and reproduced the stored validation report exactly.

Noether revision `7b7d1f7cd9c9bf8cab362d0502593557e5784e54` adds the first
theorem-complete integral family. For a supplied cyclic group `C_m`, the
period-two resolution certifies
`H^*(BC_m; Z) = Z[u]/(m u)`, `|u| = 2`; the generator is named
`beta_m(iota_1)`. TypeLake contract `integral-v1` applies this playbook to the
pinned source list and contains exactly one complete record in every order
through 32. Its 32-row JSONL is 54,126 bytes with SHA-256
`a554591b1542e05f16bf8d10749d5f8579c24d84c025f65ba12fb59218f7cf6f`.
All 32 records and their additive previews through degree 16 replayed from disk
in under 0.01 seconds. The other 112 groups remain explicitly pending; this is
an integral family checkpoint, not a complete integral census.

Noether revision `dfd2d3d40bc2c2cd43b71104c490188e128f2297` reuses the exact
rank-two `K(A,1)` theorem on a supplied finite abelian permutation group.
For invariant factors `n_1 | n_2`, the presentation has degree-two Bockstein
classes `a,b`, a degree-three integral Tor class `c`, their torsion relations,
and the parity-sensitive relation for `c^2` from Roberts, Theorem 6.3.1. The
TypeLake `integral-v2` record shape consequently permits a relation to contain
several integer monomial terms and gives Tor classes a typed origin.

The installed order-32 artifact contains 46 complete presentations: the 32
cyclic groups and 14 groups with two invariant factors. Its 92,732-byte JSONL
has SHA-256
`c4f8a7509fdb76d0102372ee4f1b7bc56f92b1553ec1465233e09ab1632cb7cb`.
Two independent builds were byte-identical; a disk reload, byte-hash check,
and complete mathematical replay took 0.46 seconds and 51,712 KiB peak resident
memory. The remaining 98 groups are explicitly pending.

Noether revision `76248daf94ce49a9d297fc6e29f5f9cbda775b27` adds the exact
universal integral truncation through degree 2 for every finite permutation
group, using `H^2(BG; Z) = Ext(G_ab, Z)`, as well as a normalized integral bar
calculation for feasible higher bounds. TypeLake contract `integral-v3`
retains the 46 complete presentations and adds 98 honest low-degree records.
The resulting 144-row JSONL is 157,588 bytes with SHA-256
`49e06b66a2481089c71e2c7b8e5ac4d1bdcfba2f88737e8b828a1fabe09de7c8`;
there are no pending integral groups, although those 98 records make no claim
above degree 2.

Noether revision `2eb46dcf7f6b7e35f28d7a356e67755a001e6880` keeps normalized
integral bar differentials sparse, applies the general homological
discrete-Morse contraction before the remaining integer normal-form work, and
rejects an oversized request before allocating any differential. TypeLake
contract `integral-v4` retains all older complete family presentations and
uses a collection budget of 100,000 estimated sparse contributions for the
other groups. The installed order-32 artifact has 144 rows: 46 complete
presentations, 90 universal exact truncations through degree 2, seven sparse
normalized-bar ring truncations through degree 3, and one through degree 4.
Each sparse row stores canonical cocycles and every basis cup product in range.
The 236,449-byte JSONL has SHA-256
`3b3e22b3294a72ce412eb0e2cb84c162883578c0661076efbc357c7acd888194`.
Construction, full in-memory replay, and immutable installation took 89.876
seconds on the development host. An independent disk reload, artifact-hash
check, and full mathematical replay took 44.527 seconds and reproduced the
stored validation report exactly. The `integral-v3` bytes remain unchanged.

## 7. Release evidence

The collection is ready for TypeBase only when:

- the source set proves all 144 SmallGroups classes through order 32;
- every required modular row has `CompletePresentation` with a passing
  certificate;
- every integral row states and verifies its exact extent;
- all record and manifest reload validators pass from installed artifacts;
- the HAP parity suite covers the hand examples and a deterministic sample in
  every order/prime bucket;
- TypeBase source and completeness pages are generated from the selected
  immutable manifest.
