---
id: ADR-005
title: Native Packages, a Plain Sum Type and a Common Result Shape
status: proposed
rfcs: [RFC-001, RFC-003]
created: 2026-11-14
---

# ADR-005: Native Packages, a Plain Sum Type and a Common Result Shape

## 1. Context

`gram-value` has to exist in Rust, TypeScript and Python first, then Go, Swift and Haskell. A library in several languages can be written once in one language and bound to the others (a Rust core with WebAssembly for JavaScript and PyO3 for Python), or written natively in each. The choice decides what a user installs, how much each call costs, and how parity is kept.

SPIKE-002 measured the options on creating 1,000 values, adding them to a list, mapping them to new values and reading them back. In TypeScript a Rust core behind WebAssembly cost 0.29 to 0.61 microseconds per value against 0.024 for plain objects (11 to 25 times), needed a `BigInt` for every 64-bit integer, handles freed by hand and asynchronous startup in a browser. In Python a PyO3 core cost 0.25 to 0.43 against 0.52 for native, so it was no slower, but needs a compiled wheel for each platform. A map run inside the core in one call cost half of the same map calling back to the host per element, which is why a pipeline system was considered and dropped (RFC-001 7.16).

Two further forces apply. Values come back from operations that can find problems (an invalid date, a lossy conversion), and each language has its own idiom for that (`Result` in Rust, exceptions or result objects in TypeScript and Python, `Either` in Haskell). And the value type has several downstream users (pattern-rs, relate-gram, schema tools), who need to wrap it in their own abstractions.

## 2. Decision

### 2.1 Every package is native

Each language package implements the model, the canonical writer and the known-tag functions in that language. There is no shared core and no compiled extension: a TypeScript user installs plain JavaScript, a Python user a pure wheel. The packages are Rust, TypeScript and Python first; Go, Swift and Haskell join by passing the conformance corpus (RFC-003 5.9). A Rust core exposed to Python through PyO3 remains available later as an optimization and would not change the contract.

### 2.2 Haskell is a peer

The Haskell package is held to the same conformance corpus as the others and has no special standing as a reference. pattern-hs can remain the reference for pattern-rs's pattern operations, which is a different layer.

### 2.3 Parity rests on the conformance corpus

The shared definition of correct behavior is the corpus (ADR-001), not an implementation. Where logic must match across languages (the canonical writer of ADR-003 and the tag functions of ADR-004), each package contains its own and is tested against the same cases.

### 2.4 `Value` is a plain sum type

`Value` has no monadic or functor interface in its contract. It is the eleven-way sum of ADR-002 in each language's natural form (a Rust enum, a TypeScript discriminated union, a Python class hierarchy or dataclasses with a kind field, a Haskell data type). A small structural traversal (a fold or map over a value's parts) is offered only when the corpus can test it identically in every language. Operations on values are plain functions.

### 2.5 One result shape

An operation that can find problems (interpreting a tag, converting a value to a host value, constructing from parts) returns a result with the same logical shape in every language: either a value with no problems, or a value together with a list of diagnostics (for a mapping, losses), or no value and a list of diagnostics. A diagnostic carries a code, a message and, where it applies, the path to the part of the value concerned. Results from independent values accumulate their diagnostics instead of stopping at the first. Each language adds idiomatic helpers on top: `Result` and `?` in Rust, result objects in TypeScript and Python, a monad in Haskell. The exact field names and the path format are in the mapping decisions (RFC-002 7.2) and the `spec/` text.

## 3. Alternatives Considered

3.1. **A Rust core with WebAssembly for TypeScript and PyO3 for Python.** Rejected: SPIKE-002 measured WebAssembly at 11 to 25 times plain objects for TypeScript, with `BigInt` integers, manual handles and asynchronous startup. For Python the speed is fine but each platform needs a wheel, which is a distribution burden for a library whose logic is small (about 800 lines of writing and validation).

3.2. **A Rust core with WebAssembly only for TypeScript and Python left native.** Rejected for the same TypeScript reasons.

3.3. **Generate every language's types and cases from a schema.** Not rejected: it is the main candidate for keeping the ports in step, and it fits native packages. It is a tool choice for a later ADR, and the corpus remains the contract either way.

3.4. **One reference implementation with ports checked against it.** This is how pattern-rs relates to pattern-hs. Rejected: it makes one language's tests and bugs the specification; the corpus gives every language the same standing.

3.5. **A Value that is a monad or functor in each language.** Rejected: it ties the type to one abstraction that does not carry across languages, and downstream projects can wrap a plain type in whatever they prefer.

3.6. **A functional pipeline system that pushes operations into a core.** Closed (RFC-001 7.16): with native packages there is no boundary to cross.

## 4. Consequences

- The known-tag validators and the writer exist once per language, about 800 lines each, and nothing but the corpus keeps them aligned, so a behavior change starts with failing corpus cases (RFC-003 5.8).
- Users of every language get a package with no native build, no startup step and values in the language's usual types, and pay no boundary cost.
- Adding a language costs a full implementation, not a binding. The corpus tells the new package what is missing from its first run.
- The result shape is a convention the corpus can test only where a case states the diagnostics or losses, so the shape's field names must stay stable across languages.
- A consumer in Rust that wants a database's speed for bulk work can still build on the Rust package directly; nothing here prevents a later core.
- Follow-up work: the first package (Rust) and its runner; the repository layout per language (RFC-003 5.5); the diagnostic path format (RFC-002 7.2).

## 5. Related

- RFC-001 (resolves questions 7.8 partly, 7.10, 7.11, 7.16, and the plain-sum-type commitment in 4.6), RFC-003 (the corpus, tiers and one repository).
- ADR-001 (the corpus), ADR-002 (the kinds), ADR-003 (the writer), ADR-004 (the tags).
- SPIKE-002 (the measurements).
