---
id: SPIKE-001
title: Inventory of Existing Gram Value Models
status: done
rfcs: [RFC-001]
created: 2026-11-14
---

# SPIKE-001: Inventory of Existing Gram Value Models

## 1. Question

RFC-001 sections 2 and 7 (questions 7.1 to 7.5 and 7.11 in particular): for each value kind the tree-sitter-gram
grammar can write, what do the existing implementations represent, what do they lose, and where do they
disagree? The answer says which representation choices RFC-001's open questions must settle, and what a shared
model has to preserve to serve both a strict consumer (relate-gram) and a permissive one (pattern-rs).

## 2. Method

Read the value type of each implementation and tabulate its variants, numeric types, map type, and the text
writing it has: the grammar (the authority), pattern-core and gram-codec in pattern-rs, pattern-hs (Haskell),
the TypeScript and Python packages of pattern-rs, and relate-gram. Then run one set of gram inputs, one value
each, through the implementations that can be run here, and record what each produces and writes back:
`scripts/rs-probe` runs pattern-rs's two parse paths (the hand-written one and the tree-sitter one lowered to
subjects) and its serializer, and `scripts/relate-gram-probe.sh` runs relate-gram's `normalize`. Haskell has no
toolchain on this machine, so pattern-hs is read, not run; the TypeScript and Python packages are read.
Outputs are `rs-probe-output.txt` and `relate-gram-output.txt`.

## 3. Time-box

One day.

## 4. Notes

2026-11-14
- pattern-rs's tree-sitter path pins the grammar submodule at v0.6.1, so its behavior is that of 0.6.1, not 0.7.0 (empty arrays and decimal measurements arrived in 0.7.0). Results below say "the pinned grammar" where that matters.
- pattern-rs's `gram_codec` exports a type called `Value` (7 variants) and parses into `pattern_core` subjects whose properties use a different `Value` (10 variants). Only the second is used for parsed values.
- Run with: `cd scripts/rs-probe && cargo run --release > ../../rs-probe-output.txt`, and `scripts/relate-gram-probe.sh <relate-gram binary> > relate-gram-output.txt`.

## 5. Findings

### 5.1 What each implementation represents

The grammar (tree-sitter-gram) writes these value kinds: integer (decimal, hexadecimal and octal spellings), decimal, measurement, boolean, symbol, string (four quoting styles), tagged string (backtick or fenced), range (closed, or open at either end, with any numeric literal, measurements included, as an end), array of scalars (a scalar includes a range, measurement or tagged string), and a map whose entries are scalars or arrays.

| Kind | pattern-core (Rust) | gram-codec `Value` (Rust) | pattern-hs | TypeScript | Python | relate-gram |
|---|---|---|---|---|---|---|
| integer | `VInteger(i64)` | `Integer(i64)` | `VInteger Integer` (unbounded) | `IntVal` (a JS number, exact to 2^53) | `IntVal` (unbounded `int`) | `Int(i64)`; larger is an error |
| decimal | `VDecimal(f64)` | `Decimal(f64)` | `VDecimal Double` | `FloatVal` (number) | `FloatVal` | `Float(f64)` |
| hex, octal | read as integers | not parsed here | not examined | not applicable | not applicable | hex as integer; octal rejected |
| boolean | `VBoolean` | `Boolean` | `VBoolean` | `BoolVal` | `BoolVal` | `Bool` |
| symbol | `VSymbol(String)` | none | `VSymbol String` | `SymbolVal` | `SymbolVal` | read as a string |
| string | `VString` | `String` | `VString String` | `StringVal` | `StringVal` | `Str` |
| tagged string | `VTaggedString{tag,content}` | `TaggedString` | `VTaggedString String String` | `TaggedStringVal` | `TaggedStringVal` | typed value for 8 tags, else a string with the tag dropped |
| range | `RangeValue{Option<f64>,Option<f64>}` | `Range{i64,i64}` | `RangeValue (Maybe Double) (Maybe Double)` | `RangeVal` (optional numbers) | `RangeVal` (optional floats) | rejected |
| measurement | `VMeasurement{unit,f64}` | none | `VMeasurement String Double` | `MeasurementVal{unit,number}` | `MeasurementVal` | rejected |
| array | `VArray(Vec<Value>)`, any kinds | `Array(Vec)` | `VArray [Value]` | `ArrayVal` | `ArrayVal` | list of one kind, no ranges |
| map | `VMap(HashMap<String,Value>)` | none | `VMap (Map String Value)` (sorted, unique keys) | `MapVal` (a `Record`) | `MapVal` (a `dict`) | rejected |
| null | none | none | none | `NullVal` | `NullVal` | none |

