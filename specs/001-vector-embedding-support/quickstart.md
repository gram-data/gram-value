# Quickstart & Validation Guide: Vector Embedding Support

**Feature**: `001-vector-embedding-support` | **Date**: 2026-11-15

Run these commands from the repository root after implementing the spec, conformance corpus, and each language package to verify parity and zero skips.

## 1. Regenerate & Verify Mapping Docs (if applicable)

```bash
python3 scripts/gen-mapping-docs.py
```

## 2. Run Each Package's Test & Conformance Suite

### Rust
```bash
cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

### TypeScript
```bash
cd typescript && npm run typecheck && npm run build && npm test
```

### Python
```bash
cd python && pytest
```

### Go
```bash
cd go && go test ./...
```

### Swift
```bash
swift test
```

## 3. Verify Parity Matrix & Zero Embedding Skips

- Confirm `skip-embedding-*` entries are removed from:
  - `rust/skips.gram`
  - `typescript/skips.gram`
  - `python/skips.gram`
  - `go/conformance/skips.gram`
  - `swift/Tests/skips.gram`
- Confirm `python3 scripts/gen-parity-matrix.py` (when given all five `conformance-results.gram` outputs) reports 100% pass with 0 skips for all `tag-embedding-*` and mapping cases.
