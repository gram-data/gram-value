"""The eleven kinds of value, their constructors, equality and order (spec/values.md, ADR-002)."""

from __future__ import annotations

import math
import re
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from decimal import Decimal as _PyDecimal
from functools import cmp_to_key
from typing import Any, Literal, TypeGuard

from .diagnostic import Result, diagnostic, fail, ok

_I64_MIN = -(2**63)
_I64_MAX = 2**63 - 1


@dataclass(frozen=True, eq=False)
class _Base:
    """Equality and hashing compare content part by part; ordering is the total order."""

    def __eq__(self, other: object) -> bool:
        return isinstance(other, _Base) and compare(self, other) == 0  # type: ignore[arg-type]

    def __lt__(self, other: Value) -> bool:
        return compare(self, other) < 0  # type: ignore[arg-type]

    def __le__(self, other: Value) -> bool:
        return compare(self, other) <= 0  # type: ignore[arg-type]

    def __gt__(self, other: Value) -> bool:
        return compare(self, other) > 0  # type: ignore[arg-type]

    def __ge__(self, other: Value) -> bool:
        return compare(self, other) >= 0  # type: ignore[arg-type]

    def __hash__(self) -> int:
        return hash(_hash_key(self))  # type: ignore[arg-type]


@dataclass(frozen=True, eq=False)
class IntegerValue(_Base):
    value: int
    kind: Literal["integer"] = "integer"


@dataclass(frozen=True, eq=False)
class BigIntegerValue(_Base):
    value: int
    kind: Literal["big_integer"] = "big_integer"


@dataclass(frozen=True, eq=False)
class DecimalValue(_Base):
    value: float
    kind: Literal["decimal"] = "decimal"


@dataclass(frozen=True, eq=False)
class BigDecimalValue(_Base):
    digits: str
    """Sign, digits, point and digits, with no trailing zeros beyond one digit."""
    kind: Literal["big_decimal"] = "big_decimal"


@dataclass(frozen=True, eq=False)
class BooleanValue(_Base):
    value: bool
    kind: Literal["boolean"] = "boolean"


@dataclass(frozen=True, eq=False)
class StringValue(_Base):
    value: str
    kind: Literal["string"] = "string"


@dataclass(frozen=True, eq=False)
class TaggedValue(_Base):
    tag: str
    content: str
    kind: Literal["tagged"] = "tagged"


NumericValue = IntegerValue | BigIntegerValue | DecimalValue | BigDecimalValue


@dataclass(frozen=True, eq=False)
class MeasurementValue(_Base):
    magnitude: NumericValue
    unit: str
    kind: Literal["measurement"] = "measurement"


@dataclass(frozen=True, eq=False)
class RangeValue(_Base):
    lower: NumericValue | MeasurementValue | None = None
    upper: NumericValue | MeasurementValue | None = None
    kind: Literal["range"] = "range"


@dataclass(frozen=True, eq=False)
class ArrayValue(_Base):
    items: tuple[Value, ...]
    kind: Literal["array"] = "array"


@dataclass(frozen=True, eq=False)
class MapValue(_Base):
    entries: tuple[tuple[str, Value], ...]
    """Entries in ascending order of the keys' UTF-8 bytes."""
    kind: Literal["map"] = "map"

    def get(self, key: str) -> Value | None:
        for k, v in self.entries:
            if k == key:
                return v
        return None


Value = (
    IntegerValue
    | BigIntegerValue
    | DecimalValue
    | BigDecimalValue
    | BooleanValue
    | StringValue
    | TaggedValue
    | RangeValue
    | MeasurementValue
    | ArrayValue
    | MapValue
)

KINDS: tuple[str, ...] = (
    "integer",
    "big_integer",
    "decimal",
    "big_decimal",
    "boolean",
    "string",
    "tagged",
    "range",
    "measurement",
    "array",
    "map",
)
"""The kinds in kind order, which is also the order of the total order."""


def is_scalar(v: Value) -> bool:
    return v.kind not in ("array", "map")


def is_homogeneous_array(v: Value) -> bool:
    """For an array, whether all elements have one kind (true when empty); false for other kinds."""
    return isinstance(v, ArrayValue) and len({x.kind for x in v.items}) <= 1


def is_numeric(v: Value) -> TypeGuard[NumericValue]:
    return v.kind in ("integer", "big_integer", "decimal", "big_decimal")


def is_symbol(s: str) -> bool:
    """True when ``s`` matches the symbol rule ``[a-zA-Z_][0-9a-zA-Z_.\\-@]*``."""
    return re.fullmatch(r"[a-zA-Z_][0-9a-zA-Z_.\-@]*", s) is not None


def is_bare_word(s: str) -> bool:
    """True when ``s`` is a symbol other than ``true`` and ``false``."""
    return is_symbol(s) and s not in ("true", "false")


def _well_formed(s: str) -> bool:
    try:
        s.encode("utf-8")
    except UnicodeEncodeError:
        return False
    return True


