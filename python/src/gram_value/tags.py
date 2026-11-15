"""Interpretation of the eleven known tags (spec/tags.md, ADR-004, ADR-008)."""

from __future__ import annotations

import base64
import re
from dataclasses import dataclass
from enum import Enum
from typing import Literal

from .diagnostic import Result, diagnostic, fail, ok
from .value import TaggedValue, Value, float_text, tagged

KNOWN_TAGS = (
    "date",
    "time",
    "localtime",
    "datetime",
    "localdatetime",
    "duration",
    "point",
    "uuid",
    "week",
    "weekDate",
    "embedding",
)


@dataclass(frozen=True)
class Date:
    year: int
    month: int
    day: int


@dataclass(frozen=True)
class Time:
    hour: int
    minute: int
    second: int
    nanosecond: int
    offset_minutes: int | None = None
    """Minutes east of UTC, when an offset is written."""


@dataclass(frozen=True)
class DateTime:
    date: Date
    time: Time
    zone: str | None = None


@dataclass(frozen=True)
class Seconds:
    negative: bool
    whole: int
    nanos: int


@dataclass(frozen=True)
class Duration:
    years: int | None = None
    months: int | None = None
    days: int | None = None
    hours: int | None = None
    minutes: int | None = None
    seconds: Seconds | None = None


@dataclass(frozen=True)
class Point:
    x: float
    y: float
    z: float | None = None
    srid: int | None = None


@dataclass(frozen=True)
class Uuid:
    bytes: bytes


@dataclass(frozen=True)
class Week:
    year: int
    week: int
    day: int | None = None


class EmbeddingDtype(str, Enum):
    F16 = "f16"
    BF16 = "bf16"
    F32 = "f32"
    F64 = "f64"

    @property
    def width(self) -> int:
        return 2 if self in (EmbeddingDtype.F16, EmbeddingDtype.BF16) else (4 if self is EmbeddingDtype.F32 else 8)

    @property
    def safetensors_name(self) -> str:
        return self.value.upper()


MAX_EMBEDDING_DIMENSION = 1_048_576


@dataclass(frozen=True)
class Embedding:
    dtype: EmbeddingDtype
    dimension: int
    bytes: bytes

    @classmethod
    def create(cls, dtype: EmbeddingDtype, dimension: int, data: bytes) -> Result[Embedding]:
        if isinstance(dimension, bool) or not isinstance(dimension, int) or dimension <= 0 or dimension > MAX_EMBEDDING_DIMENSION:
            return fail(
                diagnostic(
                    "out-of-range",
                    f"embedding: dimension {dimension} must be between 1 and {MAX_EMBEDDING_DIMENSION}",
                )
            )
        expected = dimension * dtype.width
        if len(data) != expected:
            return fail(
                diagnostic(
                    "invalid-form",
                    f"embedding: expected {expected} payload bytes for {dimension} {dtype.value} elements, got {len(data)}",
                )
            )
        return ok(cls(dtype=dtype, dimension=dimension, bytes=bytes(data)))

    def to_normal_content(self) -> str:
        return _encode_embedding_normal(self.dtype, self.dimension, self.bytes)

    def to_value(self) -> Value:
        return tagged("embedding", self.to_normal_content()).unwrap()


Typed = Date | Time | DateTime | Duration | Point | Uuid | Week | Embedding


@dataclass(frozen=True)
class Interpretation:
    """A successful interpretation."""

    tag: str
    typed: Typed
    normal_content: str
    """The normal content string (spec/tags.md section 4)."""

    def to_value(self) -> Value:
        """The tagged value with the normal content string."""
        return tagged(self.tag, self.normal_content).unwrap()


class _Fault(Exception):
    def __init__(self, code: Literal["invalid-form", "out-of-range", "unsupported"], message: str) -> None:
        super().__init__(message)
        self.code = code


def _form(message: str) -> _Fault:
    return _Fault("invalid-form", message)


def _range(message: str) -> _Fault:
    return _Fault("out-of-range", message)


def _unsupported(message: str) -> _Fault:
    return _Fault("unsupported", message)


