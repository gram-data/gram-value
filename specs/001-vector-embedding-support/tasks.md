# Tasks: Vector Embedding Support (ADR-008)

**Input**: Design documents from `/specs/001-vector-embedding-support/`
**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/embedding-tag-contract.md](contracts/embedding-tag-contract.md)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story across all five active language packages (Rust, TypeScript, Python, Go, Swift; Haskell deferred).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (`[US1]`, `[US2]`, `[US3]`)
- Every task includes exact file paths and verbatim constraints from `data-model.md`

---

## Phase 1: Setup (Baseline Verification)

**Purpose**: Confirm all five existing language test suites pass before adding `embedding` support

- [X] T001 [P] Run baseline Rust checks and tests (`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`) in `rust/`
- [X] T002 [P] Run baseline TypeScript checks and tests (`npm run typecheck && npm run build && npm test`) in `typescript/`
- [X] T003 [P] Run baseline Python tests (`pytest`) in `python/`
- [X] T004 [P] Run baseline Go tests (`go test ./...`) in `go/`
- [X] T005 [P] Run baseline Swift tests (`swift test`) in `swift/`

---

## Phase 2: Foundational (Shared Specification & Conformance Vocabulary)

**Purpose**: Update the shared specification, design cross-references, and conformance vocabulary that all language packages and corpus cases depend on

**⚠️ CRITICAL**: Complete this phase before finalizing language implementations and conformance cases

- [X] T006 Update `spec/tags.md` to add `embedding` as the 11th known tag with its strict RFC 4648 Base64 + restricted SafeTensors profile, normal content string, and `unsupported` diagnostic code
- [X] T007 [P] Update `spec/results.md` to document the `unsupported` diagnostic code alongside `invalid-form` and `out-of-range`
- [X] T008 [P] Update `conformance/vocabulary.gram` to add `Embedding` (for `rust`, `typescript`, `python`, `go`) and `GramEmbedding` (for `swift`) native type names
- [X] T009 [P] Update `design/adr/ADR-004-known-tags.md` (known-tag count 10 → 11 and diagnostic code reference) and `design/adr/ADR-006-host-language-mappings.md` (known-tag mapping prose) per ADR-008 §4

**Checkpoint**: Shared specification and conformance vocabulary are ready — user story implementation can proceed in parallel across packages

---

## Phase 3: User Story 1 - Interpret and Validate `embedding` Tagged Strings (Priority: P1) 🎯 MVP

**Goal**: Every active package interprets `embedding` tagged values (both directly and via known-tag dispatch), enforcing strict RFC 4648 Base64, pre-allocation size bounds, restricted SafeTensors JSON header rules, duplicate-key rejection, and the exact 6-step validation check order.

**Independent Test**: Pass accepted (`F16`, `BF16`, `F32`, `F64`) and rejected (`invalid-form`, `out-of-range`, `unsupported`) `embedding` tagged values to each package's `embedding` interpreter and known-tag dispatch and verify the decoded dtype, dimension, bytes, normal content string, or exact diagnostic code.

### Implementation for User Story 1

- [X] T010 [P] [US1] Implement strict RFC 4648 Base64 decoder (non-empty, `+`/`/`, required `=` padding, no whitespace, zero unused pad bits per RFC 4648 §3.5; reject > 11,190,284 chars or > 8,392,712 decoded bytes as `out-of-range` before allocation) and restricted SafeTensors reader (8-byte LE header length <= 4,096 and <= remaining bytes; valid UTF-8; strict JSON parser rejecting duplicate keys at both levels, `__metadata__`, extra keys, non-1D `shape`, non-plain base-10 integer tokens like `2.0`/`1e1`/`02`/`-1`; exact case-sensitive `F16`/`BF16`/`F32`/`F64` dtype check; dimension `1 <= n <= 1,048,576`; `data_offsets == [0, n * width]`; no trailing bytes; exact 6-step check order) + dispatch in `rust/src/tags.rs`
- [X] T011 [P] [US1] Implement strict RFC 4648 Base64 decoder, pre-allocation bounds (`> 11,190,284` chars / `> 8,392,712` bytes → `out-of-range`), custom strict JSON header scanner (do NOT use `JSON.parse` alone because it overwrites duplicate keys and accepts floats/exponents), restricted SafeTensors validation in exact 6-step check order, and known-tag dispatch in `typescript/src/tags.ts`
- [X] T012 [P] [US1] Implement strict RFC 4648 Base64 decoder, pre-allocation bounds (`> 11,190,284` chars / `> 8,392,712` bytes → `out-of-range`), custom strict JSON header scanner (rejecting duplicate keys and non-plain base-10 integer tokens), restricted SafeTensors validation in exact 6-step check order, and known-tag dispatch in `python/src/gram_value/tags.py`
- [X] T013 [P] [US1] Implement strict RFC 4648 Base64 decoder, pre-allocation bounds (`> 11,190,284` chars / `> 8,392,712` bytes → `out-of-range`), custom strict JSON header scanner (do NOT use `encoding/json` alone because it overwrites duplicate keys), restricted SafeTensors validation in exact 6-step check order, and known-tag dispatch in `go/tags.go`
- [X] T014 [P] [US1] Implement strict RFC 4648 Base64 decoder, pre-allocation bounds (`> 11,190,284` chars / `> 8,392,712` bytes → `out-of-range`), custom strict JSON header scanner (rejecting duplicate keys and non-plain base-10 integer tokens), restricted SafeTensors validation in exact 6-step check order, and known-tag dispatch in `swift/Sources/GramValue/Tags.swift`

