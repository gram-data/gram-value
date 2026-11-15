# Implementation Plan: Vector Embedding Support (ADR-008)

**Branch**: `001-vector-embedding-support` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-vector-embedding-support/spec.md`

## Summary

Implement the `embedding` known tag (11th known tag) across the shared specification, conformance corpus, and all five active language packages (Rust, TypeScript, Python, Go, Swift; Haskell deferred) per accepted [RFC-004](../../design/rfc/RFC-004-vector-embeddings.md) and [ADR-008](../../design/adr/ADR-008-embedding-tag.md). Each package implements a zero-dependency strict RFC 4648 Base64 codec, a restricted SafeTensors reader/writer with duplicate-key and plain-integer JSON header validation, pre-allocation bounds checking, and a byte-buffer `Embedding` (`GramEmbedding` in Swift) native value with exact bidirectional mapping.

## Technical Context

**Language/Version**:
- Rust (edition 2021, MSRV 1.75)
- TypeScript (Node 24 / ES module)
- Python (3.11+ / `hatchling`)
- Go (1.23)
- Swift (5.9 / SwiftPM)

**Primary Dependencies**: Zero runtime dependencies across all five packages (standard library only, matching existing `gram-value` packages).

**Storage**: N/A (in-memory value representation and text/binary serialization).

**Testing**: Native test & conformance runners per package (`cargo test`, `npm test`, `pytest`, `go test ./...`, `swift test`) driven by `conformance/tags.gram` and `conformance/mappings.gram`.

**Target Platform**: Cross-platform libraries (Linux/macOS/Windows, Node/browser for TypeScript).

**Project Type**: Multi-language library monorepo with shared specification and conformance corpus.

**Performance Goals**: Pre-allocation bounds check rejects oversized inputs (> 11,190,284 Base64 chars or > 4,096-byte header) in O(1) before buffer allocation or JSON parsing.

**Constraints**: Zero external SafeTensors/Base64 dependencies; strict check ordering (ADR-008 §2.4); duplicate JSON key rejection even where host `JSON.parse`/`json`/`encoding/json` silently overwrite; exact byte preservation without numeric normalization.

**Scale/Scope**: 1 new known tag (`embedding`), 1 new diagnostic code (`unsupported`), 4 dtypes (`F16`, `BF16`, `F32`, `F64`), max dimension $2^{20}$ (1,048,576), 5 active language packages.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

`.specify/memory/constitution.md` is currently an unfilled template in this repository; project governance is established by the accepted RFCs/ADRs (`RFC-001`–`RFC-004`, `ADR-001`–`ADR-008`):
- **Native zero-dependency packages (ADR-005)**: Satisfied — no external SafeTensors or Base64 dependencies added.
- **Language-neutral conformance corpus parity (ADR-001, RFC-003)**: Satisfied — all behavior is specified in `spec/` and tested via `conformance/` across all 5 active packages.
- **No implicit lossy conversions (RFC-002, ADR-006)**: Satisfied — `Embedding` maps strictly to/from `tagged("embedding", ...)`; ordinary arrays are never implicitly converted.

## Project Structure

### Documentation (this feature)

```text
specs/001-vector-embedding-support/
├── plan.md              # This file (/speckit-plan output)
├── research.md          # Phase 0 output (/speckit-plan output)
├── data-model.md        # Phase 1 output (/speckit-plan output)
├── quickstart.md        # Phase 1 output (/speckit-plan output)
├── contracts/
│   └── embedding-tag-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
spec/
├── tags.md              # Add `embedding` tag profile, normal form, and `unsupported` diagnostic
└── results.md           # Document `unsupported` diagnostic code if listed

design/adr/
├── ADR-004-known-tags.md            # Update known-tag count (10 -> 11) and diagnostic codes
└── ADR-006-host-language-mappings.md # Update known-tag mapping prose

conformance/
├── vocabulary.gram      # Add Embedding / GramEmbedding native type names
├── tags.gram            # Full accepted/rejected/boundary TagCases for embedding
├── mappings.gram        # Bidirectional MappingCases for Embedding / GramEmbedding
└── README.md            # Update embedding case summary

rust/
├── src/{tags.rs,convert.rs,lib.rs}
└── skips.gram           # Remove skip-embedding-* entries

typescript/
├── src/{tags.ts,convert.ts,index.ts}
└── skips.gram           # Remove skip-embedding-* entries

python/
├── src/gram_value/{tags.py,convert.py,__init__.py}
└── skips.gram           # Remove skip-embedding-* entries

go/
├── {tags.go,native.go}
└── conformance/skips.gram # Remove skip-embedding-* entries

swift/
├── Sources/GramValue/{Tags.swift,Native.swift}
└── Tests/skips.gram     # Remove skip-embedding-* entries
```

**Structure Decision**: Multi-language monorepo layout matching existing known-tag implementations (`tags.*` for tag interpretation/validation/normal-form, `convert.*`/`native.*` for `Embedding` type and bidirectional mapping, and `skips.gram` updated as each package passes).

## Complexity Tracking

No constitution or architectural violations; no extra complexity tracking required.
