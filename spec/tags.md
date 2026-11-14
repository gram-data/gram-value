# Known tags

A tagged string is a tag and a content string ([values.md](values.md)). Ten tags are *known*: a package offers one
function per tag that checks the content form and returns an interpretation. Any other tag is unknown and passes
through unchanged. A tag is matched in exactly this case. A convenience function MAY dispatch over the ten.

Interpretation does not change the value: the tagged value keeps its tag and content as written. What an
interpretation produces is a typed value (a date, a point, a UUID) defined by each language's mapping, and a
*normal content string* (section 4) that the conformance corpus compares.

## 1. Content forms

Numbers below are ASCII digits. Letters in a form are literal unless stated.

| Tag | Content form |
|---|---|
| `date` | `YYYY-MM-DD` |
| `time` | `hh:mm`, optionally `:ss`, optionally `.f` (1 to 9 digits, only with seconds), optionally an offset |
| `localtime` | the `time` form with no offset |
| `datetime` | a `date`, `T`, a `time`, and, when an offset is present, optionally `[zone]` |
| `localdatetime` | a `date`, `T`, a `localtime` |
| `duration` | `P`, then optional `nY`, `nM`, `nD`, then optionally `T` with optional `nH`, `nM`, `nS`; at least one component |
| `point` | optional `SRID=n;`, then two or three numbers separated by single spaces |
| `uuid` | 32 hexadecimal digits in groups 8-4-4-4-12 separated by `-`, either case |
| `week` | `YYYYWww` |
| `weekDate` | `YYYYWww-D` |

Detail and ranges:

- **Year** (`date`, `datetime`, `localdatetime`): at least four digits with an optional leading `-`. The calendar is proleptic Gregorian. Month 01 to 12; the day exists in that month (a leap year is divisible by 4 and not by 100, or by 400).
- **Offset**: `Z`, `+hh:mm` or `-hh:mm`, hours 00 to 23, minutes 00 to 59.
- **Time**: hours 00 to 23, minutes and seconds 00 to 59 (no leap second, no `24:00`).
- **Zone**: in square brackets after an offset (RFC 9557 notation); non-empty, characters from letters, digits and `/ _ + -`. Whether it is a known zone, and whether the offset is that zone's, is not checked.
- **Duration**: each number has an optional leading `-`; only the seconds may have a fraction (`.f`, up to 9 digits); weeks (`W`) and a leading `-` before `P` are not accepted; a `T` must be followed by at least one of `H`, `M`, `S`.
- **Point**: `n` in `SRID=n;` is a positive integer. Each number is `-?digits(.digits)?`, with no `+` and no exponent. Two numbers are x and y; three are x, y and z. The valid SRIDs, whether an SRID requires two or three numbers, and limits such as a latitude range are a consumer's rules.
- **Week**: the year is exactly four digits (no sign), week 01 to 52, or 53 only in a year that has 53 ISO weeks: a year whose 1 January is a Thursday, or a leap year whose 1 January is a Wednesday. Weekday for `weekDate` is 1 (Monday) to 7.

Forms not listed are rejected: basic dates (`20240115`), ordinal dates, a week-based duration, a date with a missing part.

## 2. Diagnostics

A failed interpretation returns no value and one diagnostic ([results.md](results.md)) with the code:

| Code | Meaning |
|---|---|
| `invalid-form` | the text does not follow the form of section 1 |
| `out-of-range` | the form is right but a part is outside its range (month 13, a nonexistent day, hour 24, week 53 in a year without one, an offset of `+24:00`) |

The diagnostic's message begins with the tag. Its path is the location of the tagged value within the value being
interpreted (empty for the value itself). A recognized tag with bad content is never reinterpreted as a plain string.

## 3. Profiles

A consumer's stricter rules (a required offset, a fixed set of SRIDs, no `week`) are its own and are applied by the
consumer. This specification ships no consumer's profile.

## 4. Normal content strings

A successful interpretation has a normal content string, the value a conformance case's `interpreted` states:

| Tag | Normal content string |
|---|---|
| `date`, `duration`, `week`, `weekDate` | the content as written |
| `uuid` | the content in lower case |
| `time`, `localtime`, `datetime`, `localdatetime` | seconds always present (`17:35` becomes `17:35:00`); a fraction written without trailing zeros and omitted when zero; an offset of `+00:00` or `-00:00` written `Z`; zone and other parts as written |
| `point` | each number as its shortest decimal text that reads back as the same number (`30.0` becomes `30`, `-0` becomes `0`); `SRID=n;` only if given |

## 5. Examples

| Tag and content | Result |
|---|---|
| ``date`2024-02-29` `` | accepted |
| ``date`2023-02-29` `` | `out-of-range` |
| ``date`2024-1-5` `` | `invalid-form` |
| ``time`17:35` `` | accepted, normal `17:35:00` |
| ``time`17:42:03.149` `` | accepted |
| ``time`24:00:00` `` | `out-of-range` |
| ``datetime`2024-01-15T10:30:00+01:00[Europe/Paris]` `` | accepted |
| ``localtime`10:30:00Z` `` | `invalid-form` |
| ``duration`PT2H3M42S` `` | accepted |
| ``duration`P1W` `` | `invalid-form` |
| ``point`30 10` `` | accepted |
| ``point`SRID=4326;12.5 55.7` `` | accepted |
| ``point`30` `` | `invalid-form` |
| ``uuid`123E4567-E89B-12D3-A456-426614174000` `` | accepted, normal is lower case |
| ``week`2024W35` `` | accepted |
| ``week`2021W53` `` | `out-of-range` (2021 has 52 ISO weeks) |
| ``weekDate`2024W35-5` `` | accepted |
