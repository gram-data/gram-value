# Contract: `embedding` Tag Interpretation & Native Mapping (ADR-008)

**Feature**: `001-vector-embedding-support` | **Date**: 2026-11-15

## 1. Specification & Conformance Artifacts

- `spec/tags.md`: Add `embedding` as the 11th known tag with its Base64 + restricted SafeTensors profile, normal form, and `unsupported` diagnostic code.
- `spec/results.md`: Document `unsupported` alongside `invalid-form` and `out-of-range` (if `results.md` lists tag diagnostic codes).
- `design/adr/ADR-004-known-tags.md` & `design/adr/ADR-006-host-language-mappings.md`: Update known-tag count (10 → 11), diagnostic code cross-reference, and mapping prose per ADR-008 §4.
- `conformance/vocabulary.gram`: Add `Embedding` (for `rust`, `typescript`, `python`, `go`) and `GramEmbedding` (for `swift`).
- `conformance/tags.gram`: Complete `tag-embedding-*` `TagCase` coverage for all accepted and rejected boundaries in ADR-008 §2.2–2.4, §2.7.
- `conformance/mappings.gram`: Add bidirectional `MappingCase` entries for `Embedding` / `GramEmbedding` across all five languages.
- `conformance/README.md`: Update embedding case documentation once full corpus cases are in place.
- `*/skips.gram`: Remove all `skip-embedding-*` entries once each package passes.

## 2. Package-Level Public API Contract

Each of the five active packages (`rust`, `typescript`, `python`, `go`, `swift`) exposes:

1. **Typed value & constructor**: `Embedding` + `EmbeddingDtype` (`GramEmbedding` + `GramEmbeddingDtype` in Swift), validating `1 <= dimension <= 1,048,576` and `bytes.len() == dimension * dtype.width`.
2. **Tag interpretation function**: Dedicated `embedding` interpreter taking a `Tagged` / `Value` and returning the common result shape (`Embedding` + normal content string, or `Diagnostic` with `code`, `message`, `path`).
3. **Convenience dispatch**: Existing known-tag dispatch includes `embedding` (now 11 known tags). Unknown tags continue to pass through unchanged.
4. **Bidirectional host mapping**:
   - `Value::Tagged("embedding", content)` → `Embedding` (exact, validates via `embedding` interpreter)
   - `Embedding` → `Value::Tagged("embedding", normal_content)` (exact, emits canonical 8-byte-aligned normal content string)
   - Ordinary `Value::Array` of numbers is never implicitly converted to `Embedding`.

## 3. Validation Check Order Contract

Every package MUST execute checks in this exact order:

1. Base64 syntax (non-empty, length % 4 == 0, no whitespace, valid `+`/`/`/`=` padding, zero unused pad bits) → `invalid-form`
2. Decoded-size bound (Base64 length > 11,190,284 or decoded length > 8,392,712) → `out-of-range`
3. SafeTensors framing & JSON profile (header length ≤ 4,096 and ≤ remaining bytes, valid UTF-8 JSON, exact keys, no duplicate keys, plain base-10 integer tokens, 1D `shape`, 2-element `data_offsets`) → `invalid-form`
4. `dtype` exact match (`F16`, `BF16`, `F32`, `F64`) → `unsupported`
5. Dimension range (`1 <= n <= 1,048,576`, fits in `u64`) → `out-of-range`
6. Exact byte length (`data_offsets == [0, n * width]` and no trailing bytes) → `invalid-form`