pattern-core, pattern-hs, the TypeScript package and the Python package share one ten-kind shape (the last two add a null that gram has no literal for). The `gram_codec::Value` is a narrower, older enum that parsing no longer uses.

### 5.2 What the implementations did with the same input

From `rs-probe-output.txt` and `relate-gram-output.txt` (pattern-rs on its pinned grammar, v0.6.1; "nom" is its hand-written parser, "cst" its tree-sitter path):

| Input value | pattern-rs | relate-gram |
|---|---|---|
| `9223372036854775808` | nom: syntax error with a misleading message; **cst: the property is silently dropped** | error `value.integer-range` at the position |
| `0x1F` | integer 31, written back as `31` | integer 31 |
| `010` | integer 10 (read as decimal), both paths | rejected as ambiguous |
| `5kg` and `5.0kg` | one value (`kg`, 5.0), both written `5kg` | rejected |
| `3.5kg` | nom parses it, then the serializer refuses ("take an integer magnitude"); cst: syntax error | rejected |
| `1..5` | range of two numbers | rejected |
| `1...`, `...5` | parse, but the serializer refuses to write them; `Display` of the type writes `1..` and `..5` (two dots, the grammar needs three) | rejected |
| `1.5..2.5` | nom: syntax error; **cst: the property is silently dropped** | rejected |
| `1.5m..2m` | nom and cst: error | rejected |
| `0x1..0x10` | range 1 to 16; written `1..16` | rejected |
| `active` (symbol) | symbol, **written back as the string `"active"`** | a string |
| single-quoted, backtick and fenced strings | one string kind, written double-quoted | same |
| ```` ```md\nbody``` ```` (tagged, fenced) | **a plain string; the tag is dropped** (both paths) | tag dropped with a warning |
| ``md`# T` `` (tagged, backtick) | tag and content kept | tag dropped with a warning |
| ``date`2024-01-15` `` | tag and content kept, no interpretation | a DATE value |
| `[1, "a", true]` | accepted | error: one kind per array |
| `[1..2, 5kg]` | accepted | error: no range |
| `{b: 1, a: 2}` as a value | a map; key order not kept, written sorted | rejected |
| `{a: 1, a: 2}` as a value | **last key wins, silently** | rejected |
| `(n {k: 1, k: 2})` | **last key wins, silently** | error `key.duplicate` |
| record `{z: 1, y: 2, x: 3, w: 4}` | written sorted (`w x y z`) | written sorted |
| `[]` | accepted | accepted |

### 5.3 Findings

