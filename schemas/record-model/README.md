# Legacy record-model prototype

This directory predates the decision to use collection-owned TypeLake contracts
and coherent object rows. It is retained temporarily as migration input and for
its existing conformance tests; it is not the target record model for the
finite-Poset collection. New collection work must follow
[`ARCHITECTURE.md`](../../ARCHITECTURE.md) and must not add dependencies on the
global `scitype:` registry or a universal fact-per-scalar envelope.

## Historical design

How a store records observations about mathematical objects. Moved here from
`tertius-ai/scitype`, which specifies how the objects themselves are written
down and should not oblige anyone to adopt our storage.

| | |
|---|---|
| `fact` · `relation` | the envelope an observation travels in |
| `collection` · `volume` · `discovery` | manifests: configuration, not content |
| `Rigor` · `Provenance` | attached to every value |
| `CompletenessCertificate` · `Erratum` | what is known to be missing, and what was wrong |
| `assumptions.yaml` | GRH and friends, as ids rather than prose |
| `*.jsonl` | conformance vectors for validating facts and records |

**These still carry `scitype:` lineage numbers and need renumbering into this
authority.** Nothing was published, so it is free now.
