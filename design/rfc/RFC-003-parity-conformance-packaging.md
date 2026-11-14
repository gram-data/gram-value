---
id: RFC-003
title: Parity, Conformance and Packaging Across Languages
status: draft
created: 2026-11-14
adrs: []
---

# RFC-003: Parity, Conformance and Packaging Across Languages

## 1. Summary

A library that exists in several languages is only useful if the languages agree. For a value model that means a
value written in gram text reads as the same value, writes back as the same text, and converts to a native value
with the same stated loss, whichever language does it. Keeping that true over time is a testing problem and a
release problem: someone must be able to see, for every language and every behavior, whether it conforms, and a
user of one language must be able to install a version of the library that matches the others.

The proposal has three parts. A conformance corpus, a set of language-neutral cases, is the contract that every
language package runs in continuous integration, and a parity matrix generated from the results shows what
conforms. One repository holds the specification, the corpus and the packages of the first-tier languages (Rust,
TypeScript and Python), so a change to the corpus and the code it affects land together. And a release process
publishes the packages to each language's registry from one tag, with a version policy that keeps the languages
in step. Further languages (Go, Swift and Haskell) join by passing the corpus.

The scope and direction are the proposal; the corpus format, the version policy and the release mechanics are left
to later decisions.

## 2. Problem / Motivation

**Parity is currently checked by hand, if at all.** pattern-rs, a Rust library of pattern and graph structures built
on gram, keeps a parity matrix in `proposals/language-parity-proposal.md`: a table of which operations the Haskell
reference, Rust, TypeScript and Python each have, written by hand and dated 2026-04-16. It records which
operations exist, not whether they behave the same, and nothing regenerates it when code changes. SPIKE-001 found
the behavior differences that such a table cannot show: the same gram text becomes a different value in different
languages (integers of three different widths, a decimal that turns into an integer, a map misread at the host
boundary).

**A shared corpus has worked for the grammar.** tree-sitter-gram keeps its grammar's test corpus (`test/corpus`,
one case per input with its expected syntax tree) in one place, and its bindings for C, Go, Node, Python, Rust
and Swift live in the same repository. relate-gram, which loads gram into the Relate database, runs every corpus
case against its own expectations file, which records for each case whether it loads and with which error. When
the grammar gained empty arrays and decimal measurements (version 0.7.0) the new corpus cases surfaced exactly
what had to change in relate-gram. A corpus about values would do the same for the value model, one level up.

**The behavior to keep in step is more than syntax.** The cases a value library needs span four kinds of
behavior: what a value is and how two values compare, the canonical text written for a value, the
interpretation and validation of known tags (SPIKE-001, RFC-001), and each host-language mapping with its loss
(RFC-002). A syntax-tree corpus covers none of those.

**Native implementations duplicate logic by design.** SPIKE-002 measured WebAssembly in TypeScript at 11 to 25
times the cost of plain objects and, for Python, a PyO3 core only as fast as native, so RFC-001 leans towards a
native TypeScript package and leaves Python open. Text writing and the tag validators (about 800 lines in
relate-gram) would then exist once per native language, and nothing but tests keeps them aligned.

**Several registries each have their own conventions.** The first tier alone publishes to crates.io, npm and
PyPI; the next tier adds Go modules and Swift packages (both versioned by repository tag) and Hackage. Their
version rules, naming rules and credentials differ, and a version of one package says nothing about the others
unless a policy ties them together. relate-gram's release workflow, which builds binaries and gates a crates.io
publish behind a repository setting, is a one-language precedent for the automation and nothing more.

## 3. Goals

3.1. Define a conformance corpus that is language-neutral, readable by a person, and diffable, with cases for each
of the four kinds of behavior above: values, canonical text, known-tag interpretation, and host mappings.

3.2. Require every first-tier package to run the whole corpus in continuous integration, and generate a parity
matrix (languages by cases) from the results, so that parity is measured and not asserted.

3.3. Make every skipped case visible: a package that does not pass a case declares the skip and the reason, and the
matrix shows it.

3.4. Put the specification, the corpus and the first-tier packages in one repository, so a change to behavior
changes the corpus and the packages in one commit.

3.5. Publish the first-tier packages to crates.io, npm and PyPI from one release, in an order that never leaves a
package depending on an unpublished version.

3.6. State how a version number relates across the languages, and what a user may rely on before 1.0.

3.7. Let a further language join by passing the corpus, with a stated process.

## 4. Non-Goals

4.1. Choosing whether each language is native or bound to a shared core. That is RFC-001's question 7.11.

4.2. Defining the value model, the text-writing rules or the host mappings. The corpus records them; other
documents decide them.

4.3. Testing the parser. tree-sitter-gram owns its own corpus, and this project relies on it.

4.4. Performance benchmarks as part of the conformance contract. They are measured when a decision needs them, as
SPIKE-002 did.

4.5. A documentation website. The parity matrix is published; a full site is separate work.

## 5. Proposed Approach (high-level)

### 5.1 The corpus is the contract

A case states an input and the behavior every conforming package must show. The kinds: (a) value cases, where a
value is constructed or read and compared; (b) text cases, where a value is written as canonical gram text; (c)
interpretation cases, where a known tag's content is accepted, rejected, or reported with a specific diagnostic;
(d) mapping cases, where a value converts to a native value with an expected entry class and loss in each
language (RFC-002). Cases for the same behavior stay together, so a reader sees all the languages' expectations at
once.

