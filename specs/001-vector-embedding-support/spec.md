# Feature Specification: Vector Embedding Support (ADR-008)

**Feature Branch**: `001-vector-embedding-support`

**Created**: 2026-11-15

**Status**: Draft

**Input**: User description: "ADR-008 for vector embedding support"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Interpret and Validate `embedding` Tagged Strings (Priority: P1)

A consumer reading gram data receives a tagged string with tag `embedding` carrying a machine-produced vector embedding. When the consumer interprets the tagged value through the known-tag layer, it receives a typed embedding value preserving the element encoding (`F16`, `BF16`, `F32`, or `F64`), positive dimension, and raw payload bytes without widening or precision loss, or a structured diagnostic explaining why the content was rejected.

**Why this priority**: Interpreting and validating the `embedding` tag is the core capability of ADR-008; without it, consumers cannot safely distinguish valid typed embeddings from malformed, oversized, or unsupported payloads.

**Independent Test**: Can be tested independently by passing accepted and rejected `embedding` tagged values from the conformance corpus to the `embedding` interpreter (and convenience known-tag dispatch) and verifying the resulting typed value, normal content string, or diagnostic code.

**Acceptance Scenarios**:

1. **Given** a valid `embedding` tagged string produced in `F16`, `BF16`, `F32`, or `F64` format, **When** a consumer interprets it, **Then** interpretation succeeds with a typed embedding holding the exact dtype, positive dimension, and unmodified payload bytes, plus the canonical normal content string.
2. **Given** an `embedding` tagged string with malformed Base64, non-canonical pad bits, invalid SafeTensors framing, header length exceeding 4,096 bytes or remaining buffer, duplicate JSON keys, unexpected entries or `__metadata__`, non-1D shape, invalid number spelling (`2.0`, `1e1`, `02`), or mismatched byte lengths, **When** a consumer interprets it, **Then** interpretation fails with no value and diagnostic code `invalid-form`.
3. **Given** an `embedding` tagged string whose content exceeds 11,190,284 Base64 characters (or 8,392,712 decoded bytes), or whose dimension is `0`, greater than `1,048,576` ($2^{20}$), or beyond 64 unsigned bits, **When** a consumer interprets it, **Then** interpretation fails with no value and diagnostic code `out-of-range`, checked from length/bounds before large allocations.
4. **Given** a structurally valid `embedding` tagged string whose `dtype` is outside the exact case-sensitive set `{F16, BF16, F32, F64}` (such as `f32`, `I8`, `U8`, or `FP8`), **When** a consumer interprets it, **Then** interpretation fails with no value and diagnostic code `unsupported`.

---

### User Story 2 - Round-Trip Typed Embeddings and Preserve Exact Equality (Priority: P1)

A producer or consumer constructs or receives a typed embedding value, converts it back to a `tagged` gram value, and compares values at both the value-model layer and the interpreted-embedding layer. Normal content round-trips byte-for-byte, non-normal accepted inputs normalize on conversion back, and all IEEE bit patterns (including signed zero, NaN, and infinities) are preserved without numeric normalization.

**Why this priority**: Exact round-tripping and unambiguous equality semantics are essential so embeddings produced by ML pipelines or reference SafeTensors writers survive transport unchanged across systems.

**Independent Test**: Can be tested independently by converting reference-writer and non-normal `embedding` inputs to typed embeddings and back, checking byte identity for normal inputs, normalization for non-normal inputs, and exact equality at both layers.

**Acceptance Scenarios**:

1. **Given** an `embedding` tagged string already in normal form (including output from the reference SafeTensors writer), **When** it is interpreted to a typed embedding and converted back to a tagged value, **Then** the resulting tagged content string is identical byte-for-byte to the input.
2. **Given** an accepted `embedding` tagged string with non-normal formatting (different JSON key order, extra JSON whitespace, or non-standard 8-byte header padding), **When** it is interpreted and converted back to a tagged value, **Then** it emits the canonical normal content string (compact JSON with keys `dtype`, `shape`, `data_offsets`, space-padded to an 8-byte multiple).
3. **Given** an `embedding` payload containing signed zero, NaN, or infinity bit patterns, **When** it is interpreted, compared, and converted back, **Then** all bit patterns are accepted and preserved unchanged, and interpreted equality compares by exact dtype, dimension, and payload bytes.
4. **Given** two accepted `embedding` tagged values with differing non-normal text spellings for the same underlying tensor, **When** compared at the value-model layer, **Then** they remain unequal tagged values; **When** compared as interpreted embeddings, **Then** they are equal.

