# TypeLake / TypeBase — architecture

**Draft revised 2026-08-25. Data creation comes first; deployment and serving
are deliberately deferred.**

There are two products:

| | Purpose | Durable commitment |
|---|---|---|
| **TypeLake** | accepted computational data and evidence | accepted batches are immutable |
| **TypeBase** | coherent catalogues selected from TypeLake | published volumes and labels are immutable |

A collection contract is an implementation component of TypeLake, not a third
product. The former SciType prototype is being reduced to these local contracts.
There is no public type authority, universal mathematical ontology, or promise
that another project should adopt our schema ids.

## 1. The boundary

TypeLake records what has been computed or contributed. TypeBase records what
we endorse as a catalogue for readers to search and cite.

| TypeLake | TypeBase |
|---|---|
| immutable computational batches | immutable editorial volumes |
| canonical content ids | stable human labels |
| partial enrichments may coexist | one coherent selected record |
| provenance and validation evidence | published scope and completeness claim |
| mathematical relations such as `has-core` | catalogues organized by an equivalence relation |
| may contain a corrected successor to an earlier result | never silently changes an existing volume |

The governing rule is:

> If two computations may validly coexist, the data belongs in TypeLake. If we
> must choose what a reader should cite, the choice belongs in TypeBase.

This is an editorial boundary, not a storage boundary. Both products may use
Parquet, JSON manifests, and the same Rust record types. TypeBase is not the web
site or API: it is the frozen dataset those delivery mechanisms read.

Replaceable generation shards, checkpoints, benchmark output, and logs are not
TypeLake. They remain under a run's `work/` directory until a validated batch is
accepted.

## 2. Collection-owned contracts

Every collection owns a small, versioned contract containing:

1. its record columns and physical value shapes;
2. its canonical byte encoding and content-id rule;
3. the equivalence relation resolved by canonicalization;
4. exact definitions and conventions for published values;
5. its relation kinds and relation payloads;
6. its batch manifest, provenance, validation, and coverage shapes; and
7. conformance examples plus executable validation tests.

For example, `finite-posets/v1` is a sufficient collection and contract name.
It does not need an opaque lineage number such as `scitype:2608.0002`.

Contracts use ordinary formats. JSON Schema is suitable for manifests and JSON
interchange. Arrow supplies the physical bulk schema carried by Parquet. Rust
types in the collection builder supply the checked implementation. Tests must
ensure these agree; no separate schema language is required.

Shared value shapes are introduced only in response to actual reuse. Integral
homology may initially be a structured value inside a Poset record. The natural
automorphism action may likewise be a structured value with a nested abstract
group description. Neither must become a globally registered object merely
because it has mathematical structure.

Contract versioning and canonical-encoding versioning are distinct:

- adding a derived column changes the record contract but not object identity;
- changing the canonical bytes changes the content-id scheme and requires an
  explicit migration or equivalence map;
- changing a convention changes the meaning of a column and therefore requires
  a new column or contract version, never a silent reinterpretation.

## 3. TypeLake records

The default unit is one coherent row produced by one computation bundle, not an
entity-attribute-value row for every scalar. This matches mathematical batch
computation and keeps records independently readable.

An accepted collection can contain four kinds of files:

```text
accepted/<collection>/
  objects/<partition>/part-*.parquet
  enrichments/<partition>/part-*.parquet
  relations/<partition>/part-*.parquet
  manifests/<batch-id>.json
```

- An **object batch** contains canonical defining data and the values computed
  coherently with it.
- An **enrichment batch** is keyed by canonical id and adds or corrects a group
  of columns without rewriting an object batch.
- A **relation batch** records typed edges between canonical ids.
- A **manifest** identifies the exact code, inputs, parameters, environment,
  validation gates, coverage, row counts, and hashes for the batch.

Batch provenance is the default because a generator normally computes many
rows under one code revision and set of assumptions. Per-row or per-column
overrides are available only where provenance or rigor actually differs. We do
not multiply identical metadata across millions of scalar facts.

Accepted files are immutable. A correction is a new enrichment or object batch
that names what it supersedes. Both remain readable. Resolution rules are part
of a view build and must preserve conflicts rather than silently choosing when
two equally qualified results disagree.

### 3.1 Identity

The lake key for a canonical object is a cryptographic hash over:

```text
collection name
canonical-encoding version
canonical defining bytes
```

For an isomorph-free generator, this canonical id is available immediately and
is the only object id needed. If a future ingest path accepts noncanonical
presentations, it may also assign a presentation hash and record a
`canonicalizes-to` relation. That machinery is not required for the initial
Poset enumeration.

Human labels never serve as lake keys. Foreign labels such as an LMFDB abstract
group label or GAP `SmallGroup` id are namespaced values, not identity.

### 3.2 Conventions and rigor

Conventions must be explicit in the collection contract and close to the data.
For Posets, examples include:

- whether height counts elements or covering edges;
- whether homology is reduced or unreduced;
- whether an automorphism group describes an abstract group or a specific
  permutation action; and
- whether a relation uses `opposite`, `dual`, or another declared meaning.

Rigor and assumptions are recorded at the narrowest scope at which they vary.
The initial Poset corpus is an exact deterministic computation, so a batch-level
statement plus validation evidence is normally sufficient.

### 3.3 Completeness

Completeness is a claim with a precise region and evidence. A Poset batch might
claim all isomorphism classes of order `n`, while a homotopy batch might claim
all cores through a different order. The manifest records:

