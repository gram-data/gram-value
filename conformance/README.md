# Conformance corpus

The language-neutral cases every `gram-value` package runs. A case ties a gram value to its canonical
description, its canonical gram text, and, per language, the native value it maps to and whether that mapping is
exact, lossy (and how) or unsupported. The parity matrix is generated from the packages' results, so parity is
measured and not asserted.

The corpus is gram text (ADR-001): `values.gram`, `text.gram` and `tags.gram` hold the cases, `vocabulary.gram` the
names a case may use, and `GRAMMAR_VERSION` the tree-sitter-gram version the files are read with. Check the files
with `gram lint --strict conformance/`. Each package parses them with its own tree-sitter-gram binding and a
lowering function in its tests; the Rust runner is `rust/tests/conformance.rs`. Mapping cases (`MappingCase`, in
`mappings.gram`) state the native type per language, one case per line so that `scripts/gen-mapping-docs.py` can read
them; it generates `docs/mappings/<language>.md`, and CI fails if those files are stale. Rust and TypeScript have entries so far.
