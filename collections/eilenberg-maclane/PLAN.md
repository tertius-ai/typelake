# Eilenberg--Mac Lane anima: collection plan

**Collection contract:** `eilenberg-maclane/invariants-v8`

**Source contract:** `eilenberg-maclane/abelian-source-v1`

**Status:** development; the parameter box is complete, invariant coverage is
not yet complete

## Boundary

Noether owns the symbolic value `EilenbergMacLaneSpace(A,n)`, reusable
effective-homology playbooks for one supplied space, canonical algebraic
results, and verifiers. TypeLake owns the selected groups, delooping and degree
bounds, external labels, build manifests, coverage counts, and records.
TypeBase will publish only a pinned TypeLake edition whose advertised invariant
coverage is true.

There is no enumeration problem up to isomorphism here. The selected parameters
determine the objects: finitely generated abelian groups are represented in
canonical invariant-factor form, and `K(A,n)` is determined by `A` and positive
`n` up to equivalence.

## First parameter box

- every finite abelian group of order at most 32, sourced from GAP SmallGrp;
- the infinite cyclic group `Z`;
- delooping degrees `1 <= n <= 4`;
- exact invariants through total degree 16;
- integral coefficients, `F_p` for each prime dividing a finite group order,
  and the declared comparison fields `F_2` and `F_3` for `A = Z`.

The box contains 55 finite groups plus `Z`, hence 224 symbolic spaces. The
checked-in source records GAP's elementary cyclic factors and SmallGroups/LMFDB
label. The builder asks Noether to normalize them to invariant factors and
checks that their product is the recorded order.

The first native checkpoint at Noether revision
`2d6c5dc084ad8ac0b43a39bf3f83460028da0e23` computes additive integral homology
for every `K(A,1)`, every `K(Z^r,2)`, and every trivial `K(0,n)`. In this
parameter box that is 60 of 224 spaces. It is an intentionally incomplete
development artifact, not the Volume I release edition. Every remaining row
records the typed unsupported capability returned by Noether.

The installed development artifact at
`data/eilenberg-maclane/invariants-v1/order-through-32-plus-Z/delooping-through-4/degree-16/`
has 224 rows and 409,513 JSONL bytes. Its record SHA256 is
`4212a468b53eeaff702ecb02759822fae1c88903a365506d3f06e1acd49658bf`;
the independently reloaded validator recomputes all 224 symbolic hashes and all
60 available homology results. The source SHA256 is
`b0cc460311cac425a96fcd65bc744f49c1b6b08d2a461496c153bd6dfb85db6d`.
That immutable `invariants-v1` artifact remains unchanged. Contract
`invariants-v2`, pinned to Noether
`9084d7d5f86866496ca33b2e973e25ad6149f8d3`, additionally stores additive
integral cohomology from universal coefficients with the exact homology result
retained and hashed as evidence. Cup products and integral ring presentations
remain separate pending cells.

The installed `invariants-v2` artifact has 224 rows and 538,145 JSONL bytes,
with SHA256
`ef84d022b3e7713453af1929c7ed3e1bbad447b35fbc05322ad52296e7598ee3`.
Both integral homology and additive integral cohomology are available for 60
spaces and pending for 164. A disk reload independently reproduced all source,
symbolic-hash, homology, universal-coefficient, and coverage checks.

Contract `invariants-v3`, pinned to Noether
`8ba91dd578d47cd73c539dfab6d528823df28ff8`, adds complete closed-form
prime-field cohomology presentations for every `K(A,1)`, `K(Z,2)`, and
supported trivial space in the declared coefficient rows. It stores the exact
graded-commutative presentation, stable generator names and origins, primary
operations on presentation generators, and the Hilbert function through
degree 16. Of 304 coefficient rows, 78 are available and 226 remain explicitly
pending. The installed 224-row artifact has 706,570 JSONL bytes and SHA256
`a4b69f6d8ad474d55a33b194937bd2a34d5d19a2244de6dc0f1eacbe3990b4af`.
A disk reload and full Noether recomputation reproduced all records. The
immutable v1 and v2 artifacts remain unchanged.

