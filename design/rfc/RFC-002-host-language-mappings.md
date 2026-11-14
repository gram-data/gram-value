---
id: RFC-002
title: Host-Language Mappings and Loss Reporting
status: draft
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

The scope and direction are the proposal; the loss kinds, the result shape and each language's table are left
to later decisions.

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

3.5. Specify what happens when a native value with no gram counterpart is written as gram (`null`, `NaN`, an
integer beyond 64 bits, a map with a non-string key).

3.6. Test every mapping against the conformance corpus, a set of language-neutral cases that every language
package runs, holding the expected native value and entry class per language.

## 4. Non-Goals

4.1. Choosing how a value is represented in memory in the core. That is RFC-001's question.

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
(precision of an integer, the integer-versus-decimal distinction, key order, the kind of a measurement's magnitude). Unsupported: the
native language has no counterpart and the entry says whether the conversion fails or produces a wrapper (5.5).

### 5.3 What the tables are expected to record

Candidate entries, from SPIKE-001, that the first tables must settle: a 64-bit or larger integer in TypeScript; the
integer-versus-decimal distinction in TypeScript; a range, a measurement, a big integer, a big decimal and a tagged string in each
language; the key order of a map in each language (the model sorts keys and has no repeated keys, RFC-001 7.2); the temporal, point and UUID types that the known
tags interpret.

### 5.4 Loss as data

Candidate direction: the conversion's result would have the shape RFC-001 proposes for interpretation, the value
together with a list of losses, each saying what was lost and where. The fields of a loss are question 7.2. The
list is accumulated across independent elements, and each language adds idiomatic helpers for composing results.

### 5.5 A lossless escape hatch

Where a native value cannot hold a gram value, a language provides a wrapper that holds the gram value intact, so a
consumer that does not understand a value can still pass it through and write it back unchanged.

### 5.6 Writing native values as gram

Candidate direction: the reverse table would say which native values can be written (integers within 64 bits,
finite floats, strings, booleans, arrays, maps with string keys, and the native temporal, point and UUID types
through known tags) and which fail (`null`, `undefined`, `NaN`, infinities, integers beyond 64 bits, non-string
map keys, functions). The exact lists are question 7.9.

### 5.7 Loss policy belongs to the consumer

Candidate direction: a consumer could fail on any loss, accept and record it, or ignore it. The policy would decide
what to do with the losses already recorded in the result, so the same table serves a validator that fails and a
viewer that tolerates. Where the choice is made is question 7.3.

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

Decided 2026-11-14: the loss kinds are `precision` (digits are lost, as when an integer past 2^53 or a `big_decimal`
becomes a 64-bit float), `kind` (the value's kind changes, as when `1.0` arrives as the integer `1`) and `order` (key
order is not kept, as when a JavaScript object moves integer-like keys first). The set is closed: a further loss kind
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

7.6. Which native types hold a range and a measurement: small record types defined by the package in each
language, or types from a library? Should a range use a language's own range type (Rust's `Range`, Python's
`range`) where one exists, given that its endpoints and inclusivity differ from gram's?

7.7. Closed: the model has no symbol kind (RFC-001 7.5, decided). A symbol in a value position is a string, so no
mapping entry and no loss concerns it; only the writer's choice of a bare word or a quoted string differs.

7.8. Which native map type is used in each language (a JavaScript object or `Map`, a Python `dict`, a Rust sorted
map), and how is key order reported, given that the model's maps are unordered with keys sorted (RFC-001 7.2)?

7.9. Which native values can be written as gram (5.6), and how is an unwritable value reported?

7.10. How do the known tags map to native temporal, point and UUID types, and how is lost precision (a nanosecond
in a JavaScript `Date`) reported?

7.11. How are the mapping tables written down so the documentation and the conformance corpus are generated from
one source (RFC-001 6.7)?

7.12. Closed: operations are not pushed down into a core (RFC-001 7.16, decided: every package is native), so
mapping is the only point at which a value changes representation, and a loss record describes one conversion.

7.13. Can a consumer override a table entry (for example, mapping decimals to its own type)?

7.14. How do the tables evolve when the grammar adds a value form (decimal measurements arrived in
tree-sitter-gram 0.7.0)?

## 8. ADRs

- ADR-001: The Conformance Corpus Is Gram Text (uses the entry classes and loss kinds)
- Related, not yet resolving a question here: ADR-004 (the interpreted tag types are defined per language by these mappings, 7.10) and ADR-005 (the result shape; the path format is 7.2).