def interpret(value: Value) -> Result[Interpretation | None]:
    """Interpret a tagged value. ``ok(None)`` for a value that is not tagged or whose tag is unknown."""
    if not isinstance(value, TaggedValue):
        return ok(None)
    return interpret_tag(value.tag, value.content)


def interpret_tag(tag: str, content: str) -> Result[Interpretation | None]:
    """Interpret a tag and content. ``ok(None)`` for an unknown tag."""
    if tag not in KNOWN_TAGS:
        return ok(None)
    try:
        return ok(_run(tag, content))
    except _Fault as e:
        return fail(diagnostic(e.code, f"{tag}: {e}"))


def _run(tag: str, content: str) -> Interpretation:
    if tag == "date":
        date, used = _parse_date_prefix(content)
        if used != len(content):
            raise _form("unexpected text after the date")
        return Interpretation(tag, date, content)
    if tag in ("time", "localtime"):
        t = _parse_time(content, tag == "time")
        return Interpretation(tag, t, _time_normal(t))
    if tag in ("datetime", "localdatetime"):
        dt, normal = _parse_datetime(content, tag == "datetime")
        return Interpretation(tag, dt, normal)
    if tag == "duration":
        return Interpretation(tag, _parse_duration(content), content)
    if tag == "point":
        p, normal = _parse_point(content)
        return Interpretation(tag, p, normal)
    if tag == "uuid":
        return Interpretation(tag, _parse_uuid(content), content.lower())
    if tag == "embedding":
        emb, normal = _parse_embedding(content)
        return Interpretation(tag, emb, normal)
    return Interpretation(tag, _parse_week(content, tag == "weekDate"), content)


def _two_digits(s: str, what: str) -> int:
    if not re.fullmatch(r"[0-9]{2}", s):
        raise _form(f"{what} must be two digits")
    return int(s)


def _is_leap(y: int) -> bool:
    return (y % 4 == 0 and y % 100 != 0) or y % 400 == 0


def _days_in_month(year: int, month: int) -> int:
    if month in (1, 3, 5, 7, 8, 10, 12):
        return 31
    if month in (4, 6, 9, 11):
        return 30
    return 29 if _is_leap(year) else 28


def _parse_date_prefix(s: str) -> tuple[Date, int]:
    m = re.match(r"(-?[0-9]{4,})-([0-9]{2})-([0-9]{2})", s)
    if not m:
        if not re.match(r"-?[0-9]{4,}", s):
            raise _form("the year needs at least four digits")
        raise _form("expected YYYY-MM-DD")
    year, month, day = int(m.group(1)), int(m.group(2)), int(m.group(3))
    if not 1 <= month <= 12:
        raise _range(f"month {month} does not exist")
    if day == 0 or day > _days_in_month(year, month):
        raise _range(f"day {day} does not exist in month {month} of {year}")
    return Date(year, month, day), m.end()


def _parse_offset(s: str) -> int:
    if s == "Z":
        return 0
    m = re.fullmatch(r"([+-])([0-9]{2}):([0-9]{2})", s)
    if not m:
        raise _form("an offset is Z, +hh:mm or -hh:mm")
    h, mi = int(m.group(2)), int(m.group(3))
    if h > 23 or mi > 59:
        raise _range("the offset is out of range")
    return (-1 if m.group(1) == "-" else 1) * (h * 60 + mi)


def _parse_time(s: str, offset_allowed: bool) -> Time:
    m = re.search(r"[Z+-]", s)
    clock = s if m is None else s[: m.start()]
    off = "" if m is None else s[m.start() :]
    offset_minutes: int | None = None
    if off:
        if not offset_allowed:
            raise _form("this tag does not allow an offset")
        offset_minutes = _parse_offset(off)
    if len(clock) < 5 or clock[2] != ":":
        raise _form("expected hh:mm")
    hour = _two_digits(clock[0:2], "the hour")
    minute = _two_digits(clock[3:5], "the minute")
    second = nanosecond = 0
    if len(clock) > 5:
        if clock[5] != ":" or len(clock) < 8:
            raise _form("expected hh:mm or hh:mm:ss")
        second = _two_digits(clock[6:8], "the second")
        if len(clock) > 8:
            if clock[8] != ".":
                raise _form("expected a fraction after the seconds")
            frac = clock[9:]
            if not 1 <= len(frac) <= 9 or not re.fullmatch(r"[0-9]+", frac):
                raise _form("a fraction has 1 to 9 digits")
            nanosecond = int(frac.ljust(9, "0"))
    if hour > 23 or minute > 59 or second > 59:
        raise _range("the time is out of range")
    return Time(hour, minute, second, nanosecond, offset_minutes)


