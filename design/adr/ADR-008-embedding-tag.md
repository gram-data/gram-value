---
id: ADR-008
title: The embedding Tag as Base64-Encoded SafeTensors
status: accepted
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

A tagged string (a tag naming a type, then text content: ``date`2024-01-15` ``) is gram's existing extension
point, and a consumer that does not know a tag passes the value through unchanged. An embedding is machine
output that is read and written whole, so the carrier does not need to be readable or editable element by
element. What it does need is a rule for fitting a binary tensor into text, a tensor format that states dtype
and shape, a profile of that format that says which of its features are an embedding, diagnostics for bad
content, and a typed result that preserves the payload without requiring every host language to have a native
half-precision tensor type.

## 2. Decision

### 2.1 The tag is `embedding`

The known tag (a tag whose content form this project defines and whose interpretation each package offers) is
exactly `embedding`, in lower case. It denotes one rank-one tensor. The name does not
promise a producer, model, metric or normalization, and it is not widened to matrices or general tensors.
The existing convenience dispatch adds `embedding`; any other tag remains unknown.

A textual example is ``embedding`AAAAAAAA...` ``, where the ellipsis stands for the rest of the Base64
content. The content has no human-readable vector form.

### 2.2 The content is Base64 of a restricted SafeTensors buffer

