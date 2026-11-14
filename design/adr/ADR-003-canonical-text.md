---
id: ADR-003
title: Canonical Gram Text for Every Value
status: accepted
rfcs: [RFC-001]
created: 2026-11-14
---

# ADR-003: Canonical Gram Text for Every Value

## 1. Context

A value can be written as gram text in many ways: `0x10` or `16`, `'ada'` or `"ada"` or bare `ada`, a map's entries in any order. A tool that writes values (an exporter, a formatter, a test) has to pick one, and two tools picking differently make files differ for no reason and values compare unequal when they should not. `gram-value` fixes one canonical text per value, written the same way by every language package. Equality of writable values agrees with equal text (ADR-002), and the corpus tests equality and the order through it, so a package writes the text itself, in its own language, with no help from a parser.

The grammar's constraints decide what is possible. A symbol is `[a-zA-Z_][0-9a-zA-Z_.\-@]*`, and a string may be single-quoted, double-quoted or backticked, with the escapes `\b \f \n \r \t \/ \\` and the quote character (and nothing like `\u`). A decimal is `-?(0|[1-9]\d*)\.\d+`, with no exponent. A measurement is a number and letters with nothing between them. Ranges are `lower..upper`, `lower...` and `...upper`. An array holds scalars, and a map entry holds a scalar or an array.

## 2. Decision

### 2.1 Scalars

- **integer, big_integer:** optional `-`, then decimal digits with no leading zeros (`0` is `0`). Never hexadecimal or octal.
- **decimal, big_decimal:** optional `-`, digits, `.`, digits, in positional notation with no exponent. A `decimal` uses, among the shortest digit strings that read back as the same 64-bit float, the one nearest the exact value, written positionally with any exponent form of the host's printer expanded and at least one digit after the point (`1.5`, `2.0`, `0.1`, `-0.0`). A `big_decimal` writes its digits exactly as held, with trailing zeros after the point dropped and at least one digit kept.
- **boolean:** `true` or `false`.
- **string:** the bare word when its text matches the symbol rule and is not `true` or `false`, otherwise double-quoted. Inside the quotes, `"` and `\` are escaped with a backslash, and the control characters backspace, form feed, line feed, carriage return and tab are written `\b \f \n \r \t`. Every other character is written as itself in UTF-8. (So `active` is `active`, `Ada Lovelace` is `"Ada Lovelace"`, `true` the string is `"true"`.) The slash is not escaped.
- **tagged:** the tag, then the content in backticks with the string escapes above and an escaped backtick in place of the double quote: ``date`2024-01-15` ``. The fenced form is not used. The tag is written as given; a tag that is not a symbol, or is `true` or `false`, cannot be constructed. A string that holds a lone surrogate (a host string that is not valid Unicode) is rejected at construction, since the writer emits UTF-8.

### 2.2 Measurements and ranges

- **measurement:** the magnitude's text, then the unit with nothing between (`5kg`, `5.0kg`, `-2.5m`). A unit is ASCII letters (`[a-zA-Z]+`) only; a unit string that is not, or is empty, cannot be constructed. A measurement whose magnitude is `0` and whose unit begins with `x` is unwritable, because its text can read as a hexadecimal literal.
- **range:** the lower end's text, `..`, the upper end's text; or the lower end and `...`; or `...` and the upper end. An end that is a measurement is written as a measurement.

### 2.3 Arrays and maps

- **array:** `[`, the elements' texts separated by `, `, `]`. The empty array is `[]`.
- **map:** `{`, then each entry as the key's text, `: ` and the value's text, separated by `, `, then `}`. Keys are written as strings (2.1; quoting a key `true` or `false` is this decision's rule, since the grammar allows them bare as keys) and entries are in ascending byte order of the keys' UTF-8 text. The empty map is `{}`.

### 2.4 A value gram cannot express is an error

A writer that is given a value the grammar has no text for returns an error (code `unwritable`) that names the part. The cases are: an array that holds an array or a map; a map entry whose value is a map; a measurement of magnitude `0` with a unit beginning `x`; and a range with no ends (which cannot be constructed). The grammar allows one more level for an element's property record: a record property may hold a map whose entries are scalars or arrays. A writer is told whether the outermost map is an element's record, in which case its values may be such maps, or an ordinary value, in which case they may not. It never writes a near equivalent. The model itself accepts these values (ADR-002 2.2), and the text cases of the corpus cannot cover them (ADR-002 2.3).

### 2.5 The writer lives in the model's package

Every language package contains the canonical writer. It is not a separate package and does not call a parser, so the model has no dependency on tree-sitter. The conformance corpus's text cases (ADR-001) test it in each language.

## 3. Alternatives Considered

3.1. **Keep the original spelling when a value came from a file.** Rejected: the model records meaning only (ADR-002), and two spellings of one value would make equality depend on history.

3.2. **Always quote strings.** Rejected: bare words are the natural spelling of the common case, readable in a file, and the grammar reads them back as the same string.

3.3. **Single quotes or backticks for strings.** Rejected: double quotes are the quote most hosts and JSON use, and one choice is enough.

3.4. **Order map keys by the host's ordering or keep insertion order.** Rejected: hosts differ (JavaScript moves integer-like keys first; Python keeps insertion order; Rust's hash maps have none), and byte order is the same everywhere.

3.5. **A separate writer package, shared by the language packages.** Rejected: every language package is native (ADR-005), so the writer would be written once per language anyway, and equality needs it in the package that holds the values.

3.6. **Use an exponent for very large or small decimals.** Rejected: the grammar has no exponent form.

## 4. Consequences

- Output is byte-stable across languages and runs, so files can be compared with a text tool and a value's equality can use its text.
- A decimal's shortest digits require each language to have a shortest round-trip float printer. Rust, JavaScript and Python (`repr`) have one, but each prints exponent forms (`1e21`) that gram cannot read, so each package expands them; a language without a printer needs its own implementation, and the corpus has cases at the boundary.
- Very large or very small floats print as long digit strings, which is correct for gram and longer than an exponent form.
- A map with unusual keys is written with quoted keys, and a reader may find the order surprising; the order carries no meaning.
- A string made only of letters but equal to `true` or `false` is quoted, and one with a hyphen inside is bare (`a-b` is a symbol); a reader should not assume bare means "no special characters".
- Follow-up work: write these rules in `spec/` with a table of examples, add `TextCase`s for each rule and each boundary (ADR-001), and decide which value forms a formatter keeps (not this project).

## 5. Related

- RFC-001 (resolves questions 7.5 and 7.7).
- ADR-002 (the kinds this text writes), ADR-001 (the text cases).
- tree-sitter-gram 0.7.0, for the symbol, string, number and range forms.