Contract `invariants-v4`, pinned to Noether
`4d5987a63ff4238939647b895d1f2104a7bd2500`, adds a theorem-bounded mod-two
lane for `n >= 2` when the coefficient group is free abelian apart from
elementary 2-primary torsion. It stores every Cartan--Serre admissible-square
algebra generator that can contribute through degree 16, including its
Steenrod word and whether the fundamental class lifts integrally. This adds 53
exact-through-degree rows while retaining the 78 complete closed-form rows:
131 of 304 modular coefficient rows are now available and 173 remain
explicitly pending. The installed 224-row artifact has 912,899 JSONL bytes and
SHA256
`27cafccae81c0822a89d7c15f30f8fd7d7987cb539b99c4ea2c1f0b3fbc68edb`.
An independent disk reload reproduced all hashes, symbolic spaces, theorem
results, and coverage counts. The bounded presentations explicitly make no
claim above degree 16; the immutable v1--v3 artifacts remain unchanged.

Contract `invariants-v5`, pinned to Noether
`ad7a6281f1f3deb1f4c289ff1a8c5e2fe93435b9`, adds the odd-primary
Cartan--Serre lane for free abelian and elementary `p`-primary coefficient
groups. Generator origins retain the complete admissible word
`beta^e_0 P^s_1 beta^e_1 ... P^s_k beta^e_k`, including whether the
fundamental class lifts integrally. This fills 104 more degree-bounded rows:
235 of 304 modular coefficient rows are available and 69 remain pending. The
remaining rows are precisely 54 higher-2-primary and 15 higher-odd-primary
cases. The installed 224-row artifact has 1,081,751 JSONL bytes and SHA256
`0a1acf3f64b216569f8b8b4913eaea514a106001f3cd5a67392fa4be7f7b4515`.
An independent disk reload rechecked its artifact hash and recomputed all
symbolic spaces, invariants, and coverage in 0.16 seconds with 19,688 KiB peak
resident memory. The immutable v1--v4 artifacts remain unchanged.

Contract `invariants-v6`, pinned to Noether
`6bd44e51f639b35a1352c25fb67b2de79c6aeb40`, extends the bounded
Cartan--Serre lanes to cyclic summands of every primary exponent. A summand
divisible by `p^r`, `r > 1`, contributes its fundamental class and higher
Bockstein as two separately identified seeds. Generator origins retain the
primary exponent, the chosen seed, and the full admissible operation word.
Consequently all 304 requested modular coefficient rows are available through
degree 16; no modular cell in the first parameter box remains pending. The
installed 224-row artifact has 1,337,610 JSONL bytes and SHA256
`5713238f98417ddcc99be4ffbe44db56dbaa991c96cf882d516ab6696e101a56`.
An independent disk reload rechecked the artifact and recomputed every
symbolic space, invariant, and coverage cell in 0.18 seconds with 22,256 KiB
peak resident memory. The immutable v1--v5 artifacts remain unchanged.

Contract `invariants-v7`, pinned to Noether
`ceca3fa021a9313181b48b00ccedf5d9645e614f`, adds a separate complete integral
cohomology-ring cell. Closed forms are available for every point,
`K(Z^r,1)`, `K(Z^r + C_m,1)` with at most one nontrivial cyclic invariant
factor, and `K(Z^r,2)`. They retain named integral fundamental/Bockstein
classes, explicit homogeneous relations (including square-zero relations for
odd torus classes), additive groups through degree 16, and the replayed
Noether semantic hash. This gives 37 complete integral rings; the other 187
rows remain explicitly pending. The installed 224-row artifact has 1,450,388
JSONL bytes and SHA256
`1cc3dd91bab780086fa5aff47b9c57b7894ecea96c9e3c1e1ead7a4f86cc0a4a`.
Two independent builds were byte-identical, and disk replay recomputed every
coverage cell in 0.20 seconds with 22,604 KiB peak resident memory. The
immutable v1--v6 artifacts remain unchanged.

