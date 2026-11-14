"""Canonical gram text (spec/text.md, ADR-003)."""

from __future__ import annotations

from typing import Literal

from .diagnostic import Diagnostic, PathSegment, Result, diagnostic, fail, ok
from .value import (
    ArrayValue,
    BigDecimalValue,
    BigIntegerValue,
    BooleanValue,
    DecimalValue,
    IntegerValue,
    MapValue,
    MeasurementValue,
    RangeValue,
    StringValue,
    TaggedValue,
    Value,
    float_text,
    is_bare_word,
)

Context = Literal["value", "record"]
"""Whether the outermost map is an ordinary value or an element's property record."""

_ESCAPES = {"\\": "\\\\", "\b": "\\b", "\f": "\\f", "\n": "\\n", "\r": "\\r", "\t": "\\t"}


def _escape(s: str, quote: str) -> str:
    out: list[str] = []
    for c in s:
        if c in _ESCAPES:
            out.append(_ESCAPES[c])
        elif c == quote:
            out.append("\\" + c)
        else:
            out.append(c)
    return "".join(out)


def string_text(s: str) -> str:
    """A string in canonical text: the bare word when it can be, else double-quoted."""
    return s if is_bare_word(s) else f'"{_escape(s, chr(34))}"'


def _unwritable(path: list[PathSegment], what: str) -> Diagnostic:
    return diagnostic("unwritable", what, path)


def _scalar_text(v: Value) -> tuple[str | None, str]:
    """Return (text, "") for a scalar, or (None, reason) when it has none."""
    if isinstance(v, (IntegerValue, BigIntegerValue)):
        return str(v.value), ""
    if isinstance(v, DecimalValue):
        return float_text(v.value), ""
    if isinstance(v, BigDecimalValue):
        return v.digits, ""
    if isinstance(v, BooleanValue):
        return ("true" if v.value else "false"), ""
    if isinstance(v, StringValue):
        return string_text(v.value), ""
    if isinstance(v, TaggedValue):
        return f"{v.tag}`{_escape(v.content, '`')}`", ""
    if isinstance(v, MeasurementValue):
        mag, why = _scalar_text(v.magnitude)
        if mag is None:
            return None, why
        if mag == "0" and v.unit.startswith("x"):
            return (
                None,
                "a measurement of 0 with a unit beginning x can read as a hexadecimal literal",
            )
        return f"{mag}{v.unit}", ""
    if isinstance(v, RangeValue):
        lo = hi = None
        for end, which in ((v.lower, "lo"), (v.upper, "hi")):
            if end is None:
                continue
            t, why = _scalar_text(end)
            if t is None:
                return None, why
            if which == "lo":
                lo = t
            else:
                hi = t
        if lo is not None and hi is not None:
            return f"{lo}..{hi}", ""
        if lo is not None:
            return f"{lo}...", ""
        if hi is not None:
            return f"...{hi}", ""
        return None, "a range with no ends"
    return None, "not a scalar"


def _write_array(items: tuple[Value, ...], path: list[PathSegment], errs: list[Diagnostic]) -> str:
    parts: list[str] = []
    for i, item in enumerate(items):
        path.append(i)
        if isinstance(item, ArrayValue):
            errs.append(_unwritable(path, "an array cannot hold an array"))
        elif isinstance(item, MapValue):
            errs.append(_unwritable(path, "an array cannot hold a map"))
        else:
            t, why = _scalar_text(item)
            if t is None:
                errs.append(_unwritable(path, why))
            else:
                parts.append(t)
        path.pop()
    return "[" + ", ".join(parts) + "]"


def _write_map(m: MapValue, nested_ok: bool, path: list[PathSegment], errs: list[Diagnostic]) -> str:
    parts: list[str] = []
    for k, v in m.entries:
        path.append(k)
        text: str | None = None
        if isinstance(v, ArrayValue):
            text = _write_array(v.items, path, errs)
        elif isinstance(v, MapValue):
            if nested_ok:
                text = _write_map(v, False, path, errs)
            else:
                errs.append(_unwritable(path, "a map entry cannot hold a map"))
        else:
            t, why = _scalar_text(v)
            if t is None:
                errs.append(_unwritable(path, why))
            else:
                text = t
        if text is not None:
            parts.append(f"{string_text(k)}: {text}")
        path.pop()
    return "{" + ", ".join(parts) + "}"


def to_text(value: Value, context: Context = "value") -> Result[str]:
    """The canonical text of a value, or an ``unwritable`` diagnostic for each part the grammar
    has no text for."""
    errs: list[Diagnostic] = []
    path: list[PathSegment] = []
    if isinstance(value, ArrayValue):
        text = _write_array(value.items, path, errs)
    elif isinstance(value, MapValue):
        text = _write_map(value, context == "record", path, errs)
    else:
        t, why = _scalar_text(value)
        if t is None:
            errs.append(_unwritable(path, why))
            t = ""
        text = t
    return ok(text) if not errs else fail(*errs)
