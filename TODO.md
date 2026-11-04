# TypeLake / TypeBase — TODO

Ordered by what blocks what. **Bold = blocks other items.** Collection schemas
are local TypeLake contracts; SciType is no longer a separate product or public
standard.

## Architecture transition

1. **Complete:** define `finite-posets/v1` as the first collection-owned record,
   relation, convention, and batch-manifest contract.
2. **Complete:** make the Poset builder emit coherent object rows with
   batch-level provenance and no universal fact-per-scalar format.
3. Inventory the legacy SciType implementation by behavior: retain useful
   canonical JSON, validation, and conformance tests; mark global lineage,
   registry, and schema-authority code for removal.
4. Rename the package and command only after replacement contract tests cover
   the retained behavior.

## TypeLake

12. **Implement the atomic commit protocol** — mutable pointer naming an immutable snapshot, committed by conditional put.
13. Define deterministic resolution of object and enrichment batches, preserving
    equally qualified conflicts rather than silently choosing one.
14. **Build the incremental view builder** over collection-owned contracts.
15. Stand up the S3 layout, per-collection manifests, and the discovery document.
16. Write the ingest gate: validate the collection contract, canonicalize, check
    provenance, sample an oracle recheck, and quarantine anything unverified.
17. Build the local MCP server with read tools only.
18. Write the cold-read test and make it a release gate.

## Collections

19. **Complete through order 10:** isomorph-free poset generation, following the
    [finite-poset data plan](collections/finite-posets/PLAN.md) and gated at each
    n on A000608 and on composition reproducing A000112.
20. **Complete through order 10:** integral homology, Stong cores, and the
    full natural automorphism action as specified in the
    [finite-poset data plan](collections/finite-posets/PLAN.md).
21. **Complete through the stable finite-particle presentation:** add Noether's
    `category` domain and its validated finite strong Steiner category surface,
    including polygraphic homology and the categorical-homology source model,
    monomorphic oriented basepoints and homotopy Posets, and Gray tensor,
    following the
    [Steiner-category census plan](collections/strong-steiner-categories/PLAN.md).
22. **Complete for identity/light through atomic size 12 and rich enrichment
    through size 11:** build redundant and orderly Steiner-category generators,
    split immutable identity, light, and rich lanes, enumerate basepoint
    orbits, link their homotopy Posets, and build the bounded Gray table.
    Next: shard and checkpoint the identity lane before sizes 13--15.
23. Compute mod-p and integral cohomology for the 144 groups of order ≤ 32, with restriction and transfer.
24. Migrate DedekindDB out of the Python package and into the lake as the ingest shakedown.

## TypeBase

25. **Complete locally:** static museum-catalogue browser using DuckDB-Wasm over
    bounded Parquet search and detail projections.
26. **Complete for initial Poset facets:** order, dimension, core, connectivity,
    self-duality, lattice status, and accession prefix.
27. **Complete:** freeze the Poset label scheme as
    `points.order-complex-dimension.canonical-index`.
28. **Complete locally through weight-2 stable component slices:** add the
    static Steiner-category 2-skeleton, linked mapping-category inspector,
    polygraphic/categorical-homology views, homotopy-Poset basepoint browser,
    Gray multiplication table, `a.d.c` labels, and curated interesting page.
29. Freeze Volume 1 and mint concept plus version DOIs on Zenodo.
30. Publish per-collection validation and coverage pages showing count checks, oracle agreement, and sample re-execution results.
31. **Complete for Poset automorphisms:** independently identify supported
    groups with GAP and render their `N.i` labels as live LMFDB links.

## Worth doing if time allows

32. Ship a `verify` command that re-derives a volume's views from facts and checks the hashes.
33. Contribute the homotopy-type sequence to OEIS, credited and linked back.

## Explicitly not Day 1

Posets beyond n = 10 · integral cohomology beyond what we can verify · a live
query server · a hosted MCP endpoint or `submit` · postcard · outside bulk ingest
with SLAs · auth or accounts · a query builder beyond facets.