- the region;
- whether coverage is complete, sampled, or constructed;
- the generator and validation methods;
- exact expected and observed counts; and
- hashes or references for independent certificates and oracle results.

Completeness does not follow merely from the presence of a file or a successful
program exit.

## 4. TypeBase volumes

A TypeBase volume is a frozen build over explicitly pinned TypeLake batches. It
contains:

- the selected record columns and relation views;
- the input selection and collection build parameters, with the source revision and source digest in newly built projection manifests;
- stable labels mapped to collection-owned canonical keys;
- the exact completeness claims the volume endorses;
- source and validation summaries; and
- review metadata; formal citations and erratum publication are later release work.

TypeBase may materialize cheap deterministic facets and aggregations needed for
search. It should not perform new expensive mathematics: such a result belongs
in TypeLake with its computation evidence before a volume selects it.

A new TypeLake correction does not mutate a volume. An erratum can point readers
from an old published value to the correction, and a later volume may select the
corrected result. The original bytes and citation remain stable.

Labels belong to a catalogue, not to mathematical identity in the abstract.
The same canonical Poset may therefore have an isomorphism-catalogue label and a
homotopy-catalogue label. Their namespaces state which editorial view they name.

## 5. Initial collections

### 5.1 Finite Posets

The first TypeLake object batch contains one row for every accepted canonical
Poset in its certified isomorphism range. Its collection contract will cover at
least:

- canonical covers and canonical incidence sequence;
- order and structural invariants;
- the faithful natural automorphism action, its orbits, and abstract-group
  labels where available;
- canonical core and beat-point reduction evidence;
- order-complex invariants and integral homology;
- canonical opposite id; and
- the validation and provenance needed to trust the bundle.

The initial relation batches contain `has-core` and `opposite`. Cores use the
same canonical Poset representation and content-id rule as every other Poset.

TypeBase will eventually build at least two catalogues:

1. **Posets up to isomorphism** — one row per canonical Poset, complete through
   a declared order.
2. **Posets up to homotopy** — one row per canonical core, with fiber counts and
   the largest source order for which those counts are certified complete.

The detailed data and computation design lives in
[`collections/finite-posets/PLAN.md`](collections/finite-posets/PLAN.md).

### 5.2 Finite strong Steiner categories

The second collection applies the same boundary to finite strong Steiner
strict `omega`-categories. Noether owns their based augmented directed-complex
model, category operations, canonicalization, symmetry, mapping categories,
polygraphic and categorical homology, monomorphic oriented basepoints and
homotopy Posets, and Gray tensor. The collection builder owns exhaustive
enumeration by atomic size, automorphism-orbit enrichment, bounded Gray tables,
records, completeness evidence, and TypeBase projection. Strict isomorphism,
categorical equivalence, and folk equivalence agree within this gaunt,
loop-free class, so the census does not need a second quotient catalogue.

The detailed mathematical, census, label, and viewer design lives in
[`collections/strong-steiner-categories/PLAN.md`](collections/strong-steiner-categories/PLAN.md).

## 6. Data flow

```text
Noether mathematical operations
        ↓
collection-specific generator and enricher
        ↓
replaceable work files
        ↓
contract validation + mathematical validation + completeness checks
        ↓
immutable TypeLake batch and manifest
        ↓
editorial selection and label assignment
        ↓
immutable TypeBase volume
```

Noether owns reusable mathematics. The TypeLake repository owns enumeration,
batch assembly, manifests, validation orchestration, and TypeBase view builds.
Noether does not acquire a database-specific enumerator or storage API.

## 7. Repository and data layout

Tracked code and contracts live together:

```text
typelake/
  collections/<collection>/
    PLAN.md
    RECORD.md
    TYPEBASE.md               where a browser contract is present
    validation/               where validation fixtures are present
    sources/                  where an external source catalogue is used
  crates/<collection-builder>/
  shared/                    only after demonstrated cross-collection reuse
```

Bulk and intermediate data stay outside Git under an explicit data root. The
initial Poset plan defines the local layout and immutable acceptance process.

The physical format for accepted bulk rows is Parquet with Zstandard
compression, accompanied by small JSON manifests. Tiny fixtures remain in Git
for review and conformance. Deployment, APIs, browser implementation, and remote
object-store layout are separate later decisions.

## 8. Transition from the SciType prototype

The current repository contains a global schema registry, `scitype:` lineage
ids, a `scitype` Python package, generated schema tooling, and conformance
vectors. Nothing has been published, so these are prototypes rather than
compatibility obligations.

The transition is deliberately non-destructive:

1. Define and test the concrete finite-Poset collection contract.
2. Reuse canonical JSON, validation, and conformance code where it is simpler
   than replacing it.
3. Stop introducing new global lineage ids or cross-domain schemas.
4. Move genuinely shared manifest utilities under TypeLake-owned names only
   when the Poset pipeline needs them.
5. Remove the legacy registry and rename the Python package after replacement
   tests cover every retained behavior.

Until step 5, the legacy files may remain in the repository, but no new data or
collection code may depend on SciType being a separate product or standard.

## 9. Deferred decisions

The current work does not require decisions about:

- deployment or serving;
- hosted APIs or MCP;
- object-store vendors;
- DOI and archival cadence;
- multi-writer coordination;
- a universal cross-collection query language; or
- a generic contract or ontology spanning all mathematical collections.

Those choices should be driven by accepted collection data and measured
workflows, not fixed in advance.