The content uses standard RFC 4648 Base64 with `+`, `/` and required `=` padding, without spaces or line
breaks; non-canonical unused pad bits (RFC 4648 §3.5) are `invalid-form`. Empty content is invalid.
Content that would decode to more than 8,392,712 bytes (8 + 4,096 + 8 × 2^20, the largest legal buffer;
Base64 content longer than 11,190,284 characters) is `out-of-range`, decided from the length alone, before
any allocation. Decoding otherwise yields one buffer in the SafeTensors format (the tensor file format of
Hugging Face's `safetensors` project): an eight-byte unsigned little-endian header length, a UTF-8 JSON header,
and a contiguous tensor byte buffer.

SafeTensors supplies the otherwise repeated architectural decisions: an explicit dtype, shape and data
offsets; little-endian numbers; packed C-order data; checked buffer boundaries; and no executable object
serialization. The embedding profile narrows it as follows:

- The header length (the eight-byte prefix) is at most 4,096 and at most the number of bytes that follow the
  prefix; a larger value is `invalid-form`, and a reader checks it before reading or parsing the header. The
  normal header is at most 80 bytes, so the cap leaves room for whitespace and padding in other producers' output
  while stopping a hostile prefix from making a reader allocate or scan a huge buffer.
- The top-level header has exactly one entry, named `embedding`, and no `__metadata__` entry. Duplicate or
  unknown entries and duplicate JSON keys are errors.
- The tensor entry has exactly `dtype`, `shape` and `data_offsets`. Accepted inputs may place keys in
  either JSON object in any order; the normal form fixes the order below.
- `shape` is `[n]`, where `n` is a positive integer. `[0]` is `out-of-range`; `[]` and every other rank are
  `invalid-form`.
- Numbers in `shape` and `data_offsets` are plain base-10 integers: digits only, with no sign, fraction or
  exponent, and no leading zero except `0` itself. Any other spelling (`2.0`, `1e1`, `02`) is `invalid-form`.
  An integer too large for 64 unsigned bits is `out-of-range` in `shape` and `invalid-form` in
  `data_offsets`. `dtype` is a JSON string, compared exactly (case-sensitively) after JSON unescaping. The
  header's only whitespace is JSON's four whitespace characters (space, tab, line feed, carriage return) between
  tokens, and the header must be valid UTF-8.
- `data_offsets` is `[0, b]`, relative to the tensor byte buffer, and `b` is exactly the byte length implied
  by the dimension and dtype. The decoded buffer ends immediately after those `b` bytes: no prefix, suffix,
  gaps or shared tensor data are accepted.
- The tensor byte buffer is copied into the interpreted value exactly as received. No arithmetic, cast,
  quantization or byte-order change occurs.

The normal content string for a successful interpretation is produced by rebuilding the buffer with the
header below, with the dtype, positive dimension and byte length substituted and with no whitespace except
the trailing padding, then encoding it with the Base64 rules above:

```text
{"embedding":{"dtype":"<dtype>","shape":[<n>],"data_offsets":[0,<b>]}}
```

The header is followed by as many ASCII spaces (0x20) as make its total byte length a multiple of eight,
possibly none. The eight-byte length prefix counts the header including that padding, and the original tensor
bytes follow it. This is the layout that the reference SafeTensors writer emits for a single tensor (compact
JSON, fields in the order `dtype`, `shape`, `data_offsets`, space-padded to eight-byte alignment), so content
produced by a standard producer is normally already in normal form and round-trips byte for byte. The
corpus carries cases generated by the reference writer to hold the normal form to that claim (2.7, 4). Accepted
inputs that differ (another key order, other whitespace, other padding length, or no padding) are converted
to the normal form by interpretation and writing back. The tag's content is not otherwise normalized.

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
diagnostic (a code, a message and a path, as in ADR-005's result shape):

| Code | Condition |
|---|---|
| `invalid-form` | Base64 is malformed, the SafeTensors header or container is malformed (including a header length above 4,096 or beyond the buffer), the profile's required entries are wrong or duplicated, the shape is not one-dimensional, or the stated and actual byte lengths disagree |
| `out-of-range` | The content would decode to more than 8,392,712 bytes, or the dimension is zero, greater than 1,048,576 (2^20) or beyond 64 unsigned bits |
| `unsupported` | The container is structurally valid but its dtype string is not exactly `F16`, `BF16`, `F32` or `F64` (so `f32` is unsupported) |

The maximum dimension is fixed for every package, so that parity does not depend on a platform's memory or
integer width. It is far above the dimensions of current embedding models, and at the widest dtype it bounds a
payload to 8 MiB. Checks run in this order: Base64 syntax, the decoded-size bound, the SafeTensors container
and profile entries, the dtype, the dimension range, and last the stated and actual byte lengths. A dimension above the maximum is therefore
`out-of-range` whatever the buffer length, which lets the corpus test the bound with a small buffer.

`unsupported` extends ADR-004's two-code diagnostic set (`invalid-form` and `out-of-range`) for tags whose
container is valid but whose inner encoding is outside the supported set. The message names the failed check.
The path locates the tagged value in the value being interpreted, as it
does for the other known tags. A recognized tag with bad content is never reduced to an unknown tagged string
by the interpretation itself; callers that choose optional tag support can still retain the original `Value`.

### 2.5 Equality remains exact, at both layers

A tagged value continues to compare by exact tag and content under the value model. Two accepted inputs that
differ only in JSON key order, JSON whitespace, or header padding are unequal tagged
values even when their normal content strings match. Because the normal form is the reference writer's own
output (2.2), this arises only for content from a non-standard or hand-edited producer; a consumer that wants
one content string per vector normalizes by interpreting and writing back.

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

The Haskell implementation has been deferred; when its package is added, it will join with the same
byte-buffer `Embedding` shape.

Conversions to NumPy, framework tensors, JavaScript typed arrays or other native vector libraries are optional
extras, not the standard mapping. An extra that converts to ordinary host floats must use the existing loss
reporting and can never silently change the dtype. Ordinary gram arrays do not implicitly become embeddings.

Each native package implements the restricted reader and normal writer without a mandatory external
SafeTensors dependency. A package may use such a dependency internally when it can enforce every profile
rule, including duplicate-key and exact-length checks.

The standard JSON parsers of TypeScript (`JSON.parse`), Python (`json`) and Go (`encoding/json`) silently keep
the last of two duplicate keys, so a package cannot rely on its default parser to reject them. The header is
small and its shape is fixed, so each package checks duplicates with a strict parser for this one object shape
(or a parser option that reports them), and the corpus tests both nesting levels (2.7).

### 2.7 The conformance corpus tests the profile

The corpus (`conformance/tags.gram`, `conformance/mappings.gram`) holds accepted cases for every rule in 2.2 and
2.3, including output of the reference writer and non-normal inputs with their normal content strings, and a
rejected case for every `invalid-form`, `out-of-range` and `unsupported` condition above, at each boundary. Which
cases exist so far is recorded in `conformance/README.md`. Every language runs the same cases; native mapping
cases record `Embedding`/`GramEmbedding` as exact in both directions.

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

3.9. **One tag per dtype (`embedding32`, `embedding16`, ...) with raw Base64 bytes.** Rejected: it needs no
container and states the dtype, but the dimension is only implied by the byte count, so a payload truncated by
a whole number of elements cannot be detected. The set of known tags would also grow with every new encoding,
each a breaking change for unknown-tag consumers.

## 4. Consequences

- Known-tag dispatch and the specification grow from ten to eleven tags. Existing ``embedding`...` `` values
  that consumers previously treated as unknown can now fail known-tag interpretation, which is a breaking
  change under the known-tag version policy.
- Base64 expands the payload by one third, and the SafeTensors prefix and header add about 80 bytes. A
  1,536-dimension `F32` embedding is about 8.3 KB of text.
- All five packages need a small binary reader, a canonical writer and the byte-buffer typed value. No ML
  dependency is needed. The profile rejects some otherwise valid SafeTensors files by design.
- Non-normal accepted content compares differently as a tagged value after conversion through the typed
  native value. This is rare, since the normal form is what the reference writer emits. Consumers that must
  retain the exact spelling retain the original `Value`, as with other known tags.
- The normal form's match with the reference writer is a claim about an external tool. It was checked on
  2026-11-15 against the `safetensors` Python package 0.8.0 (`safetensors.numpy.save`) for `F32`, `F64` and
  `F16`: compact JSON, fields in the order `dtype`, `shape`, `data_offsets`, space-padded to eight-byte
  alignment, with the length prefix counting the padding. `BF16` was not generated (NumPy has no bfloat16) and
  is assumed to behave the same, since the header code does not depend on the dtype. The outputs are the
  corpus cases `tag-embedding-ref-f32`, `-f64` and `-f16`, whose comment in `tags.gram` records the version. A
  later writer change does not alter this tag's form.
- Quantized and binary embeddings remain unsupported. Adding them requires a later ADR covering scales,
  ranges, zero points or bit packing, as appropriate.
- Follow-up work is to amend `spec/tags.md`, add the diagnostic and mapping vocabulary, add corpus cases,
  implement the five native interpreters, and update ADR-004's known-tag count and diagnostic codes as well as
  ADR-006's known-tag mapping prose. Tree-sitter-gram's known-tag documentation should link to the updated
  shared specification.

## 5. Related

- RFC-004 (resolves questions 7.1 to 7.6).
- ADR-002 (tagged-value equality), ADR-004 (known-tag interpretation), ADR-005 (diagnostics), ADR-006
  (host-language mappings), ADR-001 (conformance cases).
- [SafeTensors format](https://github.com/huggingface/safetensors#format) and [RFC 4648 Base64](https://www.rfc-editor.org/rfc/rfc4648#section-4).
