---
id: ADR-008
title: The embedding Tag as Base64-Encoded SafeTensors
status: proposed
rfcs: [RFC-004]
created: 2026-11-15
---

# ADR-008: The `embedding` Tag as Base64-Encoded SafeTensors

## 1. Context

Embeddings arrive from producers in more than one numeric representation. A 32-bit float, a 16-bit float and
bfloat16 use different bit layouts, while a quantized integer vector needs calibration information and a
bit-packed vector stores several logical dimensions in one byte. A gram array records only decimal values,
so using one as the carrier would widen every element to the model's 64-bit `decimal` and lose the producer's
representation. The carrier must instead record the vector's element type and dimension with its bytes.

RFC-004 chooses a tagged string and deliberately excludes readability and element-wise editing from the
requirements. The remaining decisions are the tag's name, how a binary tensor fits in a string, which tensor
format to adopt, which of that format's features constitute an embedding, how interpretation reports errors,
and what each native package returns. The typed result must preserve the payload without requiring every host
language to have a native half-precision tensor type.

## 2. Decision

### 2.1 The tag is `embedding`

The known tag is exactly `embedding`, in lower case. It denotes one rank-one tensor. The name does not
promise a producer, model, metric or normalization, and it is not widened to matrices or general tensors.
The existing convenience dispatch adds `embedding`; any other tag remains unknown.

A textual example is ``embedding`AAAAAAAA...` ``, where the ellipsis stands for the rest of the Base64
content. The content has no human-readable vector form.

### 2.2 The content is Base64 of a restricted SafeTensors buffer

The content uses standard RFC 4648 Base64 with `+`, `/` and required `=` padding, without spaces or line
breaks. Empty content is invalid. Decoding yields one buffer in the SafeTensors format: an eight-byte
unsigned little-endian header length, a UTF-8 JSON header, and a contiguous tensor byte buffer.

SafeTensors supplies the otherwise repeated architectural decisions: an explicit dtype, shape and data
offsets; little-endian numbers; packed C-order data; checked buffer boundaries; and no executable object
serialization. The embedding profile narrows it as follows:

- The top-level header has exactly one entry, named `embedding`, and no `__metadata__` entry. Duplicate or
  unknown entries and duplicate JSON keys are errors.
- The tensor entry has exactly `dtype`, `shape` and `data_offsets`.
- `shape` is `[n]`, where `n` is a positive integer. An empty vector and every other rank are not embeddings
  under this tag.
- `data_offsets` is `[0, b]`, relative to the tensor byte buffer, and `b` is exactly the byte length implied
  by the dimension and dtype. The decoded buffer ends immediately after those `b` bytes: no prefix, suffix,
  gaps or shared tensor data are accepted.
- The tensor byte buffer is copied into the interpreted value exactly as received. No arithmetic, cast,
  quantization or byte-order change occurs.

The normal content string for a successful interpretation is produced by rebuilding the buffer with the
header below, with the dtype, positive dimension and byte length substituted and with no whitespace or
header padding, then encoding it with the Base64 rules above:

```text
{"embedding":{"dtype":"<dtype>","shape":[<n>],"data_offsets":[0,<b>]}}
```

Eight bytes holding that header's UTF-8 byte length precede it; the original tensor bytes follow it.
Consequently an accepted SafeTensors producer may include legal header padding, but conversion to the typed
value and back removes it. The tag's content is not otherwise normalized.

### 2.3 The first version accepts four floating-point encodings

The accepted SafeTensors dtype strings are `F16`, `BF16`, `F32` and `F64`. Their element widths are two,
two, four and eight bytes. `BF16` is bfloat16, not `F16` with another spelling.

Every other dtype, including signed or unsigned integer, boolean, complex, FP8 and sub-byte formats, is
recognized as outside this tag's accepted set. Quantized `I8`/`U8` and packed binary embeddings are deferred:
transporting their codes without their quantization and packing semantics would be misleading. A later ADR
must add those semantics together with any new encodings.

All IEEE bit patterns, including signed zero, NaN and infinities, are accepted in the byte payload. An
embedding is an opaque tensor, not a sequence of gram `decimal` values, so the decimal constructor's
`not-finite` rule does not apply. Similarity and arithmetic policy belong to consumers.

### 2.4 Interpretation returns a typed value or diagnostics

Each package has one `embedding` interpretation function. It returns no typed value on failure, with one
diagnostic in the common result shape:

| Code | Condition |
|---|---|
| `invalid-form` | Base64 is malformed, the SafeTensors header or container is malformed, the profile's required entries are wrong or duplicated, the shape is not one-dimensional, or the stated and actual byte lengths disagree |
| `out-of-range` | The dimension is zero or exceeds a size the receiving package can safely length-check and hold |
| `unsupported` | The container is structurally valid but its dtype is outside `F16`, `BF16`, `F32` and `F64` |

The message names the failed check. The path locates the tagged value in the value being interpreted, as it
does for the other known tags. A recognized tag with bad content is never reduced to an unknown tagged string
by the interpretation itself; callers that choose optional tag support can still retain the original `Value`.

### 2.5 Equality remains exact, at both layers

A tagged value continues to compare by exact tag and content under the value model. Two accepted inputs that
differ only in Base64 spelling permitted by a decoder, JSON whitespace, or header padding are unequal tagged
values even when their normal content strings match.

