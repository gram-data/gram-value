---
id: RFC-002
title: Host-Language Mappings and Loss Reporting
status: review
created: 2026-11-14
adrs: [ADR-001]
---

# RFC-002: Host-Language Mappings and Loss Reporting

## 1. Summary

A gram value is a value in a notation: an integer of any width, a decimal, a measurement such as `5kg`, a range
such as `1..5`, a tagged string, an array or a map. A program wants these as the native values of its
own language, a JavaScript number or a Python `dict` or a Rust `i64`, and the languages disagree about what they can
hold. A JavaScript number keeps 53 bits of an integer, no mainstream language has a measurement as a built-in
value, the ranges that do exist differ from gram's, and an integer-valued decimal such as `1.0` is the same number
as `1` in JavaScript. Some conversions are exact and some change what the file said.

The proposal is a mapping for each language: a documented table from every gram value to a native value, in both
directions, in which every entry is marked exact, lossy (with the loss named) or unsupported, and in which a
conversion reports the loss it incurred as data the consumer can inspect. A consumer chooses its own policy for a
loss: fail, accept and record, or ignore. The same mapping tables are checked against one language-neutral
conformance corpus, so that two languages agree about what is lost.

The scope and direction are the proposal. Section 7 records the decisions on loss kinds, the result shape, the native
type of each value and the policy for losses; each language's table is written as conformance cases (ADR-001).

## 2. Problem / Motivation

**The same gram text becomes different values in different languages, and nothing says so.** SPIKE-001 compared
the value types of pattern-rs (a Rust library of pattern and graph structures built on gram), pattern-hs (its
Haskell reference), and the TypeScript and Python packages of pattern-rs. Integers are unbounded in Haskell and
Python, 64-bit in Rust, and a JavaScript number in TypeScript, which holds exact integers only up to 2^53. A file
with `{id: 9007199254740993}` cannot be read at all through pattern-rs's TypeScript package: the parse fails with
"can't be represented as a JavaScript number", so one such integer makes the whole file unreadable and the
consumer has no loss record to act on. A mapping that returned the value with a recorded loss, or as a `BigInt`,
would let the consumer decide.

**A kind can change in transit.** The TypeScript package decodes a number as an integer whenever
`Number.isInteger` is true, so the decimal `1.0` in a gram file arrives as the integer `1`, because JavaScript has
one number type and nothing in the value distinguishes them.

**The host boundary is ambiguous.** The TypeScript and Python packages receive values from Rust as JSON-shaped
data, with ad-hoc tagged objects (`{type: "symbol", ...}`) for the kinds JSON lacks. A gram map whose key `type` holds
the string `symbol`, `range`, `tagged` or `measurement` is decoded as that kind instead of as a map (the map
`{type: "symbol", value: "x"}` is indistinguishable from the symbol `x`).

**Kinds with no native counterpart are modeled in different ways, each unstated.** A measurement and a
tagged string have no built-in value in Rust, TypeScript or Python, and a range exists only in forms that differ
from gram's (Rust's `1..5` excludes its upper bound, Python's `range` is integer-only). pattern-rs holds a range
as two optional floats and a measurement as a unit and a float, so a range of measurements cannot be represented
and `5kg` and `5.0kg` are one value. relate-gram, which loads gram into the Relate database, reads a bare symbol as a
string, rejects ranges and measurements, and keeps a tagged string as a typed value when it knows the tag and as a
plain string otherwise.

**Containers differ too.** A JavaScript object reorders keys that look like integers (`{b: 1, "2": 1}` iterates
`"2"` first), a Python `dict` keeps insertion order, and a Rust `HashMap` keeps none.

**Consumers cannot tell what they lost.** Each tool maps values its own way and returns the result with no record
of what was changed, so a tool that must report exactly what a file says (a validator, a loader, an exporter)
cannot use the mapping to decide whether to proceed.

## 3. Goals

