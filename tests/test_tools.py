"""Do the tools catch what they claim to?

Negative cases, built on deliberately broken synthetic registries. A linter
nobody has watched fail is a linter nobody has tested — three real bugs got
through while these did not exist.
"""

import json
import pathlib

import pytest

from scitype.registry import load
from scitype.validate import validate_instance
from conftest import CORPUS, ROOT, cases, check as _check


def test_type_expressions_round_trip_to_json_schema():
    """Other languages get structural validation from the same declaration."""
    jsonschema = pytest.importorskip("jsonschema")
    from scitype.types import to_json_schema
    from scitype.registry import load
    for sid, s in load().items():
        for name, spec in (s.get("fields") or {}).items():
            if isinstance(spec, dict) and spec.get("type"):
                jsonschema.Draft202012Validator.check_schema(
                    to_json_schema(str(spec["type"])))

def test_duplicate_lineage_is_refused_not_silently_resolved():
    """Two contributors assigned one number must collide loudly; keying by id
    would let the second replace the first with no trace."""
    import shutil, tempfile
    from scitype.lint import lint
    from scitype.registry import DuplicateLineage, load
    d = tempfile.mkdtemp()
    src = pathlib.Path(__file__).resolve().parents[1] / "schemas"
    for f in src.glob("*.yaml"):
        shutil.copy(f, d)
    body = ("id: scitype:2608.9999\nversion: v1\nmnemonic: {m}\ntitle: {m}\n"
            "categories: [math.CO]\nstatus: draft\n"
            "identity: {{defining: [x], canonicalization: {{method: none, kind: complete}}}}\n"
            "fields: {{x: {{policy: primary, type: int}}}}\n")
    for m in ("Alpha", "Beta"):
        (pathlib.Path(d) / f"2608.9999-{m}.yaml").write_text(body.format(m=m))
    with pytest.raises(DuplicateLineage):
        load(d)
    assert any("claimed by both" in e for e in lint(d))

def test_meta_schema_rejects_unknown_and_misspelled_keys():
    """A misspelled `convension:` drops a convention silently — a correctness
    loss, not a typo. The schema format needs a schema for the same reason the
    objects do."""
    import shutil, tempfile
    from scitype.lint import lint
    d = tempfile.mkdtemp()
    src = pathlib.Path(__file__).resolve().parents[1] / "schemas"
    for f in src.glob("*.yaml"):
        shutil.copy(f, d)
    p = pathlib.Path(d) / "2608.0021-Partition.yaml"
    p.write_text(p.read_text()
                 .replace("    convention: [", "    convension: [")
                 .replace("status: draft", "status: drafty\nnotes: bogus"))
    errs = " ".join(lint(d))
    assert "convension" in errs        # a typo that would silently drop a convention
    assert "notes" in errs             # a bogus top-level key
    assert "drafty" in errs            # a status outside the vocabulary



def test_emitted_json_schemas_are_valid_and_accept_real_instances():
    jsonschema = pytest.importorskip("jsonschema")
    from scitype.emit import DIST
    poset = json.loads((DIST / "2608.0002-FinitePoset.schema.json").read_text())
    jsonschema.Draft202012Validator.check_schema(poset)
    v = jsonschema.Draft202012Validator(poset)
    assert not list(v.iter_errors(
        {"covers": {"n": 4, "arcs": [[0, 2], [0, 3], [1, 2], [1, 3]]}}))
    assert list(v.iter_errors({"covers": {"n": "four", "arcs": []}}))


def test_encoding_constraints_enforced_semantic_ones_declared():
    """We check preconditions of the identifier; mathematics is the reader's."""
    from scitype.checks import SEMANTIC, run
    assert run("lex-sorted-unique", {"arcs": [[1, 2], [0, 1]]})
    assert run("divisibility-chain", {"invariant_factors": ["4", "2"]})
    assert run("acyclic", {"n": 2, "arcs": [[0, 1], [1, 0]]}) == []
    assert "acyclic" in SEMANTIC


def test_instance_validation_covers_the_publishing_need():
    from scitype.registry import load
    from scitype.validate import validate_instance
    P = load()["scitype:2608.0002"]
    assert validate_instance(
        {"covers": {"n": 4, "arcs": [[0, 2], [0, 3], [1, 2], [1, 3]]}}, P) == []
    assert validate_instance({}, P)                                   # defining absent
    assert validate_instance({"covers": {"n": 3, "arcs": [[1, 2], [0, 1]]}}, P)  # unsorted
