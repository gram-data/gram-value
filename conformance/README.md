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
them; it generates `docs/mappings/<language>.md`, and CI fails if those files are stale. Rust, TypeScript, Python, Go and Swift have entries.

Each package also keeps a `skips.gram` (cases it does not pass yet, with reasons) and writes
`conformance-results.gram`; `scripts/gen-parity-matrix.py` merges the results of all packages into the parity matrix
(see ADR-001 2.7), which CI publishes as a job summary.

## Embedding cases (ADR-008)

The `tag-embedding-*` cases in `tags.gram` test the `embedding` profile. Accepted: output of the reference
SafeTensors writer for `F32`, `F64` and `F16` (`-ref-*`; each normal content string equals its input). Rejected:
a dimension written as `2.0`, `1e1` or `02` (`invalid-form`), a lower-case dtype (`unsupported`) and a dimension
beyond 64 bits (`out-of-range`). The rest of the profile's cases (the other dtypes and bit patterns, non-normal
spellings, duplicate keys, header-length and dimension bounds, wrong ranks and lengths, each unsupported dtype)
are still to be written; they are listed in ADR-008 2.2 to 2.4 as the conditions to cover. No package implements
the tag yet, so every package skips these cases.