**Checkpoint**: User Story 1 is functional — all five packages validate and interpret `embedding` tagged strings with identical diagnostic codes and check order

---

## Phase 4: User Story 2 - Round-Trip Typed Embeddings and Preserve Exact Equality (Priority: P1)

**Goal**: Every active package exposes `Embedding` (`GramEmbedding` in Swift) with `EmbeddingDtype` (`f16`, `bf16`, `f32`, `f64`), exact byte-buffer equality, verbatim IEEE bit-pattern preservation (signed zero, NaN, infinities), and exact bidirectional mapping to/from `tagged("embedding", ...)` emitting the canonical 8-byte-aligned normal content string.

**Independent Test**: Construct and interpret `Embedding` values (including reference-writer outputs, non-normal inputs, and NaN/Inf/-0.0 payloads), verify exact equality at both `Value` and `Embedding` layers, and verify write-back emits the canonical 8-byte-aligned normal content string.

### Implementation for User Story 2

- [X] T015 [P] [US2] Add `Embedding` (`Vec<u8>`), `EmbeddingDtype`, canonical 8-byte-aligned normal-form writer, exact equality, and bidirectional `tagged("embedding", ...)` mapping (no implicit array conversion) in `rust/src/tags.rs`, `rust/src/convert.rs`, and `rust/src/lib.rs`
- [X] T016 [P] [US2] Add `Embedding` (`Uint8Array`), `EmbeddingDtype`, canonical 8-byte-aligned normal-form writer, exact equality, and bidirectional `tagged("embedding", ...)` mapping (no implicit array conversion) in `typescript/src/tags.ts`, `typescript/src/convert.ts`, and `typescript/src/index.ts`
- [X] T017 [P] [US2] Add `Embedding` (`bytes`), `EmbeddingDtype`, canonical 8-byte-aligned normal-form writer, exact equality, and bidirectional `tagged("embedding", ...)` mapping (no implicit array conversion) in `python/src/gram_value/tags.py`, `python/src/gram_value/convert.py`, and `python/src/gram_value/__init__.py`
- [X] T018 [P] [US2] Add `Embedding` (`[]byte`), `EmbeddingDtype`, canonical 8-byte-aligned normal-form writer, exact equality, and bidirectional `tagged("embedding", ...)` mapping (no implicit array conversion) in `go/tags.go` and `go/native.go`
- [X] T019 [P] [US2] Add `GramEmbedding` (`Foundation.Data`), `GramEmbeddingDtype`, canonical 8-byte-aligned normal-form writer, exact equality, and bidirectional `tagged("embedding", ...)` mapping (no implicit array conversion) in `swift/Sources/GramValue/Tags.swift` and `swift/Sources/GramValue/Native.swift`

**Checkpoint**: User Stories 1 and 2 are complete — all five packages support full interpretation, native `Embedding` representation, exact equality, and canonical round-tripping

---

## Phase 5: User Story 3 - Cross-Language Parity, Specification, and Conformance Corpus (Priority: P2)

**Goal**: Complete the shared conformance corpus for all `embedding` rules and boundaries, enable all five packages' conformance runners to execute them with zero skips, and update generated mapping docs.