The interpreted value compares by dtype, dimension and payload bytes. There is no numeric equivalence across
dtypes, no approximate comparison, and no NaN exception: byte equality defines interpreted equality. The
canonical gram text of a tagged value retains its content exactly; the normal content string is used only by
an interpretation result and by conversion from a typed native value.

### 2.6 Every host gets an exact package-defined typed value

The host mapping is a package-defined `Embedding` value with a dtype enum (`f16`, `bf16`, `f32`, `f64`), a
dimension and a byte buffer. Conversion from the tagged value is exact; conversion back emits the normal
content string. No package is required to decode tensor elements or depend on an ML framework.

| Language | Typed value and byte storage |
|---|---|
| Rust | `Embedding`, with `EmbeddingDtype` and `Vec<u8>` |
| TypeScript | `Embedding`, with a dtype union and `Uint8Array` |
| Python | `Embedding`, with an `EmbeddingDtype` enum and `bytes` |
| Go | `Embedding`, with `EmbeddingDtype` and `[]byte` |
| Swift | `GramEmbedding`, with `GramEmbeddingDtype` and `Foundation.Data` |

Conversions to NumPy, framework tensors, JavaScript typed arrays or other native vector libraries are optional
extras, not the standard mapping. An extra that converts to ordinary host floats must use the existing loss
reporting and can never silently change the dtype. Ordinary gram arrays do not implicitly become embeddings.

Each native package implements the restricted reader and normal writer without a mandatory external
SafeTensors dependency. A package may use such a dependency internally when it can enforce every profile
rule, including duplicate-key and exact-length checks.

### 2.7 The conformance corpus tests the profile

Accepted cases cover all four dtypes, the minimum dimension, signed zero and non-finite payload bit patterns,
a padded but valid header, and a non-normal input with its normal content string. Rejected cases cover bad
Base64, missing or extra tensors and metadata, wrong rank, zero dimension, wrong offsets, trailing or missing
payload bytes, and each unsupported dtype. Every language runs the same cases; native mapping cases record
`Embedding`/`GramEmbedding` as exact in both directions.

## 3. Alternatives Considered

3.1. **Base64 of raw float32 bytes.** Rejected: the receiver cannot distinguish float32 from the other
encodings without an out-of-band agreement.

3.2. **A custom `dtype:dimension:base64` header and raw bytes.** Rejected: it duplicates tensor-container
rules such as endianness, shape, length validation and future encoding names without improving interchange
with existing tensor tooling.

3.3. **NumPy NPY.** Rejected: its standard dtype vocabulary has no portable bfloat16 representation, and its
support for arbitrary NumPy dtypes exceeds this value's needs.

3.4. **Apache Arrow fixed-shape tensors.** Rejected: the columnar container is substantially more machinery
than one scalar property value needs, and Arrow has no standard bfloat16 primitive.

3.5. **ONNX TensorProto.** Rejected: the protobuf and model-IR dependency is disproportionate to one vector,
although its broad element-type vocabulary would cover future quantized formats.

3.6. **Accept quantized integer and packed binary dtypes now.** Rejected: an `I8` code vector is not
necessarily the same logical representation as another `I8` vector, and a byte buffer does not state its
original dimension or quantization parameters.

3.7. **Reject NaN and infinities.** Rejected: no decimal text is being constructed and the bytes can be
transported exactly; a consumer doing arithmetic remains responsible for its numeric policy.

3.8. **Base64url without padding.** Rejected as an additional accepted spelling: standard padded Base64 gives
one canonical string and is available in every target standard library.

## 4. Consequences

- Known-tag dispatch and the specification grow from ten to eleven tags. Existing ``embedding`...` `` values
  that consumers previously treated as unknown can now fail known-tag interpretation, which is a breaking
  change under the known-tag version policy.
- Base64 expands the payload by about one third, and the SafeTensors header adds a small fixed overhead. That
  is the cost of keeping the value inside gram text without pretending its elements are editable numbers.
- All five packages need a small binary reader, a canonical writer and the byte-buffer typed value. No ML
  dependency is needed. The profile rejects some otherwise valid SafeTensors files by design.
- Non-normal accepted content compares differently as a tagged value after conversion through the typed
  native value. Consumers that must retain the exact spelling retain the original `Value`, as with other
  known tags.
- Quantized and binary embeddings remain unsupported. Adding them requires a later ADR covering scales,
  ranges, zero points or bit packing, as appropriate.
- Follow-up work is to amend `spec/tags.md`, add the diagnostic and mapping vocabulary, add corpus cases,
  implement the five native interpreters, and update ADR-004's known-tag count and ADR-006's known-tag
  mapping prose. Tree-sitter-gram's known-tag documentation should link to the updated shared specification.

## 5. Related

- RFC-004 (resolves questions 7.1 to 7.6).
- ADR-002 (tagged-value equality), ADR-004 (known-tag interpretation), ADR-005 (diagnostics), ADR-006
  (host-language mappings), ADR-001 (conformance cases).
- [SafeTensors format](https://github.com/huggingface/safetensors#format) and [RFC 4648 Base64](https://www.rfc-editor.org/rfc/rfc4648#section-4).
