---
id: ADR-001
title: The Conformance Corpus Is Gram Text
status: accepted
rfcs: [RFC-003, RFC-002, RFC-001]
created: 2026-11-14
---

# ADR-001: The Conformance Corpus Is Gram Text

## 1. Context

Every language package of `gram-value` has to run the same cases and report the same results, so a case must say, in one notation every language can read, which value is under test and what each language must do with it. Three forces shape the notation.

- **The model has no parser.** `gram-value` models values and does not read gram text (RFC-001 4.1), so a package cannot build a case's input from text by itself. Any case format must either describe values in a structured form that needs no parser, or rely on a parser that the tests, and only the tests, bring in.
- **A second notation for values is unwelcome.** RFC-001 4.4 excludes a serialization of values other than gram text, because gram text is the data format. A corpus whose values are written in tagged JSON (as the TOML corpus `toml-test` does, with every value a string) would need no parser but would put a second notation next to gram.
- **The grammar already has bindings.** tree-sitter-gram, the authority on what text is gram, ships bindings for C, Go, Node, Python, Rust and Swift, so each first-tier language (Rust, TypeScript, Python) and two of the next tier can parse gram in its tests. A sample corpus written as gram parses with the current grammar (0.7.0), including a map as a property value, which the sample uses for per-language expectations.

Two further constraints come from the behavior the corpus tests. A mapping case has to state a loss without a language-neutral value syntax for native values (a Rust `i64` and a JavaScript number have no common literal), and a skipped case must be visible, not silent (RFC-003 3.3).

## 2. Decision

### 2.1 Corpus files are gram

The corpus is a set of `.gram` files under `conformance/`, one topic per file (`values.gram`, `text.gram`, `tags.gram`, `mappings.gram`). A case is a node. Its identifier is the case id (lower-case words joined by hyphens, starting with a letter, unique across the whole corpus and never reused), its single label is the case kind, and its record holds the case's properties. Comments (`//`) carry notes for readers and are ignored.

Every corpus file is syntactically valid gram. A case cannot test a syntax error: tree-sitter-gram owns its own corpus. Files are validated with the `gram` command-line tool from tree-sitter-gram, `gram lint --strict conformance/`, which reports parse and semantic errors and exits non-zero on any diagnostic (`--json` gives the diagnostics as data). The `gram` agent skill, installed with `gram skill install`, teaches an agent the notation and this check, so an agent editing the corpus validates with it. The same check covers each package's `skips.gram` and its result files.

### 2.2 The value under test is a property value

Every case has a property `value` that holds the value under test, written as a gram value (an integer, a measurement, a tagged string, an array, a map, any value gram can write), and a property `name`, a string describing the case for a person. The runner takes the value from the parsed corpus, so the value is exactly what the grammar reads from the text.

### 2.3 Each language parses the corpus in its tests

A runner parses the corpus files with that language's tree-sitter-gram binding and lowers each case's `value` property to a `gram-value` value with a lowering function (a function that converts the parsed syntax of the value into a `gram-value` value) that lives in that language's test code and is not shipped in the package. A lowering is total: a parsed value it cannot represent fails the run with an error naming the case, and is never dropped or approximated. The grammar version is pinned in `conformance/GRAMMAR_VERSION`, runners check it, and a grammar upgrade is a reviewed change to the corpus (RFC-003 5.8).

### 2.4 Case kinds

| Kind (label) | Properties besides `name` and `value` | The runner checks |
|---|---|---|
| `ValueCase` | `kind`: a symbol naming the value's kind | the package classifies `value` as that kind |
| `TextCase` | `canonical`: a string | writing `value` as gram text gives exactly `canonical` |
| `TagCase` | `outcome`: `accepted` or `rejected`; `diagnostic`: a symbol, when rejected; `interpreted`: a string, when accepted | interpreting the tag of `value` (a known tag, such as `date` or `point`, is one whose content `gram-value` interprets) accepts or rejects it, with that diagnostic, and an accepted value's normal content string (ADR-004 2.5) is `interpreted` |
| `MappingCase` | one property per language, named by the lower-case language (`rust`, `typescript`, `python`, later `go`, `swift`, `haskell`), each a map with `class`, `loss`, `native` and `roundtrip` | converting `value` to that language's native value gives the entry class, loss kinds and native type, and writing the native value back as gram text gives `roundtrip` |

In a mapping entry, `class` is `exact`, `lossy` or `unsupported` (RFC-002 5.2); `loss` is an array of symbols naming loss kinds, present when the class is `lossy`; `native` is the name of the native type in that language's mapping table, or `none` when the conversion fails; `roundtrip` is the gram text that the native value writes back as, so a loss shows as a different text (the TypeScript number from `9007199254740993` writes back as `9007199254740992`), and it is absent when no native value exists.

A runner runs a `MappingCase` only for a language that has an entry in it. A case with no entry for a language is not applicable to that language, and the matrix shows it as not applicable, which is different from a skip.

### 2.5 A vocabulary file names what a case may say

`conformance/vocabulary.gram` lists the allowed kind names (the eleven kinds of ADR-002 2.2: `integer`, `big_integer`, `decimal`, `big_decimal`, `boolean`, `string`, `tagged`, `range`, `measurement`, `array`, `map`), the ten tag names of ADR-004, loss kinds (none yet: RFC-002 7.1 is open), diagnostic names (`invalid-form` and `out-of-range` of ADR-004 to start) and, per language, native type names. A corpus lint, run by every package and by the corpus's own check, fails on a name that is not in the vocabulary, so a misspelling is an error, not a case that silently never matches. The lint also requires every distinct value form that appears in any case to appear in a `ValueCase`, which is what pins each language's lowering: a lowering that reads a value differently fails that `ValueCase`.

