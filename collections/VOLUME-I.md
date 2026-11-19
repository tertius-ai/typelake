# TypeBase Volume I selection

Volume I is selected for editorial review. TypeLake 0.1.0 is the software
release target. The immutable inputs selected by this candidate are pinned by
relative path, SHA-256, and byte count in [`volume-i.json`](volume-i.json);
uncompressed `.jsonl` inputs also pin their row count, while compressed
`.jsonl.zst` inputs do not. Changing the selection produces another review
candidate; it does not rewrite an accepted computational batch.

| Collection | Selected scope | Coverage |
| --- | --- | --- |
| Finite posets | Orders 0–10; 2,769,965 objects | Integral homology, Stong cores, natural automorphism actions, and the retained abstract-group identifications |
| Strong Steiner categories | 28,010 identities through atomic size 12 | Identity, light, and mapping lanes through 12; 7,566 rich records and browser objects through 11 |
| Finite simplicial complexes | At most 6 vertices; 16,352 objects | Topology, `F2` cohomology-v2, and `F3` cohomology-v2 |
| Finite simplicial sets | At most 6 nondegenerate simplices in dimension at most 3, plus size 7 in dimension at most 2; 125,843 objects | Topology-v2, `F2`/`F3` cohomology, `F3` operations, and witnessed horn collapses |
| Finite-group cohomology | All 144 groups of order at most 32 | Modular-v7 has 198 complete coefficient presentations. Integral-v4 has 46 complete families, eight bounded sparse-bar truncations, and 90 degree-two truncations |
| Eilenberg–Mac Lane anima | 224 spaces from finite abelian groups through order 32 and `Z`, with delooping degrees 1–4 | Invariants-v8 has all 304 modular rows, integral additive data for 60 spaces, and 51 complete integral rings; the remaining integral cells are pending |

Group-identification failures remain visible. Steiner categorical homology is
the stored bounded Street-nerve and finite-particle presentation. Witnessed
collapse relations are not asserted to classify homotopy types.

## Data roots and integrity

The explicit data root contains `finite-posets/`,
`strong-steiner-categories/`, `finite-simplicial-complexes/`,
`finite-simplicial-sets/`, `finite-group-cohomology/`, `eilenberg-maclane/`,
and `steiner-groups/`. These may be durable directory mounts. Their selected
files must match the committed selection; no path in the selection depends on
a particular workstation or temporary directory.

Verify the full selection with:

```bash
python3 scripts/volume-one.py verify --data-root "$TYPELAKE_DATA_ROOT"
```

This checks file integrity and declared row counts. Mathematical replay is
reported separately, with its actual extent. The supplementary checks are:

```bash
make check
make test
make lint
python -m pytest -q
cargo run --release -p typebase-build --example replay_samples -- \
  --data-root "$TYPELAKE_DATA_ROOT" --report "$EVIDENCE_ROOT/samples.json"
```

The sample replay takes the first, middle, and last row of each selected Poset, Steiner, simplicial-complex and simplicial-set shell, including F3 operation samples. It explicitly omits finite-group cohomology and Eilenberg–Mac Lane replay. Its report records these limits and the number of objects examined. Full release and
independent-oracle checks remain part of publication qualification.

## Noether 0.3.2

The workspace locks one Noether dependency to public revision
`eebc33d4f37ed47af83c88dbc9f8886b254f72c6`, release `v0.3.2`.
New computations name that public revision. The frozen review selection retains
the earlier producer revision `4c92c269c59da02a4d32c833c2ee093407f8e22f`;
its input bytes and producer manifests are unchanged. Readers also accept the explicitly
retained historical producers while checking each input artifact's hashes and
upstream links. Unsupported producer revisions still fail validation.

An existing result hash identifies the historical Noether encoding. If a
current replay changes only such hashes, the regenerated record is installed
in a separate successor root. The old bytes and producer manifests remain
available. The migration utility records changed field paths and checks object
identities before installing a validated successor batch:

```bash
cargo run --release -p typebase-build --example noether_migration -- \
  --collection eilenberg-maclane --data-root "$HISTORICAL_DATA_ROOT" \
  --output-root "$SUCCESSOR_DATA_ROOT" --report "$EVIDENCE_ROOT/eml.json"
```

The other supported migration collection is `finite-group-modular`. A
projection's `noether_revision` names its source producer;
`builder_noether_revision` separately names the current projection builder.

The modular batch can also be reconstructed with durable per-record
checkpoints and independent per-group Rust validation jobs:

```bash
python3 scripts/migrate-group-cohomology.py \
  --data-root "$HISTORICAL_DATA_ROOT" --output-root "$SUCCESSOR_DATA_ROOT" \
  --records-root "$CHECKPOINT_ROOT" --report "$EVIDENCE_ROOT/groups.json" \
  --generator target/release/finite-group-cohomology-census \
  --validator target/release/finite-group-cohomology-census
```

Every group validation reconstructs all of its coefficient records with the
same Rust validator as the whole-census command. The combined report requires
disjoint group coverage and consistent engine and proof-range metadata.
Coordinate products and witnesses may change between Noether versions; these
changes are recorded in the successor report without rewriting the originals.

## Local browsing

See [REBUILDING.md](REBUILDING.md) for a fresh-machine setup, portable data
bundles, and a single command that rebuilds all six browser projections.

Browser data lives in durable storage and is mounted beneath the ignored
`web/typebase/public/data/` directory. Select the intended candidate before
starting the browser; every collection needs its full detail and search files,
not only a manifest. See [`web/typebase/README.md`](../web/typebase/README.md)
and its final-four browser contract for the projection builders and routes.

```bash
npm --prefix web/typebase ci
npm --prefix web/typebase run review
```

The development site opens at `http://localhost:4173/`. The review command sets
each `VITE_TYPEBASE_*_DATA_URL` to the corresponding absolute `/data/<collection>/`
path for both clean object routes and `/typebase/` HTML aliases. Existing
environment overrides are respected.
These are development mounts; public hosting, archive publication, and branch
cleanup follow editorial review.

## Mapping-model compatibility

The Noether source dependency remains the published 0.3.2 revision. The census
uses its own full-cell mapping-model compatibility implementation, with the
algorithm id recorded in new run reports. This retains complete whiskering
coordinates and verifies boundary realizations bijectively. The corresponding
upstream Noether correction is prepared separately; no unpublished upstream
revision is required to rebuild TypeLake's corrected candidate.
