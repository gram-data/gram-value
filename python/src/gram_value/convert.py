"""Mapping between values and Python values (ADR-006)."""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
from decimal import Decimal as PyDecimal
from decimal import InvalidOperation
from typing import Any

from . import value as v
from .diagnostic import Diagnostic, PathSegment, Result, diagnostic, fail, ok, under
from .value import Value


@dataclass(frozen=True)
class Tagged:
    """A tagged string: a tag and a content string."""

    tag: str
    content: str


@dataclass(frozen=True)
class Measurement:
    """A magnitude and a unit."""

    magnitude: int | float | PyDecimal
    unit: str


@dataclass(frozen=True)
class Range:
    """A range: inclusive, with an optional lower end and an optional upper end."""

    lower: int | float | PyDecimal | Measurement | None = None
    upper: int | float | PyDecimal | Measurement | None = None


Native = Any
"""A Python value a gram value maps to: int, float, Decimal, bool, str, Tagged, Range,
Measurement, list or dict."""


def _numeric_to_native(x: v.NumericValue) -> int | float | PyDecimal:
    if isinstance(x, (v.IntegerValue, v.BigIntegerValue, v.DecimalValue)):
        return x.value
    return PyDecimal(x.digits)


def to_native(value: Value) -> Result[Native]:
    """Convert a value to Python values. Every Python entry is exact, so the result has no losses
    (ADR-006 2.4); the result shape is the same as in the other languages."""
    if v.is_numeric(value):
        return ok(_numeric_to_native(value))
    if isinstance(value, (v.BooleanValue, v.StringValue)):
        return ok(value.value)
    if isinstance(value, v.TaggedValue):
        return ok(Tagged(value.tag, value.content))
    if isinstance(value, v.MeasurementValue):
        return ok(Measurement(_numeric_to_native(value.magnitude), value.unit))
    if isinstance(value, v.RangeValue):
        return ok(Range(_end_to_native(value.lower), _end_to_native(value.upper)))
    if isinstance(value, v.ArrayValue):
        items: list[Native] = []
        losses: list[Diagnostic] = []
        errs: list[Diagnostic] = []
        for i, item in enumerate(value.items):
            r = to_native(item)
            if r.ok:
                items.append(r.value)
                losses.extend(under(i, r.diagnostics))
            else:
                errs.extend(under(i, r.diagnostics))
        return fail(*errs) if errs else ok(items, losses)
    entries: dict[str, Native] = {}
    losses = []
    errs = []
    for k, item in value.entries:  # type: ignore[union-attr]
        r = to_native(item)
        if r.ok:
            entries[k] = r.value
            losses.extend(under(k, r.diagnostics))
        else:
            errs.extend(under(k, r.diagnostics))
    return fail(*errs) if errs else ok(entries, losses)


def _end_to_native(
    end: v.NumericValue | v.MeasurementValue | None,
) -> int | float | PyDecimal | Measurement | None:
    if end is None:
        return None
    if isinstance(end, v.MeasurementValue):
        return Measurement(_numeric_to_native(end.magnitude), end.unit)
    return _numeric_to_native(end)


def _unsupported(path: list[PathSegment], what: str) -> Diagnostic:
    return diagnostic("unsupported", what, path)


def _number_to_value(n: Any, path: list[PathSegment]) -> Result[Value]:
    # bool is an int subclass, so it is checked by the caller first.
    if isinstance(n, int):
        return ok(v.integer(n))
    if isinstance(n, float):
        r = v.decimal(n)
        return r if r.ok else fail(diagnostic("not-finite", "NaN and infinities cannot be written", path))
    # decimal.Decimal
    if not n.is_finite():
        return fail(diagnostic("not-finite", "NaN and infinities cannot be written", path))
    text = format(n, "f")
    if "." not in text:
        text += ".0"
    try:
        return v.decimal_from_text(text)
    except InvalidOperation:  # pragma: no cover
        return fail(_unsupported(path, "an unreadable decimal"))


def _from_native(x: Any, path: list[PathSegment]) -> Result[Value]:
    def with_path(r: Result[Value]) -> Result[Value]:
        if r.ok:
            return r
        return fail(*(Diagnostic(d.code, d.message, tuple(path)) for d in r.diagnostics))

    if isinstance(x, bool):
        return ok(v.boolean(x))
    if isinstance(x, (int, float, PyDecimal)):
        return with_path(_number_to_value(x, path))
    if isinstance(x, str):
        return with_path(v.string(x))
    if isinstance(x, Tagged):
        return with_path(v.tagged(x.tag, x.content))
    if isinstance(x, Measurement):
        mag = _number_to_value(x.magnitude, [*path, "magnitude"])
        if not mag.ok:
            return mag
        return with_path(v.measurement(mag.value, x.unit))
    if isinstance(x, Range):
        errs: list[Diagnostic] = []
        ends: list[Value | None] = []
        for key, end in (("lower", x.lower), ("upper", x.upper)):
            if end is None:
                ends.append(None)
                continue
            r = _from_native(end, [*path, key])
            if r.ok:
                ends.append(r.value)
            else:
                errs.extend(r.diagnostics)
        return fail(*errs) if errs else with_path(v.range_(ends[0], ends[1]))
    if isinstance(x, (list, tuple)):
        items: list[Value] = []
        errs = []
        for i, item in enumerate(x):
            r = _from_native(item, [*path, i])
            if r.ok:
                items.append(r.value)
            else:
                errs.extend(r.diagnostics)
        return fail(*errs) if errs else ok(v.array(items))
    if isinstance(x, Mapping):
        errs = []
        entries: list[tuple[str, Value]] = []
        for k, item in x.items():
            if not isinstance(k, str):
                errs.append(_unsupported(path, "a map key must be a string"))
                continue
            r = _from_native(item, [*path, k])
            if r.ok:
                entries.append((k, r.value))
            else:
                errs.extend(r.diagnostics)
        return fail(*errs) if errs else v.map_(entries)
    what = "None" if x is None else type(x).__name__
    return fail(_unsupported(path, f"{what} has no gram counterpart"))


def from_native(x: Any) -> Result[Value]:
    """Convert a Python value to a gram value. Anything unwritable fails with every problem
    listed (ADR-006 2.6)."""
    return _from_native(x, [])
