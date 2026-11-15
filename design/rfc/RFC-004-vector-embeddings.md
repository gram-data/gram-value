---
id: RFC-004
title: Vector Embeddings as Opaque Typed Tensors
status: draft
created: 2026-11-15
adrs: [ADR-008]
---

# RFC-004: Vector Embeddings as Opaque Typed Tensors

## 1. Summary

A vector embedding is the fixed-length list of numbers that a machine-learning model produces to represent
a piece of text, an image or another object, and that a search or retrieval system compares with other such
lists by distance. The numbers are the visible part of a more structured value: an embedding also has a
dimension (its length) and an element encoding (how each number is stored, such as a 32-bit or 16-bit
floating-point format, or a quantized integer or bit-packed form). Two programs that receive the same
embedding need that structure as well as the numbers, because the same decimal digits do not name the same
stored value under two encodings, and because a consumer that allocates or computes with the vector needs to
know which encoding it received.

Gram writes values as human-readable text: numbers, arrays, maps and tagged strings. An embedding written
as an ordinary array of decimals would be a long list that a person or agent cannot usefully read or edit,
and the text would not say which element encoding produced it. The proposal is a new known tagged string,
whose content is an opaque, self-describing transport of one vector: it carries the dimension and the
element encoding with the values, and it is meant to be copied, stored and interpreted whole rather than
read or edited in place. Gram's ordinary arrays stay the form for vectors that a person does want to write.

The tag joins gram's known-tag layer, so a consumer that does not know it passes the tagged value through
unchanged and a consumer that knows it validates the content and maps it to a native typed value. The tag's
name, content form, accepted element encodings, validation rules, equality and host-language mappings are
for child ADRs; this RFC settles the representation strategy and its scope.

## 2. Problem / Motivation

**An array of numbers does not carry what an embedding means.** The value model defines a `decimal` as a
64-bit float (`spec/values.md` 2.3) and an `array` as a sequence of any values, so an embedding written as
an array would arrive with every element widened to that float. Models and stores use other encodings —
16-bit floating point, quantized 8-bit integers, bit-packed binary vectors — and an interchange format for
embeddings records the encoding and dimension with the payload. Written as decimal text, `0.1` produced as a
32-bit float and `0.1` produced as a 16-bit float are the same text and the same gram value; the precision
the producer chose, and whether the digits are a shortened spelling of a half-precision value, is gone.

**The text would not survive contact with a host.** A 1,536-dimension embedding is 1,536 decimal literals,
plus separators, in a property value. JavaScript numbers, Python floats and Rust `f64` all hold a 64-bit
float, but a native half-precision tensor or an 8-bit quantized vector does not. A host mapping would either
report a loss on every element or convert the vector to a form the retrieval system cannot use as-is. The
mapping tables that the host-language RFC defines (RFC-002) have no entry today for a value whose useful
native form is a typed vector, not a list of numbers.

**The form invites the wrong work.** Gram text is meant to be edited by people and tools: arrays are for
values a person writes, and linters and formatters work over them. An embedding is machine output, often
thousands of elements long. Nothing in the array form says that its elements must stay together or that an
edit to one element breaks a value the producer computed; the cost is silent corruption, accidental
truncation, and agents spending effort reading or rewriting numbers they cannot meaningfully change. An
embedding's consumer needs the same bytes back, not a friendlier spelling of them.

**Every consumer would invent the same carrier.** A system that stores embeddings in gram today has three
choices, none shared: put the numbers in an array and lose the encoding; hide the payload in a string with a
private encoding, which another tool cannot interpret; or store the vector outside gram and keep only a
reference, which splits the value across systems. Each consumer that invents its own string form adds
another mapping, another loss policy and another corpus of private cases.

## 3. Goals

3.1. Represent a vector embedding as one gram value that carries its dimension and element encoding with
its data, so two programs given the same text agree on the typed tensor, not just on a list of numbers.

3.2. Keep the content opaque: the form is written and read whole by machines. It is not meant for direct
editing, and no part of the design optimizes for a person or agent reading or changing individual elements.

3.3. Use a tagged string as the carrier, so the grammar stays unchanged and a consumer that does not know
the tag passes the value through unchanged, as with any unknown tag.

3.4. Make the tag a known tag in the shared interpretation layer: one validation function per package,
shared diagnostics, one native typed value per host language, and conformance cases in the language-neutral
corpus.

