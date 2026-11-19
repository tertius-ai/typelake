#!/usr/bin/env python3
"""Verify frozen Volume I inputs and build their six browser projections."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys

COLLECTIONS = (
    "finite-posets", "strong-steiner-categories", "finite-simplicial-complexes",
    "finite-simplicial-sets", "finite-group-cohomology", "eilenberg-maclane",
)
REPOSITORY = Path(__file__).resolve().parents[1]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--selection", type=Path, default=REPOSITORY / "collections/volume-i.json")
    parser.add_argument("--builder", type=Path, default=REPOSITORY / "target/release/typebase-build")
    parser.add_argument("--collection", action="append", choices=COLLECTIONS)
    options = parser.parse_args()
    root, output = options.data_root.resolve(), options.output_root.resolve()
    builder = options.builder.resolve()
    selected = options.collection or COLLECTIONS
    if not builder.is_file():
        parser.error("build the Rust builder first: cargo build --locked --release -p typebase-build")
    for collection in selected:
        if (output / collection).exists():
            parser.error(f"immutable output already exists: {output / collection}")
    subprocess.run([sys.executable, str(REPOSITORY / "scripts/volume-one.py"), "verify",
                    "--data-root", str(root), "--selection", str(options.selection.resolve())], check=True)
    selection = json.loads(options.selection.read_text())
    complex_selection = next(item for item in selection["collections"] if item["collection"] == "finite-simplicial-complexes")
    f3_version = "2" if any("cohomology-f3-v2/" in item["path"] for item in complex_selection["inputs"]) else "1"
    commands = {
        "finite-posets": ["posets", "--data-root", root / "finite-posets", "--maximum-order", "10",
                          "--group-identifications", root / "finite-posets/enrichments/automorphism-abstract-groups/v1/records.jsonl"],
        "strong-steiner-categories": ["steiner-categories", "--data-root", root / "strong-steiner-categories",
                                     "--poset-data-root", root / "finite-posets", "--maximum-atomic-size", "11",
                                     "--group-identifications", root / "steiner-groups"],
        "finite-simplicial-complexes": ["simplicial-complexes", "--data-root", root, "--maximum-vertices", "6", "--f3-cohomology-version", f3_version],
        "finite-simplicial-sets": ["simplicial-sets", "--data-root", root, "--maximum-total-nondegenerate", "7",
                                   "--maximum-dimension", "3", "--maximum-dimension-at-top-total", "2"],
        "finite-group-cohomology": ["finite-group-cohomology", "--data-root", root, "--maximum-order", "32",
                                    "--initial-proof-degree", "8", "--maximum-proof-degree", "12",
                                    "--integral-additive-through-degree", "16", "--integral-bar-entry-limit", "100000"],
        "eilenberg-maclane": ["eilenberg-mac-lane", "--data-root", root, "--maximum-finite-order", "32",
                             "--maximum-delooping-degree", "4", "--exact-through-degree", "16"],
    }
    for collection in selected:
        print(f"Building {collection}", flush=True)
        subprocess.run([str(builder), *(str(argument) for argument in commands[collection]),
                        "--output-root", str(output / collection)], cwd=REPOSITORY, check=True)


if __name__ == "__main__":
    main()
