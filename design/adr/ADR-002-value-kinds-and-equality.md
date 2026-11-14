---
id: ADR-002
title: The Eleven Value Kinds, Their Meaning and Equality
status: proposed
rfcs: [RFC-001]
created: 2026-11-14
---

# ADR-002: The Eleven Value Kinds, Their Meaning and Equality

## 1. Context

A gram file puts values on nodes and relationships: integers, decimals, strings, measurements such as `5kg`, ranges such as `1..5`, arrays, maps, and tagged strings such as ``date`2024-01-15` ``. `gram-value` is the library that every gram tool uses to hold these values in memory. Two programs reading the same file must agree on what each value is, so the library has to say which kinds of value exist, what each one means, and when two values are equal.

Existing models disagree because the grammar leaves choices open. SPIKE-001 compared seven of them and found:

- The grammar writes integers of any size, but the models use 64-bit, 53-bit or unbounded integers.
- An octal literal such as `010` is read as ten by one model and refused by another.
- A measurement and a range hold floats in pattern-rs, so `5kg` and `5.0kg` become one value and a range of measurements cannot be held.
- A map loses its repeated keys or its key order, depending on the model.
- A map with a key `type` whose value is `symbol` is mistaken for a symbol after a trip through JSON.

The grammar (tree-sitter-gram 0.7.0) is the authority on what text is a value. It has four quoting styles for strings, a bare word called a symbol that can be a value, and integers written in decimal, hexadecimal (`0x1F`) or octal (`017`). Hexadecimal and octal literals take no sign. A value is a scalar, an array of scalars, or a map whose entries are scalars or arrays.

## 2. Decision

### 2.1 A value records meaning, not spelling

A value records what it means. How it was written (decimal or hexadecimal, which quote, a bare word or a quoted string, entry order in a map, the separator `:` or `::`, which means nothing for a value) and where it was written stay in the parser's syntax tree and never reach the model.

### 2.2 The kinds

The model has exactly these kinds, listed in the order used by 2.6.

| Kind | Holds |
|---|---|
| `integer` | a signed 64-bit integer |
| `big_integer` | an integer outside the 64-bit range, as a sign and decimal digits without leading zeros |
| `decimal` | a 64-bit float |
| `big_decimal` | a decimal that a 64-bit float cannot hold exactly, as a sign, digits, a point and digits |
| `boolean` | true or false |
| `string` | text |
| `tagged` | a tag (a symbol other than `true` and `false`) and a content string |
| `range` | an optional lower end and an optional upper end, each a numeric value or a measurement |
| `measurement` | a magnitude (any numeric value: `integer`, `big_integer`, `decimal` or `big_decimal`) and a unit string |
| `array` | an ordered sequence of values |
| `map` | an unordered set of entries with unique string keys, each a value |

