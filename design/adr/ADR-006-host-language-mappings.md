---
id: ADR-006
title: Host-Language Mappings, Loss Reporting and the Rust Conversions
status: accepted
rfcs: [RFC-002]
created: 2026-11-14
---

# ADR-006: Host-Language Mappings, Loss Reporting and the Rust Conversions

## 1. Context

A program that reads gram wants its values as the native values of its own language: a Rust `i64`, a JavaScript `number`, a Python `dict`. The languages cannot hold every gram value alike. A JavaScript `number` keeps exact integers only up to 2^53 and has no separate integer-valued decimal, so `1.0` arrives as `1`. Rust has no exact decimal in its standard library. No mainstream language has a measurement such as `5kg`, and the ranges that exist (Rust's `RangeInclusive`, Python's `range`) differ from gram's, whose ends may be decimals, measurements or absent. SPIKE-001 found tools that handled these cases in different ways, and one that failed an entire file because a single integer was too large for a JavaScript number.

`gram-value` therefore defines, for each first-tier language, which native value each kind maps to, in both directions, and what a conversion reports when it cannot keep everything. The value kinds are fixed (ADR-002), results carry diagnostics in one shape (ADR-005), and the conformance corpus holds the expectations (ADR-001). This ADR records the rules the mapping decisions of RFC-002 produced; each package implements them in its own language.

## 2. Decision

### 2.1 Entry classes

Every table entry, one per value kind (and per known tag) per language and direction, has a class:

- **exact**: converting to the native value and back gives an equal value (ADR-002 2.5);
- **lossy**: a native value results, but converting back gives a different value; the entry names the loss kinds (2.2);
- **unsupported**: the language has no native value for it; the conversion fails with a diagnostic, and the package's own value type remains available (2.8).

### 2.2 Loss kinds

There are two, and the set is closed (a new one needs an ADR):

- `precision`: digits are lost (a `big_decimal` converted to a 64-bit float);
- `kind`: the value's kind changes (`1.0` arrives as the integer-valued `number` `1`, so converting back yields the integer `1`).

Key order is not a loss, because a map is unordered (ADR-002). A value a host cannot hold at all is not a loss; its entry is unsupported.

### 2.3 Losses are data; policy is the consumer's

A conversion to a native value returns the value and a list of losses, using the result shape of ADR-005. Each loss is a diagnostic whose `code` is the loss kind, and whose `path` is a JSON Pointer (RFC 6901) when written as text: `/items/0/name`, with `~0` for `~` and `~1` for `/` in a key, and the empty string for the whole value (the list of keys and indexes is the contract). Losses from independent parts accumulate. A conversion never fails because of a loss and has no policy argument. Each package offers a helper that turns any loss into a failure, and a consumer that ignores losses does not read the list.

### 2.4 Native types