def _offset_text(minutes: int) -> str:
    if minutes == 0:
        return "Z"
    a = abs(minutes)
    return f"{'-' if minutes < 0 else '+'}{a // 60:02d}:{a % 60:02d}"


def _time_normal(t: Time) -> str:
    s = f"{t.hour:02d}:{t.minute:02d}:{t.second:02d}"
    if t.nanosecond:
        s += "." + f"{t.nanosecond:09d}".rstrip("0")
    if t.offset_minutes is not None:
        s += _offset_text(t.offset_minutes)
    return s


def _parse_datetime(s: str, offset_allowed: bool) -> tuple[DateTime, str]:
    date, used = _parse_date_prefix(s)
    if s[used : used + 1] != "T":
        raise _form("expected T after the date")
    rest = s[used + 1 :]
    clock, zone = rest, None
    if "[" in rest:
        if not rest.endswith("]"):
            raise _form("a zone name is written [name]")
        i = rest.index("[")
        clock, zone = rest[:i], rest[i + 1 : -1]
    time = _parse_time(clock, offset_allowed)
    if zone is not None:
        if time.offset_minutes is None:
            raise _form("a zone name needs an offset")
        if not re.fullmatch(r"[A-Za-z0-9/_+-]+", zone):
            raise _form("a zone name is letters, digits and / _ + -")
    normal = f"{s[:used]}T{_time_normal(time)}" + (f"[{zone}]" if zone is not None else "")
    return DateTime(date, time, zone), normal


def _parse_signed(num: str) -> int:
    if not re.fullmatch(r"-?[0-9]+", num):
        raise _form("a duration component is a number")
    return int(num)


def _parse_duration(s: str) -> Duration:
    if not s.startswith("P"):
        raise _form("a duration starts with P")
    rest = s[1:]
    fields: dict[str, int | Seconds] = {}
    in_time = False
    order_date = order_time = time_components = 0
    any_component = False
    while rest:
        if rest.startswith("T"):
            if in_time:
                raise _form("T appears once")
            in_time = True
            rest = rest[1:]
            if not rest:
                raise _form("T needs at least one component")
            continue
        m = re.search(r"[A-Z]", rest)
        if m is None:
            raise _form("a component needs a designator")
        num, designator, rest = rest[: m.start()], m.group(0), rest[m.end() :]
        any_component = True
        if in_time:
            time_components += 1
        if not in_time and designator == "Y" and order_date < 1:
            fields["years"], order_date = _parse_signed(num), 1
        elif not in_time and designator == "M" and order_date < 2:
            fields["months"], order_date = _parse_signed(num), 2
        elif not in_time and designator == "D" and order_date < 3:
            fields["days"], order_date = _parse_signed(num), 3
        elif in_time and designator == "H" and order_time < 1:
            fields["hours"], order_time = _parse_signed(num), 1
        elif in_time and designator == "M" and order_time < 2:
            fields["minutes"], order_time = _parse_signed(num), 2
        elif in_time and designator == "S" and order_time < 3:
            sm = re.fullmatch(r"(-?)([0-9]+)(?:\.([0-9]{1,9}))?", num)
            if not sm:
                raise _form("seconds are digits with an optional fraction of 1 to 9 digits")
            nanos = int(sm.group(3).ljust(9, "0")) if sm.group(3) else 0
            fields["seconds"], order_time = Seconds(sm.group(1) == "-", int(sm.group(2)), nanos), 3
        else:
            raise _form("components are in the order Y M D, then T, H M S")
    if not any_component:
        raise _form("a duration has at least one component")
    if in_time and time_components == 0:
        raise _form("T needs at least one component")
    return Duration(**fields)  # type: ignore[arg-type]


