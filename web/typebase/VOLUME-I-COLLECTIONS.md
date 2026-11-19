# TypeBase Volume I: final-four browser contract

This document covers the static reader for finite simplicial complexes, finite
simplicial sets, finite-group cohomology, and Eilenberg--Mac Lane anima. The
mathematical records and completeness claims are owned by the corresponding
collection contracts. The browser derives presentation from the immutable
TypeBase projections and does not recompute mathematics.

## Routes

| Collection | Catalogue | Persistent record |
| --- | --- | --- |
| Simplicial complexes | `/SimplicialComplex/` | `/SimplicialComplex/a.b.c` |
| Simplicial sets | `/SimplicialSet/` | `/SimplicialSet/a.b.c` |
| Finite-group cohomology | `/FiniteGroupCohomology/` | `/FiniteGroupCohomology/n.k/R` |
| Eilenberg--Mac Lane anima | `/EilenbergMacLane/` | `/EilenbergMacLane/A.n/R` |

Every collection also has `Source`, `Completeness`, and `Glossary` beneath its
catalogue path. A finite-group label links to the matching LMFDB abstract-group
page. A requested coefficient that is not in the selected projection is shown
as pending; the reader never substitutes a different coefficient silently.

## Static data contract

Each data root contains `manifest.json`, `search.json` or `search.json.zst`, and
compressed detail objects. The simplicial projections additionally provide:

- `catalogue.json`, containing only status, counts, label buckets, and detail
  part size;
- `landing.json`, containing the first 30 canonical rows;
- one compressed search file per label bucket.

Normal simplicial catalogue and object pages use `catalogue.json`; only Source
and Completeness request the full release manifest. A restriction or label
prefix that fixes a bucket downloads only that bucket. Unrestricted pagination
beyond the landing rows may load the complete compressed search index.

Audited payloads for the selected edition are:

| Collection | Startup/search data | Detail maximum | Total projection |
| --- | ---: | ---: | ---: |
| Simplicial complexes | 1,921 B + 16,058 B landing | 69,325 B | 4,318,826 B |
| Simplicial sets | 2,418 B + 20,395 B landing | 36,938 B | 34,878,291 B |
| Finite-group cohomology | 186,592 B search | 53,450 B | 992,187 B |
| Eilenberg--Mac Lane anima | 9,856 B compressed search | 1,764 B | 337,641 B |

The shared collection JavaScript is approximately 41 KB uncompressed and is a
separate entry chunk; it does not import the DuckDB search runtime used by the
older Poset reader.

## Build and hosting

`vite.volume-one.config.ts` extends the established TypeBase build instead of
forking it. It adds the collection shell, injects collection links into the
existing frames, supplies development route middleware, and emits the final
route loader as exactly `404.html`. For compatibility across the landing-page
transition, it also emits `posets.html` from `index.html` only when the build
does not already contain a dedicated Poset shell.

The deployed site must either use that file as the site-wide static 404 or
rewrite the four route prefixes to `/typebase/collections.html`. The route
loader preserves the requested URL and loads the appropriate shell, so object
pages remain persistent without a server-side application. Static assets and
data are mounted below `/typebase/`; the mathematical routes remain at the
site root.

The data roots can be overridden independently at build or development time:

```text
VITE_TYPEBASE_SIMPLICIAL_COMPLEX_DATA_URL
VITE_TYPEBASE_SIMPLICIAL_SET_DATA_URL
VITE_TYPEBASE_FINITE_GROUP_COHOMOLOGY_DATA_URL
VITE_TYPEBASE_EILENBERG_MACLANE_DATA_URL
```

With development data mounted under `public/data/`, the local gates are:

```bash
cd web/typebase
npm run build
npm run dev -- --host 127.0.0.1
```

The release audit additionally rebuilds each simplicial projection twice and
compares the output directories recursively, runs TypeScript and Vite, checks
the exact `404.html` name, and requests representative catalogue, detail,
Source, Completeness, and Glossary routes plus their JSON/Zstandard data.
