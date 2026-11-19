"""Validating an instance against its schema.

Only one job remains here. A fact envelope, its provenance, its rigor and its
assumptions belong to whatever store records observations — a consumer arrives
with a record model of their own, and SciType's claim that adopting it does not
oblige you to adopt our storage has to be true rather than aspirational.

What is left is what we need in order to publish correct examples: does this
instance supply its defining fields, are the values shaped as declared, and does
it satisfy the encoding constraints that are preconditions of its identifier?
"""

from __future__ import annotations

from typing import Any

from . import checks as _checks
from . import types as _types
from .canonical import CanonicalError, presentation_id
from .registry import Schema, load

__all__ = ["Problem", "validate_instance", "identify"]


class Problem(str):
    """A validation failure, rendered as its message."""


def validate_instance(instance: Any, schema: Schema | str, *,
                      schemas: dict[str, Schema] | None = None) -> list[Problem]:
    """Is this a well-formed instance of the schema?"""
    schemas = load() if schemas is None else schemas
    if isinstance(schema, str):
        found = schemas.get(schema)
        if found is None:
            return [Problem(f"unknown schema {schema}")]
        schema = found

    if not isinstance(instance, dict):
        return [Problem(f"expected an object, got {type(instance).__name__}")]

    out: list[Problem] = []
    fields = schema.get("fields") or {}
    for name in schema.defining:
        if name not in instance:
            out.append(Problem(f"missing defining field {name!r}"))
    if out:
        return out

    for name, value in instance.items():
        spec = fields.get(name)
        if not isinstance(spec, dict):
            out.append(Problem(f"{schema.mnemonic} declares no field {name!r}"))
            continue
        out += _value(value, schema, name, schemas)
    if not out:
        for constraint in schema.get("constraints") or []:
            if isinstance(constraint, dict) and not (constraint.get("args") or {}).get("key"):
                out += [Problem(str(problem)) for problem in _checks.run(
                    constraint["check"], instance, **(constraint.get("args") or {}))]
    return out


def identify(instance: dict, schema: Schema, *,
             schemas: dict[str, Schema] | None = None) -> str:
    """Validate, then compute the presentation identifier."""
    problems = validate_instance(instance, schema, schemas=schemas)
    if problems:
        raise CanonicalError("; ".join(problems))
    return presentation_id(schema.id, {k: instance[k] for k in schema.defining})


def _value(value: Any, schema: Schema, field: str,
           schemas: dict[str, Schema]) -> list[Problem]:
    """Structure from the declared type, then the encoding constraints.

    Encoding constraints bite only on defining fields: elsewhere a canonical form
    is specified and left to the producer, since no identifier depends on it.
    """
    spec = (schema.get("fields") or {}).get(field)
    if not isinstance(spec, dict):
        return []

    out: list[Problem] = []
    declared = spec.get("type")
    if declared:
        try:
            out += [Problem(p) for p in _types.check(value, str(declared), path=field)]
        except _types.TypeError_ as exc:
            return [Problem(f"{schema.mnemonic}.{field}: bad type expression: {exc}")]
        node = _types.parse(str(declared))
        if not out:
            out += _components(value, node, schemas, field, enforce=field in schema.defining)
    if out or field not in schema.defining:
        return out

    named = spec.get("constraint")
    if named:
        out += [Problem(f"{field}: {p}") for p in _checks.run(str(named), value)]
    for c in (schema.get("constraints") or []):
        if isinstance(c, dict) and (c.get("args") or {}).get("key") == field:
            out += [Problem(str(p))
                    for p in _checks.run(c["check"], value, **(c.get("args") or {}))]
    return out


def _components(value, node, schemas, path, *, enforce):
    if node.kind == "ref":
        return _component(value, node.payload, schemas, path, enforce=enforce)
    if node.kind == "list":
        return [p for i, item in enumerate(value) for p in
                _components(item, node.payload, schemas, f"{path}[{i}]", enforce=enforce)]
    if node.kind == "tuple":
        return [p for i, (item, subtype) in enumerate(zip(value, node.payload)) for p in
                _components(item, subtype, schemas, f"{path}[{i}]", enforce=enforce)]
    if node.kind in {"map", "object"}:
        return [p for key, item in value.items() for p in
                _components(item, node.payload[1] if node.kind == "map" else node.payload[key],
                            schemas, f"{path}.{key}", enforce=enforce)]
    return []


def _component(value: dict, ref: str, schemas: dict[str, Schema], path: str,
               *, enforce: bool) -> list[Problem]:
    inner = schemas.get(ref)
    if inner is None:
        return [Problem(f"{path}: unknown component {ref}")]
    if enforce:
        return [Problem(f"{path}: {problem}") for problem in
                validate_instance(value, inner, schemas=schemas)]
    out: list[Problem] = []
    for name, spec in (inner.get("fields") or {}).items():
        if not isinstance(spec, dict) or spec.get("policy") != "primary":
            continue
        if name not in value:
            out.append(Problem(f"{path}.{name}: required by {inner.mnemonic}"))
        elif spec.get("type"):
            out += [Problem(p) for p in
                    _types.check(value[name], str(spec["type"]), path=f"{path}.{name}")]
            if not out:
                out += _components(value[name], _types.parse(str(spec["type"])), schemas,
                                   f"{path}.{name}", enforce=False)
    if out or not enforce:
        return out
    for c in (inner.get("constraints") or []):
        if isinstance(c, dict) and c.get("check"):
            out += [Problem(f"{path}: {p}")
                    for p in _checks.run(c["check"], value, **(c.get("args") or {}))]
    return out
