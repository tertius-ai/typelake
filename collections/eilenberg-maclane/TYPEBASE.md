# TypeBase projection: Eilenberg--Mac Lane anima

Collection path: `/EilenbergMacLane/`.

Object and coefficient paths are:

```text
/EilenbergMacLane/8.2.2
/EilenbergMacLane/8.2.2/Z
/EilenbergMacLane/8.2.2/F2
/EilenbergMacLane/Z.3/F3
```

The title renders both `K(A,n)` and the label. A finite group field links to
`https://www.lmfdb.org/Groups/Abstract/<order.index>` and is accompanied by the
invariant-factor decomposition, so the external label is not the only account
of the group.

The page uses the compact TypeBase frame and opens with two adjacent degree
tables for homology and cohomology. The principal visual is an operation
genealogy rooted at `iota_n`, with restrained colored edges for Bocksteins,
Steenrod squares, and reduced powers. Selecting a class shows its canonical
cocycle, products, alternate operation names, and evidence without navigating
away from the object.

Every modular ring view distinguishes a `complete closed form` from `exact
through degree d`. The latter prints “no claim above degree d” beside the
presentation rather than hiding the qualification in a source note. For the
Cartan--Serre lanes, the generator genealogy is reconstructed directly from
the stored admissible square or Bockstein/reduced-power words; an empty
operation table means that only generator origins, not all operation values,
were claimed.

Search fields include finite/infinite group, group order, LMFDB label or
structure text, delooping degree, coefficient characteristic, presentation
extent, integral-torsion status, generator and relation degrees, and named
operations. Integral rows also carry separate homology and cohomology
signatures, so a reader can restrict a selected additive degree by free rank or
torsion order. Search results are compact adjacent rows, not cards.

The source page identifies GAP, SmallGrp, LMFDB links, Noether revision, and
artifact hashes. The completeness page is generated from the pinned manifest
and displays the entire group-by-delooping-by-coefficient coverage matrix. A
development page may expose pending cells, but Volume I cannot call the
collection complete until the accepted parameter box is filled and all release
gates in `PLAN.md` pass.

The implemented projection selects `invariants-v8`, writes a compact
coefficient-level JSON search index, and stores one Zstandard-compressed detail
object per symbolic space. Its 528 search rows comprise 224 integral views and
304 prime-field views. All modular views are available; the integral search
rows preserve the exact 60 available / 164 pending split for both homology and
additive cohomology and expose 51 complete closed-form integral rings / 173
explicit pending ring cells. Integral search rows include generator and
relation degrees and the integral Bockstein and Tor operation families when a
complete ring is available. The component manifest therefore says
`modular_complete_integral_partial` and includes the selected TypeLake hashes
rather than hiding the pending integral work.

## Implemented reader

The catalogue loads one 9,856-byte compressed coefficient-level search index
and then one of 224 compressed space details; the largest detail is 1,764
bytes.
Persistent coefficient routes render the operation-named presentation or an
explicit pending cell, with LMFDB links for finite groups. Source,
Completeness, and Glossary live at the corresponding collection subpaths and
are generated from the selected manifest. The audited component is 337,436
bytes in 226 files.
