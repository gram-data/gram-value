# ADRs — Architecture Decision Records

ADRs capture the technical **HOW** for one architecturally-significant decision — the Develop
half of the double-diamond design process described in the project's
[CLAUDE.md](../../CLAUDE.md). Every ADR requires at least one existing parent
[RFC](../rfc/README.md) — usually one, but an ADR MAY serve more than one RFC when a decision is
genuinely shared infrastructure (e.g. a text-spelling rule that every language package
depends on); an ADR never writes implementation code itself.

## Audience

Write for a working engineer who has just found this repository. Assume they are fluent in at least one
programming language and comfortable with the gram notation as a general idea, and assume nothing about this
project's specifics: not its sibling RFCs or ADRs, not the other gram projects it serves. Name project
artifacts (the conformance corpus, a host-language mapping) on first use; one clause is enough.

The failure mode to avoid is a document addressed to its own authors: one that argues with past decisions,
defends choices nobody has questioned, and cites sibling documents before naming its subject.
`/rfc-review` and `/adr-review` criterion 8 check for this, and the `rfc-create` and `adr-create` "Voice and
audience" sections hold the rules. Section 1 in particular has to teach before it locates: strip every
`§`-reference and RFC or ADR number out of it, and what remains must still explain the topic to a newcomer.

## When to write an ADR

- A parent RFC has settled the problem/direction and a specific technical decision now needs to
  be made and recorded before implementation.
- The decision is concrete enough to implement from.
- If a decision would genuinely serve more than one RFC, list all of them as parents rather than
  picking one arbitrarily and mentioning the rest only in prose — an unlisted relationship doesn't
  show up in the index tables.

## Format

Created/updated via `/adr-create`. File: `design/adr/ADR-NNN-short-title.md`, `NNN` zero-padded.

**Front-matter** (YAML):

| Field | Description |
|---|---|
| `id` | `ADR-NNN` |
| `title` | Short descriptive title |
| `status` | `proposed` \| `accepted` \| `rejected` \| `superseded` |
| `rfcs` | Parent RFC id(s), a list — **required, at least one**, set at creation, only ever grows (never hand-remove an entry) |
| `created` | ISO date (`YYYY-MM-DD`) |

**Required sections** (numbered `##`, renumber if one is omitted) — `status` lives only in
front-matter, no matching body section:

1. Context
2. Decision — split into numbered `###` subsections (`2.1`, `2.2`, ...) when the decision has
   more than one genuinely distinct facet (e.g. format, location, seeding plan)
3. Alternatives Considered
4. Consequences
5. Related — RFC link(s)

Use `/adr-review` before moving a draft to `accepted`.

## Index

| ADR | Title | Status | RFCs |
|---|---|---|---|
| [ADR-001](ADR-001-conformance-corpus-format.md) | The Conformance Corpus Is Gram Text | accepted | RFC-003, RFC-002 |
| [ADR-002](ADR-002-value-kinds-and-equality.md) | The Eleven Value Kinds, Their Meaning and Equality | accepted | RFC-001 |
| [ADR-003](ADR-003-canonical-text.md) | Canonical Gram Text for Every Value | accepted | RFC-001 |
| [ADR-004](ADR-004-known-tags.md) | Ten Known Tags as Plain Interpretation Functions | accepted | RFC-001 |
| [ADR-005](ADR-005-native-packages-and-results.md) | Native Packages, a Plain Sum Type and a Common Result Shape | accepted | RFC-001, RFC-003 |
| [ADR-006](ADR-006-host-language-mappings.md) | Host-Language Mappings, Loss Reporting and the Rust Conversions | accepted | RFC-002 |
| [ADR-007](ADR-007-release-process-and-versions.md) | Releasing the Packages: Versions, Names, Trusted Publishing and an Approval Gate | accepted | RFC-003 |
| [ADR-008](ADR-008-embedding-tag.md) | The `embedding` Tag as Base64-Encoded SafeTensors | proposed | RFC-004 |