1. **One shape, six copies.** The ten-kind model of pattern-core is duplicated faithfully in pattern-hs, TypeScript and Python (with an added null), plus an orphan in gram-codec and a strict subset in relate-gram. The shape is consistent; the numeric types are where they diverge.
2. **Integers have no common width.** Unbounded in Haskell and Python, 64 bits in Rust and relate-gram, 53 bits in TypeScript. Gram text has no limit, so an integer beyond 64 bits is valid gram. pattern-rs's tree-sitter path drops such a property without a word; nom fails with an unrelated syntax error; relate-gram reports an error with a position.
3. **Octal has no agreed meaning.** pattern-rs reads `010` as ten; relate-gram refuses to guess. The grammar names the token an octal.
4. **Spellings are mostly irrelevant, except where a number's kind changes.** Hexadecimal, the four string quotings and tag spellings lose nothing that matters when written canonically. `5.0kg` and `5kg`, and `1.5..2.5`, show where the numeric type decides: a measurement or range endpoint held as an `f64` cannot be an integer, decimal, hexadecimal or measurement, so the model cannot say what was written.
5. **Ranges and measurements are modeled narrower than the grammar.** Endpoints are `f64`, so ranges of measurements and decimal measurements cannot be represented, and open-ended ranges cannot be written back.
6. **Silent drops exist.** pattern-rs's tree-sitter path drops a property whose value it cannot lower (an integer past 64 bits, a decimal range) and parsing still reports success. This breaks the rule that nothing is lost without a report.
7. **Symbols collapse on write.** The distinction exists in memory in all the Rust, Haskell, TypeScript and Python models, and the pattern-rs serializer writes a symbol as a quoted string.
8. **Maps and records lose order and repeated keys.** `HashMap`, a JavaScript `Record` and a Python `dict` keep no order the grammar's text had, and a repeated key keeps the last value without an error. Haskell's `Data.Map` is sorted. Output order is by key in every writer.
9. **Tagged strings need an interpretation layer that is separate.** pattern-rs keeps `date` as an uninterpreted tag; relate-gram interprets eight tags. A fenced tagged string loses its tag in pattern-rs.
10. **Null is added in TypeScript and Python.** Gram has no null.
11. **The host boundary is JSON with ad-hoc tags.** The TypeScript and Python values are decoded from JSON produced by the Rust gram-codec, using objects `{type: "symbol"|"range"|"tagged"|"measurement", ...}`. A gram map that has a key `type` whose value is one of those words is decoded as that kind instead of as a map, and integer-valued floats become `IntVal` in TypeScript (`Number.isInteger`). Each value crosses the boundary by serialization.
12. **Writing is not specified.** Each writer decides quoting and spelling, and the pattern-rs serializer errors on some values (open ranges, decimal measurements) instead of writing them.
13. **The grammar version matters.** pattern-rs's cst path rejects `5.0kg` and `3.5kg` because it pins v0.6.1; tree-sitter-gram 0.7.0 accepts them.

### 5.4 Not examined

The Haskell parser and serializer (no toolchain here), running the TypeScript and Python packages, the `Pattern` level (element and subject values beyond property values), and equality and ordering semantics.

## 6. Conclusion

A shared model has to keep more than the ten-kind shape the existing copies agree on, and a few of its answers are now evidenced:

- **Numbers carry their kind.** The model must distinguish integer, decimal, hexadecimal-or-octal spellings that change a value's meaning, and measurement, and ranges and measurements must hold numeric literals, not `f64`s (7.4). An integer beyond 64 bits has to be representable, or its overflow has to be an explicit, reported outcome; it can never be dropped (7.3, finding 2 and 6).
- **Octal needs an explicit answer** from RFC-001 or the grammar owner, because the one implementation that reads it reads it as decimal, and the other refuses (7.3).
- **Spellings can stay out of the model** (finding 4): hexadecimal, string quoting and tag spelling lose nothing that matters. That supports the leaning toward a model that records what a value means (7.1), provided the numeric kinds above are distinct.
- **Maps need order and repeated-key policy** (7.2): an ordered representation, with duplicate keys detected by the parser layer.
- **Symbols should stay distinct from strings** in the model (7.5), with the canonical text writer deciding how each is written.
- **No null.** TypeScript and Python add one, and gram has none; the mapping tables should say what a host null means (7.8).
- **The host boundary needs a design.** Today TypeScript and Python reach Rust values through JSON with ad-hoc tags, which is ambiguous for a map with a `type` key and costs a serialization per value (finding 11); the implementation-strategy spike (7.11) should measure that cost against native implementations.

Text for RFC-001 section 2: "SPIKE-001 compared seven representations. The ten-kind shape is shared; the numeric types are not (unbounded, 64-bit and 53-bit integers), ranges and measurements are held as floats, open ranges cannot be written, a parser path drops properties it cannot lower, and a map with a key `type` is misread at the TypeScript and Python boundary."
