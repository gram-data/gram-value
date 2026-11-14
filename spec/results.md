# Results and diagnostics

An operation that can find a problem (interpreting a tag, constructing a value, converting to a host value, writing
text) returns a result with the same logical shape in every language.

## 1. Shape

A result is one of:

- a value and no diagnostics;
- a value and one or more diagnostics (used when a conversion succeeds with a recorded loss; the diagnostics are then losses);
- no value and one or more diagnostics.

A diagnostic has three fields:

| Field | Content |
|---|---|
| `code` | a lower-case word with hyphens, from the vocabulary (`conformance/vocabulary.gram`) |
| `message` | text for a person, not compared by the corpus |
| `path` | the part of the value concerned: a list of map keys (strings) and array indexes (integers), empty for the whole value |

Operations over several independent values (the elements of an array, the entries of a map) accumulate diagnostics from
all of them instead of stopping at the first, with each path relative to the whole.

## 2. Codes in use

`duplicate-key`, `not-finite`, `invalid-tag`, `invalid-unit`, `no-ends`, `invalid-range-end`, `invalid-string`,
`invalid-integer`, `invalid-decimal` ([values.md](values.md)); `unwritable` ([text.md](text.md)); `invalid-form`,
`out-of-range` ([tags.md](tags.md)). Loss codes belong to the host mappings and are not defined yet.

## 3. Idiom

Each language wraps the shape in its usual idiom (`Result` and `?` in Rust, result objects in TypeScript and Python, a
monad in Haskell). The idiom is an adaptor and not part of this contract. The text form of a path is not defined yet
(RFC-002 question 7.2).