3.5. Preserve the payload exactly on a round trip through the tagged value, so the encoding a producer
wrote is the encoding a consumer receives.

## 4. Non-Goals

4.1. Ergonomics for authoring or editing embeddings. A person who wants a small readable vector writes an
ordinary array; the new tag is for machine-produced values transported as a unit.

4.2. A new value kind. The set of kinds is closed and changing it is a breaking change to every consumer;
a tagged string carries this value inside the existing model.

4.3. Choosing the payload container, byte layout, element-encoding names, compression or transport
references. These are child-ADR decisions.

4.4. General tensors of rank two and above. The subject is the rank-one vectors that embedding systems
exchange; a later RFC can widen the tag or add another if a need appears.

4.5. Similarity search, model metadata, versioning or provenance. A consumer stores those beside the value,
in element properties or its own tables.

## 5. Proposed Approach (high-level)

### 5.1 A known tag, not a new kind

The value is a `tagged` value in the model, as a date or a UUID is. The tag joins the known-tag set: each
language package gets one function that takes the tagged value, validates the content and returns a typed
native value, and a dispatch helper over the known tags. An unknown-tag consumer is unaffected, and adding
the tag is the same kind of extension the temporal tags made.

### 5.2 The content is a self-describing tensor payload

The tag's content encodes one vector together with the two facts a consumer needs to interpret it: the
dimension and the element encoding. The content is machine-formed and machine-read — not a spelling of each
number for a person — and its exact syntax is an ADR. The design principle is that the payload travels as a
unit, so a consumer can validate, store and write it back without inspecting the numbers.

### 5.3 Interpretation validates; transport preserves

The tagged value's meaning is its tag and content, unchanged, which keeps equality and text writing defined
for every value (RFC-001 2.5, 7.7). The interpretation function is where form and range validation happen,
as with the other known tags: it returns the typed vector or a diagnostic, and it never repairs a malformed
payload. What a successful interpretation returns, and how it compares two vectors, are ADR questions.

### 5.4 Ordinary arrays remain the readable form

The value model does not stop treating an array of numbers as an array. Conversions between an array and an
embedding tag, where a host offers them, are explicit and report their losses, like any other mapping; they
are not part of equality and never happen implicitly.

## 6. Alternatives Considered

6.1. **An array of decimals.** Rejected: it loses the element encoding, widens every element to the model's
64-bit float, and offers a form that invites element-wise edits to a value that only means something whole.

6.2. **A new value kind.** Rejected: the kind set is closed by design and a new kind breaks every
consumer's exhaustive handling, for a value the tagged-string kind already carries.

6.3. **A consumer-defined tag.** Rejected: each system's private string form would repeat the work the
shared value layer exists to remove, with no shared validation, native mappings or conformance cases.

6.4. **A map of metadata and an array.** Rejected: it keeps the numbers as editable text and still stores
the encoding apart from them, so the two can disagree; it is also larger than the opaque form for the same
data.

6.5. **An out-of-band reference only.** Rejected: a pointer to a vector stored elsewhere is a different
value — useful to some consumers, but it does not transport the embedding in gram, and it splits one
property across systems.

## 7. Open Questions

ADR-008 resolves all six questions below (name, content form, accepted encodings, validation and diagnostics, equality, host mappings). They are kept as the record of what the RFC left to it.

7.1. What is the tag's name, and does the name scope it to embeddings or to vectors generally?

7.2. What is the content form: which container, which text encoding of it, and how are the dimension and
element encoding stated inside it?

7.3. Which element encodings does the first version accept — which floating-point widths, and whether
quantized integer or bit-packed forms are in scope — and what does a consumer do with an encoding it does
not support?

7.4. What are the validation rules and diagnostic codes, including the form checks, the dimension check
against the payload length, and whether non-finite values are accepted or rejected?

7.5. What does equality mean for an embedding — whether two tagged values with the same tag and content
are equal regardless of how a native library normalizes them, and what the conformance corpus compares?

7.6. What is each host language's native value, which conversions are exact, lossy or unsupported, and what
does a package without a native half-precision or quantized vector type return?

## 8. ADRs

- [ADR-008](../adr/ADR-008-embedding-tag.md): The `embedding` Tag as Base64-Encoded SafeTensors.
