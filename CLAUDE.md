# CLAUDE.md

## Purpose

`gram-value` is the shared model of the data values of the [gram](https://github.com/gram-data/tree-sitter-gram)
notation, in several languages: every value a gram file can write, a way to write each back as gram text, and
documented mappings to each host language's native values. Consumers such as pattern-rs and relate-gram depend
on it instead of keeping their own copies. It began as a note in relate-gram
(`/Users/abk/Developer/relatedb/relate-gram/design/notes/2026-11-14-gram-value.md`).

## Scope (decided in the kickoff conversation)

- **All gram values, as meaning only.** Kinds: integer (64-bit), big integer, decimal (64-bit float), big decimal,
  boolean, string, tagged string, range, measurement, array and map. Spellings (hex, octal, quoting style, a bare
  symbol) are not kept; a symbol in a value position is a string. No null. The set of kinds is closed. Consumers may support only a
  subset, without gram-value naming it; subsets and profiles belong to the consumers.
- **Not a parser, not a graph model.** Parsing belongs to tree-sitter-gram; a property-graph layer, if any, is a
  separate project (`gram-graph` is only an idea).
- **Authority:** tree-sitter-gram (its grammar and test corpus) is the authoritative specification. This project
  defers to it for syntax and adds the meaning of values, the text-writing rules, and the mappings.
- **Tagged strings stay `{tag, content}`.** Known tags (`date`, `time`, `localtime`, `datetime`,
  `localdatetime`, `duration`, `point`, `uuid`, `week`, `weekDate`; forms defined in this project's spec, RFC-001
  7.17) are an optional interpretation layer, a fixed set of plain functions with no registry. Unknown tags, such as pattern-rs's `ts`, `re`, `zod`, `cypher`, `pydantic` and `gram`, pass through.
- **Standard host-language mappings, even lossy.** Each language documents how every value maps to a native
  value; where information is lost, the table says what, and a consumer can detect it.
- **Languages.** First tier: Rust, TypeScript, Python. Next tier: Go, Swift, Haskell.
- **Values are plain sum types.** `Value` has no monad or functor interface, so any language can wrap it.
  Interpretation and mapping return a language-neutral result (the value plus the diagnostics or losses found,
  accumulated across independent values), with idiomatic helpers per language; a small structural traversal is
  offered only if the conformance corpus can test it identically everywhere. A pipeline system that
  pushes operations down across a language boundary is out of scope: every package is native (RFC-001 7.11, 7.16).
- **Every language package is native** (RFC-001 7.11, measured in SPIKE-002): no shared core and no
  WebAssembly or PyO3 bindings. Haskell is a peer, not a reference. The canonical text writer lives in the model's
  package. Equality is by kind and canonical text, with a total order.
- **Parity** is defined by a language-neutral conformance corpus that every language runs in CI, not by one
  implementation.
- **License:** MIT. **Remote:** `git@github.com:gram-data/gram-value.git` (public).

RFC-001 and RFC-002 are decided, ADR-001 to ADR-006 are accepted, `spec/` states the rules, and the Rust (`rust/`) and TypeScript (`typescript/`) packages implement them and pass 153 conformance cases. Next: the Python package (adds its entries to `mappings.gram`).

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
