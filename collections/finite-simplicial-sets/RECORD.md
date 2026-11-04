# Finite simplicial-set record

The collection contract is `finite-simplicial-sets/v1`.

```text
SimplicialSetIdentityRecord {
  record_version
  key: { nondegenerate_faces }
  total_nondegenerate
  dimension
  nondegenerate_counts
  canonical_bytes_sha256
  automorphism_group: { degree, order, generators, verified }
}
```

A nondegenerate face is either a cell one dimension below or a canonical
degeneracy `{ simplex_dimension, simplex, surjection }`. The defining face data
is stored in dimension-major canonical order.

Every manifest states both its total-cell and dimension bounds. Coverage tables
are refined by `nondegenerate_counts`; a total-size count without its dimension
cap is never called complete.

## Topology enrichment

The current enrichment contract is `finite-simplicial-sets/topology-v2`.

```text
SimplicialSetTopologyRecord {
  record_version
  simplicial_set
  euler_characteristic
  normalized_boundaries: [{ degree, rows, columns, canonical_bytes_sha256 }]
  normalized_chain_reduction {
    method = "universal-unit-algebraic-morse"
    coefficient_context = "Z"
    source_cells
    reduced_cells
    cancellations: [{ degree, row, column, unit }]
    reduced_modules: [{ degree, rank }]
    reduced_differentials: [
      { degree, rows, columns, canonical_bytes_sha256 }
    ]
    witness_verified = true
  }
  integral_homology
}
```

The boundary matrices are the normalized boundaries on nondegenerate cells;
degenerate simplices do not add chain generators. Validation reconstructs and
hashes every boundary, recomputes integral homology, and compares both chain
and homology Euler characteristics. Every stored cancellation is interpreted
in the current basis at that replay step. Validation reconstructs the source
normalized chain complex, replays all `+1` and `-1` pivots through Noether, and
checks the target module ranks and differential hashes. This is certified
chain-homotopy reduction; no simplicial simple or anodyne equivalence is
inferred from it.

## Cohomology enrichment

The cohomology contract is `finite-simplicial-sets/cohomology-v1`. It stores
canonical cocycles in the normalized nondegenerate-simplex bases, every basis
cup product, an exact connected graded-commutative presentation when `H^0` is
the coefficient field, and a complete finite multiplication-table certificate
otherwise. Every record is therefore complete, not truncated.

At `F2`, the operation table contains every `Sq^k(x)` for
`0 <= k <= degree(x)`, including zero values. Evidence distinguishes `Sq^0`,
the unstable top square, and genuinely lower squares computed with Noether's
normalized Medina--Mardones cup-i formula. The `F3` pass stores the complete
graded-commutative ring and currently makes no unsupported odd-primary
operation claim. Manifests link the identity artifact revision and the exact
Noether cohomology revision separately; `validate-cohomology` hash-checks and
recomputes every installed record. Supported odd-primary claims live in the
separately versioned sidecar below, so this immutable base contract remains
unchanged.

The installed frontier artifacts are:

| Box | Field | Records | Bytes | SHA-256 |
|---|---:|---:|---:|---|
| `(6,3)` | `F2` | 89,531 | 502,324,680 | `228781d187bebaeebef04f4d1a1d7c0054f0e9f3f561ef24b4332ec8407a0fa8` |
| `(7,2)` | `F2` | 30,808 | 165,497,707 | `87e434d254fd9038ed5d5aa6eb0e0b1c4cc1fd31a65eae733f4ce6e6ff5eee45` |
| `(6,3)` | `F3` | 89,531 | 298,310,946 | `7905330e6c9c901ff916336f8f67b50fc84efdaaecbef58ce2a82db16071eb8f` |
| `(7,2)` | `F3` | 30,808 | 114,470,172 | `dcdc1e185d5ef1263ae4c3fca0d5bc98bd8bb25669c601a20deaad83b5950d87` |

Smaller shells complete the downward-closed frontier. The two installed
cohomology trees occupy approximately 663 MB at `F2` and 410 MB at `F3`.

