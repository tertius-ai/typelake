"""Command line: ``uvx scitype …``, no environment to manage.

Deliberately small. Authoring, linting and validating are not hot paths — the
expensive parts of the pipeline are canonicalization and the mathematics, which
live elsewhere.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from .canonical import CanonicalError, canonical_preimage, presentation_id
from .checks import SEMANTIC, TERMS, definition_of
from .lint import lint
from .registry import load
from .validate import validate_instance


def _resolve(schemas, name: str):
    if name in schemas:
        return schemas[name]
    for s in schemas.values():
        if s.mnemonic.lower() == name.lower():
            return s
    sys.exit(f"no schema {name!r}; try `scitype schemas`")


def _read(path: str):
    return sys.stdin.read() if path == "-" else Path(path).read_text()


def _corpus_coverage(schemas) -> set:
    """Which schemas have an authored example, directly or as an embedded component.

    Not "appears in the conformance corpus": conformance is about implementations
    agreeing, an example is evidence a schema expresses real mathematics, and only
    the second is a claim about an entry.
    """
    from .registry import ROOT, component_refs
    direct = set()
    for f in sorted((ROOT / "conformance").glob("*.jsonl")):
        for line in f.read_text().splitlines():
            if line.strip() and (lid := json.loads(line).get("lineage")):
                direct.add(lid)
    return direct | {r for sid in direct if sid in schemas
                     for r in component_refs(schemas[sid])}


def cmd_schemas(args) -> int:
    """The registry, with the evidence behind each entry's declared status."""
    schemas = load()
    covered = _corpus_coverage(schemas)
    print(f"{'schema':26} {'status':12} {'canonicalization':16} {'ex':>3} {'op':>3}  categories")
    for sid, s in sorted(schemas.items()):
        fields = s.get("fields") or {}
        opaque = sum(1 for v in fields.values()
                     if isinstance(v, dict) and str(v.get("type")) == "opaque")
        print(f"{s.mnemonic:26} {str(s.get('status')):12} "
              f"{s.canonicalization_kind or '?':16} "
              f"{'yes' if sid in covered else '-':>3} {opaque or '':>3}  "
              f"{','.join(s.get('categories', []))}")
    print("\nex = has an authored example.  op = fields carrying unvalidated payload.")
    return 0


def cmd_id(args) -> int:
    schemas = load()
    s = _resolve(schemas, args.schema)
    doc = json.loads(_read(args.file))
    defining = {k: doc[k] for k in s.defining if k in doc}
    missing = [k for k in s.defining if k not in doc]
    if missing:
        sys.exit(f"missing defining field(s): {missing}")
    try:
        print(presentation_id(s.id, defining))
    except CanonicalError as exc:
        sys.exit(str(exc))
    return 0


def cmd_check(args) -> int:
    """Is this a well-formed instance of the schema?

    Facts, provenance and rigor belong to whatever store records observations;
    what remains here is what publishing correct examples requires.
    """
    schemas = load()
    schema = _resolve(schemas, args.schema)
    problems = validate_instance(json.loads(_read(args.file)), schema, schemas=schemas)
    for p in problems:
        print(p, file=sys.stderr)
    print(f"{'FAIL' if problems else 'ok'}: {len(problems)} problem(s)", file=sys.stderr)
    return 1 if problems else 0


def cmd_lint(args) -> int:
    problems = lint()
    for p in problems:
        print(p, file=sys.stderr)
    print(f"{'FAIL' if problems else 'ok'}: {len(problems)} problem(s)", file=sys.stderr)
    return 1 if problems else 0


def cmd_terms(args) -> int:
    """The vocabularies, and who is responsible for each term."""
    from .checks import SEMANTIC, SUFFICIENCY, TERMS, definition_of
    from .registry import component_refs, conventions

    schemas = load()
    bearing = {r for s in schemas.values() for r in component_refs(s)}
    live: dict[str, set[str]] = {}
    for sid, s in schemas.items():
        for c in (s.get("constraints") or []):
            if isinstance(c, dict) and (
                    sid in bearing or (c.get("args") or {}).get("key") in s.defining):
                live.setdefault(c.get("check"), set()).add(s.mnemonic)
        for name, spec in (s.get("fields") or {}).items():
            if isinstance(spec, dict) and spec.get("constraint") and name in s.defining:
                live.setdefault(spec["constraint"], set()).add(s.mnemonic)

    print("CONSTRAINTS\n")
    for name, kind in sorted(TERMS().items()):
        if kind == "encoding":
            where = ", ".join(sorted(live.get(name, [])))
            print(f"  {name:24} enforced" + (f" via {where}" if where
                  else " — guards no defining field"))
        else:
            print(f"  {name:24} {kind}, checked by the reader")
            print(f"  {'':24} {definition_of(name)}")
    print("\nCONVENTIONS\n")
    for name, e in sorted(conventions().items()):
        print(f"  {name}\n    {e.get('title','')}")
        for case in e.get("discriminates") or []:
            print(f"    e.g. {case['case']}")
            print(f"         this: {' '.join(case['under_this'].split())}")
            print(f"         {case['alternative']}: "
                  f"{' '.join(case['under_alternative'].split())}")
    return 0


def cmd_vector(args) -> int:
    """Turn an instance into a conformance vector.

    A contributor supplies the mathematics; the identifier is computed here,
    because a hand-written expected id is a wrong vector waiting to be trusted.
    """
    schemas = load()
    s = _resolve(schemas, args.schema)
    doc = json.loads(_read(args.file))
    missing = [k for k in s.defining if k not in doc]
    if missing:
        sys.exit(f"missing defining field(s): {missing}")
    defining = {k: doc[k] for k in s.defining}
    try:
        pid = presentation_id(s.id, defining)
    except CanonicalError as exc:
        sys.exit(f"not encodable: {exc}")
    print(json.dumps({"name": args.name or f"{s.mnemonic.lower()}-instance",
                      "lineage": s.id, "defining": defining,
                      "preimage": canonical_preimage(s.id, defining).decode(),
                      "presentation_id": pid}, ensure_ascii=False))
    return 0


def cmd_emit(args) -> int:
    """Regenerate dist/. Generated, never hand-edited."""
    from .emit import emit
    written = emit()
    print(f"wrote {len(written)} files to {written[0].parent}")
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="scitype", description=__doc__.splitlines()[0])
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("schemas", help="list the registry").set_defaults(fn=cmd_schemas)
    sub.add_parser("lint", help="check the registry").set_defaults(fn=cmd_lint)
    sub.add_parser("terms", help="the constraint and convention vocabularies"
                   ).set_defaults(fn=cmd_terms)
    sub.add_parser("emit", help="regenerate dist/ (JSON Schema + registry index)"
                   ).set_defaults(fn=cmd_emit)

    q = sub.add_parser("id", help="presentation id of a record's defining data")
    q.add_argument("schema")
    q.add_argument("file", nargs="?", default="-")
    q.set_defaults(fn=cmd_id)

    q = sub.add_parser("check", help="validate an instance against its schema")
    q.add_argument("schema")
    q.add_argument("file", nargs="?", default="-")
    q.set_defaults(fn=cmd_check)

    q = sub.add_parser("vector", help="turn an instance into a conformance vector")
    q.add_argument("schema")
    q.add_argument("file", nargs="?", default="-")
    q.add_argument("--name", default=None)
    q.set_defaults(fn=cmd_vector)

    args = p.parse_args(argv)
    return args.fn(args)


if __name__ == "__main__":
    raise SystemExit(main())
