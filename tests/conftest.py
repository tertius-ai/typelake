"""Shared fixtures: the corpus loader and the problem matcher."""

import json
import pathlib

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[1]
CORPUS = ROOT / "conformance"


def cases(name):
    """Load a conformance file. Corpus data drives the tests rather than the
    other way round, so adding a vector adds a test."""
    with (CORPUS / name).open() as fh:
        return [json.loads(line) for line in fh if line.strip()]


def check(got, expect, name):
    if expect:
        for want in expect:
            assert any(want in g for g in got), f"{name}: expected {want!r}, got {got}"
    else:
        assert got == [], f"{name}: expected clean, got {got}"


@pytest.fixture(scope="session")
def schemas():
    from scitype.registry import load
    return load()
