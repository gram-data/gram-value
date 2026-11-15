# Data Model: Vector Embedding Support (ADR-008)

**Feature**: `001-vector-embedding-support` | **Date**: 2026-11-15

## Entities

### 1. `EmbeddingDtype` / `GramEmbeddingDtype`

Enumerates the four supported floating-point element encodings in v1:

| Variant | SafeTensors String | Element Width (bytes) |
|---|---|---|
| `f16` / `F16` | `"F16"` | 2 |
| `bf16` / `BF16` | `"BF16"` | 2 |
| `f32` / `F32` | `"F32"` | 4 |
| `f64` / `F64` | `"F64"` | 8 |

### 2. `Embedding` / `GramEmbedding`

Package-defined typed value representing a rank-one dense floating-point tensor:

| Field | Type / Constraint | Notes |
|---|---|---|
| `dtype` | `EmbeddingDtype` | One of `F16`, `BF16`, `F32`, `F64` |
| `dimension` | Positive integer (`1 ..= 1,048,576`) | Logical element count ($n$) |
| `bytes` | Contiguous byte buffer | Length MUST equal `dimension * dtype.width`; little-endian IEEE bit patterns preserved verbatim |

#### Language Representations (ADR-008 §2.6)

| Language | Type Name | Dtype Representation | Byte Storage |
|---|---|---|---|
| Rust | `Embedding` | `EmbeddingDtype` enum | `Vec<u8>` |
| TypeScript | `Embedding` | `"f16" \| "bf16" \| "f32" \| "f64"` (or `EmbeddingDtype`) | `Uint8Array` |
| Python | `Embedding` | `EmbeddingDtype` enum | `bytes` |
| Go | `Embedding` | `EmbeddingDtype` | `[]byte` |
| Swift | `GramEmbedding` | `GramEmbeddingDtype` enum | `Foundation.Data` |

*(Haskell is deferred per ADR-008 §2.6.)*

#### Equality Rules

- **Value-model layer (`Value::Tagged`)**: Exact string equality on `(tag, content)`. Two accepted inputs differing in JSON key order, JSON whitespace, or header padding are unequal `Tagged` values.
- **Interpreted layer (`Embedding`)**: Exact equality on `(dtype, dimension, bytes)`. Byte-identical payloads are equal regardless of NaN bit patterns; no cross-dtype equivalence or approximate float comparison.

### 3. Normal Content String

Canonical serialization emitted when converting an `Embedding` back to a `tagged` value (or returned as the normal content string on interpretation):

1. Build compact UTF-8 JSON header (no whitespace, fixed key order):
   ```text
   {"embedding":{"dtype":"<dtype>","shape":[<n>],"data_offsets":[0,<b>]}}
   ```
2. Pad header with ASCII spaces (`0x20`) to the next multiple of 8 bytes (0–7 spaces).
3. Prepend 8-byte unsigned little-endian header length `L`.
4. Append `b` payload bytes (`b = n * width`).
5. Encode the entire `8 + L + b` buffer using standard RFC 4648 Base64 with `+`, `/`, and `=` padding.

### 4. `Diagnostic` Codes

| Code | Trigger Conditions (ADR-008 §2.4) |
|---|---|
| `invalid-form` | Empty content; malformed Base64 or non-zero pad bits; buffer < 8 bytes; header length > 4,096 or > remaining bytes; invalid UTF-8; malformed JSON; duplicate JSON keys; missing/extra top-level or tensor keys; `__metadata__` present; non-1D `shape`; invalid integer spelling (`2.0`, `1e1`, `02`, `-1`); `data_offsets` > `u64::MAX` or != `[0, n * width]`; trailing/missing payload bytes |
| `out-of-range` | Base64 > 11,190,284 chars; decoded bytes > 8,392,712; `dimension == 0`; `dimension > 1,048,576` ($2^{20}$); `shape` integer > `u64::MAX` |
| `unsupported` | Structurally valid SafeTensors profile container whose `dtype` is not exactly `"F16"`, `"BF16"`, `"F32"`, or `"F64"` |
