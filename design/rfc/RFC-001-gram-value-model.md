---
id: RFC-001
title: A Shared Model of Gram's Data Values
status: draft
created: 2026-11-14
adrs: []
---

# RFC-001: A Shared Model of Gram's Data Values

## 1. Summary

Gram is a plain-text notation for graphs and patterns. Every element in a gram file can carry a record of
properties, and a property value can be any of a small family of kinds: integers (written in decimal,
hexadecimal or octal), decimals, booleans, symbols, strings in four quoting styles, ranges such as `1..5`,
measurements such as `5kg`, arrays, maps, and tagged strings such as ``date`2024-01-15` `` that name a type for
a piece of text. A program that reads gram has to decide what each of these is in memory, how to write it back
as gram text, and how it corresponds to the host language's own values.

The proposal is `gram-value`: one model of every value gram can write, with canonical text writing, an
optional layer that interprets known tags, and documented mappings to the values of each host language,
published in several languages and checked against one language-neutral conformance corpus, so that gram
tools stop rebuilding this model. The model describes all gram values; a consumer that supports fewer kinds
rejects the rest itself, and the model does not name those subsets. The tree-sitter-gram grammar and test
corpus remain the authority on what text is a gram value.

This RFC settles the scope and the direction. The representation choices, the mapping tables and the
packaging are decided in later documents.

## 2. Problem / Motivation

**The value model is built several times, and the copies disagree.** The pattern-rs repository has two: a
`Value` with ten variants in `pattern-core/src/subject.rs` (integer, decimal, boolean, string, symbol,
tagged string, array, map, range, measurement), and a second in `gram-codec/src/value.rs`. The relate-gram
repository, which loads gram into the Relate graph database, has a third (`PropValue`) with five variants,
because it accepts only the values that become a Cypher type. A fourth is the Haskell `Value` in pattern-hs (`libs/subject/src/Subject/Value.hs`), the reference for
pattern-rs, and the TypeScript and Python packages of pattern-rs each define another
(`typescript/packages/pattern/src/value.ts`, `python/packages/relateby/relateby/pattern/_value.py`). What each one preserves differs, and a file can mean different things to different tools.

**Existing models lose what a consumer needs.** relate-gram's SPIKE-003 compared the parsers of pattern-rs
with the grammar's syntax tree. A map is a `HashMap` in pattern-core, so key order and repeated keys are lost;
a measurement is a unit and an `f64`, so `5kg` and `5.0kg` are one value; an octal literal is read as a decimal
number (`010` is 10) although the grammar names it an octal; a tag on a fenced string was dropped. A tool that
must report exactly what a file says cannot be built on them.

**Value interpretation is work every tool repeats.** relate-gram wrote validators for eight tags (`date`,
`time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point`, `uuid`), with their ISO 8601 and extended
well-known-text forms. Nothing in them is specific to a database, and another tool that wants to read a date in
a gram file would write them again.

**Consumers support different subsets, and a shared model must not decide which.** relate-gram supports
scalars and arrays of one kind. pattern-rs supports all of gram. Tools for ontologies and schemas (pattern-rs's
`pato`, gram-ontology) use tagged strings as constraint dialects (`ts`, `re`, `zod`, `cypher`, `pydantic`) that
no value layer should interpret.

**Host languages need a stated correspondence.** A gram integer is wider than a JavaScript number, a gram range
has no Python counterpart, and a measurement has none in most languages. Each tool currently invents its own
mapping and does not say what it loses.

## 3. Goals

3.1. Define each value kind the grammar can write, and what each means, so that two programs given the same
gram text agree on its values.

3.2. Define the canonical gram text of every value, so that a value written by any language reads back as the
same value and the same text is produced everywhere.

3.3. Keep known-tag interpretation (dates, points, UUIDs and any later tag) as a layer separate from the value,
extensible by a consumer, with tags it does not know passing through untouched.

3.4. Document, for each first-tier language (Rust, TypeScript, Python), how every value maps to a native value,
and mark each mapping exact, lossy (stating what is lost) or unsupported, with loss detectable by the consumer.

3.5. Measure parity between languages with a conformance corpus that every language package runs in
continuous integration, and publish the resulting matrix.

3.6. Publish packages for the first-tier languages. The next tier (Go, Swift and Haskell) follows once the
first tier is stable.

## 4. Non-Goals

4.1. A parser. Turning text into values is the work of tree-sitter-gram, and this model sits beside it.

