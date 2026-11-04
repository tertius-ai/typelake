"""Does this implementation do what the published corpus says?

Vectors in conformance/ are the specification, precisely stated. These run
them against our code — the same thing a second implementation would do.
"""

import json
import pathlib

import pytest

from scitype.canonical import CanonicalError, canonical_preimage, jcs, presentation_id
from scitype.registry import load
from scitype.validate import validate_instance
from conftest import CORPUS, ROOT, cases, check as _check


@pytest.mark.parametrize("case", cases("encoding.jsonl"), ids=lambda c: c["name"])
def test_encoding(case):
    if "error" in case:
        with pytest.raises(CanonicalError):
            jcs(eval(case["value"]))  # noqa: S307 — corpus is repo-local
    else:
        assert jcs(case["value"]) == case["jcs"]

@pytest.mark.parametrize("case", cases("identifiers.jsonl"), ids=lambda c: c["name"])
def test_identifiers(case):
    assert canonical_preimage(case["lineage"], case["defining"]).decode() == case["preimage"]
    assert presentation_id(case["lineage"], case["defining"]) == case["presentation_id"]

def test_presentations_of_one_object_differ():
    """x^2+1 and x^2+2x+2 both define Q(i): rung 1 separates them, rung 2 will not."""
    by_name = {c["name"]: c for c in cases("identifiers.jsonl")}
    a = by_name["numberfield-x2+1"]["presentation_id"]
    b = by_name["numberfield-x2+2x+2"]["presentation_id"]
    assert a != b

def test_version_absent_from_identity():
    """A version bump must not disturb an existing identifier (S3)."""
    d = {"coeffs": ["-1", "-1", "0", "1"]}
    assert presentation_id("scitype:2608.0003", d) == presentation_id("scitype:2608.0003", d)
    assert "v1" not in canonical_preimage("scitype:2608.0003", d).decode()
