---
id: RFC-003
title: Parity, Conformance and Packaging Across Languages
status: review
created: 2026-11-14
adrs: [ADR-001, ADR-005, ADR-007]
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

The scope and direction are the proposal. The corpus format is decided (ADR-001); the version policy, the release
mechanics and the matrix are left to later decisions.

## 2. Problem / Motivation

**Parity has been planned and never automated.** pattern-rs, a Rust library of pattern and graph structures built
on gram, keeps a hand-written parity matrix (`proposals/language-parity-proposal.md`, dated 2026-04-16) that
records which operations the Haskell reference, Rust, TypeScript and Python each have, not whether they behave the
same. It also defines a JSON test-case format (`scripts/sync-tests`, `tests/common/test_cases.json`) and a plan to
extract cases from its Haskell reference, pattern-hs, which has a `gramref` command that can generate JSON test
suites; the extraction and comparison scripts (`extract.sh` and `compare.sh`) are placeholders. SPIKE-001 found the
behavior differences such a table cannot show: the same gram text becomes a different value in different languages
(integers of three different widths, a decimal that turns into an integer, a map misread at the host boundary).

**A shared corpus has worked for the grammar.** tree-sitter-gram, the grammar that defines what text is gram, keeps its test corpus (`test/corpus`,
one case per input with its expected syntax tree) in one place, and its bindings for C, Go, Node, Python, Rust
and Swift live in the same repository. relate-gram, which loads gram into the Relate database, runs every corpus
case against its own expectations file, which records for each case whether it loads and with which error. When
the grammar gained empty arrays and decimal measurements (version 0.7.0) the new corpus cases surfaced exactly
what had to change in relate-gram. A corpus about values would do the same for the value model, one level up.

**Cross-implementation suites exist for other notations.** JSON-Schema-Test-Suite, a language-agnostic set of test
files, is run by implementations of JSON Schema in many languages, and Bowtie coordinates running those
implementations and reports their results. `toml-test` does the same for TOML parsers and expresses every value as
tagged JSON with the value always a string, so no language's number type changes it.

**The behavior to keep in step is more than syntax.** The cases a value library needs span four kinds of
behavior: what a value is and how two values compare, the canonical text written for a value, the
interpretation and validation of known tags (tags such as `date` or `point` whose content `gram-value`
interprets), and each host-language mapping with its loss
(RFC-002). A syntax-tree corpus covers none of those.

**Native implementations duplicate logic by design.** SPIKE-002 measured WebAssembly in TypeScript at 11 to 25
times the cost of plain objects and, for Python, a PyO3 core only as fast as native, so RFC-001 (7.11) makes every
language package native. Text writing and the tag validators (about 800 lines in relate-gram) therefore exist once
per language, and nothing but tests keeps them aligned.

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

3.5. Publish the first-tier packages to crates.io, npm and PyPI from one release, so that no registry offers a
version the others lack beyond the duration of a release run, with a stated recovery if one registry fails after
another has succeeded.

3.6. State how a version number relates across the languages, and what a user may rely on before 1.0.

3.7. Let a further language join by passing the corpus, with a stated process.

## 4. Non-Goals

4.1. Choosing whether each language is native or bound to a shared core. RFC-001 7.11 decided: native in every
language.

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

Each first-tier package runs the whole corpus in CI. Candidate direction: a case a package does not yet pass is
listed in that package's own skip list with a reason, so a skip is a visible gap in the matrix and not a hidden
omission (how a skip is declared is question 7.3). A mapping case whose expected outcome is a recorded loss or an
unsupported entry passes when the package reports exactly that; a skip means the package does not yet do what the
case asks. The matrix is generated from the runs and published with the repository.

### 5.4 Tiers

Candidate direction: the first tier (Rust, TypeScript, Python) passes the whole corpus. A next-tier language (Go,
Swift, Haskell) appears in the matrix as soon as it runs the corpus, with its gaps visible, and is promoted when it
passes.

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

Candidate direction: a release workflow publishes every first-tier package from one tag and verifies each from the
published artifact, in dependency order where one package depends on another. How the workflow authenticates to
the registries is question 7.8. A release that fails partway has a documented recovery.

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
RFC-001 (6.5) gives every language the same standing. Differential runs remain useful as an additional check. pattern-rs chose this route for its Haskell reference and
did not finish the automation (section 2).

6.3. **Property-based cross-language testing only (random values, compare text round trips).** Rejected as the
contract: it finds disagreements but does not say what is right. It complements the corpus.

6.4. **One repository per language, sharing the corpus as a dependency.** Each package keeps its own release
cadence. Rejected for now: a behavior change would need coordinated commits in several repositories, and the
corpus would need its own versioned distribution. It remains available if a package outgrows the shared cadence.

