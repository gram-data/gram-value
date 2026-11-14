---
id: ADR-004
title: Ten Known Tags as Plain Interpretation Functions
status: accepted
rfcs: [RFC-001]
created: 2026-11-14
---

# ADR-004: Ten Known Tags as Plain Interpretation Functions

## 1. Context

A tagged string in gram names a type for a piece of text: ``date`2024-01-15` ``, ``point`30 10` ``, ``uuid`...` ``. The grammar leaves tags open, and other projects use tags for their own purposes (pattern-rs names constraint dialects such as `ts`, `re`, `zod`, `cypher`, `pydantic`; tree-sitter-gram highlights `md`, `sql` and `json`). A model that interpreted every tag would be wrong for some of them, so the model keeps a tagged string as a tag and a content string (ADR-002), and an optional layer interprets the tags whose content has a standard form.

relate-gram, which loads gram into the Relate database, already validates eight tags (its ADR-011), but its forms are stricter than the grammar's own examples require: its `time` needs an offset and seconds, and its `point` needs an SRID in extended well-known text, while tree-sitter-gram's test corpus writes ``time`17:35` `` and ``point`30 10` ``, and also ``week`2024W35` `` and ``weekDate`2024W35-5` ``. The general forms have to accept what the grammar's authority writes; a stricter consumer narrows them.

## 2. Decision

### 2.1 The set

Ten tags are known, written in exactly this case: `date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point`, `uuid`, `week` and `weekDate`. Any other tag, including `url`, `md` and the constraint dialects, is unknown and passes through unchanged.

### 2.2 One function per tag, no registry

Each known tag has its own function that takes a `tagged` value and returns an interpretation result (2.4). A convenience function dispatches over the ten and returns the value unchanged for an unknown tag. There is no registry object. A consumer extends the layer by writing its own function over a tagged value, and supports fewer tags by calling only the functions it wants. The interpreted value types (a date, a point, and so on) are defined per language by that language's mapping (RFC-002); this decision fixes only the accepted content forms and the diagnostics.

### 2.3 Content forms

- **`date`:** `YYYY-MM-DD`, a year of at least four digits with an optional leading `-`, a month 01 to 12 and a day that exists in that month (leap years included).
- **`time`:** `hh:mm`, optionally `:ss`, optionally a fraction of 1 to 9 digits after the seconds, optionally followed by `Z` or `+hh:mm` or `-hh:mm`. Hours are 00 to 23, minutes and seconds 00 to 59, and an offset's hours are 00 to 23 and its minutes 00 to 59. A fraction requires seconds.
- **`localtime`:** the `time` form with no offset allowed.
- **`datetime`:** a `date`, `T`, and a `time`, optionally followed, when an offset is present, by a zone name in square brackets as in `2024-01-15T10:30:00+01:00[Europe/Paris]` (the notation of RFC 9557). The zone name must be non-empty and use only letters, digits and `/ _ + -`. Whether the name is a known zone, and whether the offset is that zone's, is not checked here.
- **`localdatetime`:** a `date`, `T`, and a `localtime`.
- **`duration`:** an upper-case `P`, then optionally `nY`, `nM`, `nD`, then optionally `T` with `nH`, `nM`, `nS`, at least one component present. Each number may carry a leading `-`; only the seconds may have a fraction, of up to 9 digits. Weeks and a leading `-` before the `P` are not accepted.
- **`point`:** an optional prefix `SRID=n;` with `n` a positive integer, then two or three numbers separated by single spaces, each `-?digits(.digits)?` with no exponent and no `+` (``point`30 10` ``, ``point`30 10 5` ``, ``point`SRID=4326;12.5 55.7` ``). Two numbers are x and y, three are x, y and z. Which SRIDs are valid, whether a given SRID needs two or three numbers, and the range of a latitude are a consumer's rules.
- **`uuid`:** 32 hexadecimal digits in 8-4-4-4-12 groups, either case.
- **`week`:** `YYYYWww`, a four-digit year and a week 01 to 53 (week 53 only in years that have one in ISO 8601).
- **`weekDate`:** `week`, `-`, and a weekday 1 to 7.

The forms are the extended-format ISO 8601 representations, restricted as stated (the expanded year, the zone name and per-number duration signs go beyond ISO 8601; a point and a UUID are not ISO 8601). Basic forms (`20240115`), ordinal dates and week durations are not accepted.

