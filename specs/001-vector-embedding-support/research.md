# Research: Vector Embedding Support (ADR-008)

**Feature**: `001-vector-embedding-support` | **Date**: 2026-11-15

All architectural decisions for `embedding` were settled in [RFC-004](../../design/rfc/RFC-004-vector-embeddings.md) and [ADR-008](../../design/adr/ADR-008-embedding-tag.md) (both `accepted`). This document records the implementation-level research across the five active language packages and shared conformance tooling.

## 1. Zero-Dependency Base64 & SafeTensors Parsing

- **Decision**: Implement the strict RFC 4648 Base64 decoder/encoder and restricted SafeTensors reader/writer using each language's standard library only (no external SafeTensors or Base64 crates/packages).
- **Rationale**:
  - `gram-value` packages have zero runtime dependencies by design (ADR-005, `Package.swift`, `pyproject.toml`, `go.mod`).
  - Strict RFC 4648 rules (required `=` padding, no whitespace, zero unused pad bits per §3.5) and the 11,190,284-character pre-decode bound are simpler and more uniform to enforce in ~40–60 lines of stdlib/direct code than configuring external decoders that may silently ignore whitespace or non-zero pad bits.
- **Alternatives considered**:
  - Adding `safetensors` / `base64` dependencies per language: rejected per ADR-008 §2.6 and ADR-005's zero-dependency native package model.

## 2. Strict JSON Header Parsing & Duplicate-Key Detection

- **Decision**: Implement a dedicated, minimal JSON object parser (or strict token scanner) for the SafeTensors header in every language, rather than relying on `JSON.parse`, Python `json.loads`, or Go `encoding/json`.
- **Rationale**:
  - ADR-008 §2.6 explicitly notes that TypeScript (`JSON.parse`), Python (`json`), and Go (`encoding/json`) silently overwrite duplicate keys instead of rejecting them.
  - The SafeTensors embedding header has a tiny, fixed two-level structure (`{"embedding":{"dtype":"...","shape":[...],"data_offsets":[...]}}`) capped at 4,096 bytes.
  - A small recursive-descent/token scanner over the UTF-8 header bytes naturally enforces:
    - Valid UTF-8
    - Exact JSON whitespace (space, tab, LF, CR)
    - Duplicate-key rejection at both top level and tensor level
    - Plain base-10 integer spelling in `shape` and `data_offsets` (digits only, no sign, fraction, exponent, or leading zero except `0`)
    - Exact JSON string unescaping for keys and `dtype`
- **Alternatives considered**:
  - `json.loads(..., object_pairs_hook=...)` in Python + custom scanner only in TS/Go: viable in Python for duplicate keys, though `json.loads` still accepts `2.0`, `1e1`, and `-1` as numbers without distinguishing raw token spelling. A uniform minimal header parser across all 5 languages handles both duplicate keys and exact number spelling identically.

## 3. Pre-Allocation Bounds & Validation Check Order

- **Decision**: Enforce the exact check order from ADR-008 §2.4 in all five packages:
  1. Base64 syntax (empty check, length multiple of 4, valid alphabet `+`/`/`, valid `=` padding, zero unused pad bits) → `invalid-form`
  2. Decoded-size bound (Base64 length > 11,190,284 chars or decoded byte count > 8,392,712) → `out-of-range` (checked before allocating the decoded buffer)
  3. SafeTensors container & profile entries (buffer ≥ 8 bytes, 8-byte LE header length ≤ 4,096 and ≤ remaining bytes, valid UTF-8 JSON, exact top-level `embedding` key, no `__metadata__`, exact tensor keys `dtype`/`shape`/`data_offsets`, no duplicate keys, plain base-10 integers, `shape` is 1-element array, `data_offsets` is 2-element array) → `invalid-form`
  4. `dtype` check (exact case-sensitive match against `F16`, `BF16`, `F32`, `F64`) → `unsupported`
  5. Dimension range (`1 <= n <= 1,048,576`, and `shape` fits in `u64`) → `out-of-range`
  6. Stated vs. actual byte lengths (`data_offsets == [0, n * width]`, and decoded buffer ends immediately after `8 + header_len + b`) → `invalid-form`
- **Rationale**: Guarantees identical diagnostic codes across all 5 packages when an input violates multiple rules simultaneously.

## 4. Conformance Vocabulary, Mapping Docs & CI Parity

- **Decision**:
  - Update `conformance/vocabulary.gram` to add `Embedding` (Rust, TypeScript, Python, Go) and `GramEmbedding` (Swift).
  - Add full `TagCase` and `MappingCase` coverage in `conformance/tags.gram` and `conformance/mappings.gram`, update `conformance/README.md`, and remove `skip-embedding-*` from all five `skips.gram` files as each package lands.
  - Re-run `scripts/gen-mapping-docs.py` if mapping docs under `docs/mappings/` are checked in CI.
- **Rationale**: `conformance/vocabulary.gram` already has `(embedding:Tag)` and `(unsupported:Class:Diagnostic)`; adding the native type names keeps `mappings.gram` and `gen-mapping-docs.py` valid.