3.1. For each first-tier language (Rust, TypeScript, Python), document the native value that each gram value kind
maps to, and that each known tag maps to, in both directions. A known tag, such as `date` or `point`, is a tag
whose content `gram-value` interprets (RFC-001's interpretation layer).

3.2. Mark every entry of every table exact (converting back gives the same gram value), lossy (stating what
information is lost) or unsupported (no native counterpart, stating what happens).

3.3. Make loss detectable: a conversion returns the converted value together with the losses it incurred, each
naming the kind of loss and where in the value it occurred, accumulated across the elements of an array or map
instead of stopping at the first.

3.4. Let a consumer choose what a loss means: fail the conversion, accept and record it, or ignore it, without
changing the table.

3.5. Specify what happens when a native value with no gram counterpart is written as gram (`null`, `NaN`, a map
with a non-string key).

3.6. Test every mapping against the conformance corpus, a set of language-neutral cases that every language
package runs, holding the expected native value and entry class per language.

## 4. Non-Goals

4.1. Choosing the value model itself (the kinds and their meaning). That is RFC-001's subject.

4.2. Mapping graph or pattern structures (nodes, relationships, paths). Only values are mapped.

4.3. A serialization framework. The tables say how values map; building a schema or object mapper on top is the
consumer's work.

4.4. Silent coercion beyond the table. A conversion not in the table is an error, not a best guess.

4.5. Mappings for Go, Swift and Haskell in the first round. They follow the first tier.

## 5. Proposed Approach (high-level)

### 5.1 A mapping is a pair of conversions with a loss annotation

For each language, one conversion takes a gram value to a native value and one takes a native value to a gram
value. Each reports what it lost. A table lists every value kind (and every known tag) and, per entry, the native
representation, the entry class, and the named loss if there is one.

### 5.2 Three entry classes

Exact: the round trip returns the same gram value. Lossy: information is lost and the entry names the loss kind
(`precision` of a decimal, or the `kind` of a number, as in the integer-versus-decimal distinction). Unsupported: the
native language has no counterpart and the entry says whether the conversion fails or produces a wrapper (5.5).

### 5.3 What the tables are expected to record

The tables record, among others, the entries that SPIKE-001 showed to differ between languages and that section 7 now settles: a 64-bit or larger integer in TypeScript; the
integer-versus-decimal distinction in TypeScript; a range, a measurement, a big integer, a big decimal and a tagged string in each
language; the native map type in each language (the model's maps are unordered with unique keys, RFC-001 7.2); the temporal, point and UUID types that the known
tags interpret.

### 5.4 Loss as data

The conversion's result has the shape of ADR-005: the value together with a list of losses, each with a `code` (the loss
kind), a `message` and a `path`. The list is accumulated across independent elements, and each language adds idiomatic
helpers for composing results (decided, 7.1 to 7.3).

### 5.5 A lossless escape hatch

Where a native value cannot hold a gram value, a language provides a wrapper that holds the gram value intact, so a
consumer that does not understand a value can still pass it through and write it back unchanged.

### 5.6 Writing native values as gram

The reverse table says which native values can be written (integers of any width, finite floats, strings, booleans,
arrays, maps with string keys, and the native temporal, point and UUID types through known tags) and which fail
(`null`, `undefined`, `NaN`, infinities, non-string map keys, functions), as decided in 7.9.

### 5.7 Loss policy belongs to the consumer

A conversion records its losses and never fails because of one. A consumer fails on any loss with a helper, or ignores
the list, so the same table serves a validator that fails and a viewer that tolerates (decided, 7.3).

### 5.8 Parity through the conformance corpus

RFC-003 elaborates this section. The corpus holds, for each case, the expected native value and entry class for every first-tier language. A
package passes when its conversions match every case, and the parity matrix is generated from the results.

## 6. Alternatives Considered

6.1. **Each consumer defines its own mapping.** The present state. Rejected: the mappings disagree, and none states
what it loses.

6.2. **Represent every value with `gram-value`'s own types in every language and never convert.** Lossless.
Rejected as the only route: every consumer would work in unfamiliar types, and many only want a `number` or a
`dict`. It remains available as the escape hatch (5.5).

6.3. **JSON as the host representation.** The route pattern-rs uses. Rejected: JSON has one number type and no
symbol, range, measurement or tagged string, so it loses the integer-versus-decimal distinction and the width of
an integer, and its ad-hoc tagged objects make a map with a `type` key ambiguous.

6.4. **Fail on any loss, with no recording.** Simple, and rejected: a viewer wants the value and the knowledge of
what changed, and a validator wants the full list of losses, not the first.

6.5. **Lean on each language's serialization framework (serde, pydantic, an Effect schema) to define the
mapping.** Rejected as the standard: the frameworks differ, so parity could not be tested. A consumer can build on
the tables with whichever framework it prefers.

6.6. **Always map to the most faithful native type (a `BigInt` for every integer, an exact decimal for every
decimal).** Lossless by construction. Rejected as the only rule: it makes every consumer work with unfamiliar
types for common cases, and not every language has such a type. It remains the choice of a consumer that asks for
it.

## 7. Open Questions

7.1. What are the loss kinds (precision, kind, order, spelling, and others), and is the set closed?

Decided 2026-11-14: the loss kinds are `precision` (digits are lost, as when a `big_decimal` becomes a 64-bit float) and
`kind` (the value's kind changes, as when `1.0` arrives as the integer `1`). A third, `order`, was dropped the same day
(see 7.8): a map is unordered, so a host that reorders keys loses nothing. The set is closed: a further loss kind
needs an ADR, so a consumer can match every kind. A value that a host cannot hold at all is not a loss; its entry is
unsupported and the conversion fails with a diagnostic. Spelling is not a loss kind, because the model keeps none
(RFC-001 7.1). The kind `structure` is added only if a decision on 7.6 maps ranges or measurements to generic forms.

7.2. What are the fields of a loss record, and how is the path to a part of a value written the same way in every
language?

Decided 2026-11-14: a loss is a diagnostic with the three fields of ADR-005, `code` (the loss kind of 7.1), `message`
and `path`. The path is a list of map keys and array indexes, and its text form is a JSON Pointer (RFC 6901): `/items/0/name`,
with `~0` for `~` and `~1` for `/` inside a key, and the empty string for the whole value. The list is the contract; the
text form is what messages and the conformance corpus use.

7.3. Where is the loss policy chosen (per call, per consumer, per language), and what is the default?

Decided 2026-11-14: a conversion to a host value never fails because of a loss. It returns the native value together
with its list of losses (accept and record). Each package offers a helper that turns any loss into a failure, for a
consumer that wants strictness, and ignoring losses is not reading the list. There is no policy argument on the
conversion, so one behavior is tested by the corpus.

7.4. In TypeScript, how is an integer beyond 2^53 (an `integer` or a `big_integer`) represented: a number with a recorded loss, a `BigInt` always, or
a number when the value is safe and a `BigInt` otherwise?

Decided 2026-11-14: an integer within plus or minus 2^53 - 1 becomes a `number` and any other integer, whether `integer` or
`big_integer`, becomes a `bigint`. Nothing is rounded, so the entry has no loss, and a consumer handles both types. This
applies to the TypeScript package, which follows the Rust package.

7.5. In each language, is a gram decimal a float or an exact decimal type where one exists (Python `Decimal`, no
JavaScript equivalent)? This follows RFC-001 7.4.

Decided 2026-11-14: a `decimal` becomes the language's 64-bit float (`f64`, `float`, `number`). A `big_decimal` becomes
`decimal.Decimal` in Python, exactly. Rust and TypeScript have no standard exact decimal, so in Rust the `big_decimal` entry
is unsupported as a native value and the package's own `BigDecimal` stays the value (the wrapper of 5.5), and in
TypeScript it becomes a `number` with a `precision` loss. In TypeScript a whole decimal such as `1.0` also arrives as an
integer-valued number, reported as a `kind` loss.

7.6. Which native types hold a range and a measurement: small record types defined by the package in each
language, or types from a library? Should a range use a language's own range type (Rust's `Range`, Python's
`range`) where one exists, given that its endpoints and inclusivity differ from gram's?

Decided 2026-11-14: each package defines its own small types: a `Range` (optional lower end, optional upper end,
inclusive) and a `Measurement` (magnitude and unit), whose ends and magnitude are that language's native numbers (a
range end may also be a measurement). They hold the value exactly, so the entries are exact and need no loss kind.
A language's own range type (Rust's `RangeInclusive`, Python's `range`) is never used, since its ends and inclusivity
differ from gram's; a consumer who wants one converts it. In Rust these types are the model's own.

