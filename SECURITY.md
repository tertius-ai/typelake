# Security

TypeLake's readers and validators consume mathematical records, manifests,
schemas, and conformance cases. TypeBase renders selected records in a browser.
Provenance `recreate` snippets document computations; readers do not execute
those snippets. Census and projection commands run local mathematical
computations when explicitly invoked.

Of particular interest:

- Records that cause code execution, browser script injection, or access outside
  the intended data directory.
- Malformed input that causes unbounded memory or computation during validation
  or ordinary catalogue browsing.
- Distinct mathematical objects receiving the same identifier, or one object
  receiving inconsistent identifiers under the same contract.
- Checks that accept corrupted artifacts or misstate validation coverage.
