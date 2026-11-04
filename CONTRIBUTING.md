# Contributing

TypeLake stores mathematical records with provenance and validation. TypeBase
selects those records into coherent, citable catalogues. Both are part of the
[Tertius academic collaboration](https://tertius.cc/).

Start with [README.md](README.md), [ARCHITECTURE.md](ARCHITECTURE.md), and the
relevant collection's `PLAN.md`, `RECORD.md`, and browser contract under
[`collections/`](collections/). The current Volume I selection is an editorial
review candidate, not a released catalogue.

## Changes to collections

- State the mathematical conventions, scope, and evidence behind a change.
- Keep record contracts local to their collections. The retained SciType
  prototype is validation infrastructure, not a separate public schema authority.
- Include small, reproducible examples and meaningful validation cases.
- Preserve accepted record bytes and producer metadata. Corrections and
  recomputations belong in new immutable batches with their actual coverage.
- Keep completeness and rigor claims within the extent that was checked.
- Keep bulk records, browser projections, checkpoints, and local paths out of Git.

For a browser change, check the affected catalogue and direct record routes
with local review data. For a Rust change, use the compatible Noether/FLINT
setup in [the rebuilding guide](collections/REBUILDING.md), then run `make check`,
`make test`, and `make lint`.

The source quickstart runs the Python tests and browser build without any
catalogue data. If you change the retained schemas or canonical encodings,
also run `.venv/bin/scitype emit` and include the updated conformance artifacts.
New collection work must not depend on permanent SciType lineage identifiers.

## Provenance and licensing

Sign off commits (`git commit -s`) to certify the
[Developer Certificate of Origin](https://developercertificate.org/).
There is no contributor license agreement.

Code contributions use the [MIT license](LICENSE). Submitted facts are dedicated
[CC0 1.0](LICENSE-DATA); see [the data norms](DATA_NORMS.md) for provenance and
citation expectations. Tell us before submitting if you cannot contribute
under these terms.