## Odd-primary Steenrod-operation enrichment

The additive contract
`finite-simplicial-sets/steenrod-operations-v1` augments an immutable odd-prime
`cohomology-v1` shell without repeating its algebra. Every record is linked to
the SHA-256 digest of the exact serialized cohomology record, while its
manifest also links the complete identity and cohomology JSONL artifacts.

```text
SimplicialSetSteenrodOperationsRecord {
  record_version
  simplicial_set
  coefficient_context
  cohomology_record_sha256
  operations: [
    {
      operation
      source_id
      target_terms: [{ basis_id, coefficient }]
      evidence
    }
  ]
}
```

For every homogeneous basis element `x`, the `F3` first edition stores exactly
the unstable ranges

```text
P^i(x)       for 2i <= degree(x)
beta P^i(x)  for 2i <  degree(x).
```

Zero values are retained. Evidence distinguishes `P^0`, the unstable top
power, the general Kaufmann--Medina-Mardones May--Steenrod construction, and
its beta-reduced-power variant. Validation checks source hashes, the complete
unstable index range, unique `(source, operation)` names, named evidence, and
full mathematical recomputation. The underlying cochain construction follows
[Kaufmann--Medina-Mardones](https://doi.org/10.1515/forum-2020-0296).

The maximal installed artifacts are:

| Box | Field | Records | Operation values | Bytes | SHA-256 |
|---|---:|---:|---:|---:|---|
| `(6,3)` | `F3` | 89,531 | 668,055 | 194,292,356 | `d79a3d69ad430033fa2e1dd7452d321268a8faeb67d9419e9e31faaa559641f8` |
| `(7,2)` | `F3` | 30,808 | 210,826 | 55,060,495 | `9701bc68e8630e3ceb1740a669744ae81dffddecfe84d730b143561e389c20d9` |

## Witnessed homotopy relations

The homotopy sidecar contract is `finite-simplicial-sets/homotopy-v1`.

```text
SimplicialSetHomotopyRecord {
  record_version
  simplicial_set
  method = "elementary-horn-collapse/anodyne-expansion-v1"
  elementary_horn_relations: [
    {
      step: { simplex_dimension, simplex, face, free_face }
      target
    }
  ]
  greedy_horn_collapse {
    target
    source_cells
    target_cells
    steps: [{ simplex_dimension, simplex, face, free_face }]
    terminal
    verified
  }
}
```

Each immediate step removes one nondegenerate simplex together with the stated
free face. Its reverse is a horn attachment, hence an anodyne extension. The
target key is the canonical isomorphism representative after the collapse, so
it links to an identity record in an earlier total-size shell.

The canonical source key together with the structured greedy steps is the
complete Noether witness; the target before canonical relabeling is derived by
replay and is not duplicated as opaque JSON. Validation reconstructs every
immediate target, replays the complete sequence, checks source and target cell
counts, checks its canonical target and terminal claim, and independently
compares integral homology before and after reduction. `terminal` means only
that this deterministic target admits no further elementary horn collapse.
Neither it nor the greedy choice is asserted to be a unique or canonical
homotopy core.

TypeBase may form witnessed-equivalence components by taking the undirected
closure of these collapse/attachment edges inside the complete census
frontier. Such a component remains explicitly a classification under the
stored move relation, not a claim to classify all weak homotopy equivalences.

The installed maximal-shell artifacts are:

| Box | Records | Immediate edges | Nonempty greedy reductions | Bytes | SHA-256 |
|---|---:|---:|---:|---:|---|
| `(6,3)` | 89,531 | 21,789 | 18,780 | 197,072,999 | `32c3078ac9270a5a51f7150827fb0203239c653d0de8a5f37363e22ed83af26f` |
| `(7,2)` | 30,808 | 11,717 | 9,534 | 49,421,103 | `b3b24684002d15265a59e806785cb3cbbd6a20950de538cc6841face0749169b` |

Smaller identity shells have corresponding immutable `homotopy-v1` sidecars,
so this coverage is the entire measured first-edition frontier rather than
only the two boundary boxes.
