"""Constraint vocabulary.

SciType is a repository of *opinions* about how to specify mathematical objects.
It does not verify mathematics and ships no verifier: deciding that a polynomial
is irreducible, or that an action is faithful, needs a computer-algebra system,
and that capability belongs with a compute engine rather than a schema language.

But not every constraint is mathematics. Some exist so that **one object has one
encoding** — if `arcs` may arrive unsorted then the same graph has two canonical
JSONs and therefore two identifiers, and identity stops meaning anything. Those we
must enforce, because making identifiers well-defined is precisely our job.

    ENCODING   enforced here. Violating it breaks identity.
    SEMANTIC   defined here, checked by the consumer. Violating it is
               false mathematics, not a malformed record.

A semantic term is still *named and defined* rather than left as prose, so two
schemas mean the same thing by `acyclic` and a consumer can implement against a
fixed vocabulary. That is the interoperability claim in miniature: we publish the
opinion, someone else does the computing.
"""

from __future__ import annotations

from typing import Any, Callable

__all__ = ["ENCODING", "SEMANTIC", "SUFFICIENCY", "TERMS", "run", "kind_of",
           "definition_of", "run_sufficiency"]

ENCODING: dict[str, Callable[..., list[str]]] = {}   # enforced: preconditions of the hash


def _vocabulary() -> tuple[dict[str, str], dict[str, str]]:
    """Semantic and sufficiency terms are mathematics, so they are registry data
    rather than strings in this file. Encoding checks are mechanism and stay."""
    import yaml
    from .registry import SCHEMA_DIR
    path = SCHEMA_DIR / "constraints.yaml"
    if not path.exists():
        return {}, {}
    doc = yaml.safe_load(path.read_text()) or {}
    sem = {k: v.get("definition", "") for k, v in (doc.get("semantic") or {}).items()}
    suf = {k: v.get("definition", "") for k, v in (doc.get("sufficiency") or {}).items()}
    return sem, suf


SEMANTIC, SUFFICIENCY = _vocabulary()   # declared: claims readers check


def _encoding(name: str):
    def wrap(fn):
        ENCODING[name] = fn
        return fn
    return wrap


def run_sufficiency(name: str, defining: dict) -> list[str]:
    """Always empty: sufficiency is the consumer's to check. Present so a caller
    can ask without special-casing, and so the term must still be registered."""
    return [] if name in SUFFICIENCY else [f"unknown sufficiency check {name!r}"]


def kind_of(name: str) -> str | None:
    if name in ENCODING:
        return "encoding"
    if name in SEMANTIC:
        return "semantic"
    return None


def definition_of(name: str) -> str | None:
    return SEMANTIC.get(name) or SUFFICIENCY.get(name)


def TERMS() -> dict[str, str]:
    return {**{k: "encoding" for k in ENCODING},
            **{k: "semantic" for k in SEMANTIC},
            **{k: "sufficiency" for k in SUFFICIENCY}}


def run(name: str, value: Any, **args) -> list[str]:
    """Enforce an encoding constraint. A semantic term returns no problems: it is
    a declaration about the mathematics, and the consumer checks it."""
    if name in SEMANTIC:
        return []
    fn = ENCODING.get(name)
    if fn is None:
        return [f"unknown constraint {name!r}"]
    return fn(value, **args)


# --- encoding constraints: identity depends on these -------------------------

@_encoding("lex-sorted-unique")
def _lex_sorted_unique(value: Any, *, key: str = "arcs") -> list[str]:
    seq = value.get(key, []) if isinstance(value, dict) else value
    pairs = [tuple(x) if isinstance(x, list) else x for x in seq]
    out = []
    if pairs != sorted(pairs):
        out.append(f"{key}: not lex-sorted, so the same object would encode two ways")
    if len(set(pairs)) != len(pairs):
        out.append(f"{key}: contains duplicates")
    return out


@_encoding("endpoints-in-range")
def _endpoints(value: Any) -> list[str]:
    n = value.get("n", 0)
    key = "arcs" if "arcs" in value else "edges"
    for a, b in value.get(key, []):
        if not (0 <= a < n and 0 <= b < n):
            return [f"{key}: endpoint out of range for n={n}"]
        if a == b:
            return [f"{key}: self-loop"]
    return []


@_encoding("divisibility-chain")
def _divisibility_chain(value: Any, *, key: str = "invariant_factors") -> list[str]:
    seq = [int(x) for x in (value.get(key, []) if isinstance(value, dict) else value)]
    out = [f"{key}: {d} is not > 1" for d in seq if d <= 1]
    for a, b in zip(seq, seq[1:]):
        if b % a:
            out.append(f"{key}: {a} does not divide {b}; not invariant-factor form")
    return out


@_encoding("positive")
def _positive(value: Any, *, key: str) -> list[str]:
    v = int(value[key]) if isinstance(value, dict) else int(value)
    return [] if v > 0 else [f"{key}: must be positive, got {v}"]


@_encoding("non-negative")
def _non_negative(value: Any, *, key: str) -> list[str]:
    v = int(value[key]) if isinstance(value, dict) else int(value)
    return [] if v >= 0 else [f"{key}: must be non-negative, got {v}"]


@_encoding("coprime")
def _coprime(value: Any, *, num: str = "num", den: str = "den") -> list[str]:
    from math import gcd
    g = int(value[den])
    for x in value[num]:
        g = gcd(g, int(x))
    return [] if g == 1 else [f"{num}/{den}: not in lowest terms (gcd {g})"]


@_encoding("monic")
def _monic(value: Any, *, key: str = "coeffs") -> list[str]:
    seq = value.get(key, []) if isinstance(value, dict) else value
    if not seq:
        return [f"{key}: empty"]
    return [] if int(seq[-1]) == 1 else [f"{key}: leading coefficient is not 1"]


@_encoding("weakly-decreasing")
def _weakly_decreasing(value, *, key: str = "parts") -> list[str]:
    seq = [int(x) for x in (value.get(key, []) if isinstance(value, dict) else value)]
    out = [f"{key}: {x} is not positive" for x in seq if x <= 0]
    if seq != sorted(seq, reverse=True):
        out.append(f"{key}: not weakly decreasing, so one object would encode two ways")
    return out


@_encoding("facets-are-maximal")
def _facets_are_maximal(value, *, key: str = "facets") -> list[str]:
    facets = [frozenset(f) for f in
              (value.get(key, []) if isinstance(value, dict) else value)]
    for i, a in enumerate(facets):
        for j, b in enumerate(facets):
            if i != j and a < b:
                return [f"{key}: {sorted(a)} is contained in {sorted(b)}; "
                        f"list maximal faces only"]
    return []
