# gram-value

The data values of the [gram](https://github.com/gram-data/tree-sitter-gram) notation, as a library in several
languages: the integers, decimals, booleans, strings, symbols, ranges, measurements, arrays, maps and tagged
strings that a gram file can put on an element, with a way to write each back as gram text and a documented
mapping to each host language's native values.

Gram tools keep rebuilding this model. [pattern-rs](https://github.com/relateby/pattern-rs) has two copies,
and [relate-gram](https://github.com/relatedb/relate-gram), which loads gram into the Relate graph database,
built a third. `gram-value` is meant to be the one they share.

## Status

Early. The design is written: [RFCs](design/rfc/README.md) and [ADRs](design/adr/README.md) fix the value kinds,
equality, canonical text, the ten known tags and the native-package strategy, and [`spec/`](spec/README.md) states
the rules. The Rust package in [`rust/`](rust/) implements them, including the conversions to Rust's own types, and passes
the [conformance cases](conformance/README.md) (154). The Rust mapping table is in
[`docs/mappings/rust.md`](docs/mappings/rust.md). The TypeScript package in [`typescript/`](typescript/) does the same for JavaScript
([`docs/mappings/typescript.md`](docs/mappings/typescript.md)). The Python package in [`python/`](python/) completes the first tier
([`docs/mappings/python.md`](docs/mappings/python.md)). CI runs all three against the same cases and publishes the
parity matrix (`scripts/gen-parity-matrix.py`) in the job summary. Packages are published to crates.io (`gram-value`),
npm (`@gram-data/value`) and PyPI (`gram-value`) by tag; see [RELEASING.md](RELEASING.md). Nothing is published yet. The origin of the project is the note
[gram-value: a shared home for gram's data values](https://github.com/relatedb/relate-gram/blob/main/design/notes/2026-11-14-gram-value.md).

## What it is, and is not

- It models **every** value gram can write. A consumer that supports fewer types decides that for itself and
  rejects the rest with its own errors; `gram-value` does not name or ship consumer subsets.
- It is **not** a parser and **not** a graph model. Parsing is [tree-sitter-gram](https://github.com/gram-data/tree-sitter-gram)'s
  job, and the tree-sitter-gram grammar and its test corpus are the authoritative specification of what a
  gram value is.
- Tagged strings stay a tag and a content string. Interpreting known tags (dates, points, UUIDs and so on) is
  a separate layer, so tags that other projects define for their own purposes pass through untouched.
- Each language gets a documented mapping from gram values to its native values. Where a mapping loses
  information the mapping says what, and the loss can be detected.

## Languages

First tier: **Rust, TypeScript and Python**. Next tier: **Go, Swift and Haskell**. Whether each language is
a native implementation or a binding to a shared core is not decided; it is a question for a spike and an
ADR, and the cost to a user of crossing a language boundary (WebAssembly especially) counts heavily.

Parity between languages is defined by a language-neutral conformance corpus that every language runs, not by
one implementation being the reference. See [`conformance/`](conformance/README.md).

## Layout

| Path | Contents |
|---|---|
| `spec/` | The normative text of what a value means (it defers to tree-sitter-gram for syntax) |
| `conformance/` | The language-neutral corpus every language runs |
| `design/` | RFCs, ADRs, spikes and notes: how decisions are made here |
| `rust/`, `typescript/`, `python/` | The packages (not created yet) |
| `.agents/skills/` | `/rfc-create`, `/rfc-review`, `/adr-create`, `/adr-review`, `/spike-create` (symlinked from `.claude/skills/`) |

## License

MIT. See [LICENSE](LICENSE).