### 5.2 Value cases need a way to describe values without a parser

`gram-value` does not parse (RFC-001 4.1), so a package cannot read gram text to build the input of a case. A case
therefore describes values some other way. Candidates: a small structured description used only inside the
corpus, which is test data and not a serialization to ship (RFC-001 4.4 excludes a second serialization of
values); or gram text, parsed in each language's tests with that language's tree-sitter-gram binding and a
test-only lowering. The trade-offs are question 7.1.

### 5.3 Every package runs everything, and skips are declared

Each first-tier package runs the whole corpus in CI. A case a package cannot yet pass is listed in that package's
own skip list with a reason, and a skip is a visible gap in the matrix and not a hidden omission. The matrix is
generated from the runs and published with the repository.

### 5.4 Tiers

The first tier (Rust, TypeScript, Python) passes the corpus without skips, or with skips that an ADR has
accepted. A next-tier language (Go, Swift, Haskell) is welcome to appear in the matrix as soon as it runs the
corpus, with its gaps visible, and is promoted when it passes.

### 5.5 One repository

The specification, the corpus and the language packages live in one repository, in the layout the project already
has (`spec/`, `conformance/`, one directory per language). The corpus and every package move together, and
tree-sitter-gram, which keeps its bindings with its grammar, is the precedent. A package can still be split out
later if it needs its own cadence.

### 5.6 A version policy that keeps languages in step

Candidate direction: one version number for the project, shared by all packages and the corpus, bumped together
whenever the corpus changes in a way that affects behavior. Before 1.0, a minor version may change behavior. A
fix confined to one package may ship as a patch of that package only if the policy allows it (question 7.6).

### 5.7 Release from one tag

A release workflow publishes every first-tier package from one tag, builds and verifies each from the published
artifact, and publishes in dependency order. Registry credentials are held by the workflow and not by a person
(question 7.8). A release that fails partway leaves a documented recovery.

### 5.8 Changes start in the corpus

A change to behavior (a new value kind, a new known tag, a changed mapping) first adds or changes corpus cases
that fail, then changes the packages until they pass. A grammar release in tree-sitter-gram triggers a review of
the corpus against the new grammar, as the 0.7.0 release did for relate-gram.

### 5.9 Adding a language

A language joins by adding a package directory, a runner for the corpus, a skip list and a release job. The process
is a checklist, and the language appears in the matrix from its first run.

## 6. Alternatives Considered

6.1. **Per-language test suites and a hand-kept parity matrix.** The pattern-rs approach. Rejected: it records what
exists and not how it behaves, and it drifts.

6.2. **Generate the expected outputs from the Rust implementation and compare the other languages with them.** A
differential approach. Rejected as the contract: it makes Rust's behavior, bugs included, the specification, and
RFC-001 (6.5) gives every language the same standing. Differential runs remain useful as an additional check.

6.3. **Property-based cross-language testing only (random values, compare text round trips).** Rejected as the
contract: it finds disagreements but does not say what is right. It complements the corpus.

6.4. **One repository per language, sharing the corpus as a dependency.** Each package keeps its own release
cadence. Rejected for now: a behavior change would need coordinated commits in several repositories, and the
corpus would need its own versioned distribution. It remains available if a package outgrows the shared cadence.

6.5. **Independent version numbers per package.** Rejected as the default: a user combining languages could not
tell which versions agree. A package-only patch is the exception (5.6).

6.6. **No release automation; publish each package by hand.** Rejected: it invites a package published against an
unpublished dependency, and it leaves the release knowledge with one person.

## 7. Open Questions

7.1. How does a corpus case describe a value without a parser: a structured description used only in the corpus
(readable, no parser needed, but a second notation next to gram text, in tension with RFC-001 4.4), or gram text
parsed in tests with each language's tree-sitter-gram binding and a test-only lowering (one notation, but a
dependency and duplicated lowering code in every language)?

7.2. How does a case state the expected native value per language: a literal in that language's syntax, evaluated by
the runner; or a neutral description that each runner builds a native value from and compares?

7.3. How is a skip declared, and can the matrix tell an accepted skip from an unreviewed one?

7.4. How does the corpus stay in step with tree-sitter-gram's corpus when the grammar changes: derived cases,
a script that flags new grammar cases, or a manual review?

7.5. Is one repository the right shape for all tiers, or does a language with heavy tooling (Haskell, Swift) belong
in its own repository?

7.6. What is the version policy: one number for everything or one per package, how does a package-only fix ship, and
what may a user rely on before 1.0?

7.7. What are the package names on each registry (crates.io `gram-value`; npm `gram-value` or `@gram-data/value`;
PyPI `gram-value`; the Go module path; the Swift package; Hackage), and who owns them?

7.8. How do release workflows authenticate to the registries (trusted publishing, tokens held as repository
secrets), and what is the recovery when one registry fails after another succeeds?

7.9. What does CI cost with several toolchains, and which languages run on every change and which on a schedule?

7.10. Where and how is the parity matrix published?

7.11. Do consumers (relate-gram, pattern-rs) run a subset of the corpus against their own use of the library, or
only their own tests?

7.12. If operations are pushed down into the core (RFC-001 7.16), how does the corpus describe a pipeline so every
language can run it?

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
