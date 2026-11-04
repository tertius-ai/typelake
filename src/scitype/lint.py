"""Registry lint: the checks that keep the conventions from rotting.

Run with ``python -m scitype.lint``.
"""

from __future__ import annotations

import pathlib
import sys

from .checks import TERMS
from .registry import REF, DuplicateLineage, conventions, load
from .types import TypeError_, parse

POLICIES = {"primary", "derived-cheap", "derived-expensive"}
KINDS = {"complete", "invariant-based"}


META = pathlib.Path(__file__).resolve().parents[2] / "schemas" / "format.json"


def _meta_check(schema, where: str) -> list[str]:
    """Validate a schema document against the published meta-schema."""
    import json

    import jsonschema

    meta = json.loads(META.read_text())
    doc = {k: v for k, v in schema.items() if not k.startswith("_")}
    return [f"{where}: {'.'.join(str(x) for x in e.absolute_path) or '<root>'}: {e.message}"
            for e in jsonschema.Draft202012Validator(meta).iter_errors(doc)]


def lint(directory: pathlib.Path | str | None = None) -> list[str]:
    try:
        schemas = load(directory)
    except DuplicateLineage as exc:
        return [f"{exc}"]
    vocab = conventions(directory)
    errs: list[str] = []
    mnemonic = {sid: s.mnemonic for sid, s in schemas.items()}

    for sid, s in schemas.items():
        path = s.path
        where = path.name

        errs += _meta_check(s, where)

        want = f"{sid.split(':')[1]}-{s.mnemonic}.yaml"
        if where != want:
            errs.append(f"{where}: filename should be {want}")

        for ref in s.refs():
            if ref not in schemas:
                errs.append(f"{where}: dangling reference {ref}")

        if s.canonicalization_kind not in KINDS:
            errs.append(f"{where}: canonicalization kind must be one of {sorted(KINDS)}")

        for name, spec in (s.get("fields") or {}).items():
            if not isinstance(spec, dict):
                errs.append(f"{where}: field {name} is not a mapping")
                continue
            if spec.get("policy") not in POLICIES:
                errs.append(f"{where}: field {name} has policy {spec.get('policy')!r}")
            if spec.get("policy", "").startswith("derived") and not spec.get("inputs"):
                errs.append(f"{where}: derived field {name} declares no inputs")
            for dep in spec.get("inputs", []):
                if dep not in (s.get("fields") or {}):
                    errs.append(f"{where}: field {name} inputs unknown field {dep}")

        fields = s.get("fields") or {}
        for c in (s.get("constraints") or []):
            if not isinstance(c, dict) or "check" not in c:
                errs.append(f"{where}: constraint {c!r} is not a named check")
                continue
            if c["check"] not in TERMS():
                errs.append(f"{where}: constraint {c['check']!r} is not in the vocabulary")
            # a constraint naming a field that does not exist can never run, and
            # reads as a guarantee that silently is not one
            for arg, val in (c.get("args") or {}).items():
                if isinstance(val, str) and val not in fields and arg in {"key", "num", "den"}:
                    errs.append(f"{where}: constraint {c['check']!r} names "
                                f"{val!r}, which is not a field")

        for name, spec in (s.get("fields") or {}).items():
            if not isinstance(spec, dict):
                continue
            if spec.get("constraint") and spec["constraint"] not in TERMS():
                errs.append(f"{where}: field {name} names {spec['constraint']!r}, "
                            f"not in the constraint vocabulary")
            if spec.get("policy") == "derived-expensive" and "convention" not in spec:
                errs.append(f"{where}: field {name} is derived-expensive and does "
                            f"not declare `convention`; use [] to say you "
                            f"considered it and there is no ambiguity")
            for conv in spec.get("convention") or []:
                if conv not in vocab:
                    errs.append(f"{where}: field {name} names convention {conv!r}, "
                                f"which is not registered")
            if spec.get("type"):
                try:
                    parse(str(spec["type"]))
                except TypeError_ as exc:
                    errs.append(f"{where}: field {name} has unparseable type: {exc}")


        status = s.get("status")
        if status not in {"draft", "provisional", "stable"}:
            errs.append(f"{where}: status must be draft, provisional or stable, "
                        f"got {status!r}")

        for name in s.defining:
            policy = s.policy(name)
            if policy is None:
                errs.append(f"{where}: defining field {name} is not declared")
            elif policy != "primary":
                errs.append(f"{where}: defining field {name} is {policy}, must be primary")

        # mnemonic comments must name the schema they sit beside
        for i, line in enumerate(path.read_text().split("\n"), 1):
            if "#" not in line:
                continue
            code, comment = line.split("#", 1)
            for ref in REF.findall(code):
                if ref in mnemonic and mnemonic[ref] not in comment:
                    errs.append(f"{where}:{i}: comment should name {mnemonic[ref]}")

    return errs


def main() -> int:
    errs = lint()
    for e in errs:
        print(e, file=sys.stderr)
    print(f"{'FAIL' if errs else 'ok'}: {len(errs)} problem(s)", file=sys.stderr)
    return 1 if errs else 0


if __name__ == "__main__":
    raise SystemExit(main())
