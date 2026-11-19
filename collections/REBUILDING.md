# Rebuilding and running Volume I on another machine

Git contains the source and the frozen input selection. The mathematical
records and browser projections are separate, ignored data. Transfer a data
bundle as well as cloning the repository. The bundles below are private review
snapshots, not published releases.

Public source checks and the browser build need no data bundle; see the
[source quickstart](../README.md#source-quickstart). The catalogue commands below
are for readers who already have access to a review bundle. A public download
location will accompany the catalogue release.

The selected Eilenberg–Mac Lane batch was rebuilt and validated with Noether
0.3.2. Finite-group cohomology still selects its validated historical batch;
the reconstructed 0.3.2 modular successor is awaiting its last group replay.
Historical producer metadata is retained throughout.

## Run the existing catalogue

This path needs Git, Node.js 22 with npm, and `zstd`. Rust, FLINT, and a
database server are unnecessary for reading the transferred projections.
The commands work from a Linux or macOS shell.

```bash
git clone https://github.com/tertius-ai/typelake.git
cd typelake

# Set these to the original machine and its prepared transfer directory.
TYPELAKE_SOURCE=your-ssh-host
TYPELAKE_TRANSFER=/absolute/path/to/transfer-directory
mkdir -p ../typelake-transfer
scp "$TYPELAKE_SOURCE:$TYPELAKE_TRANSFER/typelake-volume-i-browser.tar.zst" ../typelake-transfer/
scp "$TYPELAKE_SOURCE:$TYPELAKE_TRANSFER/typelake-volume-i-browser.tar.zst.sha256" ../typelake-transfer/

(cd ../typelake-transfer && shasum -a 256 -c typelake-volume-i-browser.tar.zst.sha256)
mkdir -p web/typebase/public/data
zstd -dc ../typelake-transfer/typelake-volume-i-browser.tar.zst | tar -xf - -C web/typebase/public/data

npm --prefix web/typebase ci
npm --prefix web/typebase run build
npm --prefix web/typebase run review
```

Open <http://localhost:4173/>. The review command checks that every collection
has a manifest and sets the seven data URL variables needed by the landing
page, collection pages, and clean record routes. Stop the server with Ctrl-C.
On Linux, `sha256sum -c` can replace `shasum -a 256 -c`.

The browser archive contains all six collection directories and a
`review-snapshot.json` that records the source commit, input selection hash,
and projection manifest hashes. It contains file contents rather than the
original machine's symlinks. Extract it into a fresh data directory.

For a server accessed over SSH, run the review command there, and run this
in a terminal on the machine with your browser:

```bash
ssh -N -L 4173:127.0.0.1:4173 your-ssh-host
```

## Rebuild the projections from frozen mathematical records

This path also needs Python 3.10+, a Rust toolchain, a C/C++ toolchain,
`make`, `m4`, `curl`, and `pkg-config`. The source build was checked with
Rust 1.95.0 on Linux; the workspace declares Rust 1.89 as its minimum.
For example, install/select the tested toolchain with:

```bash
rustup toolchain install 1.95.0
rustup override set 1.95.0
```

Install a compatible FLINT 3.6 stack. The tagged Noether installer builds its
pinned GMP, MPFR, and FLINT dependencies locally. There is no need to build or
install Noether's Python extension for the TypeLake Rust builders:

```bash
# Run from the TypeLake repository root; this creates a sibling checkout.
git clone --branch v0.3.2 --depth 1 https://github.com/tertius-ai/noether.git ../noether
(cd ../noether && bash scripts/install-flint.sh)

export FLINT_PREFIX="$(cd ../noether/.deps/flint && pwd)"
export PKG_CONFIG_PATH="$FLINT_PREFIX/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export LD_LIBRARY_PATH="$FLINT_PREFIX/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="$FLINT_PREFIX/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
pkg-config --atleast-version=3.6 flint
cargo build --locked --release -p typebase-build
```

Cargo fetches the exact Noether revision in `Cargo.lock`. The sibling checkout
supplies the native dependency installer; it does not replace that Cargo pin.
An existing compatible FLINT prefix can be used instead of the installer.

Transfer and verify the frozen records. The input archive expands to about
7.8 GB and includes exactly the selected files, its own `volume-i.json`, and
snapshot metadata:

```bash
scp "$TYPELAKE_SOURCE:$TYPELAKE_TRANSFER/typelake-volume-i-inputs.tar.zst" ../typelake-transfer/
scp "$TYPELAKE_SOURCE:$TYPELAKE_TRANSFER/typelake-volume-i-inputs.tar.zst.sha256" ../typelake-transfer/
(cd ../typelake-transfer && shasum -a 256 -c typelake-volume-i-inputs.tar.zst.sha256)

export TYPELAKE_DATA_ROOT="$(cd .. && pwd)/typelake-volume-i-inputs"
mkdir -p "$TYPELAKE_DATA_ROOT"
zstd -dc ../typelake-transfer/typelake-volume-i-inputs.tar.zst | tar -xf - -C "$TYPELAKE_DATA_ROOT"
python3 scripts/volume-one.py verify \
  --data-root "$TYPELAKE_DATA_ROOT" \
  --selection "$TYPELAKE_DATA_ROOT/volume-i.json"
```

Use an empty browser-data destination. If you already restored the browser
bundle, use a separate fresh checkout for this alternative. The builder
refuses to overwrite an existing collection directory:

```bash
python3 scripts/build-volume-one.py \
  --data-root "$TYPELAKE_DATA_ROOT" \
  --selection "$TYPELAKE_DATA_ROOT/volume-i.json" \
  --output-root "$PWD/web/typebase/public/data"

npm --prefix web/typebase ci
npm --prefix web/typebase run build
npm --prefix web/typebase run review
```

The script verifies every frozen input before invoking the six Rust projection
builders. It includes the retained group-identification enrichments and the
selected collection bounds. To rebuild one projection into another empty
destination, add `--collection finite-simplicial-complexes`, for example.
No GAP installation is needed to reuse the transferred identifications.

This reconstructs browser projections from stored, validated records.
Recomputing the mathematical censuses is a separate, potentially long-running
operation; collection commands are available through `simplicial-complex-census --help`, `simplicial-set-census --help`, and `eilenberg-maclane-build --help`, as well as the root README and collection plans. The input bundle's selection identifies its exact
snapshot even if a later Git checkout selects a newer batch.

For source checks, from the repository root with the FLINT variables above:

```bash
python3 -m venv .venv
.venv/bin/python -m pip install -e '.[dev]'
make check
make test
make lint
.venv/bin/python -m pytest -q
```

The browser is served locally for review. Publishing, the final TypeLake
0.1.0 tag, public hosting, and branch cleanup follow editorial review.

## Builder root conventions and validation

The Poset and Steiner builders take their collection root (the directory
containing `census/`); the simplicial, finite-group and Eilenberg–Mac Lane
builders take the bundle root containing all named collection directories.
`STEINER_CENSUS_DATA_ROOT` and `POSET_CENSUS_DATA_ROOT` are collection roots;
`SIMPLICIAL_COMPLEX_DATA_ROOT`, `SIMPLICIAL_SET_DATA_ROOT`,
`FINITE_GROUP_COHOMOLOGY_DATA_ROOT`, and `EILENBERG_MACLANE_DATA_ROOT` are
bundle roots. `scripts/build-volume-one.py` supplies the appropriate level.

The simplicial-complex projection takes `--f3-cohomology-version 2` by default.
An old selection is rebuilt with `--f3-cohomology-version 1`; the full-volume
script derives that choice from its verified selection. Other installed lanes
cannot silently substitute for the declared one.

`make test-oracles` runs the ignored order-6/7 Poset key-set comparisons and
the size-8 Steiner comparison. `make test-hap-oracle` requires a complete
`FINITE_GROUP_COHOMOLOGY_DATA_ROOT` and an installed GAP/HAP. It includes
family and resolution paths and the C2, C4, V4, D8 and Q8 hand examples.
Unrequested HAP checks are reported as ignored, not as passing tests.