7.7. Closed: the model has no symbol kind (RFC-001 7.5, decided). A symbol in a value position is a string, so no
mapping entry and no loss concerns it; only the writer's choice of a bare word or a quoted string differs.

7.8. Which native map type is used in each language (a JavaScript object or `Map`, a Python `dict`, a Rust sorted
map), and how is key order reported, given that the model's maps are unordered with keys sorted (RFC-001 7.2)?

Decided 2026-11-14: Rust uses `BTreeMap<String, T>`, Python a `dict` filled in key order, and TypeScript a plain object with
a null prototype (so `__proto__` and `constructor` are ordinary keys). Key order is never reported, because a map is
unordered (RFC-001 7.2) and a host that reorders keys, such as a JavaScript object moving `"2"` first, changes nothing
the model means. That removed the `order` loss kind of 7.1.

7.9. Which native values can be written as gram (5.6), and how is an unwritable value reported?

Decided 2026-11-14: writable are booleans, strings, integers of any width (an unsigned integer past 64-bit signed
becomes a `big_integer`), finite floats, arrays and string-keyed maps of writable values, and the date, point and UUID
types through known tags (7.10). Anything else (`None`, `null`, `undefined`, `NaN`, an infinity, a non-string map key,
a function) makes the conversion fail: it returns no value and one diagnostic per offending part, each with its
JSON Pointer path, code `unsupported` (or `not-finite` for `NaN` and the infinities). Nothing is skipped, coerced or
defaulted; a consumer that wants null entries dropped filters them before converting.

