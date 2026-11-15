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

The `tag-embedding-*` cases in `tags.gram` and `map-embedding-*` cases in `mappings.gram` test the `embedding` profile across all five active packages:
- Accepted: `F16`, `BF16`, `F32`, `F64`, non-normal header key order/whitespace normalization, dimension `1`, and verbatim IEEE bit patterns (`-0.0`, `±inf`, `NaN`).
- Rejected (`invalid-form`): empty content, malformed Base64, whitespace, non-zero pad bits, short buffers, header length `4097` or truncated, invalid UTF-8, malformed JSON, duplicate keys (both levels), `__metadata__`, extra/missing keys, wrong rank, negative or non-plain-integer dimensions, mismatched `data_offsets`, and trailing/missing payload bytes.
- Rejected (`out-of-range`): dimension `0`, `1048577`, and dimensions beyond 64 bits.
- Rejected (`unsupported`): non-matching `dtype` strings (`f32`, `I32`, `F8_E4M3`).