**Independent Test**: Run every package's conformance suite and confirm 100% pass rate with zero `skip-embedding-*` entries remaining in any `skips.gram` file.

### Implementation for User Story 3

- [X] T020 [US3] Add complete accepted, non-normal, IEEE bit-pattern, and rejected boundary `TagCase`s (empty content, bad Base64, non-zero pad bits, > 11,190,284 chars, header length 4,097 and > buffer, invalid UTF-8, duplicate keys at both levels, `__metadata__`, extra/missing keys, wrong rank, `shape: [0]`, `shape: [1048577]`, wrong `data_offsets`, trailing/missing bytes, unsupported dtypes) in `conformance/tags.gram`
- [X] T021 [P] [US3] Add bidirectional `Embedding` / `GramEmbedding` mapping cases in `conformance/mappings.gram` and update the embedding case summary in `conformance/README.md`
- [X] T022 [P] [US3] Remove all `skip-embedding-*` entries and verify conformance runner support for `Embedding` / `unsupported` in `rust/skips.gram` (and `rust/` conformance tests)
- [X] T023 [P] [US3] Remove all `skip-embedding-*` entries and verify conformance runner support for `Embedding` / `unsupported` in `typescript/skips.gram` (and `typescript/` conformance tests)
- [X] T024 [P] [US3] Remove all `skip-embedding-*` entries and verify conformance runner support for `Embedding` / `unsupported` in `python/skips.gram` (and `python/` conformance tests)
- [X] T025 [P] [US3] Remove all `skip-embedding-*` entries and verify conformance runner support for `Embedding` / `unsupported` in `go/conformance/skips.gram` (and `go/conformance/` tests)
- [X] T026 [P] [US3] Remove all `skip-embedding-*` entries and verify conformance runner support for `GramEmbedding` / `unsupported` in `swift/Tests/skips.gram` (and `swift/Tests/` conformance tests)
- [X] T027 [US3] Regenerate mapping documentation with `python3 scripts/gen-mapping-docs.py` into `docs/mappings/`

**Checkpoint**: All five active packages pass 100% of the shared `embedding` conformance corpus with zero skips

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final cross-language verification and diff hygiene

- [X] T028 [P] Run full Rust validation (`cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`)
- [X] T029 [P] Run full TypeScript validation (`cd typescript && npm run typecheck && npm run build && npm test`)
- [X] T030 [P] Run full Python validation (`cd python && pytest`)
- [X] T031 [P] Run full Go validation (`cd go && go test ./...`)
- [X] T032 [P] Run full Swift validation (`swift test`)
- [X] T033 Verify clean diff (`git diff --check`) and confirm zero `skip-embedding-*` entries remain across the repository

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — can run immediately in parallel across all 5 packages.
- **Phase 2 (Foundational)**: Can run alongside or right after Phase 1 — updates `spec/` and `conformance/vocabulary.gram` before corpus/package work.
- **Phase 3 (US1 - P1 MVP)**: Depends on Phase 2; all 5 language tasks (`T010`–`T014`) are independent and parallelizable.
- **Phase 4 (US2 - P1)**: Depends on Phase 3 per language (needs `Embedding` interpreter/dtype foundation); all 5 language tasks (`T015`–`T019`) are parallelizable across languages.
- **Phase 5 (US3 - P2)**: `T020`–`T021` depend on Phase 2; `T022`–`T026` depend on `T020`–`T021` and each package's Phase 3 & 4 tasks; `T027` depends on `T021`.
- **Phase 6 (Polish)**: Depends on Phase 5 completion across all 5 packages.

### Parallel Opportunities

- Within Phase 1, Phase 2 (`T007`–`T009`), Phase 3 (`T010`–`T014`), Phase 4 (`T015`–`T019`), Phase 5 (`T022`–`T026`), and Phase 6 (`T028`–`T032`), all five language packages touch disjoint directories and can be implemented and tested completely in parallel.

### Implementation Strategy

1. **MVP First (US1 + US2 in Rust first, or all 5 in parallel)**: Complete Phase 2 (`spec/` + `vocabulary.gram`), then implement US1 + US2 in Rust (`T010`, `T015`) as the first end-to-end validation before/alongside TypeScript, Python, Go, and Swift.
2. **Corpus & Parity Lock-in (US3)**: Expand `conformance/tags.gram` and `conformance/mappings.gram`, remove `skip-embedding-*` from all five `skips.gram` files, and run `quickstart.md` validation across all five toolchains.
