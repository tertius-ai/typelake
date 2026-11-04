#!/usr/bin/env python3
"""Pin and verify the existing Volume I input selection without rewriting data."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath

NOETHER_REVISION = "4c92c269c59da02a4d32c833c2ee093407f8e22f"
FORMAT = "typelake/volume-selection/v1"


def selected_collections() -> list[dict]:
    """The finite bounds and explicit enrichment choices of Volume I."""
    posets = [f"finite-posets/census/record-version=1/order={n}/run.json" for n in range(11)]
    posets.append("finite-posets/enrichments/automorphism-abstract-groups/v1/manifest.json")
    steiner = [f"strong-steiner-categories/census/record-version=1/atomic-size={n}/run.json" for n in range(12)]
    for lane in ["census/identity-version=1", "enrichments/light-version=1", "enrichments/mapping-version=1"]:
        steiner.append(f"strong-steiner-categories/{lane}/atomic-size=12/run.json")
    steiner.append("steiner-groups/manifest.json")
    complexes = []
    for n in range(7):
        for lane in ["", "enrichments/topology-v1/", "enrichments/cohomology-f2-v2/", "enrichments/cohomology-f3-v1/"]:
            complexes.append(f"finite-simplicial-complexes/v1/{lane}vertices-{n}/manifest.json")
    sets = []
    for n, dimension in [(n, 3) for n in range(7)] + [(7, 2)]:
        for lane in ["", "enrichments/topology-v2/", "enrichments/cohomology-f2-v1/", "enrichments/cohomology-f3-v1/", "enrichments/steenrod-operations-f3-v1/", "enrichments/homotopy-v1/"]:
            sets.append(f"finite-simplicial-sets/v1/{lane}dimension-{dimension}/total-{n}/manifest.json")
    return [
        dict(collection="finite-posets", object_count=2_769_965,
             bounds=dict(maximum_order=10),
             coverage="Complete isomorphism classes, integral homology, Stong cores, and automorphism actions; abstract group identification has explicit unavailable cases.",
             manifests=posets),
        dict(collection="strong-steiner-categories", object_count=28_010,
             bounds=dict(identity_light_mapping_maximum_atomic_size=12, rich_maximum_atomic_size=11, browser_rich_maximum_atomic_size=11),
             coverage="Identity, light, and mapping data through 12; 7,566 rich records through 11. Categorical homology uses the stored bounded presentation and particle slices.",
             manifests=steiner, extra_files=["strong-steiner-categories/gray-products.jsonl"]),
        dict(collection="finite-simplicial-complexes", object_count=16_352,
             bounds=dict(maximum_vertices=6),
             coverage="Complete identity shells, topology, F2 cohomology-v2 and F3 cohomology-v1.", manifests=complexes),
        dict(collection="finite-simplicial-sets", object_count=125_843,
             bounds=dict(boxes=[dict(maximum_nondegenerate_simplices=6, maximum_dimension=3), dict(nondegenerate_simplices=7, maximum_dimension=2)]),
             coverage="Identity, topology-v2, F2/F3 cohomology, F3 Steenrod operations, and witnessed horn collapses across the selected frontier.", manifests=sets),
        dict(collection="finite-group-cohomology", object_count=144,
             bounds=dict(maximum_order=32, initial_modular_proof_degree=8, maximum_modular_proof_degree=12, integral_additive_degree=16, integral_bar_entry_limit=100_000),
             coverage="modular-v7: 198 complete coefficient presentations. integral-v4: 46 complete families, eight bounded sparse-bar truncations, and 90 degree-two truncations.",
             manifests=["finite-group-cohomology/modular-v7/order-through-32/proof-degree-8-through-12/manifest.json", "finite-group-cohomology/integral-v4/order-through-32/additive-degree-16/bar-entry-limit-100000/manifest.json"]),
        dict(collection="eilenberg-maclane", object_count=224,
             bounds=dict(maximum_finite_order=32, includes_infinite_cyclic=True, maximum_delooping_degree=4, exact_through_degree=16),
             coverage="invariants-v8: 304 modular rows available; integral additive data available for 60 spaces and 51 complete integral rings. Remaining integral cells are explicitly pending.",
             manifests=["eilenberg-maclane/invariants-v8/order-through-32-plus-Z/delooping-through-4/degree-16/manifest.json"]),
    ]


def relative_path(value: str) -> PurePosixPath:
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ValueError(f"unsafe artifact path: {value}")
    return path


def inspect_file(root: Path, name: str) -> dict:
    path = root / relative_path(name)
    digest = hashlib.sha256()
    size = lines = 0
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
            size += len(block)
            lines += block.count(b"\n")
    result = dict(path=name, byte_count=size, sha256=digest.hexdigest())
    if path.suffix == ".jsonl":
        result["row_count"] = lines
    return result


def check_artifact(root: Path, name: str, expected: dict) -> dict:
    observed = inspect_file(root, name)
    for field in ["byte_count", "sha256", "row_count"]:
        if field in expected and field in observed and observed[field] != expected[field]:
            raise ValueError(f"{name}: {field} differs from its manifest")
    return observed


def freeze(root: Path) -> dict:
    collections = selected_collections()
    for collection in collections:
        inputs = {}
        for name in collection.pop("manifests"):
            path = root / relative_path(name)
            document = json.loads(path.read_text())
            if document.get("status", "passed") != "passed":
                raise ValueError(f"unvalidated source manifest: {name}")
            inputs[name] = inspect_file(root, name)
            artifacts = document.get("artifacts", document.get("outputs", []))
            if "output" in document:
                artifacts = [document["output"]]
            if "output_path" in document:
                artifacts = [dict(path=document["output_path"], byte_count=document["output_byte_count"], sha256=document["output_sha256"])]
            if not artifacts:
                raise ValueError(f"manifest has no artifacts: {name}")
            for artifact in artifacts:
                child = str(relative_path(name).parent / relative_path(artifact["path"]))
                observed = check_artifact(root, child, artifact)
                if child in inputs and inputs[child] != observed:
                    raise ValueError(f"inconsistent duplicate input: {child}")
                inputs[child] = observed
        for name in collection.pop("extra_files", []):
            inputs[name] = inspect_file(root, name)
        collection["inputs"] = [inputs[key] for key in sorted(inputs)]
    return dict(format=FORMAT, volume=1, status="selected-for-review", software_target="0.1.0",
                noether=dict(version="0.3.2", revision=NOETHER_REVISION),
                provenance="Input manifests retain their original producer revisions. Current-engine replay evidence is separate from this selection.",
                collections=collections)


def verify(root: Path, selection: dict) -> dict:
    if selection["format"] != FORMAT or selection["volume"] != 1:
        raise ValueError("unsupported volume selection")
    files = size = 0
    for collection in selection["collections"]:
        for expected in collection["inputs"]:
            observed = check_artifact(root, expected["path"], expected)
            files += 1
            size += observed["byte_count"]
    return dict(status="passed", collections=len(selection["collections"]), files=files, byte_count=size,
                checks=["sha256", "byte_count", "uncompressed_jsonl_row_count"], mathematical_replay="separate")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["freeze", "verify"])
    parser.add_argument("--data-root", type=Path, required=True)
    parser.add_argument("--selection", type=Path, default=Path("collections/volume-i.json"))
    options = parser.parse_args()
    if options.operation == "freeze":
        selection = freeze(options.data_root)
        with options.selection.open("x") as stream:
            json.dump(selection, stream, indent=2)
            stream.write("\n")
        print(f"Pinned {sum(len(c['inputs']) for c in selection['collections'])} inputs in {options.selection}")
    else:
        print(json.dumps(verify(options.data_root, json.loads(options.selection.read_text())), indent=2))


if __name__ == "__main__":
    main()
