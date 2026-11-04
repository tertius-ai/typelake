# TypeLake / TypeBase

A store and a publication system for mathematical data.

Part of [Tertius](https://tertius.cc/), a 2026 academic collaboration by
[Benjamin Antieau](https://github.com/antieau) and
[Deven Manam](https://github.com/dmanam) building open source tools for mathematics.

| | |
|---|---|
| **TypeLake** | immutable, versioned computational records together with their provenance, validation, and coverage |
| **TypeBase** | coherent, frozen, citable catalogues selected from TypeLake and published like a journal |

There is no third SciType product or public schema authority. Each collection
owns the small, versioned contract needed to read and validate its records. A
contract states the record columns and value shapes, canonical encoding,
relations, conventions, and completeness vocabulary for that collection. It is
part of TypeLake, not a standard that another project must adopt.

**Status: pre-release.** Design is in [`ARCHITECTURE.md`](ARCHITECTURE.md) and
work is tracked in [`TODO.md`](TODO.md).

The Volume I selection for editorial review is pinned in
[`collections/volume-i.json`](collections/volume-i.json). Its exact bounds,
enrichment choices, integrity checks, and local browsing workflow are described
in [`collections/VOLUME-I.md`](collections/VOLUME-I.md). All Rust builders use
the public Noether 0.3.2 source revision. Historical data keeps its original
producer metadata; current-engine replay evidence and successor batches are
recorded separately. TypeLake 0.1.0 remains the release target pending review.

For cloning onto another machine, transferring the catalogue data, rebuilding
the browser projections, and running the local site, see
[`collections/REBUILDING.md`](collections/REBUILDING.md).

## Source quickstart

Clone the source over HTTPS; no GitHub account is required:

```bash
git clone https://github.com/tertius-ai/typelake.git
cd typelake
python3 -m venv .venv
.venv/bin/python -m pip install -e '.[dev]'
.venv/bin/scitype lint
.venv/bin/python -m pytest -q

# Node.js 22 with npm is required for the browser build.
npm --prefix web/typebase ci
npm --prefix web/typebase run build
```

These checks and the browser build work without catalogue data. The `scitype`
command is retained validation tooling described under
[Repository transition](#repository-transition).

The full Volume I catalogue remains an editorial review snapshot. Its bulk
records and browser bundles are not included in Git or available as a public
download yet. Running the populated catalogue requires an existing review
bundle, as described in [the rebuilding guide](collections/REBUILDING.md).
Rust census and projection builders additionally require the Noether and FLINT
setup in that guide. Opening this source does not declare a Volume I release.

## Collection implementations

The first collection is finite posets. Its data-focused implementation plan is
[`collections/finite-posets/PLAN.md`](collections/finite-posets/PLAN.md).

The implemented second collection is finite strong Steiner categories. Its
mathematics, census, polygraphic homology, categorical-homology spectrum,
homotopy-Poset, Gray-product, record, label, and interactive-view plan is
[`collections/strong-steiner-categories/PLAN.md`](collections/strong-steiner-categories/PLAN.md).

The accepted common plan for the final four Volume I collections---finite
simplicial complexes, finite simplicial sets, finite-group cohomology, and
Eilenberg--Mac Lane anima---is
[`collections/VOLUME-I-FINAL-FOUR.md`](collections/VOLUME-I-FINAL-FOUR.md).

The finite-group builder publishes `modular-v7` and `integral-v4`. The former
contains all 198 complete `(G,p)` presentations through group order 32 with
typed operation-adapted generator witnesses. The latter contains one `/Z`
record for every one of the 144 groups: 46 complete family presentations,
eight sparse normalized-bar ring truncations (seven through degree 3 and one
through degree 4), and 90 universal truncations through degree 2. Build the
immutable TypeLake artifacts and their static TypeBase projection with:

```bash
make finite-group-modular \
  FINITE_GROUP_COHOMOLOGY_DATA_ROOT=/absolute/path/to/data
make finite-group-integral \
  FINITE_GROUP_COHOMOLOGY_DATA_ROOT=/absolute/path/to/data
make typebase-finite-groups \
  FINITE_GROUP_COHOMOLOGY_DATA_ROOT=/absolute/path/to/data \
  TYPEBASE_FINITE_GROUP_OUTPUT_ROOT=/absolute/path/to/typebase-finite-groups
```

The certified identity, light, and mapping enrichments contain all 28,010
classes through total atomic size 12. The rich region contains all 7,566
classes through size 11 and stores every exact object-pair mapping category,
integral polygraphic homology, a bounded Street-nerve presentation of categorical homology,
monomorphic oriented basepoints with homotopy Posets, dualities,
automorphisms, and the fixed ordered Gray table. The reader-facing categorical
invariants are the homotopy Posets of
`Omega^infinity(H(N) ∧ Sigma^infinity_cat C+)`. The release stores exact
particle-number slices of `pi_0` through weight 2, links every slice that lies
in the current finite-Poset edition, and records the exact product/permutation
rule for higher finite-particle sectors.

Build the immutable lanes independently:

```bash
make steiner-identity \
  STEINER_CENSUS_DATA_ROOT=/absolute/path/to/steiner-data ATOMIC_SIZE=12
make steiner-light \
  STEINER_CENSUS_DATA_ROOT=/absolute/path/to/steiner-data ATOMIC_SIZE=12
# Exact cells and mappings advance independently of rich enrichment.
make steiner-mappings \
  STEINER_CENSUS_DATA_ROOT=/absolute/path/to/steiner-data ATOMIC_SIZE=12
# Rich enrichment is deliberately a separate optional lane.
make steiner-rich \
  STEINER_CENSUS_DATA_ROOT=/absolute/path/to/steiner-data ATOMIC_SIZE=11
```

Each lane writes its own run, validation, upstream-hash, and per-invariant
coverage manifest. Build and identify abstract groups for the fully enriched
TypeBase region with:

```bash
make identify-steiner-groups \
  STEINER_CENSUS_DATA_ROOT=/absolute/path/to/steiner-data \
  STEINER_GROUP_IDENTIFICATION_OUTPUT_ROOT=/absolute/path/to/steiner-groups

make typebase-steiner \
  STEINER_CENSUS_DATA_ROOT=/absolute/path/to/steiner-data \
  POSET_CENSUS_DATA_ROOT=/absolute/path/to/poset-data \
  STEINER_GROUP_IDENTIFICATIONS=/absolute/path/to/steiner-groups \
  TYPEBASE_STEINER_OUTPUT_ROOT=/absolute/path/to/typebase-steiner
```

The first database-independent builder lives in
[`crates/poset-census/`](crates/poset-census/). Its redundant reference
generator is intended for complete small-order fixtures and as an oracle for
the hereditary orderly generator. Bulk output requires an explicit data root:

```bash
POSET_CENSUS_DATA_ROOT=/absolute/path/to/data make reference-order-3
POSET_CENSUS_DATA_ROOT=/absolute/path/to/data make orderly ORDER=7
```

Once order `n-1` is a completed, validated census directory, a large order `n`
can be built as restartable canonical-parent ranges. Shards may run on separate
workers; rerunning the same command resumes from its last hashed part.

```bash
export POSET_CENSUS_DATA_ROOT=/absolute/path/to/data
export RUN_ID=posets-order-10-v1
export ORDER=10
export SHARD_COUNT=1024

make orderly-shard SHARD_ID=0
# Run SHARD_ID=1,...,1023 under the worker scheduler.
make merge-orderly-shards
```

The merge writes count- and integrity-validated candidate files under
`work/finite-posets/$RUN_ID/merged/`. It does not publish an immutable census
order before the independent mathematical validation gates have run.

The Makefile uses a compatible `FLINT_PREFIX` when supplied and otherwise
recognizes the pinned FLINT build in a sibling Noether checkout.

The completed census is projected into TypeBase Volume I by the separate
[`typebase-build`](crates/typebase-build/) crate. Labels have the form
`points.dimension.index`; the projection contains no labeled copies of a
Poset. Abstract automorphism-group identifications are an independently
versioned enrichment, not mutations of the census:

```bash
make identify-poset-groups \
  POSET_CENSUS_DATA_ROOT=/absolute/path/to/data \
  GROUP_IDENTIFICATION_OUTPUT_ROOT=/absolute/path/to/groups

make typebase-posets \
  POSET_CENSUS_DATA_ROOT=/absolute/path/to/data \
  TYPEBASE_GROUP_IDENTIFICATIONS=/absolute/path/to/groups/records.jsonl \
  TYPEBASE_OUTPUT_ROOT=/absolute/path/to/typebase-volume-1
```

The museum-catalogue browser lives in [`web/typebase/`](web/typebase/). It is a
static TypeScript site over the Parquet projection, constructs Hasse SVGs from
the exact cover relation, and links identified automorphism groups to LMFDB.
Build it with `make typebase-web`.

Release-mode profiling of individual Noether operations and the complete
reference pipeline is reproducible with:

```bash
make bench-posets
make bench-pipeline BENCH_MAX_ORDER=6
make bench-orderly ORDERLY_BENCH_MAX_ORDER=8
```

## The two boundaries

TypeLake answers: what was computed or contributed, by whom, using what code,
under which conventions, and with what evidence? Accepted data arrives in
immutable batches. A correction adds another batch; it does not rewrite the
history. Replaceable enumeration shards, checkpoints, and logs are work files,
not lake data.

TypeBase answers: what coherent catalogue do we endorse for readers to search
and cite? A TypeBase volume pins exact TypeLake batches, resolves any competing
results under a published rule, asserts a scope of completeness, and assigns
stable human labels. Existing volumes do not change when the lake grows.

The boundary is editorial rather than physical. Both products may use Parquet
and JSON manifests. Mathematical results land in TypeLake first; selection,
labelling, catalogue organization, and citation belong to TypeBase.

## Collection contracts

A collection contract is deliberately ordinary and local:

- a versioned record schema suitable for Arrow/Parquet and JSON interchange;
- a canonical byte encoding and content-id rule;
- definitions for every published column and convention;
- declared relation kinds and their payloads;
- a batch-manifest schema for provenance, validation, and coverage; and
- conformance cases and executable validation tests.

No opaque global type ids are required. Shared value shapes should be factored
out only after multiple collections actually need them. A homology group or
automorphism action may remain a well-defined structured value inside a Poset
record rather than becoming an independently registered object.

## Repository transition

This repository began as the SciType prototype. The existing [`schemas/`](schemas/),
`src/scitype/`, examples, and conformance tools are retained temporarily so
useful validation and canonical-encoding work can be migrated deliberately.
They are not the target architecture, and new collection work must not depend
on permanent `scitype:` lineage numbers or on SciType as a separately shipped
standard.

Bulk data stays outside Git. Code is MIT and published data is CC0 1.0; see
[`DATA_NORMS.md`](DATA_NORMS.md).
