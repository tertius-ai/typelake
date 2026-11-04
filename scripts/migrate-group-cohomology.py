#!/usr/bin/env python3
"""Recompute modular records and replay the Rust validator in per-group jobs."""

from __future__ import annotations

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
import os
from pathlib import Path
import subprocess

REVISION = "4c92c269c59da02a4d32c833c2ee093407f8e22f"
RELATIVE = Path("finite-group-cohomology/modular-v7/order-through-32/proof-degree-8-through-12")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def changed_paths(old, new, path="") -> list[str]:
    if old == new:
        return []
    if isinstance(old, dict) and isinstance(new, dict):
        paths = []
        for key in sorted(old.keys() | new.keys()):
            child = f"{path}/{key}"
            paths.extend(changed_paths(old[key], new[key], child) if key in old and key in new else [child])
        return paths
    if isinstance(old, list) and isinstance(new, list) and len(old) == len(new):
        return [child for index, (a, b) in enumerate(zip(old, new)) for child in changed_paths(a, b, f"{path}/{index}")]
    return [path]


def combine_reports(reports: list[dict]) -> dict:
    """Combine complete, disjoint source-group validation reports."""
    labels = [report["group_label"] for report in reports]
    if len(set(labels)) != len(labels):
        raise ValueError("duplicate validation group")
    if not reports:
        raise ValueError("no validation reports")
    fields = set(reports[0]["validation"])
    for report in reports:
        if report["noether_version"] != "0.3.2" or report["noether_revision"] != REVISION:
            raise ValueError("validator engine mismatch")
        if set(report["validation"]) != fields:
            raise ValueError("inconsistent validator report fields")
        if report["validation"]["groups"] != 1:
            raise ValueError("validator scope is not exactly one group")
    combined = {}
    parameters = {"contract", "maximum_order", "initial_proof_degree", "maximum_proof_degree"}
    for key, value in reports[0]["validation"].items():
        values = [report["validation"][key] for report in reports]
        if key in parameters:
            if any(other != value for other in values):
                raise ValueError(f"inconsistent validator parameter: {key}")
            combined[key] = value
        elif isinstance(value, bool):
            if not all(other is True for other in values):
                raise ValueError(f"failed validation gate: {key}")
            combined[key] = True
        elif key == "proof_depths":
            counts = Counter()
            for depths in values:
                counts.update(depths)
            combined[key] = dict(sorted(counts.items(), key=lambda item: int(item[0])))
        elif isinstance(value, int):
            combined[key] = sum(values)
        else:
            raise ValueError(f"unknown validator field: {key}")
    return combined


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["data-root", "output-root", "records-root", "generator", "validator", "report"]:
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=6)
    parser.add_argument("--threads", type=int, default=4)
    options = parser.parse_args()
    if options.jobs < 1 or options.threads < 1:
        parser.error("jobs and threads must be positive")
    options.generator = options.generator.resolve()
    options.validator = options.validator.resolve()
    destination = options.output_root / RELATIVE
    if destination.exists():
        raise ValueError("immutable successor already exists")
    source = options.data_root / RELATIVE
    manifest = json.loads((source / "manifest.json").read_text())
    artifact = manifest["artifacts"][0]
    if sha256(source / "records.jsonl") != artifact["sha256"]:
        raise ValueError("historical artifact hash mismatch")
    old = [json.loads(line) for line in (source / "records.jsonl").read_text().splitlines()]
    if len(old) != artifact["row_count"]:
        raise ValueError("historical row count mismatch")
    master = Path("collections/finite-group-cohomology/sources/groups-through-32.json")
    if sha256(master) != manifest["source_sha256"]:
        raise ValueError("pinned source-group hash mismatch")
    groups = json.loads(master.read_text())["groups"]
    labels = [group["lmfdb_label"] for group in groups if group["small_groups_order"] <= manifest["maximum_order"]]
    options.records_root.mkdir(parents=True, exist_ok=True)
    producer = dict(noether_version="0.3.2", noether_revision=REVISION,
                    executable_sha256=sha256(options.generator), source_manifest_sha256=sha256(source / "manifest.json"))
    producer_path = options.records_root / "producer.json"
    if producer_path.exists() and json.loads(producer_path.read_text()) != producer:
        raise ValueError("checkpoint producer mismatch")
    producer_path.write_text(json.dumps(producer, indent=2) + "\n")
    environment = {**os.environ, "NOETHER_NUM_THREADS": str(options.threads)}

    def replay(record):
        prime = record["coefficient_context"]["characteristic"]
        path = options.records_root / f"{record['group_label']}-F{prime}.json"
        receipt = path.with_suffix(".sha256")
        if not (path.is_file() and receipt.is_file() and sha256(path) == receipt.read_text().strip()):
            computation = record["computation"]
            degree = computation["certificate"]["hilbert_function_through_degree"] if computation["kind"] == "finite_abelian_classifying_space" else computation["resolution"]["computed_through_degree"]
            result = subprocess.run([str(options.generator), "modular-record", "--group-label", record["group_label"], "--prime", prime, "--exact-through-degree", str(degree)], env=environment, capture_output=True, check=True)
            temporary = path.with_suffix(".pending")
            temporary.write_bytes(result.stdout)
            temporary.replace(path)
            receipt.write_text(sha256(path) + "\n")
        new = json.loads(path.read_text())
        for key in ["group_label", "group_canonical_sha256", "coefficient_context"]:
            if new[key] != record[key]:
                raise ValueError(f"object identity changed: {record['group_label']}/{prime}")
        return new

    with ThreadPoolExecutor(max_workers=options.jobs) as pool:
        futures = [pool.submit(replay, record) for record in old]
        for index, future in enumerate(as_completed(futures), 1):
            future.result()
            print(f"recomputed {index}/{len(old)} coefficient rows", flush=True)
    validation_root = options.records_root / "validation"
    validation_root.mkdir(exist_ok=True)
    validator_sha = sha256(options.validator)

    def validate(label):
        inputs = {path.name: sha256(path) for path in sorted(options.records_root.glob(f"{label}-F*.json"))}
        receipt = dict(inputs=inputs, validator_sha256=validator_sha)
        path = validation_root / f"{label}.json"
        receipt_path = validation_root / f"{label}.receipt.json"
        if not (path.is_file() and receipt_path.is_file() and json.loads(receipt_path.read_text()) == {**receipt, "report_sha256": sha256(path)}):
            result = subprocess.run([str(options.validator), "validate-modular-group", "--group-label", label, "--records-root", str(options.records_root), "--initial-proof-degree", str(manifest["initial_proof_degree"]), "--maximum-proof-degree", str(manifest["maximum_proof_degree"])], env=environment, capture_output=True, check=True)
            path.write_bytes(result.stdout)
            receipt_path.write_text(json.dumps({**receipt, "report_sha256": sha256(path)}, indent=2) + "\n")
        report = json.loads(path.read_text())
        if report["group_label"] != label:
            raise ValueError("validation group mismatch")
        return report

    reports = []
    with ThreadPoolExecutor(max_workers=options.jobs) as pool:
        futures = [pool.submit(validate, label) for label in labels]
        for index, future in enumerate(as_completed(futures), 1):
            report = future.result()
            reports.append(report)
            print(f"validated {index}/{len(labels)} groups: {report['group_label']}", flush=True)
    validation = combine_reports(reports)
    if validation["groups"] != manifest["groups"] or validation["coefficient_rows"] != len(old) or validation["complete_presentations"] != len(old):
        raise ValueError("global census coverage changed")
    new = [replay(record) for record in old]
    changes = [dict(label=a["group_label"], coefficient=a["coefficient_context"]["characteristic"], paths=changed_paths(a, b)) for a, b in zip(old, new)]
    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = destination.with_name("." + destination.name + ".migration")
    staging.mkdir()
    record_path = staging / "records.jsonl"
    record_path.write_text("".join(json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n" for record in new))
    manifest.update(noether_revision=REVISION, validation=validation)
    for key in ["proof_depths", "finite_abelian_presentations", "symonds_presentations", "symonds_parameter_presentations", "symonds_universal_presentations", "complete_presentations"]:
        manifest[key] = validation[key]
    manifest["artifacts"] = [dict(path="records.jsonl", row_count=len(new), byte_count=record_path.stat().st_size, sha256=sha256(record_path))]
    (staging / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    staging.rename(destination)
    options.report.write_text(json.dumps(dict(status="passed", collection="finite-group-modular", **producer,
        validator_sha256=validator_sha, rows=len(new), validated_groups=len(reports),
        changed_records=sum(bool(change["paths"]) for change in changes), object_identities_unchanged=True,
        historical_inputs_preserved=True, validation=validation, changes=changes), indent=2) + "\n")


if __name__ == "__main__":
    main()
