# Specification

The normative text of what each gram value means: which kinds of value exist, how a gram literal becomes a value,
when two values are equal, what the canonical gram text of a value is, and what an interpretation of a known tag
must check. The [tree-sitter-gram](https://github.com/gram-data/tree-sitter-gram) grammar and test corpus are the
authority on what text is a gram value, and this specification defers to them for syntax.

| Document | Defines | Decided in |
|---|---|---|
| [values.md](values.md) | the eleven kinds, lowering from grammar literals, construction rules, equality, hashing, order | ADR-002 |
| [text.md](text.md) | the canonical text of every value, with examples | ADR-003 |
| [tags.md](tags.md) | the ten known tags: content forms, diagnostics, normal forms | ADR-004 |
| [results.md](results.md) | the result and diagnostic shape every package returns | ADR-005 |

The key words MUST, MUST NOT and MAY have their usual meaning. Where a document and its ADR disagree, the ADR wins
and the document is a defect. Every rule here is tested by the conformance corpus in `conformance/` (ADR-001),
except behavior on values gram cannot write, which each package tests with unit tests derived from these tables.