7.10. How do the known tags map to native temporal, point and UUID types, and how is lost precision (a nanosecond
in a JavaScript `Date`) reported?

Decided 2026-11-14: each package defines the typed values (in Rust `Date`, `Time`, `DateTime`, `Duration`, `Point`, `Uuid` and
`Week`) and these are the mapping, so every known tag converts with no loss and no dependency. Conversions to popular
library types are optional extras and separate from the table: in Rust behind cargo features (`chrono`, `time`, `uuid`),
in Python to `datetime` and `uuid.UUID` where exact, in TypeScript none at first. Any precision a library type cannot
hold (a nanosecond in a JavaScript `Date`) is reported as a `precision` loss.

7.11. How are the mapping tables written down so the documentation and the conformance corpus are generated from
one source (RFC-001 6.7)?

Decided 2026-11-14: the corpus is the source. `conformance/mappings.gram` holds the `MappingCase`s of ADR-001 and is the only
place the tables are written. A small script generates each language's documentation table (`docs/mappings/<language>.md`)
from it, and CI fails when the generated files are stale.

7.12. Closed: operations are not pushed down into a core (RFC-001 7.16, decided: every package is native), so
mapping is the only point at which a value changes representation, and a loss record describes one conversion.

7.13. Can a consumer override a table entry (for example, mapping decimals to its own type)?

Decided 2026-11-14: no. The tables are fixed and the conversions are plain functions. A consumer that wants a different
native type converts to the package's own value, which is always exact, and applies its own function. There is one
behavior per entry, so the corpus covers all of it and parity holds.

7.14. How do the tables evolve when the grammar adds a value form (decimal measurements arrived in
tree-sitter-gram 0.7.0)?

Answered 2026-11-14 by ADR-002 and RFC-003 5.8: a new value form is a new kind, which is a breaking change (ADR-002 2.4);
its `MappingCase`s are added to the corpus first, then each package adds its entry, and a grammar release is reviewed
against the corpus before the pinned version changes.

7.15. In a statically typed language such as Rust, where the package's `Value` is already the dynamic form, what does
a mapping look like?

Decided 2026-11-14 (found by the review of this RFC): the standard conversion traits. `From<i64>`, `From<&str>`,
`From<Vec<T>>` and similar build a `Value`, and `TryFrom<&Value>` extracts `i64`, `f64`, `bool`, `String`, `Vec<T>`
and `BTreeMap<String, T>`. A mismatch fails with a diagnostic (`type-mismatch`, or `out-of-range` for a narrower
integer type such as `i32`). Every Rust entry is exact or unsupported, so Rust never reports a loss, and each table
entry is a trait implementation the corpus can test. serde support may be added later as an optional feature; it is
not the mapping (6.5).

## 8. ADRs

- ADR-001: The Conformance Corpus Is Gram Text (uses the entry classes and loss kinds)
- Related, not yet resolving a question here: ADR-004 (the interpreted tag types are defined per language by these mappings, 7.10) and ADR-005 (the result shape; the path format is 7.2).