# ---- number text --------------------------------------------------------------------------------

_INTEGER_TEXT = re.compile(r"-?(0|[1-9][0-9]*)")
_DECIMAL_TEXT = re.compile(r"-?(0|[1-9][0-9]*)\.[0-9]+")


def _normalize_decimal(s: str) -> str:
    int_part, _, frac = s.partition(".")
    frac = frac.rstrip("0")
    return f"{int_part}.{frac or '0'}"


def float_text(x: float) -> str:
    """The shortest positional text of a finite float, with at least one digit after the point."""
    text = format(_PyDecimal(repr(x)), "f")
    return text if "." in text else f"{text}.0"


# ---- constructors -------------------------------------------------------------------------------


def integer(n: int) -> Value:
    """An integer: an ``integer`` if it fits in 64 bits, else a ``big_integer``."""
    if isinstance(n, bool) or not isinstance(n, int):
        raise TypeError(f"{n!r} is not an int")
    return IntegerValue(n) if _I64_MIN <= n <= _I64_MAX else BigIntegerValue(n)


def integer_from_digits(text: str) -> Result[Value]:
    """An integer from ``-?(0|[1-9][0-9]*)``."""
    if not _INTEGER_TEXT.fullmatch(text) or text == "-0":
        return fail(diagnostic("invalid-integer", f"{text!r} is not an integer without leading zeros"))
    return ok(integer(int(text)))


def big_integer(n: int) -> Result[Value]:
    """A ``big_integer``; the number must be outside the 64-bit range."""
    if _I64_MIN <= n <= _I64_MAX:
        return fail(diagnostic("invalid-integer", f"{n} fits in 64 bits, so it is an integer"))
    return ok(BigIntegerValue(n))


def decimal(x: float) -> Result[Value]:
    """A ``decimal``. ``NaN`` and the infinities are rejected (``not-finite``)."""
    if x != x or x in (float("inf"), float("-inf")):
        return fail(diagnostic("not-finite", "a decimal must be finite"))
    return ok(DecimalValue(float(x)))


def decimal_from_text(text: str) -> Result[Value]:
    """A decimal from ``-?(0|[1-9][0-9]*)\\.[0-9]+``: a ``decimal`` when the nearest float writes
    back as the same number, else a ``big_decimal``."""
    if not _DECIMAL_TEXT.fullmatch(text):
        return fail(diagnostic("invalid-decimal", f"{text!r} is not a decimal"))
    normalized = _normalize_decimal(text)
    f = float(normalized)
    if f not in (float("inf"), float("-inf")) and float_text(f) == normalized:
        return ok(DecimalValue(f))
    return ok(BigDecimalValue(normalized))


def big_decimal(text: str) -> Result[Value]:
    """A ``big_decimal``. The text must be normalized and beyond what a float holds."""
    if _DECIMAL_TEXT.fullmatch(text) and _normalize_decimal(text) == text:
        r = decimal_from_text(text)
        if r.ok and isinstance(r.value, BigDecimalValue):
            return r
    return fail(diagnostic("invalid-decimal", f"{text!r} is not a normalized decimal beyond a 64-bit float"))


def boolean(b: bool) -> Value:
    return BooleanValue(bool(b))


def string(s: str) -> Result[Value]:
    """A ``string``. A string with a lone surrogate is rejected (``invalid-string``)."""
    if not _well_formed(s):
        return fail(diagnostic("invalid-string", "a string must be valid Unicode"))
    return ok(StringValue(s))


def tagged(tag: str, content: str) -> Result[Value]:
    """A ``tagged`` value. The tag must be a symbol other than ``true`` and ``false``."""
    if not is_bare_word(tag):
        return fail(diagnostic("invalid-tag", f"{tag!r} is not a symbol other than true or false"))
    if not _well_formed(content):
        return fail(diagnostic("invalid-string", "the content must be valid Unicode"))
    return ok(TaggedValue(tag, content))


def range_(lower: Value | None = None, upper: Value | None = None) -> Result[Value]:
    """A ``range``. Each end must be numeric or a measurement; at least one end is needed."""
    if lower is None and upper is None:
        return fail(diagnostic("no-ends", "a range needs at least one end"))
    for end in (lower, upper):
        if end is not None and not is_numeric(end) and not isinstance(end, MeasurementValue):
            return fail(
                diagnostic(
                    "invalid-range-end",
                    f"a range end must be numeric or a measurement, not {end.kind}",
                )
            )
    return ok(RangeValue(lower, upper))  # type: ignore[arg-type]


def measurement(magnitude: Value, unit: str) -> Result[Value]:
    """A ``measurement``. The magnitude must be numeric and the unit ASCII letters."""
    if not is_numeric(magnitude):
        return fail(diagnostic("invalid-range-end", f"a magnitude must be numeric, not {magnitude.kind}"))
    if not re.fullmatch(r"[a-zA-Z]+", unit):
        return fail(diagnostic("invalid-unit", f"{unit!r} is not one or more ASCII letters"))
    return ok(MeasurementValue(magnitude, unit))