---

### User Story 3 - Cross-Language Parity, Specification, and Conformance Corpus (Priority: P2)

Maintainers and downstream consumers across all active language packages rely on a single shared specification and conformance corpus so every package accepts, rejects, normalizes, and maps `embedding` values identically.

**Why this priority**: Cross-language parity is a foundational project requirement; without shared specification updates and comprehensive conformance cases, individual packages could diverge on edge cases such as duplicate JSON keys or check ordering.

**Independent Test**: Can be tested independently by running the shared conformance suite across every active package and verifying zero unacknowledged failures or divergences on all `TagCase` and native mapping cases.

**Acceptance Scenarios**:

1. **Given** the updated shared specification (`spec/tags.md` and design index references), **When** any active package runs the conformance corpus (`conformance/tags.gram` and `conformance/mappings.gram`), **Then** every accepted, rejected, and bidirectional native-mapping case passes with matching normal strings and diagnostic codes.
2. **Given** a consumer that does not opt into `embedding` interpretation (treating it as an unknown tag), **When** it reads and writes `embedding` tagged values, **Then** the tagged value passes through unchanged without error.

---

### Edge Cases

- **Empty content**: `embedding`` `` is rejected as `invalid-form`.
- **Oversized Base64 before decode**: Content longer than 11,190,284 Base64 characters (or decoding to more than 8,392,712 bytes) is rejected as `out-of-range` from content length alone before allocating a decode buffer.
- **Hostile header length prefix**: An 8-byte little-endian header length greater than 4,096 or greater than the remaining buffer bytes is rejected as `invalid-form` before reading or parsing the header.
- **Duplicate JSON keys**: Duplicate keys at either the top level or inside the `embedding` tensor object are rejected as `invalid-form`, even in hosts whose default JSON parser normally keeps the last key.
- **Multiple simultaneous violations**: Checks run in fixed order (Base64 syntax → decoded-size bound → SafeTensors container and profile entries → dtype → dimension range → stated vs. actual byte lengths), ensuring deterministic diagnostics (e.g., an unsupported `dtype` with a valid container yields `unsupported` even if dimension or byte lengths also fail later checks; a dimension above $2^{20}$ with a short payload yields `out-of-range`).
- **Whitespace and UTF-8**: Only JSON's four whitespace characters (space, tab, LF, CR) are permitted between header tokens, and invalid UTF-8 in the header is `invalid-form`.
- **No implicit array conversions**: Ordinary gram numeric arrays are never implicitly converted to `embedding` values, and invalid `embedding` tagged strings are never downgraded to plain strings by interpretation.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST recognize `embedding` (exact lower case) as a known tag in both dedicated `embedding` interpretation and convenience known-tag dispatch, while leaving unknown tags unaffected.
- **FR-002**: System MUST validate `embedding` content against strict RFC 4648 Base64 (`+`, `/`, required `=` padding, no whitespace, non-empty, zero unused pad bits per RFC 4648 §3.5), returning `invalid-form` on failure.
- **FR-003**: System MUST enforce a pre-allocation decoded-size bound of at most 8,392,712 bytes (11,190,284 Base64 characters) and return `out-of-range` from length alone when exceeded.
- **FR-004**: System MUST validate the decoded buffer against the restricted SafeTensors profile: 8-byte little-endian header length ≤ 4,096 and ≤ remaining bytes; valid UTF-8 JSON header; exactly one top-level key `embedding` and no `__metadata__`; tensor entry with exact keys `dtype`, `shape`, `data_offsets` in any key order; no duplicate JSON keys at either level; `shape` as `[n]` with plain base-10 integer spelling (no sign, fraction, exponent, or leading zeros except `0`); `data_offsets` as `[0, b]` with plain base-10 integer spelling matching exact payload byte length `b`; and no trailing bytes.
- **FR-005**: System MUST accept only exact case-sensitive `dtype` values `F16` (2 bytes), `BF16` (2 bytes), `F32` (4 bytes), and `F64` (8 bytes) after JSON unescaping, returning `unsupported` for any other valid container dtype.
- **FR-006**: System MUST enforce dimension bounds `1 <= n <= 1,048,576` ($2^{20}$), returning `out-of-range` for `0`, values above $2^{20}$, or integers exceeding 64 unsigned bits in `shape` (and `invalid-form` for integers exceeding 64 unsigned bits in `data_offsets`).
- **FR-007**: System MUST execute validation checks in the fixed order: (1) Base64 syntax, (2) decoded-size bound, (3) SafeTensors container and profile entries, (4) dtype, (5) dimension range, (6) stated vs. actual byte lengths.
- **FR-008**: System MUST accept and preserve all IEEE bit patterns in the payload (including signed zero, NaN, and infinities) without arithmetic, casting, or byte-order changes.
- **FR-009**: System MUST expose an exact native byte-buffer `Embedding` value (with dtype, dimension, and raw bytes) in each of the five active target packages (Rust, TypeScript, Python, Go, Swift), with exact bidirectional conversion to/from `tagged` values emitting the canonical 8-byte-aligned normal content string on write-back.
- **FR-010**: System MUST define exact equality at both layers: tagged values compare by exact tag and content string; interpreted `Embedding` values compare by exact dtype, dimension, and payload bytes.
- **FR-011**: System MUST update the shared specification (`spec/tags.md` and related design docs ADR-004/ADR-006) and complete the conformance corpus (`conformance/tags.gram`, `conformance/mappings.gram`, `conformance/README.md`) covering all accepted, rejected, boundary, and native-mapping cases across all five active packages (removing all `skip-embedding-*` entries once implemented).

### Key Entities

- **Embedding (`Embedding` / `GramEmbedding`)**: A typed rank-one tensor value consisting of an element dtype (`f16`, `bf16`, `f32`, `f64`), a positive integer dimension ($1 \dots 2^{20}$), and an exact contiguous little-endian byte payload of length `dimension * element_width`.
- **Diagnostic**: A structured failure record carrying a `code` (`invalid-form`, `out-of-range`, or `unsupported`), a descriptive `message`, and the `path` to the offending tagged value.
- **Conformance Case (`TagCase` / mapping case)**: A language-neutral test record specifying input gram value, expected outcome (`accepted` or `rejected`), normal content string or diagnostic code, and bidirectional native mapping expectations.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of `embedding` conformance cases (accepted, rejected, boundary, and native mapping) pass across all 5 active packages with zero `skip-embedding-*` entries remaining.
- **SC-002**: 100% of reference-writer `embedding` inputs round-trip through interpretation and write-back with byte-for-byte identical content strings.
- **SC-003**: 100% of oversized inputs (> 11,190,284 Base64 characters or > 4,096-byte header length) are rejected deterministically before full payload allocation or header parsing.
- **SC-004**: Zero regressions in existing known-tag, unknown-tag passthrough, or value-model conformance suites across all 5 active packages.

## Assumptions

- Five active language packages (Rust, TypeScript, Python, Go, Swift) are in scope for this implementation; the Haskell package is deferred and will adopt the same `Embedding` byte-buffer contract when added.
- Optional convenience conversions to host float arrays or ML framework tensors (e.g., NumPy, typed arrays) are outside the core standard mapping and, if added later, must use explicit conversion functions with loss reporting.
- Quantized integer (`I8`, `U8`), FP8, and bit-packed binary embeddings remain `unsupported` under this specification and require a separate ADR for quantization/packing metadata.
- The `embedding` tag's addition (10 → 11 known tags) and the `unsupported` diagnostic code are documented in `spec/tags.md` and cross-referenced from ADR-004 and ADR-006.
