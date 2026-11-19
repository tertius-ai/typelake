# TypeBase browser

This is the static reader for TypeBase Volume I. It is a small, framework-free
TypeScript application; DuckDB-Wasm reads the immutable Parquet projection in
the browser. There is no application server, database process, or generated SVG
corpus.

The production layout is:

```text
/typebase/
  index.html
  posets.html
  steiner.html
  reference.html
  assets/                         content-hashed site and DuckDB-Wasm assets
  data/finite-posets/
    manifest.json
    landing.json                    first catalogue page; no query engine needed
    search/order=<n>/posets.parquet
    details/order=<n>/dimension=<d>/part-<p>.parquet
  data/strong-steiner-categories/
    manifest.json
    catalogue.json
    gallery/
      index.json                    ordered list of bounded thumbnail shards
      part-<p>.json                 48 lightweight ι₂ diagram records
    interesting.json
    gray-table.json
    details/<a.d.c>.json
```

For a fresh checkout and data transfer, start with
[`collections/REBUILDING.md`](../../collections/REBUILDING.md). A Git clone
does not include the catalogue data. Once all six projections are installed
under `public/data/`, build and review with:

```bash
npm ci
npm run build
npm run review
```

Development data can be exposed under `public/data/`; that path is ignored by
Git. Alternatively, set `VITE_TYPEBASE_DATA_URL` and
`VITE_TYPEBASE_STEINER_DATA_URL` to absolute or site-relative data roots.
Production uses the same-origin paths shown above.

The fast path is deliberately separate from analytical search. Initial Poset
browsing reads `landing.json`; a direct record reads its known row from one
bounded detail part with the small JavaScript Parquet reader. DuckDB-Wasm is
loaded only after a user submits a filtered search or moves beyond the cached
landing page. Direct Steiner records likewise fetch only their detail JSON.
The Volume I landing page has a gallery switcher for all six collections. It reads
the small Steiner gallery index and streams one 48-row diagram shard as it
approaches the end of the visible contact sheet. The Poset gallery is lazy: it
reads bounded existing detail parts only after Posets is selected and appends
48 diagrams at a time. Neither route loads a full catalogue at startup, and
neither initializes DuckDB-Wasm. Do not move query-engine initialization back
into application startup.

## Static-host requirements

- Serve `.wasm` as `application/wasm` and `.parquet` as
  `application/octet-stream`.
- Exact-label lookups download one bounded detail part. `rows_per_detail_part` controls that bound; the current reader does not issue Parquet range requests.
- Cache content-hashed assets and frozen volume files as immutable. Publish a
  new volume path instead of overwriting an existing one.
- If data is placed on another origin, allow `GET`, `HEAD`, and range requests
  from `https://tertius.cc` with CORS. Same-origin hosting is simpler.

Production builds use the Vite base `/typebase/`; the development server stays
at `/` for convenient SSH-tunnel review. The built directory can therefore be
copied directly to the production `/typebase/` path. Public records and
references have clean, persistent routes:

```text
/Poset/10.3.266967
/Steiner/10.2.51
/Glossary#height
/Posets/Source
/Posets/Completeness
/Steiner/Source
/Steiner/Completeness
```

On GitHub Pages, copy the built `404.html` to the site root as `404.html`. It is
a small route loader: it selects the Poset, Steiner, or reference shell while
preserving the requested clean URL. Hosts with edge rewrites may send the same
routes directly to `/typebase/posets.html`, `/typebase/steiner.html`, or
`/typebase/reference.html`. Old `?label=` links redirect to the clean route.

`npm run preview` serves the built directory under `/typebase/` only. It runs
neither the route loader nor the development route middleware, so root-level
routes, such as those above and the final-four routes in
[`VOLUME-I-COLLECTIONS.md`](VOLUME-I-COLLECTIONS.md), return 404 there. Check
routes with `npm run review`.

## Presentation proof room

Presentation is deliberately independent of the immutable mathematical data.
Every catalogue page has a compact proof stepper, and every open record repeats
that control beside a diagram-plate stepper. Choices survive navigation between
the Poset and Steiner collections and are encoded in shareable URLs:

```text
/Poset/4.1.7?style=swiss-index&plate=survey
/Steiner/10.2.51?style=nocturne&plate=cyanotype
```

The 25 website proofs are Technical memorandum, Card catalogue, Mathematical
folio, Field notes, Swiss index, Blueprint office, Tabulator, Biblioteca,
Constructivist, Hydrographic chart, Laboratory sheet, Microfilm reader,
Concrete index, Herbarium, Bureau form, Blackboard, Phosphor terminal,
Risograph, Theorem journal, Punch card, Cartographic index, Type specimen,
Nocturne, Illuminated codex, and Archival box.

The 10 diagram plates apply to both Hasse diagrams and strong-Steiner computad
plates: Byrne plate, Cyanotype, Copperplate, Bauhaus, Survey sheet, Manuscript,
Oscilloscope, Tabular ink, Cartographic, and Chalk study. They change only the
derived SVG presentation; geometry and mathematical records are unchanged.
The authoritative registries and stable URL identifiers live in
`src/presentation.ts`.

## Collection data roots

`VITE_TYPEBASE_DATA_URL` and `VITE_TYPEBASE_STEINER_DATA_URL` select the Poset
and Steiner projections. The other four are
`VITE_TYPEBASE_SIMPLICIAL_COMPLEX_DATA_URL`,
`VITE_TYPEBASE_SIMPLICIAL_SET_DATA_URL`,
`VITE_TYPEBASE_FINITE_GROUP_COHOMOLOGY_DATA_URL`, and
`VITE_TYPEBASE_EILENBERG_MACLANE_DATA_URL`. Source and Completeness pages use
these same roots.

Space-name searches match names attached by exact presentation checks in
`typebase-build::named_spaces`. They include spheres, circles, points,
intervals, and bouquets of circles. Homology alone never supplies a name.
The manifest and `builder_source` identify the edition behind every result.