4.2. A graph or pattern model. How nodes, relationships and paths are represented is a separate layer.

4.3. Consumer subsets or profiles. Which values a tool supports, and the errors it gives for the rest, belong to
that tool.

4.4. A serialization other than gram text. Gram text is the data format, as JSON text is for JSON values; a
second format would duplicate it.

4.5. Defining gram's syntax. The grammar and its corpus define what text is a value, and this project follows
them.

## 5. Proposed Approach (high-level)

### 5.1 One model of all values, recording what a value means

The model has a representation for every value kind in the grammar, including the ones some consumers will
reject (ranges, measurements, maps, octal literals). Candidate direction: a value records what it means (the
number, the unit, the ordered entries), while how it was written (hexadecimal or decimal, which quote) and where
it was written stay in the parser's syntax tree. That keeps the model small and lets text writing be canonical.

### 5.2 Classification, not subsets

The model offers the means to ask what a value is (its kind, whether it is a scalar, whether an array holds one
kind), so a consumer can implement its own subset in a few lines. It ships no consumer's subset, so adding a
consumer never changes the model.

### 5.3 Tagged strings stay a tag and a content string

A tagged string is `{tag, content}` in the model. A registry of known tags, kept as an optional layer,
interprets the ones it knows and validates their content forms. A consumer can add tags to the registry or
decline to use it.

### 5.4 Canonical text writing

Each value has one canonical text: string quoting, decimal spelling and name quoting are fixed, so
exports are byte-stable. Whether the writing rules belong in the model's package or a sibling is open.

### 5.5 Host-language mappings

Each language documents a table from every value kind to a native value, with the loss class of each entry. A
consumer can ask whether a conversion lost information. Mappings are tested against the same corpus as the
model.

### 5.6 Parity through a conformance corpus

A language-neutral corpus of cases ties a gram value to its canonical description, its canonical text and, per
language, its native mapping and loss class. Every package runs all of it in CI. The parity matrix comes from
the results.

### 5.7 Implementation per language

Whether each language is a native implementation or a binding to a shared core is not decided here. Crossing a
language boundary is costly for users, and WebAssembly in particular is expected to be a poor experience. A
spike measures candidates on simple operations (creating 1,000 values, adding them to a list, mapping the list
to new values), and an ADR decides.

## 6. Alternatives Considered

6.1. **Each tool keeps its own model.** The present state. Rejected: the copies disagree, none preserves what
a strict consumer needs, and the interpretation work repeats.

6.2. **Extract pattern-rs's `Value` as it is.** Rejected: its map, measurement and octal behavior lose
information (section 2), and it has no text-writing or host-mapping contract.

6.3. **Share relate-gram's model.** Rejected: it models only the values that become a Cypher type, so it cannot
represent ranges, measurements or maps.

6.4. **A neutral interchange format for values (JSON, say).** Rejected: gram text is the serialization, and a
second one would duplicate it.

6.5. **One reference implementation, with the other languages as ports checked by tests.** This is how
pattern-rs relates to pattern-hs. Rejected for now: it makes parity depend on one language's tests, and the
corpus gives every language the same standing. Whether the Haskell implementation is a peer or a reference is
an open question (7.10).

## 7. Open Questions

7.1. Does a value record only what it means, or also how it was written (hexadecimal versus decimal, quoting
style)?

7.2. How is a map stored: ordered, and do repeated keys survive, fail, or collapse?

7.3. What does an octal literal mean (`010` is 8 or 10), and what are the width and range of an integer, given
that gram text has no limit?

7.4. Is a decimal a 64-bit float or an exact decimal? How is a measurement represented (a number and a unit,
with the spelling kept?), and how is a range represented, including half-open ranges and measurements as ends?

7.5. Are the four quoting styles of a string one kind, and is a symbol a string or a kind of its own?

7.6. How does a consumer extend the known-tag registry, and how does it state that it supports only some tags?

7.7. Do the text-writing rules live in the model's package or a sibling?

7.8. How are the host-language mappings specified, tested and reported as lossy?

7.9. How does the value type evolve when several projects depend on it?

7.10. Is the Haskell implementation (pattern-hs) a peer of the other languages or their reference?

7.11. Per language, is the package native or a binding to a shared core, and what does a user pay to cross
the boundary?

7.12. The grammar allows `::` before a label or key. Does that mean anything for a value?

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
