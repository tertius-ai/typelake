#!/usr/bin/env bash
# Run the six-collection browser against local, ignored projection data.
set -euo pipefail

repository="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for collection in finite-posets strong-steiner-categories finite-simplicial-complexes finite-simplicial-sets finite-group-cohomology eilenberg-maclane; do
    if [[ ! -f "$repository/web/typebase/public/data/$collection/manifest.json" ]]; then
        printf 'Missing browser data: %s\nSee collections/REBUILDING.md for data transfer and rebuild instructions.\n' "$collection" >&2
        exit 1
    fi
done

export VITE_TYPEBASE_DATA_URL="${VITE_TYPEBASE_DATA_URL:-/data/finite-posets/}"
export VITE_TYPEBASE_STEINER_DATA_URL="${VITE_TYPEBASE_STEINER_DATA_URL:-/data/strong-steiner-categories/}"
export VITE_TYPEBASE_SIMPLICIAL_COMPLEX_DATA_URL="${VITE_TYPEBASE_SIMPLICIAL_COMPLEX_DATA_URL:-/data/finite-simplicial-complexes/}"
export VITE_TYPEBASE_SIMPLICIAL_SET_DATA_URL="${VITE_TYPEBASE_SIMPLICIAL_SET_DATA_URL:-/data/finite-simplicial-sets/}"
export VITE_TYPEBASE_GROUP_COHOMOLOGY_DATA_URL="${VITE_TYPEBASE_GROUP_COHOMOLOGY_DATA_URL:-/data/finite-group-cohomology/}"
export VITE_TYPEBASE_FINITE_GROUP_COHOMOLOGY_DATA_URL="${VITE_TYPEBASE_FINITE_GROUP_COHOMOLOGY_DATA_URL:-/data/finite-group-cohomology/}"
export VITE_TYPEBASE_EILENBERG_MACLANE_DATA_URL="${VITE_TYPEBASE_EILENBERG_MACLANE_DATA_URL:-/data/eilenberg-maclane/}"

cd "$repository/web/typebase"
exec npm run dev -- --host 127.0.0.1 --port 4173 --strictPort "$@"