_NUMBER = re.compile(r"-?[0-9]+(\.[0-9]+)?")


def _number_text(f: float) -> str:
    if f == 0:
        return "0"
    text = float_text(f)
    return text[:-2] if text.endswith(".0") else text


def _parse_point(s: str) -> tuple[Point, str]:
    srid: int | None = None
    coords = s
    if s.startswith("SRID="):
        if ";" not in s:
            raise _form("SRID=n; needs the semicolon")
        n, coords = s[5:].split(";", 1)
        if not re.fullmatch(r"[0-9]+", n) or int(n) == 0:
            raise _form("the SRID is a positive integer")
        srid = int(n)
    parts = coords.split(" ")
    if len(parts) not in (2, 3) or not all(_NUMBER.fullmatch(p) for p in parts):
        raise _form("a point is two or three numbers separated by single spaces")
    nums = [float(p) for p in parts]
    if any(n in (float("inf"), float("-inf")) for n in nums):
        raise _range("a coordinate is out of range")
    normal = (f"SRID={srid};" if srid is not None else "") + " ".join(_number_text(n) for n in nums)
    return Point(nums[0], nums[1], nums[2] if len(nums) == 3 else None, srid), normal


def _parse_uuid(s: str) -> Uuid:
    if not re.fullmatch(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}", s):
        raise _form("a UUID is 8-4-4-4-12 hexadecimal digits")
    return Uuid(bytes.fromhex(s.replace("-", "")))