| Kind | Rust | TypeScript | Python |
|---|---|---|---|
| `integer` | `i64` (exact) | `number` if within ±(2^53 - 1), else `bigint` (exact) | `int` (exact) |
| `big_integer` | any std integer type that holds it (`u64`, `i128`, ...), else `out-of-range` | `bigint` (exact) | `int` (exact) |
| `decimal` | `f64` (exact) | `number` (`kind` loss when the float is integral) | `float` (exact) |
| `big_decimal` | unsupported: no std type (the package's `BigDecimal` remains) | `number`, `precision` loss | `decimal.Decimal` (exact) |
| `boolean` | `bool` | `boolean` | `bool` |
| `string` | `String` | `string` | `str` |
| `tagged` | the package's `Tagged` | the package's `Tagged` | the package's `Tagged` |
| `range` | the package's `Range` | the package's `Range` | the package's `Range` |
| `measurement` | the package's `Measurement` | the package's `Measurement` | the package's `Measurement` |
| `array` | `Vec<T>` | `Array` | `list` |
| `map` | `BTreeMap<String, T>` | an object with a null prototype | `dict`, filled in key order |

A decimal is always a float, and the table records the one case where a host with no exact type cannot hold a `big_decimal`. Package-defined `Range` and `Measurement` types hold the value exactly: a range has an optional lower end and an optional upper end, inclusive, each a native number or (for a range) a measurement; a measurement has a magnitude (a native number) and a unit. A language's own range type is never used. For a `bigint` in TypeScript and an `int` in Python, the boundary between `integer` and `big_integer` is not visible to the consumer, and the reverse conversion gives the right kind from the magnitude.

### 2.5 Known tags

The native value of a known tag is the package's typed value for it: in Rust `Date`, `Time`, `DateTime`, `Duration`, `Point`, `Uuid` and `Week`, with equivalents in the other packages. Every known tag converts with no dependency, and the typed value keeps its meaning. Converting a typed value back writes its normal content string (spec/tags.md section 4), so a non-normal spelling such as ``time`17:35` `` or an upper-case UUID comes back as a different tagged value; a consumer that needs the exact string keeps the tagged value. Conversions to popular library types (Rust's `chrono`, `time` and `uuid`; Python's `datetime` and `uuid.UUID`) are optional extras, separate from the table (in Rust behind cargo features), and report `precision` when a library type cannot hold the value (a nanosecond in a JavaScript `Date`).

### 2.6 Writing native values as gram

A native value converts to a gram value when it is a boolean, a string, an integer of any width (an unsigned integer past the signed 64-bit range becomes a `big_integer`), a finite float, an array or a string-keyed map of such values, a package value (`Tagged`, `Range`, `Measurement`, a typed tag value), or a Python `Decimal`. Anything else makes the conversion fail: it returns no value and one diagnostic per offending part, each with its path, with the code `unsupported` (for `null`, `None`, `undefined`, a non-string map key, a function) or `not-finite` (for `NaN` and the infinities). Nothing is skipped, coerced or defaulted. A consumer that wants null entries dropped removes them first. A number in TypeScript becomes an `integer` when it is integral and a `decimal` otherwise; a Python `bool` is checked before `int`.

### 2.7 Rust conversions use the standard traits

`From<i64>` (and the other integer types, `u64` and `i128` becoming a `big_integer` when needed), `From<f64>` through a checked constructor, `From<bool>`, `From<&str>` and `From<String>`, `From<Vec<T>>` and `From<BTreeMap<String, T>>` for `T` convertible, and `From` for the package types build a `Value`. `TryFrom<&Value>` extracts `i64`, narrower integer types, `f64`, `bool`, `String`, `Vec<T>`, `BTreeMap<String, T>` and the package types. The error is a `Diagnostics` value that holds every problem found, so an array reports all its failing elements with their paths. A value of the wrong kind fails with `type-mismatch`; a number that does not fit the requested integer type fails with `out-of-range`; a `big_integer` converts to any std integer type that holds it and fails with `out-of-range` otherwise, and a `big_decimal` fails with `out-of-range` for `f64`, since the std library has no exact decimal. Every Rust entry is exact or unsupported, so Rust never reports a loss. A serde integration may be added as an optional feature; it is not the mapping.

### 2.8 The package's own value is the escape hatch

Where a native type cannot hold a value, a consumer keeps the package's own `Value` (or a part of it) and still writes it back unchanged. This is the lossless route for anything the tables mark unsupported or lossy.

### 2.9 The tables live in the corpus

`conformance/mappings.gram` holds a `MappingCase` per entry in the shape of ADR-001 (class, loss kinds, native type name and round-trip text per language), and is the only place the tables are written. A small script generates each language's documentation table, `docs/mappings/<language>.md`, from it, and CI fails when a generated file is stale. A consumer cannot override a table entry; it converts to the package's own value and applies its own function.

### 2.10 Diagnostic codes

The mapping adds these codes to the vocabulary: `precision` and `kind` (losses), `type-mismatch`, `out-of-range` (also used by ADR-004 for tags), `unsupported` and `not-finite`.

### 2.11 Go (added 2026-11-14)

Go, a next-tier language, follows the same rules. Its native types: `integer` becomes `int64`; `big_integer` a `*big.Int`; `decimal` a `float64`; `big_decimal` is unsupported (the standard library has `math/big` but no decimal, and a `*big.Rat` is an unusual way to hold one, so the package's own `BigDecimalValue` stays the value, as in Rust); `boolean` a `bool`; `string` a `string`; `tagged`, `range` and `measurement` the package's `Tagged`, `Range` and `Measurement`; `array` a `[]any`; `map` a `map[string]any` (Go maps are unordered, as the model's are); the known tags the package's typed values (`Date`, `Time`, `DateTime`, `Duration`, `Point`, `UUID`, `Week`). Every Go entry is exact or unsupported, so Go never reports a loss, and `ToNative` returns `(any, error)` where a failure is a `Diagnostics` value holding every problem with its path. Writing accepts `bool`, `string`, every integer type (a `uint64` past the signed range becomes a `big_integer`), `*big.Int`, finite `float64`, the package types, and slices, arrays and string-keyed maps of those; `nil`, `NaN`, the infinities, `float32` (convert it explicitly), a non-string key, a function and a struct fail with every problem listed.

### 2.12 Swift (added 2026-11-14)

Swift follows the same rules. Its native types: `integer` becomes `Int64`; `decimal` a `Double`; `big_integer` and `big_decimal` are unsupported (the standard library has no big integer, and Foundation's `Decimal` holds 38 digits and would round a `big_decimal`, so the package's own `Value` stays the value, as in Rust and Go); `boolean` a `Bool`; `string` a `String`; `tagged`, `range` and `measurement` the package's `GramTagged`, `GramRange` and `GramMeasurement` (prefixed so that `Range` does not shadow Swift's); `array` an `[Any]`; `map` a `[String: Any]`; the known tags the package's typed values (`GramDate`, `GramTime`, `GramDateTime`, `GramDuration`, `GramPoint`, `GramUUID`, `GramWeek`). Every Swift entry is exact or unsupported, so Swift never reports a loss; `toNative()` throws a `Diagnostics` holding every problem with its path. Because Swift compares `String`s by canonical equivalence while gram compares them by code points, two map keys that differ only in normalization cannot be held apart by a Swift dictionary: `toNative()` fails with `unsupported` for such a map instead of dropping an entry. Writing accepts `Bool`, `String`, every integer type (a `UInt64` past the signed range becomes a `big_integer`), finite `Double`, the package types, and arrays and string-keyed dictionaries of those; `nil`, `NaN`, the infinities, `Float` (convert it explicitly), a non-string key, a closure and an arbitrary object fail with every problem listed.

## 3. Alternatives Considered

3.1. **Fail on any loss.** Rejected: a viewer wants the value and what changed, and a validator wants every loss, not the first.

3.2. **A policy argument on each conversion (fail, record, ignore).** Rejected: three behaviors per entry to test in three languages, for what a one-line helper provides.

3.3. **A JavaScript `number` for every integer, with a `precision` loss.** Rejected: the value in the file would silently differ unless the loss list is read. The `number`-when-safe, `bigint`-otherwise choice loses nothing and keeps the common case plain.

3.4. **An exact decimal for every decimal.** Rejected: an unfamiliar type for the common case in Python, and a decimal dependency in the Rust crate.

3.5. **Generic containers for ranges and measurements (`{lower, upper}`).** Rejected: a map with those keys cannot be told from a range, the ambiguity that pattern-rs's JSON boundary had.

3.6. **A language's own range type where one fits.** Rejected: two outcomes for one kind, and Python's `range` cannot hold an upper end near 2^63.

3.7. **Library types for the known tags in the main table.** Rejected: it adds a dependency to Rust, makes JavaScript's `Date` lossy, and has no home for a duration that has months.

3.8. **Skip null entries when writing native values as gram.** Rejected: it silently removes data and needs a loss kind the closed set does not have.

3.9. **A dynamic `Native` enum in Rust.** Rejected: it would repeat `Value` variant for variant; the standard traits are the Rust idiom.

3.10. **serde as the mapping.** Rejected as the standard: its frameworks differ per language, so parity could not be tested. It remains possible as an optional extra.

3.11. **Per-kind overrides supplied by a consumer.** Rejected: every override combination is behavior the corpus cannot cover, and a consumer can compose over the package's own value.

3.12. **Hand-written documentation and cases.** Rejected: the two can drift, which is the failure the corpus exists to prevent.

## 4. Consequences

- A consumer gets one rule for losses in every language (recorded, never fatal) and a helper for strictness. A loader that must report exactly what a file says uses the helper and the unsupported failures.
- Rust has no native value for a `big_decimal` or for an integer beyond 128 bits, so a consumer reading very large numbers keeps `Value`. This is stated in the table, not hidden.
- Converting a typed tag value back normalizes its content, so the round trip of a tagged value is exact only for content already in normal form; the corpus cases use normal forms.
- TypeScript consumers handle `number` and `bigint`, and a whole decimal like `1.0` reports a `kind` loss that the consumer may ignore.
- Each language defines `Range`, `Measurement` and `Tagged` types, and a consumer who wants a language range converts it.
- The mapping needs three `MappingCase` kinds of expectation per entry in the corpus; adding a language adds a property to every case (ADR-001).
- The documentation generator is new work and a CI check.
- A TypeScript and a Python package must each implement the table above; only Rust is built now.
- Follow-up work: add `conformance/mappings.gram` and the loss and diagnostic names to the vocabulary; implement the Rust conversions and a `Diagnostics` error type, and a runner for `MappingCase`; write the documentation generator; then the TypeScript and Python packages. The text form of a path in the corpus is the JSON Pointer of 2.3.

## 5. Related

- RFC-002 (resolves questions 7.1 to 7.6 and 7.8 to 7.15; 7.7, 7.12 and 7.14 are closed or answered by other decisions).
- ADR-001 (the `MappingCase` shape), ADR-002 (the kinds and equality), ADR-004 (the typed tag values), ADR-005 (the result shape).
- SPIKE-001 (what the existing models lose), SPIKE-002 (native packages).
