"""Canonical encoding and presentation identifiers (S1).

Two independent implementations MUST derive the same identifier from the same
object, so everything here is specified rather than inherited from a library.

Canonical JSON is RFC 8785 (JCS). Note that JCS sorts object keys, so the
*declared order* of defining fields does not enter the encoding: a schema's
`defining` list selects which fields are hashed, and JCS fixes their order. That
removes the one place two implementations could plausibly diverge.
"""

from __future__ import annotations

import hashlib
import json
import re
from typing import Any

__all__ = ["jcs", "presentation_id", "canonical_preimage", "CanonicalError"]

_SAFE_INT = 2**53
_LINEAGE = re.compile(r"^[a-z][a-z0-9_-]*:\d{4}\.\d{4}$")


class CanonicalError(ValueError):
    """The value cannot be canonically encoded."""


def _sort_key(key: str) -> tuple[int, ...]:
    """JCS orders keys by UTF-16 code unit, which differs from code point above
    the BMP. Encoding to UTF-16BE and reading back pairs gives the right order.
    """
    try:
        raw = key.encode("utf-16-be")
    except UnicodeEncodeError as exc:
        raise CanonicalError("lone surrogates are not valid Unicode") from exc
    return tuple(int.from_bytes(raw[i : i + 2], "big") for i in range(0, len(raw), 2))


def _encode(value: Any, out: list[str]) -> None:
    if value is True:
        out.append("true")
    elif value is False:
        out.append("false")
    elif value is None:
        out.append("null")
    elif isinstance(value, int):
        # Arbitrary-precision integers travel as decimal strings; a bare JSON
        # number beyond the double-safe range would not round-trip.
        if abs(value) >= _SAFE_INT:
            raise CanonicalError(
                f"integer {value} exceeds the JSON-safe range; encode it as a "
                f"decimal string"
            )
        out.append(str(value))
    elif isinstance(value, float):
        raise CanonicalError(
            "floats are not permitted in defining data; use a decimal string or "
            "a RealBall"
        )
    elif isinstance(value, str):
        try:
            value.encode("utf-8")
        except UnicodeEncodeError as exc:
            raise CanonicalError("lone surrogates are not valid Unicode") from exc
        out.append(json.dumps(value, ensure_ascii=False))
    elif isinstance(value, (list, tuple)):
        out.append("[")
        for i, item in enumerate(value):
            if i:
                out.append(",")
            _encode(item, out)
        out.append("]")
    elif isinstance(value, dict):
        for k in value:
            if not isinstance(k, str):
                raise CanonicalError(f"object keys must be strings, got {k!r}")
        out.append("{")
        for i, k in enumerate(sorted(value, key=_sort_key)):
            if i:
                out.append(",")
            out.append(json.dumps(k, ensure_ascii=False))
            out.append(":")
            _encode(value[k], out)
        out.append("}")
    else:
        raise CanonicalError(f"cannot canonically encode {type(value).__name__}")


def jcs(value: Any) -> str:
    """Serialize to RFC 8785 canonical JSON."""
    out: list[str] = []
    _encode(value, out)
    return "".join(out)


def canonical_preimage(lineage: str, defining: dict[str, Any]) -> bytes:
    """The exact bytes hashed by :func:`presentation_id`.

    Exposed so a conformance corpus can pin the preimage as well as the digest —
    a mismatch then says whether the encoder or the hash construction diverged.
    """
    if not _LINEAGE.match(lineage):
        raise CanonicalError(f"not a lineage id: {lineage!r} (want authority:YYMM.NNNN)")
    return (lineage + "\n" + jcs(defining)).encode("utf-8")


def presentation_id(lineage: str, defining: dict[str, Any]) -> str:
    """Identifier for a record, from its lineage and its defining fields only.

    The version is deliberately absent: defining fields never change within a
    lineage (S3), so a version bump cannot disturb an existing identifier.
    """
    digest = hashlib.sha256(canonical_preimage(lineage, defining)).hexdigest()
    return f"sha256:{digest}"