Every value has one kind. An integer that fits in 64 bits is always `integer`. A decimal is `decimal` when its 64-bit float writes back as the same number (the literal, with trailing zeros after the point dropped and at least one digit kept, equals the float's shortest positional spelling: `0.1`, `3.14`, and `1.50` as `1.5`); otherwise it is `big_decimal`. Hexadecimal and octal literals are integers (`0x10` is 16, `010` is 8); `019` is not an octal and is a syntax error in the grammar.

There is no `symbol` kind: a symbol in a value position is a `string`. The grammar uses symbols for identifiers, property names and tags, none of which is a value, and only a tag is stored (as the tag of a `tagged` value, whose text must be a symbol). There is no `null` kind: gram has no null literal, so every value can be written as gram text.

A range is inclusive at each end, and an absent end is unbounded. A range has at least one end, since gram has no text for a range with neither. The model does not check that the two ends are of the same kind; a consumer may. A float is finite: the model rejects `NaN` and the infinities at construction, because gram cannot write them.

The model allows any value inside an array or a map, to match the way hosts hold JSON-like data and to survive a grammar that relaxes its nesting rule. Writing a value the grammar cannot express (an array inside an array) is a reported error (ADR-003).

An element's property record is a `map`; there is no separate record kind.

### 2.3 A repeated key is an error

A `map` has unique keys. A constructor that builds a map from entries and is given a repeated key reports an error (code `duplicate-key`) that names the key; it never keeps the first or the last silently. (`gram lint` in tree-sitter-gram already reports `duplicate-key` for the same input.) Behavior on values gram cannot write (a repeated key, `NaN`, an infinity, an unwritable nesting, an invalid tag or unit) cannot appear in a corpus that must pass `gram lint --strict` (ADR-001), so each package tests it with unit tests derived from the tables in `spec/`, until a construction-case kind is decided.

### 2.4 The set of kinds is closed

Adding a kind is a breaking change, released as such under the project's version policy (RFC-003 5.6), after which every consumer's exhaustive handling of the kinds fails to compile or check until it decides what the new kind means. A new kind lands first as conformance cases (RFC-003 5.8).

### 2.5 Equality and hashing

Two values are equal when they have the same kind and the same content, compared part by part: numbers by their exact digits, strings by code points, tags by tag and content, arrays element by element, maps by their entry sets. For every value gram can write this is the same as having equal canonical text (ADR-003), and the conformance corpus tests it that way. Therefore `1` differs from `1.0`, `5kg` from `5.0kg`, and `-0.0` from `0.0`, and two maps with the same entries are equal whatever order the entries were written in. Equality is defined for values gram cannot write too (an array inside an array). Hashing agrees with equality. Other equivalence functions (numeric equivalence across kinds, for example) may be added later as separate named functions; none replaces equality or hashing.

### 2.6 Total order

A total order is defined as a convenience for sorting and for sorted containers. Values compare first by kind, in the order of the table in 2.2, and then within a kind: `integer`, `big_integer`, `decimal` and `big_decimal` numerically; `boolean` with false before true; `string` by its UTF-8 bytes; `tagged` by tag, then content; `range` by lower end, then upper end, an absent end before a present one; `measurement` by magnitude, then unit; `array` lexicographically; `map` by its entries sorted by key, compared as pairs. The order is consistent with equality: it reports equal only for equal values, so `-0.0` sorts before `0.0`, and a measurement's magnitudes compare by this same order. A range's absent end sorts before any present end, so an unbounded lower end comes first and an unbounded upper end also comes first among ranges with the same lower end. Because kinds compare first, every `big_integer` sorts after every `integer`, and numeric order across kinds is not offered here. The order means nothing about the values themselves.

## 3. Alternatives Considered

3.1. **Keep how a value was written (a hexadecimal flag, the quote used).** Rejected: formatters and linters work from the syntax tree, and spelling changes meaning only where a number's kind changes (`5.0kg`, `1.0`, octal), which 2.2 captures as kinds.

3.2. **One `number` kind covering integers and decimals, with a flag.** Rejected: `1` and `1.0` differ in gram and SPIKE-001 found a host decoding `1.0` as `1`; separate kinds keep them apart.

3.3. **Unbounded integers only.** Rejected: most languages' common integer is 64-bit, so a consumer would convert every integer. A 64-bit `integer` with a separate overflow kind keeps the common case plain, at the price of two kinds a consumer must handle.

3.4. **An exact decimal for every decimal.** Rejected: most languages have no exact decimal type, and most decimals in files are ordinary floats. `big_decimal` is used only when a float would change the number.

3.5. **A `symbol` kind.** Rejected: the symbol of a value position is spelling of a string (ADR-003 picks the bare form when it can), and a kind would need its own row in every mapping table.

3.6. **A `null` kind.** Rejected: gram cannot write it. Hosts' nulls are unsupported entries in the mapping tables (RFC-002).

3.7. **Ordered maps with repeated keys kept.** Rejected: key order is spelling, repeated keys are a lint error in the grammar's own tool, and unordered equal maps make equality by canonical text possible.

3.8. **Numeric equality across kinds (`1` equal to `1.0`).** Rejected as the default: it makes equality depend on which pairs of kinds convert, and breaks the rule that equal values write the same text. It remains available as a named function.

3.9. **An open set of kinds that consumers extend.** Rejected: a closed set lets every consumer handle all kinds exhaustively. Extension happens at the tag layer (ADR-004).

## 4. Consequences

- A consumer's code over the model handles eleven kinds, two of which (`big_integer`, `big_decimal`) are rare. A consumer that only supports some kinds rejects the rest with its own errors; the model names no subsets.
- Every package decides one form per value, so conversion code compares by kind and never inspects both `integer` and `big_integer` for one number.
- A decimal's kind depends on a round-trip check (a literal beyond the float range, such as one with 400 digits, is a `big_decimal`), which every language package implements; the conformance corpus needs cases at the boundary (a decimal such as `0.1`, one with more digits than a float holds, a very large positive integer).
- Spelling-sensitive tools (formatters) cannot use the model alone; they read the syntax tree.
- Equality is defined on content and agrees with canonical text for every writable value, so the corpus can test it through ADR-003's text.
- Each package exposes a value's kind, whether it is a scalar, and whether an array holds values of one kind, so a consumer can implement its own subset (RFC-001 5.2); the exact functions are in `spec/`.
- Follow-up work: write these rules in `spec/`, including the lowering rules that turn grammar literals into values (escape decoding, the fenced string form, the fenced tagged form); add `ValueCase`s for every kind and for each boundary named above (ADR-001); name the kinds in the conformance vocabulary (done in ADR-001 2.5).

## 5. Related

- RFC-001 (resolves questions 7.1 to 7.5, 7.9, 7.12 to 7.15).
- ADR-003 (canonical text) and ADR-004 (tags) complete the model.
- SPIKE-001 (what existing models lose); tree-sitter-gram 0.7.0 (the grammar).