6.5. **Independent version numbers per package.** Rejected as the default: a user combining languages could not
tell which versions agree. A package-only patch is the exception (5.6).

6.6. **No release automation; publish each package by hand.** Rejected: it invites a registry left a version behind the
others after a partial failure, and it leaves the release knowledge with one person.

6.7. **A central harness that runs each package through a protocol and produces one compatibility report (the
approach of Bowtie for JSON Schema).** Packages are tested from outside, in isolation. Not rejected: it decouples the
matrix from each package's CI, at the cost of a protocol every package must implement; question 7.15 asks which
fits.

## 7. Open Questions

Questions 7.1 to 7.3 are decided. The rest are deferred to later ADRs (the version policy, registries and release, CI cost, the matrix, coverage).

7.1. (Decided in ADR-001: gram text, parsed in each language's tests.) How does a corpus case describe a value without a parser: a structured description used only in the corpus
(readable, no parser needed, but a second notation next to gram text, in tension with RFC-001 4.4), or gram text
parsed in tests with each language's tree-sitter-gram binding and a test-only lowering (one notation, but a
dependency and duplicated lowering code in every language)? `toml-test`, a corpus for another notation, describes
values as tagged JSON with every value a string.

7.2. (Decided in ADR-001: a neutral description.) How does a case state the expected native value per language: a literal in that language's syntax, evaluated by
the runner; or a neutral description that each runner builds a native value from and compares?

7.3. (Decided in ADR-001: a per-package `skips.gram`.) How is a skip declared, and can the matrix tell an accepted skip from an unreviewed one?

7.4. How does the corpus stay in step with tree-sitter-gram's corpus when the grammar changes: derived cases,
a script that flags new grammar cases, or a manual review?

7.5. Is one repository the right shape for all tiers, or does a language with heavy tooling (Haskell, Swift) belong
in its own repository?

7.6. What is the version policy: one number for everything or one per package, how does a package-only fix ship, and
what may a user rely on before 1.0?

Decided 2026-11-14 (ADR-007 2.1): the corpus and every package share a major.minor; a fix confined to one package is a patch of that package only. Before 1.0 a minor may change behavior.

7.7. What are the package names on each registry (crates.io `gram-value`; npm `gram-value` or `@gram-data/value`;
PyPI `gram-value`; the Go module path; the Swift package; Hackage), and who owns them? 

Decided 2026-11-14 (ADR-007 2.2): crates.io `gram-value`, npm `@gram-data/value`, PyPI `gram-value`; Go, Swift and Hackage names are decided when those languages join.

7.8. How do release workflows authenticate to the registries (trusted publishing, tokens held as repository
secrets), and what is the recovery when one registry fails after another succeeds?

Decided 2026-11-14 (ADR-007 2.4 to 2.6): trusted publishing (a token only for the first release of crates.io and npm); a tag starts the release, a protected `release` environment approves it; a re-run completes a partial release and a defective version is yanked and fixed by a new patch.

7.9. What does CI cost with several toolchains, and which languages run on every change and which on a schedule? (Deferred to an ADR.)

7.10. Where and how is the parity matrix published? (Deferred to an ADR.) For now ADR-001 2.7 publishes it as the continuous integration job summary and a build artifact; a permanent page is open.

7.11. Do consumers (relate-gram, pattern-rs) run a subset of the corpus against their own use of the library, or
only their own tests?

7.12. Closed: there are no pushed-down pipelines (RFC-001 7.16, decided), so the corpus has no pipeline cases.

7.13. What is the corpus's coverage target: every grammar value form, every known tag, every mapping entry?

7.14. (Partly answered by ADR-001 2.3: the grammar version is pinned in `conformance/GRAMMAR_VERSION`.) How does a package obtain the corpus (a path in the repository, a vendored copy, a published artifact), and
how is a corpus version tied to a package version?

7.15. Does the parity matrix come from each package's own CI or from a central harness (6.7)?

Decided 2026-11-14 in practice (ADR-001 2.7): the matrix comes from each package's own run, merged in one job. A central harness that drives every package through a protocol (6.7) is not built; it remains an option if packages outgrow one workflow.

## 8. ADRs

- ADR-001: The Conformance Corpus Is Gram Text (resolves questions 7.1, 7.2 and 7.3)
- ADR-005: Native Packages, a Plain Sum Type and a Common Result Shape (parity rests on the corpus, ADR-005 2.3)
- ADR-007: Releasing the Packages: Versions, Names, Trusted Publishing and an Approval Gate (resolves questions 7.6, 7.7 and 7.8)
