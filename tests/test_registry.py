"""Is OUR registry in good order?

Not about the implementation and not about the standard: these check the 26
schemas we ship. Most rules live in `scitype lint`, which anyone can run on
any registry including their own; here we assert ours passes, plus the
editorial rules that are about curation rather than correctness.
"""

import json
import pathlib

import pytest

from scitype.lint import lint
from scitype.registry import load
from conftest import CORPUS, ROOT, cases, check as _check


def test_identifiers_cover_real_lineages():
    schemas = load()
    for case in cases("identifiers.jsonl"):
        assert case["lineage"] in schemas

def test_defining_fields_match_the_schema():
    """A corpus record must supply exactly its schema's defining fields."""
    schemas = load()
    for case in cases("identifiers.jsonl"):
        assert set(case["defining"]) == set(schemas[case["lineage"]].defining), case["name"]

def test_registry_lints_clean():
    assert lint() == []

def test_every_schema_declares_identity_and_categories():
    for sid, s in load().items():
        assert s.canonicalization_kind in {"complete", "invariant-based"}, sid
        assert s.get("categories"), sid
        assert s.get("mnemonic"), sid


# --- validation -------------------------------------------------------------

from scitype.validate import validate_instance  # noqa: E402


def _check(got, expect, name):
    if expect:
        for want in expect:
            assert any(want in g for g in got), f"{name}: expected {want!r}, got {got}"
    else:
        assert got == [], f"{name}: expected clean, got {got}"

def test_every_declared_constraint_is_in_the_vocabulary():
    from scitype.checks import TERMS
    from scitype.registry import load
    vocab = TERMS()
    for sid, s in load().items():
        for c in (s.get("constraints") or []):
            assert c["check"] in vocab, (sid, c)
        for name, spec in (s.get("fields") or {}).items():
            if isinstance(spec, dict) and spec.get("constraint"):
                assert spec["constraint"] in vocab, (sid, name)

def test_constraints_name_real_fields():
    """A constraint naming a field that does not exist can never run, and reads
    as a guarantee that silently is not one."""
    from scitype.registry import load
    for sid, s in load().items():
        fields = s.get("fields") or {}
        for c in (s.get("constraints") or []):
            for arg, val in (c.get("args") or {}).items():
                if arg in {"key", "num", "den"} and isinstance(val, str):
                    assert val in fields, (sid, c["check"], val)

def test_every_declared_convention_is_registered():
    from scitype.registry import conventions, load
    vocab = conventions()
    for sid, s in load().items():
        for name, spec in (s.get("fields") or {}).items():
            if isinstance(spec, dict):
                for c in spec.get("convention") or []:
                    assert c in vocab, (sid, name, c)

def test_conventions_are_defined_not_merely_named():
    from scitype.registry import conventions
    for name, entry in conventions().items():
        assert entry.get("title"), name
        assert len(entry.get("definition", "")) > 60, name

def test_computed_fields_acknowledge_convention():
    """Every derived-expensive field declares `convention`, possibly empty.

    Replaces a hand-listed set of "fields that hide a choice", which gave false
    comfort: it checked the four fields its author happened to think of and
    passed while EllipticCurve.rank did not distinguish algebraic from analytic
    and EllipticCurve.regulator pinned no normalization. Ambiguity is not
    detectable, but *considering* it can be required.
    """
    from scitype.registry import load
    for sid, s in load().items():
        for name, spec in (s.get("fields") or {}).items():
            if isinstance(spec, dict) and spec.get("policy") == "derived-expensive":
                assert "convention" in spec, f"{s.mnemonic}.{name}"

def test_conventions_are_pinned_by_examples_not_only_prose():
    """A definition in English moves ambiguity up a level; a discriminating
    example removes it. Every convention with a live alternative must name a case
    where the alternative gives a different answer."""
    from scitype.registry import conventions
    vocab = conventions()
    for name, entry in vocab.items():
        ex = entry.get("discriminates") or []
        cited = entry.get("references")
        assert ex or cited, f"{name}: neither a discriminating example nor a citation"
        for case in ex:
            assert case.get("case"), name
            assert case.get("under_this"), name
            assert case.get("alternative"), name
            assert case.get("under_alternative"), name
            assert case["under_this"] != case["under_alternative"] or \
                "agree" in case["under_alternative"], name

def test_discriminating_examples_name_a_real_alternative():
    """The alternative must be either registered, or explicitly outside our
    registry — never a term invented in passing."""
    from scitype.registry import conventions
    vocab = conventions()
    known = set(vocab) | {
        "homology-of-hasse-diagram", "f-vector-with-empty-face",
        "euler-characteristic-reduced", "height-covering-steps",
        "automorphism-including-order-reversing", "permutation-left-action",
        "regulator-squared-unit", "unit-group-with-torsion",
        "signature-counting-all-complex-embeddings", "grading-homological",
        "matrix-column-major", "chain-complex-cohomological",
        "partition-weakly-increasing", "graph-with-loops", "simplicial-all-faces",
        "weierstrass-as-given", "quadratic-form-coefficients",
        "newform-unnormalized", "cartan-matrix-transpose",
        "code-equivalence-monomial", "rank-analytic", "regulator-twice-height",
    }
    for name, entry in vocab.items():
        for case in entry.get("discriminates") or []:
            assert case["alternative"] in known, (name, case["alternative"])

def test_stable_entries_have_an_example():
    """A schema nobody has instantiated is a schema nobody has checked. `stable`
    is a claim about an entry, and a claim should rest on evidence."""
    from scitype.cli import _corpus_coverage
    from scitype.registry import load
    schemas = load()
    covered = _corpus_coverage(schemas)
    for sid, s in schemas.items():
        if s.get("status") == "stable":
            assert sid in covered, f"{s.mnemonic} claims stable with no example"

def test_every_entry_declares_a_status():
    from scitype.registry import load
    for sid, s in load().items():
        assert s.get("status") in {"draft", "provisional", "stable"}, sid

def test_emitted_artifacts_are_current():
    """published/ and the identity vectors are generated. A stale committed copy is a
    lie told with authority."""
    import tempfile
    from scitype.emit import CORPUS, DIST, emit, identity_vectors
    tmp = tempfile.mkdtemp()
    for fresh in emit(tmp):
        home = CORPUS if fresh.suffix == ".jsonl" else DIST
        committed = home / fresh.name
        assert committed.exists(), f"{fresh.name} missing; run `scitype emit`"
        assert committed.read_text() == fresh.read_text(), \
            f"{fresh.name} is stale; run `scitype emit`"

def test_every_vector_comes_from_an_authored_example():
    """Vectors are derived, so nobody hand-computes a hash — a hand-written
    expected id is a wrong vector waiting to be trusted."""
    from scitype.emit import identity_vectors
    derived = {v["presentation_id"] for v in identity_vectors()}
    for case in cases("identifiers.jsonl"):
        assert case["presentation_id"] in derived, case["name"]