### 2.6 Skips live in the package

Each package keeps a file `skips.gram` next to its runner, with one node per skipped case: `(skip-1:Skip {case: integer-beyond-2-53, reason: "BigInt mapping not built yet"})`. A skipped case is not run and is reported as skipped. A skip that names an unknown case fails the run, and a skipped case that passes is reported as an unexpected pass and also fails the run, so a skip cannot outlive its reason.

### 2.7 Runners report in gram

A runner writes `conformance-results.gram`, one node per case, labelled `Result`, with the case id and a `status` of `passed`, `failed` or `skipped`. The parity matrix is generated from the result files of all packages, so the generator is language-neutral. The shape is a starting point that may change when the matrix and its generator are designed (RFC-003 7.15), and where the matrix is published is not decided here (RFC-003 7.10).

### 2.8 Not decided here

Behavior on values gram cannot write (repeated keys, `NaN`, unwritable nesting, an invalid tag or unit at construction) cannot appear in a corpus that must pass `gram lint --strict`, so each package tests it with unit tests derived from the tables in `spec/` until a construction-case kind is decided. Also not decided: the case kind for the native-to-gram direction (RFC-002 5.6 and 7.9), cases that compare two values for equality or order (RFC-001 7.15), and the path format that would say where in an array or map a loss occurred (RFC-002 7.2). The kinds in 2.4 do not prevent adding them.

## 3. Alternatives Considered

3.1. **Tagged JSON, as `toml-test` does.** Values written as `{"type": "integer", "value": "9007199254740993"}`. Rejected: it puts a second notation for values beside gram text, every value becomes a string with the kind named beside it, and a value reads as a record of strings instead of as the value itself. Its advantage, no parser in any test, remains available for a language that cannot parse gram.

3.2. **A bespoke corpus text, like tree-sitter's (a name, the input, a separator, the expectation).** Rejected: every language needs its own reader for the format, and the input is still gram text that needs a parser.

3.3. **The value under test as a string of gram text (`input: "5kg"`).** Rejected: the text is quoted and escaped inside another string, tooling and syntax highlighting cannot see it, and the runner would have to wrap each string in a dummy element to parse it, a second layer of text inside the first.

3.4. **A YAML or TOML file per topic, holding gram text inputs.** Rejected: a third notation, and the same parsing need for the gram inside it.

3.5. **Expected native results as literals in each language's syntax.** Rejected: Rust cannot evaluate a literal at run time, and one case would hold three syntaxes. The neutral type name and round-trip text in 2.4 express the same expectation in one notation.

3.6. **Skip declarations inline in each case.** Rejected: the corpus would change whenever a package's progress changes, and one package's state would sit in the shared file.

3.7. **No shared lowering, so each package builds a case's value by hand in code.** Rejected: the corpus would then hold only expectations while the values lived in test code in each language, and the languages could disagree about what a case's input is.

## 4. Consequences

- Every language needs a tree-sitter-gram binding and a lowering function in its tests. The first-tier languages have bindings. Of the next tier, Go and Swift have them, and Haskell would need one or a binding to the C library; that cost falls on the language when it joins.
- The lowering functions are duplicated per language, in test code only. The `ValueCase`s pin them (2.5): every value form in any case also appears in a `ValueCase`, so two languages cannot lower the same text to different values without a failure.
- Adding a language changes every `MappingCase`: the layout keeps each behavior's expectations together at the price of that edit.
- The value kinds are ADR-002 and the canonical text rules are ADR-003, so the `canonical` and `roundtrip` expectations follow them. The first cases cover the forms those ADRs fix, and each ADR's boundary cases (decimals at the float limit, a very large integer, a leap day, week 53) are the first to add.
- The corpus depends on the pinned grammar version. A grammar release can change what a case's text means, so upgrades are reviewed together with the corpus, as RFC-003 5.8 requires.
- A corpus file is valid gram, but relate-gram would reject much of it (ranges, measurements, maps are outside its Cypher subset). That is expected; the corpus is not for loading into a database.
- The `roundtrip` text states a loss without saying where it occurred. A loss inside an array or a map is a path question left to RFC-002 7.2.
- The vocabulary file and the lint are new things to write and keep current. Native type names are per language and grow with each mapping table.
- Follow-up work: write the vocabulary and the value-kind names in `spec/`, the corpus lint (including the `gram lint --strict` check), the first runner (Rust) and the result file shape, then the TypeScript and Python runners, and then the matrix generator.

## 5. Related

- RFCs: RFC-003 (resolves questions 7.1, 7.2 and 7.3), RFC-002 (the entry classes and loss kinds a mapping case uses), RFC-001 (4.4, no second notation for values; 4.1, no parser in the model).
- Prior art: JSON-Schema-Test-Suite, Bowtie and `toml-test`, cited in RFC-003.
- RFC-003 passed its review for the `review` status; RFC-002 remains a draft.
- The `gram` command-line tool and its skill: https://github.com/gram-data/tree-sitter-gram (`cargo install gram-data`, then `gram skill install`).