def _jan1_weekday(year: int) -> int:
    """Day of the week of 1 January, 0 = Monday (proleptic Gregorian)."""
    y = year - 1
    return (y * 365 + y // 4 - y // 100 + y // 400) % 7


def _has_53_weeks(year: int) -> bool:
    w = _jan1_weekday(year)
    return w == 3 or (_is_leap(year) and w == 2)


def _parse_week(s: str, with_day: bool) -> Week:
    pattern = r"([0-9]{4})W([0-9]{2})-([0-9])" if with_day else r"([0-9]{4})W([0-9]{2})"
    m = re.fullmatch(pattern, s)
    if not m:
        raise _form("expected YYYYWww-D" if with_day else "expected YYYYWww")
    year, week = int(m.group(1)), int(m.group(2))
    maximum = 53 if _has_53_weeks(year) else 52
    if week == 0 or week > maximum:
        raise _range(f"{year} has {maximum} ISO weeks, not week {week}")
    if with_day:
        day = int(m.group(3))
        if not 1 <= day <= 7:
            raise _range("the weekday is 1 to 7")
        return Week(year, week, day)
    return Week(year, week)


# ---- embedding (ADR-008) --------------------------------------------------------------------

_MAX_BASE64_CHARS = 11_190_284
_MAX_DECODED_BYTES = 8_392_712
_MAX_HEADER_BYTES = 4_096
_B64_ALPHABET = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
_B64_LOOKUP = {b: i for i, b in enumerate(_B64_ALPHABET)}
_U64_MAX = (1 << 64) - 1


def _decode_strict_base64(content: str) -> bytes:
    try:
        raw = content.encode("ascii")
    except UnicodeEncodeError as exc:
        raise _form("non-ASCII character in Base64 content") from exc
    if not raw:
        raise _form("empty Base64 content")
    if len(raw) % 4 != 0:
        raise _form("Base64 length must be a multiple of 4")
    if raw.endswith(b"=="):
        pad_count = 2
    elif raw.endswith(b"="):
        pad_count = 1
    else:
        pad_count = 0
    data_end = len(raw) - pad_count
    for i, b in enumerate(raw[:data_end]):
        if b not in _B64_LOOKUP:
            raise _form(f"invalid Base64 byte at index {i}")
    if pad_count == 1:
        if (_B64_LOOKUP[raw[-2]] & 0x03) != 0:
            raise _form("non-canonical Base64 pad bits")
    elif pad_count == 2:
        if (_B64_LOOKUP[raw[-3]] & 0x0F) != 0:
            raise _form("non-canonical Base64 pad bits")

    decoded_len = (len(raw) // 4) * 3 - pad_count
    if len(raw) > _MAX_BASE64_CHARS or decoded_len > _MAX_DECODED_BYTES:
        raise _range(f"embedding exceeds maximum decoded size of {_MAX_DECODED_BYTES} bytes")
    return base64.b64decode(raw, validate=True)


def _encode_embedding_normal(dtype: EmbeddingDtype, dimension: int, payload: bytes) -> str:
    json_str = (
        f'{{"embedding":{{"dtype":"{dtype.safetensors_name}",'
        f'"shape":[{dimension}],"data_offsets":[0,{len(payload)}]}}}}'
    )
    json_bytes = json_str.encode("utf-8")
    pad = (8 - (len(json_bytes) % 8)) % 8
    header_len = len(json_bytes) + pad
    buf = header_len.to_bytes(8, "little") + json_bytes + (b" " * pad) + payload
    return base64.b64encode(buf).decode("ascii")


class _HeaderParser:
    def __init__(self, s: str) -> None:
        self.s = s
        self.pos = 0

    def _skip_ws(self) -> None:
        while self.pos < len(self.s) and self.s[self.pos] in (" ", "\t", "\n", "\r"):
            self.pos += 1

    def _expect(self, ch: str) -> None:
        self._skip_ws()
        if self.pos >= len(self.s) or self.s[self.pos] != ch:
            raise _form(f"expected '{ch}' in JSON header")
        self.pos += 1

    def _parse_hex4(self) -> int:
        if self.pos + 4 > len(self.s):
            raise _form("short \\uXXXX escape in JSON string")
        hex_str = self.s[self.pos : self.pos + 4]
        if not re.fullmatch(r"[0-9a-fA-F]{4}", hex_str):
            raise _form("invalid \\uXXXX escape in JSON string")
        self.pos += 4
        return int(hex_str, 16)

    def _parse_string(self) -> str:
        self._skip_ws()
        if self.pos >= len(self.s) or self.s[self.pos] != '"':
            raise _form("expected JSON string")
        self.pos += 1
        out: list[str] = []
        escapes = {
            '"': '"',
            "\\": "\\",
            "/": "/",
            "b": "\b",
            "f": "\f",
            "n": "\n",
            "r": "\r",
            "t": "\t",
        }
        while self.pos < len(self.s):
            ch = self.s[self.pos]
            if ch == '"':
                self.pos += 1
                return "".join(out)
            if ord(ch) < 0x20:
                raise _form("unescaped control character in JSON string")
            if ch == "\\":
                self.pos += 1
                if self.pos >= len(self.s):
                    raise _form("unterminated JSON escape")
                esc = self.s[self.pos]
                self.pos += 1
                if esc in escapes:
                    out.append(escapes[esc])
                elif esc == "u":
                    code = self._parse_hex4()
                    if 0xD800 <= code <= 0xDBFF:
                        if self.s[self.pos : self.pos + 2] != "\\u":
                            raise _form("unpaired UTF-16 high surrogate in JSON string")
                        self.pos += 2
                        low = self._parse_hex4()
                        if not (0xDC00 <= low <= 0xDFFF):
                            raise _form("invalid UTF-16 low surrogate in JSON string")
                        cp = 0x10000 + (((code - 0xD800) << 10) | (low - 0xDC00))
                        out.append(chr(cp))
                    elif 0xDC00 <= code <= 0xDFFF:
                        raise _form("unpaired UTF-16 low surrogate in JSON string")
                    else:
                        out.append(chr(code))
                else:
                    raise _form("invalid JSON escape sequence")
            else:
                out.append(ch)
                self.pos += 1
        raise _form("unterminated JSON string")

    def _parse_plain_uint_token(self) -> int:
        self._skip_ws()
        start = self.pos
        while self.pos < len(self.s) and "0" <= self.s[self.pos] <= "9":
            self.pos += 1
        if self.pos == start:
            raise _form("expected plain base-10 integer")
        if self.pos - start > 1 and self.s[start] == "0":
            raise _form("leading zero in JSON integer")
        if self.pos < len(self.s) and self.s[self.pos] in (".", "e", "E"):
            raise _form("non-integer JSON number")
        return int(self.s[start : self.pos])

    def _parse_shape(self) -> int:
        self._expect("[")
        n = self._parse_plain_uint_token()
        self._expect("]")
        return n

    def _parse_data_offsets(self) -> tuple[int, int]:
        self._expect("[")
        a = self._parse_plain_uint_token()
        self._expect(",")
        b = self._parse_plain_uint_token()
        self._expect("]")
        if a > _U64_MAX or b > _U64_MAX:
            raise _form("data_offsets exceeds 64 unsigned bits")
        return (a, b)

    def parse_header(self) -> tuple[str, int, tuple[int, int]]:
        self._expect("{")
        top_key = self._parse_string()
        if top_key != "embedding":
            raise _form(f'unexpected top-level key "{top_key}"')
        self._expect(":")
        self._expect("{")

        dtype: str | None = None
        shape_dim: int | None = None
        offsets: tuple[int, int] | None = None

        for idx in range(3):
            if idx > 0:
                self._expect(",")
            key = self._parse_string()
            self._expect(":")
            if key == "dtype":
                if dtype is not None:
                    raise _form('duplicate key "dtype"')
                dtype = self._parse_string()
            elif key == "shape":
                if shape_dim is not None:
                    raise _form('duplicate key "shape"')
                shape_dim = self._parse_shape()
            elif key == "data_offsets":
                if offsets is not None:
                    raise _form('duplicate key "data_offsets"')
                offsets = self._parse_data_offsets()
            else:
                raise _form(f'unexpected tensor key "{key}"')
        self._expect("}")
        self._expect("}")
        self._skip_ws()
        if self.pos != len(self.s):
            raise _form("trailing bytes in SafeTensors JSON header")
        assert dtype is not None and shape_dim is not None and offsets is not None
        return (dtype, shape_dim, offsets)


def _parse_embedding(content: str) -> tuple[Embedding, str]:
    decoded = _decode_strict_base64(content)
    if len(decoded) < 8:
        raise _form("SafeTensors buffer shorter than 8 bytes")
    header_len = int.from_bytes(decoded[:8], "little")
    if header_len > _MAX_HEADER_BYTES or header_len > len(decoded) - 8:
        raise _form("SafeTensors header length exceeds 4,096 or buffer")
    try:
        header_str = decoded[8 : 8 + header_len].decode("utf-8")
    except UnicodeDecodeError as exc:
        raise _form("SafeTensors header is not valid UTF-8") from exc

    dtype_raw, shape_dim, offsets = _HeaderParser(header_str).parse_header()

    dtype_map = {
        "F16": EmbeddingDtype.F16,
        "BF16": EmbeddingDtype.BF16,
        "F32": EmbeddingDtype.F32,
        "F64": EmbeddingDtype.F64,
    }
    dtype = dtype_map.get(dtype_raw)
    if dtype is None:
        raise _unsupported(f'unsupported embedding dtype "{dtype_raw}"')

    if shape_dim > _U64_MAX:
        raise _range("embedding dimension exceeds 64 unsigned bits")
    if shape_dim == 0 or shape_dim > MAX_EMBEDDING_DIMENSION:
        raise _range(f"embedding dimension {shape_dim} must be between 1 and {MAX_EMBEDDING_DIMENSION}")

    expected_bytes = shape_dim * dtype.width
    if offsets != (0, expected_bytes):
        raise _form(f"data_offsets {list(offsets)} does not match [0, {expected_bytes}]")
    payload_start = 8 + header_len
    if len(decoded) != payload_start + expected_bytes:
        raise _form(f"payload length {len(decoded) - payload_start} does not match expected {expected_bytes}")

    payload = decoded[payload_start:]
    emb = Embedding(dtype=dtype, dimension=shape_dim, bytes=payload)
    normal = _encode_embedding_normal(dtype, shape_dim, payload)
    return (emb, normal)
