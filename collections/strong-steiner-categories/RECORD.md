# Strong Steiner category record v1

This document freezes the first durable record for the
strong-steiner-categories/v1 collection. The records classify finite strong
Steiner strict omega-categories. In this class, strict isomorphism,
categorical equivalence, and folk equivalence agree.

## Identity and labels

The mathematical object is a finite unital strongly loop-free based augmented
directed complex. In degree q, atom_counts[q] is the rank of its preferred
basis. Every positive-degree atom stores the disjoint degree-local supports

    boundary(atom) = positive - negative.

Noether forms a directed signed incidence graph whose vertices are atoms,
whose dimension-coloured self-loops distinguish degree blocks, and whose
lower-to-upper boundary edges have weights `-1` and `+1`. It minimizes the
weighted edge list by exact individualization/refinement. The induced atom
order is then rendered as the following unsigned word:

    number of occupied degrees
    atom counts, in increasing degree
    for each positive degree and each atom in its new order:
      negative support length, then its sorted indices
      positive support length, then its sorted indices

The decimal rendering of this sequence is SteinerKey@1. It reconstructs the
entire category; relabeling invariance follows from the canonical incidence
graph. It is the permanent relation key. A TypeBase label is
assigned only after a complete bucket is frozen:

    atomic-size.dimension.one-based-canonical-index

The empty category is 0.-1.1. Relabelings of atoms are never stored as new
objects.

## Identity, light, mapping, and rich lanes

The scalable v1 pipeline separates four immutable contracts. Adding an
enrichment never rewrites its upstream identity shell.

```text
census/identity-version=1/atomic-size=<a>/
  identities.jsonl
  coverage.json
  validation.json
  run.json

enrichments/light-version=1/atomic-size=<a>/
  light-enrichments.jsonl.zst
  coverage.json
  validation.json
  run.json

enrichments/mapping-version=1/atomic-size=<a>/
  mapping-enrichments.jsonl.zst
  mapping-categories.jsonl.zst
  coverage.json
  validation.json
  run.json

enrichments/rich-version=1/atomic-size=<a>/
  rich-enrichments.jsonl.zst
  mapping-categories.jsonl.zst
  monomorphic-basepoints.jsonl.zst
  homotopy-posets.jsonl.zst
  categorical-homology.jsonl.zst
  categorical-homotopy-posets.jsonl.zst
  coverage.json
  validation.json
  run.json
```

`strong-steiner-categories/identities/v1` stores only the canonical key,
atomic dimension vector, and signed boundary data needed to reconstruct the
category. `light-enrichments/v1` stores complete cheap invariants: structural
statistics, connectivity, integral polygraphic homology, the natural
automorphism action, named-family recognition, and dualities.
`mapping-enrichments/v1` stores complete exact-cell counts and every ordered
object-pair mapping category, including the ambient cells that make up each
mapping category. `rich-enrichments/v1` retains those facts in the compatibility
assembly and owns the more expensive sidecars. Every
enrichment manifest records the validation hash of its upstream layer and
states coverage separately for every invariant.

The earlier `census/record-version=1` monolithic shell remains a supported
assembled representation for the frozen fully enriched region. It is exactly
recoverable from one identity, light, and rich row plus the rich sidecars; its
existence does not force later identity shells to materialize rich data.

## Assembled primary category row

categories.jsonl contains one key-sorted row with:

- record_version = 1;
- the SteinerKey;
- atom_counts and every signed boundary column;
- atomic size, categorical dimension, object and generating-arrow counts;
- boundary mass, precedence-Poset statistics, and generating-1-skeleton
  component sizes;
- exact-dimensional nonidentity cell counts;
- integral polygraphic homology in invariant-factor form;
- the complete natural automorphism action on flattened degree blocks;
- recognized exact named families;
- every nonempty dimension-duality mask, whether the same directed basis
  remains strongly loop-free, and its canonical target when it does.

The natural action is not a transitive-group identity. An optional LMFDB
abstract-group label is external naming information.

Composition uses the convention x.compose_at(y, q) = x followed by y;
therefore target_q(x) = source_q(y). A duality mask lists the positive
globular dimensions whose signed boundary is reversed.

## Exact cell encoding

A cell is not identified by its top atomic support. Whiskering can change
lower support while leaving the top chain unchanged. Every cell in a sidecar
therefore stores both complete Steiner sequences:

    dimension
    negative_supports[0..dimension]
    positive_supports[0..dimension]

Noether validates and reconstructs this double sequence exactly. Atomic ids
are degree-local integers; a cell record is not an atom id.

## Independently versioned sidecars

All sidecars carry record_version = 1 and refer to the source SteinerKey.

- mapping-categories.jsonl.zst stores every ordered object pair, all exact
  relative cells, and its relative dimension. When the finite mapping category
  has a verified free strong-Steiner model, it also stores that model's
  `SteinerKey` and atomic size. This optional relation records membership in the
  present **strong** Steiner collection; its absence does not say that the
  mapping category is non-Steiner. A model may also lie outside the selected
  TypeBase atomic range.
