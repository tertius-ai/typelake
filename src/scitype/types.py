"""The type expression language, and structural validation from it.

Type expressions are what make structural checking generic: no schema ships code
to say that ``arcs`` is a list of integer pairs. The grammar is deliberately tiny,
because every implementation has to agree on it.

    int | bool | string | decimal | opaque   primitives
    [T]                                      list of T
    [T, U, ...]                              fixed-length tuple
    {K: V}                                   map, K a primitive
    {a: T, b: U}                             object with named fields
    scitype:YYMM.NNNN                        a component, checked against its schema

``decimal`` is an arbitrary-precision integer carried as a string; ``opaque``
means deliberately unchecked here, not "anything goes" — such values are checked
by a named invariant (see :mod:`scitype.checks`) or not at all, and saying so is
better than implying a check that does not happen.
"""

from __future__ import annotations

import re
from typing import Any

__all__ = ["parse", "check", "to_json_schema", "TypeError_", "Node"]

PRIMITIVES = {"int", "bool", "string", "decimal", "opaque"}
REF = re.compile(r"^scitype:\d{4}\.\d{4}$")
DECIMAL = re.compile(r"^-?(0|[1-9][0-9]*)$")


class TypeError_(ValueError):
    """The type expression is malformed."""


class Node(tuple):
    """(kind, payload). Kinds: prim, ref, list, tuple, map, object."""

    @property
    def kind(self) -> str:
        return self[0]

    @property
    def payload(self) -> Any:
        return self[1]


def _split(body: str, sep: str = ",") -> list[str]:
    """Split on a separator at depth zero."""
    out, depth, cur = [], 0, ""
    for ch in body:
        if ch in "[{":
            depth += 1
        elif ch in "]}":
            depth -= 1
        if ch == sep and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur)
    return [p.strip() for p in out if p.strip()]


def parse(expr: str) -> Node:
    expr = expr.strip()
    if not expr:
        raise TypeError_("empty type expression")
    if expr in PRIMITIVES:
        return Node(("prim", expr))
    if REF.match(expr):
        return Node(("ref", expr))
    if expr.startswith("[") and expr.endswith("]"):
        parts = _split(expr[1:-1])
        if not parts:
            raise TypeError_(f"empty list type: {expr!r}")
        if len(parts) == 1:
            return Node(("list", parse(parts[0])))
        return Node(("tuple", [parse(p) for p in parts]))
    if expr.startswith("{") and expr.endswith("}"):
        parts = _split(expr[1:-1])
        if not parts:
            raise TypeError_(f"empty object type: {expr!r}")
        entries = []
        for part in parts:
            key, _, val = part.partition(":")
            if not val.strip():
                raise TypeError_(f"expected 'key: type' in {expr!r}, got {part!r}")
            entries.append((key.strip(), val.strip()))
        # a single entry whose key is a primitive is a map, not an object
        if len(entries) == 1 and entries[0][0] in PRIMITIVES:
            k, v = entries[0]
            return Node(("map", (parse(k), parse(v))))
        return Node(("object", {k: parse(v) for k, v in entries}))
    raise TypeError_(f"cannot parse type expression: {expr!r}")


def check(value: Any, expr: str | Node, *, path: str = "value") -> list[str]:
    """Structural check. Component references are not descended into here — the
    caller resolves them against the registry, since a component has its own
    fields, constraints, and type expressions."""
    node = expr if isinstance(expr, Node) else parse(expr)
    kind, payload = node.kind, node.payload

    if kind == "prim":
        if payload == "opaque":
            return []
        if payload == "int":
            ok = isinstance(value, int) and not isinstance(value, bool)
        elif payload == "bool":
            ok = isinstance(value, bool)
        elif payload == "string":
            ok = isinstance(value, str)
        else:  # decimal
            ok = isinstance(value, str) and bool(DECIMAL.match(value))
        return [] if ok else [f"{path}: expected {payload}, got {value!r}"]

    if kind == "ref":
        return [] if isinstance(value, dict) else [f"{path}: expected an object for {payload}"]

    if kind == "list":
        if not isinstance(value, list):
            return [f"{path}: expected a list, got {type(value).__name__}"]
        return [p for i, v in enumerate(value) for p in check(v, payload, path=f"{path}[{i}]")]

    if kind == "tuple":
        if not isinstance(value, list):
            return [f"{path}: expected a list of {len(payload)}, got {type(value).__name__}"]
        if len(value) != len(payload):
            return [f"{path}: expected {len(payload)} entries, got {len(value)}"]
        return [p for i, (v, t) in enumerate(zip(value, payload))
                for p in check(v, t, path=f"{path}[{i}]")]

    if kind == "map":
        if not isinstance(value, dict):
            return [f"{path}: expected a map, got {type(value).__name__}"]
        keytype, valtype = payload
        out = []
        for k, v in value.items():
            # JSON object keys are always strings; an int key travels as its decimal form
            if keytype.payload == "int" and not re.match(r"^-?\d+$", str(k)):
                out.append(f"{path}: key {k!r} is not an integer")
            out += check(v, valtype, path=f"{path}[{k!r}]")
        return out

    if kind == "object":
        if not isinstance(value, dict):
            return [f"{path}: expected an object, got {type(value).__name__}"]
        out = [f"{path}: missing key {k!r}" for k in payload if k not in value]
        out += [f"{path}: unexpected key {k!r}" for k in value if k not in payload]
        out += [p for k, t in payload.items() if k in value
                for p in check(value[k], t, path=f"{path}.{k}")]
        return out

    raise TypeError_(f"unknown node kind {kind}")


def to_json_schema(expr: str | Node, resolve=None) -> dict:
    """Emit JSON Schema, so implementations in other languages get structural
    validation from the same declaration rather than reimplementing this.

    `resolve` maps a component id to its schema and is used to INLINE it. Without
    inlining a component degrades to `{"type": "object"}`, which accepts anything
    — a published artifact that looks like validation and is not.
    """
    node = expr if isinstance(expr, Node) else parse(expr)
    kind, payload = node.kind, node.payload
    if kind == "prim":
        return {
            "int": {"type": "integer"},
            "bool": {"type": "boolean"},
            "string": {"type": "string"},
            "decimal": {"type": "string", "pattern": r"^-?(0|[1-9][0-9]*)$"},
            "opaque": {},
        }[payload]
    if kind == "ref":
        inlined = resolve(payload) if resolve else None
        if inlined is None:
            return {"type": "object", "$comment": f"validates against {payload}"}
        return {**inlined, "$comment": f"inlined from {payload}"}
    if kind == "list":
        return {"type": "array", "items": to_json_schema(payload, resolve)}
    if kind == "tuple":
        return {"type": "array",
                "prefixItems": [to_json_schema(t, resolve) for t in payload],
                "minItems": len(payload), "maxItems": len(payload)}
    if kind == "map":
        return {"type": "object",
                "additionalProperties": to_json_schema(payload[1], resolve)}
    return {"type": "object", "additionalProperties": False,
            "required": sorted(payload),
            "properties": {k: to_json_schema(t, resolve) for k, t in payload.items()}}
