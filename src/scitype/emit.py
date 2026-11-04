"""Emit language-neutral artifacts.

A consumer in Rust, Julia or JavaScript should not have to parse our YAML and
implement our type-expression grammar to read the registry. These files let them
use standard tooling and none of our code — which is the practical test of
whether another institution can adopt this.

Generated, not authored: CI regenerates and fails if the committed copy differs.
"""

from __future__ import annotations

import json
import pathlib

from .canonical import canonical_preimage, presentation_id
from .registry import ROOT, SCHEMA_DIR, conventions, load
from .types import TypeError_, to_json_schema

DIST = ROOT / "published"
EXAMPLES = ROOT / "examples"
CORPUS = ROOT / "conformance"


def identity_vectors(examples=None, schemas=None) -> list[dict]:
    """Derive conformance vectors from authored examples.

    An example is evidence that a schema can express real mathematics; a vector
    is a statement of what an implementation must reproduce. One artifact, two
    jobs — writing the first should not oblige anyone to hand-compute the second.
    """
    schemas = schemas if schemas is not None else load()
    out = []
    for path in sorted(pathlib.Path(examples or EXAMPLES).glob("*.json")):
        doc = json.loads(path.read_text())
        schema = schemas[doc["schema"]]
        defining = {k: doc["instance"][k] for k in schema.defining}
        out.append({
            "name": doc["name"],
            "lineage": schema.id,
            "defining": defining,
            "preimage": canonical_preimage(schema.id, defining).decode(),
            "presentation_id": presentation_id(schema.id, defining),
        })
    return out


def _inliner(schemas, depth=0):
    """Resolve a component reference to its own structure, so the emitted file is
    self-contained: no $ref resolver needed, and no silent `{"type": "object"}`
    that would accept anything."""
    def resolve(ref: str):
        inner = schemas.get(ref)
        if inner is None or depth > 4:
            return None
        props, required = {}, []
        for name, spec in (inner.get("fields") or {}).items():
            if not isinstance(spec, dict) or spec.get("policy") != "primary":
                continue
            if spec.get("type"):
                try:
                    props[name] = to_json_schema(str(spec["type"]),
                                                 _inliner(schemas, depth + 1))
                except TypeError_:
                    props[name] = {}
            required.append(name)
        return {"type": "object", "properties": props, "required": sorted(required)}
    return resolve


def json_schema_for(schema, schemas=None) -> dict:
    """One JSON Schema per SciType, describing a record's field values."""
    schemas = schemas if schemas is not None else load()
    resolve = _inliner(schemas)
    props, required = {}, []
    for name, spec in (schema.get("fields") or {}).items():
        if not isinstance(spec, dict):
            continue
        entry: dict = {}
        if spec.get("type"):
            try:
                entry = to_json_schema(str(spec["type"]), resolve)
            except TypeError_:
                entry = {}
        bits = [b for b in [
            (spec.get("note") or "").strip().rstrip("."),
            f"policy: {spec.get('policy')}" if spec.get("policy") else "",
            f"convention: {', '.join(spec['convention'])}" if spec.get("convention") else "",
        ] if b]
        entry["description"] = ". ".join(bits) + "." if bits else None
        props[name] = {k: v for k, v in entry.items() if v is not None}
        if name in schema.defining:
            required.append(name)
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": f"urn:{schema.id.replace(':', ':')}",
        "title": schema.get("title", schema.mnemonic),
        "description": " ".join((schema.get("note") or "").split()) or None,
        "type": "object",
        "properties": props,
        "required": sorted(required),
    }


def index() -> dict:
    """The registry as data: what exists, how it is identified, how good it is."""
    schemas = load()
    return {
        "authority": "scitype",
        "generated_from": "schemas/",
        "note": "Identity is the lineage id plus the defining fields, JCS-encoded "
                "(RFC 8785), hashed with sha256. See conformance/.",
        "conventions": "published/conventions.json",
        "schemas": [
            {
                "id": s.id,
                "version": s.get("version"),
                "mnemonic": s.mnemonic,
                "title": s.get("title"),
                "categories": s.get("categories"),
                "status": s.get("status"),
                "defining": s.defining,
                "canonicalization": (s.get("identity") or {}).get("canonicalization"),
                "sufficiency": (s.get("identity") or {}).get("sufficiency"),
                "json_schema": f"published/{s.id.split(':')[1]}-{s.mnemonic}.schema.json",
                "source": f"schemas/{s.path.name}",
            }
            for _, s in sorted(schemas.items())
        ],
    }


def emit(directory: pathlib.Path | str | None = None) -> list[pathlib.Path]:
    out = pathlib.Path(directory or DIST)
    out.mkdir(exist_ok=True)
    written = []
    schemas = load()
    for _, s in sorted(schemas.items()):
        path = out / f"{s.id.split(':')[1]}-{s.mnemonic}.schema.json"
        doc = {k: v for k, v in json_schema_for(s, schemas).items() if v is not None}
        path.write_text(json.dumps(doc, indent=2) + "\n")
        written.append(path)
    vectors = (out if out != DIST else CORPUS) / "identifiers.jsonl"
    vectors.write_text("".join(
        json.dumps(v, ensure_ascii=False) + "\n" for v in identity_vectors()))
    written.append(vectors)

    conv = out / "conventions.json"
    conv.write_text(json.dumps(
        {name: {"title": e.get("title"),
                "definition": " ".join((e.get("definition") or "").split()),
                "references": e.get("references") or [],
                "discriminates": e.get("discriminates") or []}
         for name, e in sorted(conventions().items())}, indent=2) + "\n")
    written.append(conv)

    idx = out / "index.json"
    idx.write_text(json.dumps(index(), indent=2) + "\n")
    written.append(idx)
    return written