Contract `invariants-v8`, pinned to Noether
`cb2619170e52a11a6791b4d44df5f74771b7268d`, extends the complete integral
classifying-space lane to groups with two invariant factors. Besides the two
degree-two integral Bockstein classes, it records the degree-three Tor class,
its annihilator, and its square. When `n1 | n2`, both orders are even, and
`n2 / n1` is odd, that square is the exceptional nonzero decomposable class
in Theorem 6.3.1 of [Roberts's finite-abelian cohomology-ring
calculation](https://uwspace.uwaterloo.ca/items/abb76e24-301d-45c4-9c0e-d78b8b92e471).
This adds all 14 rank-two finite abelian groups in the first box: 51 integral
rings are complete and 173 remain explicitly pending. The installed 224-row
artifact has 1,480,913 JSONL bytes and SHA256
`435df142c6248bfeffb261f5da4c4ed3e1eb6214c0dd36c333d11ca9ac546d95`.
Two independent builds were byte-identical; disk replay recomputed every
coverage cell in 1.14 seconds with 25,980 KiB peak resident memory. The
immutable v1--v7 artifacts remain unchanged.

## Labels

The object label is the group label followed by the delooping degree:

```text
8.2.2     K(C4 x C2, 2)
Z.3       K(Z, 3)
```

For a finite group, the first two fields are its LMFDB/SmallGroups label
`order.index`; the third is `n`. Thus the dots have mathematical meanings and
there is no arbitrary bucket index. `Z` is a reserved group field, not an LMFDB
label. Coefficients are route components, not part of the object label.

## Computation sequence

1. Freeze and validate the finite-abelian source list independently against GAP.
2. Materialize all 224 symbolic objects and exact native additive homology
   wherever the current Noether playbook applies.
3. Add effective chain models for torsion groups at `n >= 2` and for `K(Z,n)`
   at `n = 3,4`; certify integral homology through degree 16.
4. Derive integral cohomology additively by the universal coefficient theorem,
   retaining exact extension data and canonical cocycles.
5. Extend the complete integral closed forms now present to three or more
   torsion factors and then compute degree-bounded graded-commutative
   presentations on effective models for the remaining spaces. Odd integral
   squares are allowed to be nonzero 2-torsion unless a family theorem
   supplies an explicit square-zero relation.
6. Extend the complete and bounded prime-field rings now present with
   chain-level witnesses for Bocksteins, Steenrod squares, reduced powers, and
   Bockstein reduced powers through degree 16. The bounded Cartan--Serre
   presentations now cover every requested prime-field row; explicit operation
   evaluation beyond generator-origin names remains future effective-model
   work.
7. Name the fundamental class `iota_n`; assign every other operation name by a
   shortest deterministic admissible word backed by a chain-level witness.
8. Compare a declared sample with HAP and published Cartan--Serre calculations.
   HAP is a parity oracle, never the hidden correctness floor.
9. Freeze TypeBase routes only after every requested coefficient/invariant cell
   in the parameter box is present and independently replayed.

## Exact playbooks still required in Noether

- effective chain data through an explicit degree for iterated classifying
  spaces, without pretending that `K(A,n)` is a finite simplicial set;
- tensor and twisted tensor products with Eilenberg--Zilber reductions;
- certified algebraic discrete-Morse reduction before Smith reduction;
- integral and field cochain contractions retaining representatives;
- cup products and May--Steenrod operations on the effective model;
- integral ring formulas for classifying spaces with three or more invariant
  factors, including their interacting Tor classes;
- further result types for the effective-model lanes; the bounded
  Cartan--Serre result already states an inclusive exact degree, refuses a
  claim above it, and replays its generator enumeration.

The builder may exploit closed forms such as
`K(Z,2) = CP^infinity`, but the corresponding mathematical operation and
verification remain in Noether. Census-specific loops and coverage policies do
not move there.

## Release gates

- all 224 object keys are unique, sorted, and reconstruct their parameter;
- all 55 finite source groups agree with the pinned GAP/SmallGrp source and link
  to the corresponding LMFDB page;
- every available invariant is recomputed from the symbolic source and matches
  its Noether semantic hash;
- integral homology/cohomology satisfy universal coefficients and known
  connectivity/first-nonzero-degree constraints;
- products are associative and graded-commutative through the exact extent;
- operations satisfy degree, instability, Adem, Cartan, and Bockstein checks in
  the supported range;
- independent HAP parity passes on the declared sample;
- the manifest derives coverage counts from records, and the completeness page
  makes every missing cell visible.