- monomorphic-basepoints.jsonl.zst stores one canonical representative per
  automorphism orbit of injective globe-boundary maps. It includes the full
  recursive cell pairs, orbit size, stabilizer order, and linked homotopy
  PosetKey.
- homotopy-posets.jsonl.zst stores each distinct canonical homotopy Poset
  once. Its key is exactly the finite-Poset collection's principal-downset
  PosetKey, so TypeBase can resolve the existing Poset accession.
- categorical-homology.jsonl.zst stores Heine's exact free
  stratified simplicial commutative-monoid source presentation on the Street
  nerve through the declared uniform degree. Each generator records its
  oriental atom images, thin marking, faces, and degeneracies.
- categorical-homotopy-posets.jsonl.zst is an independently versioned
  reader-facing enrichment. It stores exact finite particle-number slices of
  `pi_0 Omega^infinity(H(N) ∧ Sigma^infinity_cat C+)` through weight 2.
  Each slice carries its multiset elements and Hasse diagram. Slices of order
  at most 10 carry a canonical PosetKey and resolve into the current Poset
  edition; larger slices stay in deterministic multiset order. The exact
  higher-sector rule is a disjoint union over first-level permutations of
  Cartesian products of the one-particle homotopy Posets stored in the
  basepoint sidecar.
- Gray-product rows store an ordered pair of factor keys and the canonical
  product key. The tensor is not symmetric, even when two particular factors
  happen to give isomorphic results in both orders.

Neither categorical sidecar is ordinary group-valued homology.
Integral polygraphic homology is the homology of the atomic chain complex and
is stored separately in the primary row. Ordinary integral homology of the
Street nerve, if added later, will be a third explicitly named enrichment.

## Completeness and validation

A published shell has fixed total atomic size. Its validation.json must say:

- the record and contract versions agree;
- every primary and sidecar row reconstructs in Noether;
- canonical keys and all relation targets verify;
- orbit--stabilizer checks pass for monomorphic basepoints;
- homotopy Posets reconstruct canonically;
- Street operators satisfy all stored simplicial identities;
- per-shell generator agreement is claimed only where it actually ran;
- separate `make test-oracles` evidence compares full matrix/attachment key sets through size 8. Recursive-reference comparisons through size 11 are a separate qualification command, not a field present in every shell report;
- the row count matches validation/counts.json.

The identity, light, and mapping lanes cover every atomic size from 0 through 12, with
shell counts `1, 1, 1, 2, 3, 8, 17, 50, 137, 451, 1483, 5412, 20444`, totaling
28,010 categories. The mapping lane contains 391,023 size-12 ordered mapping
records. The fully enriched release remains complete through atomic size 11,
totaling 7,566 categories. At size 12 the coverage manifests explicitly mark
basepoints, homotopy Posets, Street data, and particle sectors as not
materialized.

The categorical-homology source presentation in the rich region is
materialized through simplicial degree 2; the independent categorical-
homotopy-Poset sidecar is materialized through particle number 2. Additional
invariants, SVG layouts, abstract-group identifications, and Poset label
resolutions are replaceable enrichments; they do not alter a SteinerKey or an
existing a.d.c label.

The TypeBase projection derives a reader-facing description of every stored
oriented boundary (for example, `v0 -> v1; a0 => a2`) and summarizes the linked
Poset's shape, connected components, Stong-core order, and integral homology.
These are replaceable display/search fields computed from the certified
basepoint and Poset records; the opaque local basepoint index remains only a
stable local label within its category.

## Mathematical scope of the strict model

A finite strong Steiner category is gaunt: strong loop-freeness excludes
nonidentity reversible cells, in every iterated mapping category. Induct on
cell dimension. Essential surjectivity in the presence of only identity
equivalences is surjectivity on objects; local equivalence reduces by the same
argument to bijections on the cells of each mapping category. Thus a strict
categorical or folk equivalence between these gaunt finite models is a strict
isomorphism. The census still constructs and compares strict boundary models.

For an oriented globe boundary, the only nonidentity cells in each represented
degree are its displayed source and target. A functor from it is injective on
all cells exactly when those two images are distinct in every degree. Images
in positive degree cannot become identities: their lower source and target
are already distinct. Identity lifts of different nonidentity cells remain
distinct. This proves the distinct-pair test for monomorphisms of the strict
cell models used here; it makes no claim about arbitrary infinity-categorical
presentations.

The canonical parent deletes the last atom in the highest occupied degree of
the canonical representative. No other atom has that atom in its boundary,
so deletion preserves the remaining boundary, unitality, and loop-freeness.
Canonical representatives make this rule invariant under isomorphism. The
parent has one fewer atom, so iterating reaches the empty category. Reversing
this sequence is among the permitted cell attachments. Taking one attachment
per parent-automorphism orbit retains that sequence up to isomorphism.

Mapping-model reconstruction checks the full degreewise ambient cell
realization and its source and target boundaries bijectively. Equal cell
counts alone do not certify a model. The Noether 0.3.2 compatibility
implementation is in `steiner-census::mapping_model`.
