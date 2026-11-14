---
id: RFC-002
title: Host-Language Mappings and Loss Reporting
status: draft
created: 2026-11-14
adrs: []
---

# RFC-002: Host-Language Mappings and Loss Reporting

## 1. Summary

A gram value is a value in a notation: an integer of any width, a decimal, a measurement such as `5kg`, a range
such as `1..5`, a symbol, a tagged string, an array or a map. A program wants these as the native values of its
own language, a JavaScript number or a Python `dict` or a Rust `i64`, and the languages disagree about what they can
hold. A JavaScript number keeps 53 bits of an integer, no mainstream language has a measurement or a range as a
built-in value, and an integer-valued decimal such as `1.0` is the same number as `1` in JavaScript. Some
conversions are exact and some silently change what the file said.

The proposal is a mapping for each language: a documented table from every gram value to a native value, in both
directions, in which every entry is marked exact, lossy (with the loss named) or unsupported, and in which a
conversion reports the loss it incurred as data the consumer can inspect. A consumer chooses its own policy for a
loss: fail, accept and record, or ignore. The same mapping tables are checked against one language-neutral
conformance corpus, so that two languages agree about what is lost.

The scope and direction are the proposal; the loss categories, the result shape and each language's table are left
to later decisions.

## 2. Problem / Motivation

**The same gram text becomes different values in different languages, and nothing says so.** SPIKE-001 compared
the value types of pattern-rs (a Rust library of pattern and graph structures built on gram), pattern-hs (its
Haskell reference), and the TypeScript and Python packages of pattern-rs. Integers are unbounded in Haskell and
Python, 64-bit in Rust, and a JavaScript number in TypeScript, which holds exact integers only up to 2^53. A file
with `{id: 9007199254740993}` reaches TypeScript as `9007199254740992` with no warning.

**A kind can change in transit.** The TypeScript package decodes a number from JSON as an integer whenever
`Number.isInteger` is true, so the decimal `1.0` in a gram file arrives as the integer `1`, because JavaScript has
one number type and the JSON text no longer distinguishes them.

**The host boundary is ambiguous.** The TypeScript and Python packages receive values from Rust as JSON, with
ad-hoc tagged objects (`{type: "symbol", ...}`) for the kinds JSON lacks. A gram map that has a key `type` whose
value is `symbol`, `range`, `tagged` or `measurement` is decoded as that kind instead of as a map.

**Kinds with no native counterpart are modeled in different ways, each unstated.** A range, a measurement, a
symbol and a tagged string have no built-in value in Rust, TypeScript or Python. pattern-rs holds a range as two
optional floats and a measurement as a unit and a float, so a range of measurements cannot be represented and
`5kg` and `5.0kg` are one value. relate-gram, which loads gram into the Relate database, reads a symbol as a
string and rejects the other three.

**Containers differ too.** A JavaScript object reorders keys that look like integers (`{b: 1, "2": 1}` iterates
`"2"` first), a Python `dict` keeps insertion order, and a Rust `HashMap` keeps none.

**Consumers cannot tell what they lost.** Each tool maps values its own way and returns the result with no record
of what was changed, so a tool that must report exactly what a file says (a validator, a loader, an exporter)
cannot use the mapping to decide whether to proceed.

## 3. Goals

3.1. For each first-tier language (Rust, TypeScript, Python), document the native value that each gram value kind
maps to, and that each known tag (RFC-001's interpretation layer) maps to, in both directions.

3.2. Mark every entry of every table exact (converting back gives the same gram value), lossy (stating what
information is lost) or unsupported (no native counterpart, stating what happens).

3.3. Make loss detectable: a conversion returns the converted value together with the losses it incurred, each
naming the kind of loss and where in the value it occurred, accumulated across the elements of an array or map
instead of stopping at the first.

3.4. Let a consumer choose what a loss means: fail the conversion, accept and record it, or ignore it, without
changing the table.

3.5. Specify what happens when a native value with no gram counterpart is written as gram (`null`, `NaN`, an
integer beyond 64 bits, a map with a non-string key).

3.6. Test every mapping against the conformance corpus, which holds the expected native value and loss class per
language.

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
representation, the loss class, and the named loss if there is one.

### 5.2 Three loss classes

Exact: the round trip returns the same gram value. Lossy: information is lost and the entry names it (precision of
an integer, the integer-versus-decimal distinction, key order, the spelling of a measurement). Unsupported: the
native language has no counterpart and the entry says whether the conversion fails or produces a wrapper (5.5).

### 5.3 What the tables are expected to record

Candidate entries, from SPIKE-001, that the first tables must settle: a 64-bit or larger integer in TypeScript; the
integer-versus-decimal distinction in TypeScript; a range, a measurement, a symbol and a tagged string in each
language; the order and repeated keys of a map in each language; the temporal, point and UUID types that the known
tags interpret.

### 5.4 Loss as data

The conversion's result has the shape RFC-001 proposes for interpretation: the value together with a list of
losses, each a record of the kind of loss, the path to the part of the value, and a short description. The list is
accumulated across independent elements. Each language adds idiomatic helpers for composing results.

### 5.5 A lossless escape hatch

Where a native value cannot hold a gram value, a language provides a wrapper that holds the gram value intact, so a
consumer that does not understand a value can still pass it through and write it back unchanged.

### 5.6 Writing native values as gram

The reverse table says which native values can be written (integers within 64 bits, finite floats, strings,
booleans, arrays, maps with string keys, and the native temporal, point and UUID types through known tags), and
which fail (`null`, `undefined`, `NaN`, infinities, integers beyond 64 bits, non-string map keys, functions).

### 5.7 Loss policy belongs to the consumer

A consumer can fail on any loss, accept and record it, or ignore it, per conversion or per consumer. The policy
decides what to do with the losses already recorded in the result, so the same table serves a validator that fails
and a viewer that tolerates.

### 5.8 Parity through the conformance corpus

The corpus holds, for each case, the expected native value and loss class for every first-tier language. A
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

## 7. Open Questions

7.1. What are the loss categories (precision, kind, order, spelling, and others), and is the set closed?

7.2. What are the fields of a loss record, and how is the path to a part of a value written the same way in every
language?

7.3. Where is the loss policy chosen (per call, per consumer, per language), and what is the default?

7.4. In TypeScript, how is an integer beyond 2^53 represented: a number with a recorded loss, a `BigInt` always, or
a number when the value is safe and a `BigInt` otherwise?

7.5. In each language, is a gram decimal a float or an exact decimal type where one exists (Python `Decimal`, no
JavaScript equivalent)? This follows RFC-001 7.4.

7.6. Which native types hold a range and a measurement: small record types defined by the package in each
language, or types from a library?

7.7. Is a symbol a native string with a recorded loss, or a distinct type?

7.8. Which native map type is used in each language (a JavaScript object or `Map`, a Python `dict`, a Rust ordered
map), and how are key order and repeated keys reported?

7.9. Which native values can be written as gram (5.6), and how is an unwritable value reported?

7.10. How do the known tags map to native temporal, point and UUID types, and how is lost precision (a nanosecond
in a JavaScript `Date`) reported?

7.11. How are the mapping tables written down so the documentation and the conformance corpus are generated from
one source (RFC-001 6.7)?

7.12. If operations are pushed down into the core (RFC-001 7.16), is mapping the only point at which values cross
the boundary, and does the loss record then describe the whole pipeline?

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