### 2.4 Interpretation results

An interpretation returns the result shape of ADR-005: the interpreted value, or, on failure, no value and a list of diagnostics. A diagnostic has a code, a message that names what is wrong, and a path (ADR-005); for a tag, the path names the tag. Two codes exist: `invalid-form` (the text does not follow the form) and `out-of-range` (the form is right but a part is outside its range, such as month 13 or a nonexistent day). A recognized tag with bad content is never reinterpreted as a plain string.

### 2.5 The value is not normalized; the interpretation has a normal form

Interpretation does not change the value: a `tagged` value keeps its tag and content exactly as written, so ``uuid`ABCDEF...` `` and its lower-case spelling are different values by ADR-002's equality, and both interpret to the same UUID. Each successful interpretation also has a normal content string, which is what the conformance corpus's `interpreted` property states (ADR-001): `uuid` in lower case; `time`, `localtime`, `datetime` and `localdatetime` with seconds present (`17:35` becomes `17:35:00`), a trailing zero-free fraction, and `Z` for an offset of `+00:00`; `point` with the numbers in the shortest form that reads back as the same number and an explicit `SRID=n;` only when given; `date`, `duration`, `week` and `weekDate` as written. A consumer that wants one spelling converts at its own boundary.

### 2.6 Consumers narrow with profiles

A consumer's stricter rules (a required offset, a fixed set of SRIDs, a latitude limit, rejecting `week`) are applied to the interpreted result or to the accepted forms in the consumer's own code. `gram-value` ships no consumer's profile (RFC-001 4.3).

## 3. Alternatives Considered

3.1. **Only the three tags tree-sitter-gram's documentation lists (`date`, `datetime`, `time`).** Rejected: the grammar's own corpus writes `duration`, `point`, `week` and `weekDate`, and relate-gram's `localtime`, `localdatetime` and `uuid` are the other forms that a database stores.

3.2. **relate-gram's stricter forms as the general forms.** Rejected: its required offset and SRID reject the corpus's ``time`17:35` `` and ``point`30 10` ``, and they encode Relate's types, not gram's.

3.3. **A registry that consumers add tags to.** Rejected: it adds a mutable global or an object to pass around in every language, while a function over a tagged value does the same work. Fewer tags mean calling fewer functions.

3.4. **Interpret every tag a consumer knows, including `md` and `ts`.** Rejected: those name languages for highlighting and validation, not a value type with a content form that the model could check.

3.5. **Accept ISO 8601's basic forms and week or ordinal dates in `date`.** Rejected: more spellings per type mean more cases per language, and relate-gram's experience (its SPIKE-005) is that one spelling is enough. The week forms exist only as the separate `week` and `weekDate` tags.

3.6. **Write a point as extended well-known text (`SRID=4326;POINT(x y)`) as relate-gram does.** Rejected for the general form: it does not accept ``point`30 10` ``, which the grammar's corpus writes. relate-gram can narrow, or convert at its boundary.

## 4. Consequences

- A tool such as relate-gram can drop its own validators for the shared forms and keep only its profile; relate-gram's ADR-011 section 2.9 says so, and its point form (extended well-known text) differs from this decision's `SRID=n;x y` and needs a follow-up in either direction.
- Every language implements ten validators (about 800 lines in relate-gram for eight). Parity rests on the conformance corpus's `TagCase`s (ADR-001): at least one accepted and one rejected case per tag and per bound, including leap days, week 53 and fractional seconds.
- Adding an eleventh known tag turns content that was unknown into possible `invalid-form` errors, so it is a breaking change under ADR-002 2.4's policy for new kinds.
- A time with no offset is a `time`, not a `localtime`, so two tags have overlapping accepted text; the difference is only that `localtime` forbids an offset. A consumer that distinguishes them gets that from the tag.
- tree-sitter-gram's well-known-tags document lists three of these tags; it should link here once the specification text exists.
- Follow-up work: write the forms in `spec/` with examples, add the `TagCase`s, and the diagnostic codes are in the conformance vocabulary (ADR-001 2.5).

## 5. Related

- RFC-001 (resolves questions 7.6 and 7.17).
- ADR-002 (the `tagged` kind), ADR-005 (the result shape), ADR-001 (the tag cases).
- relate-gram ADR-011 (its profile of these forms, amended 2026-11-14); tree-sitter-gram's `docs/tagged-strings-and-injections.md` and test corpus.
