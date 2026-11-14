# CLAUDE.md

## Purpose

`gram-value` is the shared model of the data values of the [gram](https://github.com/gram-data/tree-sitter-gram)
notation, in several languages: every value a gram file can write, a way to write each back as gram text, and
documented mappings to each host language's native values. Consumers such as pattern-rs and relate-gram depend
on it instead of keeping their own copies. It began as a note in relate-gram
(`/Users/abk/Developer/relatedb/relate-gram/design/notes/2026-11-14-gram-value.md`).

## Scope (decided in the kickoff conversation)

- **All gram values.** Integers (decimal, hexadecimal and octal spellings), decimals, booleans, symbols, strings
  (four quoting styles), ranges, measurements, arrays, maps and tagged strings. Consumers may support only a
  subset, without gram-value naming it; subsets and profiles belong to the consumers.
- **Not a parser, not a graph model.** Parsing belongs to tree-sitter-gram; a property-graph layer, if any, is a
  separate project (`gram-graph` is only an idea).
- **Authority:** tree-sitter-gram (its grammar and test corpus) is the authoritative specification. This project
  defers to it for syntax and adds the meaning of values, the text-writing rules, and the mappings.
- **Tagged strings stay `{tag, content}`.** Known tags (`date`, `time`, `localtime`, `datetime`,
  `localdatetime`, `duration`, `point`, `uuid`, from relate-gram's ADR-011) are an optional interpretation
  layer. Unknown tags, such as pattern-rs's `ts`, `re`, `zod`, `cypher`, `pydantic` and `gram`, pass through.
- **Standard host-language mappings, even lossy.** Each language documents how every value maps to a native
  value; where information is lost, the table says what, and a consumer can detect it.
- **Languages.** First tier: Rust, TypeScript, Python. Next tier: Go, Swift, Haskell.
- **Values are plain sum types.** `Value` has no monad or functor interface, so any language can wrap it.
  Interpretation and mapping return a language-neutral result (the value plus the diagnostics or losses found,
  accumulated across independent values), with idiomatic helpers per language; a small structural traversal is
  offered only if the conformance corpus can test it identically everywhere. A pipeline system that
  pushes operations down across a language boundary is out of scope: every package is native (RFC-001 7.11, 7.16).
- **Implementation strategy is not decided.** One Rust core with bindings versus native implementations per
  language is a question for a spike and an ADR. Crossing a language boundary is expensive for users, and
  WebAssembly in particular is assumed to be a poor experience; Python may be acceptable. The spike measures
  simple operations in each candidate (create 1,000 values, add them to a list, map the list to new values).
- **Parity** is defined by a language-neutral conformance corpus that every language runs in CI, not by one
  implementation.
- **License:** MIT. **Remote:** `git@github.com:gram-data/gram-value.git` (public).

Still open, and the first RFC's job: whether a value records only what it means or also how it was written;
map ordering and duplicate keys; the value of an octal literal and integer width; decimal and measurement
representation; how consumers extend the known-tag registry; where the text-writing rules live; how host-language
mappings are specified, tested and report loss; how the value type evolves with several projects depending on it.

## Reference repos (local, read-mostly)

- `/Users/abk/Developer/gram-data/tree-sitter-gram`: the grammar, its corpus, and bindings for c, go, node, python, rust, swift.
- `/Users/abk/Developer/relateby/pattern-rs`: two existing `Value` types (`crates/pattern-core/src/subject.rs`, `crates/gram-codec/src/value.rs`) and a parity matrix (`proposals/language-parity-proposal.md`).
- `/Users/abk/Developer/relatedb/relate-gram`: typed-tag forms (ADR-011, `src/typed/`), and the fidelity evidence in SPIKE-003, SPIKE-004 and SPIKE-005.
- `/Users/abk/Developer/gram-data/gram-ontology`: a sibling project using the same design process.

## Design process: RFC → ADR → implementation (with spikes)

The same process as relate-gram. Skills live in `.agents/skills/` and are symlinked from `.claude/skills/`.

- **RFC** (`design/rfc/RFC-NNN-*.md`, `/rfc-create`, `/rfc-review`): the WHAT and WHY, with candidate directions. No implementation detail.
- **Spike** (`design/spikes/SPIKE-NNN-*/SPIKE.md`, `/spike-create`): a time-boxed investigation of one empirical question.
- **ADR** (`design/adr/ADR-NNN-*.md`, `/adr-create`, `/adr-review`): one technical decision, always with a parent RFC.
- **Notes** (`design/notes/YYYY-MM-DD-*.md`): freeform, non-binding.
- Don't hand-edit the index tables in the `design/*/README.md` files; the skills maintain them.
- Work happens on feature branches and goes through pull requests; an RFC, ADR or spike passes its review skill
  before its status moves toward `review` or `accepted`.
- Write design documents for a newcomer: name project artifacts on first use.
- Spec Kit (`/speckit-*`) and a constitution will be installed in `.specify/` before the first implementation.

## Build and test

Validate every `.gram` file in the repository with the `gram` CLI from tree-sitter-gram (`cargo install gram-data`): `gram lint --strict <paths>`. Install its agent skill with `gram skill install`, so an agent editing gram knows the notation and this check (ADR-001).

Nothing else to build yet. Each language package will say how to build and test it, and every package runs the whole
conformance corpus.