def array(items: Iterable[Value]) -> Value:
    return ArrayValue(tuple(items))


def map_(entries: Iterable[tuple[str, Value]] | Mapping[str, Value]) -> Result[Value]:
    """A ``map``. A repeated key is an error (``duplicate-key``) that names the key."""
    pairs = entries.items() if isinstance(entries, Mapping) else entries
    seen: dict[str, Value] = {}
    for k, v in pairs:
        if k in seen:
            return fail(diagnostic("duplicate-key", f"the key {k!r} is repeated", [k]))
        seen[k] = v
    return ok(MapValue(tuple(sorted(seen.items(), key=lambda kv: kv[0]))))


# ---- equality, hashing and order ----------------------------------------------------------------


def _hash_key(v: Value) -> Any:
    if isinstance(v, DecimalValue):
        return ("decimal", v.value.hex())
    if isinstance(v, BigDecimalValue):
        return ("big_decimal", v.digits)
    if isinstance(v, (IntegerValue, BigIntegerValue, BooleanValue, StringValue)):
        return (v.kind, v.value)
    if isinstance(v, TaggedValue):
        return ("tagged", v.tag, v.content)
    if isinstance(v, RangeValue):
        return ("range", _opt_key(v.lower), _opt_key(v.upper))
    if isinstance(v, MeasurementValue):
        return ("measurement", _hash_key(v.magnitude), v.unit)
    if isinstance(v, ArrayValue):
        return ("array", tuple(_hash_key(x) for x in v.items))
    return ("map", tuple((k, _hash_key(x)) for k, x in v.entries))


def _opt_key(v: Value | None) -> Any:
    return None if v is None else _hash_key(v)


def _cmp(a: Any, b: Any) -> int:
    return -1 if a < b else 1 if a > b else 0


def _compare_digits(a: str, b: str) -> int:
    na, nb = a.startswith("-"), b.startswith("-")
    if na != nb:
        return -1 if na else 1
    ai, _, af = a.lstrip("-").partition(".")
    bi, _, bf = b.lstrip("-").partition(".")
    m = _cmp(len(ai), len(bi)) or _cmp(ai, bi) or _cmp(af, bf)
    return -m if na else m


def _compare_optional(a: Value | None, b: Value | None) -> int:
    if a is None:
        return 0 if b is None else -1
    if b is None:
        return 1
    return compare(a, b)


def compare(a: Value, b: Value) -> int:
    """The total order of spec/values.md section 6; consistent with equality."""
    ka, kb = KINDS.index(a.kind), KINDS.index(b.kind)
    if ka != kb:
        return -1 if ka < kb else 1
    if isinstance(a, (IntegerValue, BigIntegerValue)) and isinstance(b, (IntegerValue, BigIntegerValue)):
        return _cmp(a.value, b.value)
    if isinstance(a, DecimalValue) and isinstance(b, DecimalValue):
        if a.value == 0 and b.value == 0:
            sa, sb = math.copysign(1.0, a.value) < 0, math.copysign(1.0, b.value) < 0
            return 0 if sa == sb else (-1 if sa else 1)
        return _cmp(a.value, b.value)
    if isinstance(a, BigDecimalValue) and isinstance(b, BigDecimalValue):
        return _compare_digits(a.digits, b.digits)
    if isinstance(a, BooleanValue) and isinstance(b, BooleanValue):
        return _cmp(a.value, b.value)
    if isinstance(a, StringValue) and isinstance(b, StringValue):
        return _cmp(a.value, b.value)
    if isinstance(a, TaggedValue) and isinstance(b, TaggedValue):
        return _cmp(a.tag, b.tag) or _cmp(a.content, b.content)
    if isinstance(a, RangeValue) and isinstance(b, RangeValue):
        return _compare_optional(a.lower, b.lower) or _compare_optional(a.upper, b.upper)
    if isinstance(a, MeasurementValue) and isinstance(b, MeasurementValue):
        return compare(a.magnitude, b.magnitude) or _cmp(a.unit, b.unit)
    if isinstance(a, ArrayValue) and isinstance(b, ArrayValue):
        for x, y in zip(a.items, b.items, strict=False):
            c = compare(x, y)
            if c:
                return c
        return _cmp(len(a.items), len(b.items))
    if isinstance(a, MapValue) and isinstance(b, MapValue):
        for (kx, vx), (ky, vy) in zip(a.entries, b.entries, strict=False):
            c = _cmp(kx, ky) or compare(vx, vy)
            if c:
                return c
        return _cmp(len(a.entries), len(b.entries))
    raise TypeError("values of one kind have one class")  # pragma: no cover


def equals(a: Value, b: Value) -> bool:
    """Equality: same kind and same content, part by part (ADR-002 2.5)."""
    return compare(a, b) == 0


sort_key = cmp_to_key(compare)
"""A ``key=`` function that sorts values in the total order."""
