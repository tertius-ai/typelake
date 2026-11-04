"""Loading the schema registry."""

from __future__ import annotations

import pathlib
import re
from typing import Any

import yaml

__all__ = ["Schema", "load", "SCHEMA_DIR", "ROOT", "conventions",
           "component_refs"]

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCHEMA_DIR = ROOT / "schemas"
REF = re.compile(r"scitype:\d{4}\.\d{4}")


class Schema(dict):
    """One schema document, with the accessors worth having.

    The source path is an attribute rather than a key, so the document stays
    exactly what was on disk — ``refs()`` serializes it and a stray Path would
    not survive that.
    """

    path: pathlib.Path

    @property
    def id(self) -> str:
        return self["id"]

    @property
    def mnemonic(self) -> str:
        return self["mnemonic"]

    @property
    def defining(self) -> list[str]:
        return self.get("identity", {}).get("defining", [])

    @property
    def canonicalization_kind(self) -> str | None:
        return self.get("identity", {}).get("canonicalization", {}).get("kind")

    def policy(self, field: str) -> str | None:
        spec = self.get("fields", {}).get(field)
        return spec.get("policy") if isinstance(spec, dict) else None

    def refs(self) -> set[str]:
        return set(REF.findall(yaml.safe_dump(dict(self))))


def conventions(directory: pathlib.Path | str | None = None) -> dict[str, dict]:
    """The convention vocabulary: which value a field name actually denotes."""
    path = pathlib.Path(directory or SCHEMA_DIR) / "conventions.yaml"
    if not path.exists():
        return {}
    return (yaml.safe_load(path.read_text()) or {}).get("conventions") or {}


def component_refs(schema: "Schema", fields: list[str] | None = None) -> set[str]:
    """Schemas embedded by the named fields (defining fields by default)."""
    from .types import TypeError_, parse
    out: set[str] = set()
    for name in (schema.defining if fields is None else fields):
        spec = (schema.get("fields") or {}).get(name)
        if not isinstance(spec, dict) or not spec.get("type"):
            continue
        try:
            stack = [parse(str(spec["type"]))]
        except TypeError_:
            continue
        while stack:
            n = stack.pop()
            if n.kind == "ref":
                out.add(n.payload)
            elif n.kind in {"list"}:
                stack.append(n.payload)
            elif n.kind in {"tuple", "map"}:
                stack.extend(n.payload)
            elif n.kind == "object":
                stack.extend(n.payload.values())
    return out


class DuplicateLineage(ValueError):
    """Two schemas claim one lineage number.

    Keying by id would let the second silently replace the first, which is
    precisely what happens when two contributors are assigned the same number —
    and the loser leaves no trace to notice.
    """


def load(directory: pathlib.Path | str | None = None) -> dict[str, Schema]:
    """Load every schema in a directory, keyed by lineage id."""
    directory = pathlib.Path(directory or SCHEMA_DIR)
    out: dict[str, Schema] = {}
    for path in sorted(directory.glob("*.yaml")):
        doc = yaml.safe_load(path.read_text())
        if not isinstance(doc, dict) or "id" not in doc:
            continue  # not a schema (e.g. the assumption or convention registry)
        schema = Schema(doc)
        schema.path = path
        if schema.id in out:
            raise DuplicateLineage(
                f"{schema.id} claimed by both {out[schema.id].path.name} "
                f"and {path.name}")
        out[schema.id] = schema
    return out
