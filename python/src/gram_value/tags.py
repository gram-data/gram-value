"""Interpretation of the ten known tags (spec/tags.md, ADR-004)."""

from __future__ import annotations

import re
from dataclasses import dataclass
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


Typed = Date | Time | DateTime | Duration | Point | Uuid | Week


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
    def __init__(self, code: Literal["invalid-form", "out-of-range"], message: str) -> None:
        super().__init__(message)
        self.code = code


def _form(message: str) -> _Fault:
    return _Fault("invalid-form", message)


def _range(message: str) -> _Fault:
    return _Fault("out-of-range", message)


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
