# Values

## 1. Kinds

A value has exactly one of these kinds. The order of the table is the kind order of section 6.

| Kind | Holds | Constraint |
|---|---|---|
| `integer` | a signed 64-bit integer | -2^63 to 2^63-1 |
| `big_integer` | sign and decimal digits | outside the `integer` range; no leading zeros |
| `decimal` | a 64-bit float | finite; writes back as the same number (section 2.3) |
| `big_decimal` | sign, digits, `.`, digits | a decimal a `decimal` cannot hold (section 2.3) |
| `boolean` | `true` or `false` | |
| `string` | text | valid Unicode (no lone surrogates) |
| `tagged` | tag and content | the tag is a symbol (below) other than `true` and `false`; the content is a string |
| `range` | optional lower end, optional upper end | each end is a numeric value or a `measurement`; at least one end; inclusive |
| `measurement` | magnitude and unit | the magnitude is `integer`, `big_integer`, `decimal` or `big_decimal`; the unit matches `[a-zA-Z]+` |
| `array` | an ordered sequence of values | any values |
| `map` | an unordered set of entries | unique string keys; any values |

A *symbol* is text matching `[a-zA-Z_][0-9a-zA-Z_.\-@]*`. There is no `symbol` kind and no `null` kind. The set of
kinds is closed (ADR-002 2.4). A value records meaning only: spelling, quoting style, the order entries were written
in and positions are not part of a value.

A package MUST expose, for any value, its kind, whether it is a scalar (every kind except `array` and `map`), and
for an array whether all its elements have one kind.

## 2. Lowering: grammar literals to values

Lowering turns the syntax tree of a gram value into a value. It is total: text the rules cannot lower is an error
that names the position, never a dropped or approximated value. The parser is tree-sitter-gram; the model does not
parse.

### 2.1 Integers

- A decimal integer literal `-?(0|[1-9]\d*)` lowers to `integer` when it fits in 64 bits, else to `big_integer`. `-0` lowers to the integer `0`.
- A hexadecimal literal `0x[0-9a-fA-F]+` lowers to the value it denotes (digits are case-insensitive) as an `integer` or, beyond 64 bits, a `big_integer` holding its decimal digits. It has no sign.
- An octal literal `0[0-7]+` is base 8: `010` is 8, `042` is 34. `019` is not an octal literal and the grammar rejects it. It has no sign.

### 2.2 Strings

- A single-quoted, double-quoted or backticked string lowers to the string whose text is the content with each escape replaced: `\b` backspace, `\f` form feed, `\n` line feed, `\r` carriage return, `\t` tab, `\/` slash, `\\` backslash, and `\'`, `\"` or `` \` `` for the quote character of that style. No other escape exists.
- A fenced string (three backticks, a newline, content, three backticks) lowers to its content exactly, with no escape processing. An absent content is the empty string.
- A symbol in a value position lowers to the string of its text.
- The `true` and `false` keywords lower to `boolean`.

### 2.3 Decimals and the `decimal`/`big_decimal` boundary

A decimal literal `-?(0|[1-9]\d*)\.\d+` has a *normalized literal*: its trailing zeros after the point dropped, keeping at
least one digit (`1.50` is `1.5`, `2.00` is `2.0`). It lowers to a `decimal` when the 64-bit float nearest to the literal
has, as its shortest round-trip digits written positionally with at least one digit after the point, exactly the
normalized literal; otherwise it lowers to a `big_decimal` that holds the normalized literal. `-0.0` is a `decimal`.
A literal beyond the float range (overflow or underflow) is a `big_decimal`. Examples: `0.1` and `3.14` are
`decimal`; `0.10000000000000000001` and a literal of 400 digits are `big_decimal`.

### 2.4 Measurements and ranges

A measurement literal lowers to a `measurement` whose magnitude is the number part lowered by 2.1 and 2.3 and whose unit
is the letters after it. A range literal lowers to a `range` whose present ends are each a numeric literal lowered by
2.1, 2.3 and this section. `5kg` and `5.0kg` therefore differ in the kind of their magnitude.

### 2.5 Tagged strings

A tagged string (backtick or fenced form) lowers to a `tagged` value: the tag is the symbol and the content is
lowered like a backticked string (with escapes) or a fenced string (without).

### 2.6 Arrays, maps and records

An array lowers to an `array` of its elements. A map or an element's property record lowers to a `map`. A key written
as a symbol, a backticked name or a double-quoted name lowers to the string of its text (the backticked and
double-quoted forms decode escapes as in 2.2). A repeated key is an error (`duplicate-key`) naming the key and its
position; no entry is silently dropped. The separator `:` or `::` is not recorded.

## 3. Construction

A package builds values from parts through constructors. A constructor MUST report an error, in the result shape of
[results.md](results.md), for each of these, and never repairs input:

| Code | Condition |
|---|---|
| `duplicate-key` | a map built from entries with a repeated key (the error names the key) |
| `not-finite` | a `decimal` that is `NaN` or an infinity |
| `invalid-tag` | a tag that is not a symbol, or is `true` or `false` |
| `invalid-unit` | a measurement unit that is empty or not `[a-zA-Z]+` |
| `no-ends` | a range with neither end |
| `invalid-range-end` | a range end that is not numeric or a measurement |
| `invalid-string` | a string with a lone surrogate |
| `invalid-integer` | a `big_integer` that fits in 64 bits, or has leading zeros or a non-digit |
| `invalid-decimal` | a `big_decimal` that a `decimal` could hold, or that has a malformed digit string |

## 4. Equality and hashing

Two values are equal when they have the same kind and the same content, compared part by part: numbers by exact
digits (`decimal` by its 64-bit pattern, so `-0.0` differs from `0.0`), strings by code points, tagged values by tag
and content, ranges by ends (an absent end equals only an absent end), measurements by magnitude and unit, arrays
element by element, and maps by entry sets (key order never matters). For every value gram can write this is the same
as equal canonical text ([text.md](text.md)). Hashing MUST agree with equality. `1` is not equal to `1.0`, `5kg` is not
equal to `5.0kg`, and `[1, 2]` is not equal to `[2, 1]`.

## 5. Writability

The model accepts any value inside any container. A value is *writable* when the grammar has text for it. Arrays hold
scalars; a map entry holds a scalar or an array; an element's property record is a map whose values may also be maps
of scalars and arrays, one level deep. See [text.md](text.md) section 4.

## 6. Order

Values compare first by kind in the order of the table in section 1, then within a kind:

| Kind | Order |
|---|---|
| `integer`, `big_integer` | numerically |
| `decimal`, `big_decimal` | numerically; `-0.0` before `0.0` |
| `boolean` | false before true |
| `string` | by UTF-8 bytes |
| `tagged` | by tag, then content, each by UTF-8 bytes |
| `range` | by lower end, then upper end; an absent end before any present end |
| `measurement` | by magnitude (this order), then unit by UTF-8 bytes |
| `array` | lexicographically |
| `map` | by entries sorted by key (UTF-8 bytes), compared as (key, value) pairs lexicographically |

The order returns "equal" only for equal values. Every `big_integer` sorts after every `integer` and every
`big_decimal` after every `decimal`; there is no numeric order across kinds. A package MAY offer other named
equivalence or ordering functions; none replaces equality, hashing or this order.
