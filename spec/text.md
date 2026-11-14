# Canonical text

Every value has one canonical gram text, written identically by every package. Writing needs no parser. All text is
UTF-8.

## 1. Scalars

| Kind | Canonical text | Examples |
|---|---|---|
| `integer`, `big_integer` | optional `-`, decimal digits, no leading zeros | `0`, `-42`, `9223372036854775808` |
| `decimal` | see 2 | `1.5`, `2.0`, `0.1`, `-0.0` |
| `big_decimal` | its digits exactly, trailing zeros after the point dropped, one digit kept | `0.10000000000000000001` |
| `boolean` | `true` or `false` | `true` |
| `string` | see 3 | `active`, `"Ada Lovelace"`, `"true"`, `""` |
| `tagged` | tag, then the content in backticks with the escapes of 3 and `` \` `` instead of `\"` | ``date`2024-01-15` `` |

## 2. Decimals

A `decimal` is written positionally (never with an exponent) using, among the shortest digit strings that read back as
the same 64-bit float, the one nearest the exact value, with at least one digit after the point. A host printer's exponent
form (`1e21`, `1e-7`) is expanded: `1e21` is `1000000000000000000000.0`, `1e-7` is `0.0000001`.

## 3. Strings

A string is written as the bare word when its text matches the symbol rule (`[a-zA-Z_][0-9a-zA-Z_.\-@]*`) and is not
`true` or `false`. Otherwise it is double-quoted. Inside the quotes `"` is `\"`, `\` is `\\`, and backspace, form feed,
line feed, carriage return and tab are `\b`, `\f`, `\n`, `\r`, `\t`. Every other character is written as itself. The slash
is not escaped. The empty string is `""`. The fenced string form is never written.

## 4. Measurements and ranges

- `measurement`: the magnitude's text immediately followed by the unit: `5kg`, `5.0kg`, `-2.5m`.
- `range`: `lower..upper`, `lower...` or `...upper`, each end written as a numeric value or measurement: `1..5`, `1.5m..2m`, `1...`, `...5`.

## 5. Arrays and maps

- `array`: `[`, elements separated by `, `, `]`: `[1, 2]`, `[]`.
- `map`: `{`, entries as `key: value` separated by `, `, `}`: `{a: 1, b: [1, 2]}`, `{}`. Keys are written as strings (section 3), including quoting `true` and `false` although the grammar allows them bare, and entries are in ascending byte order of the keys' UTF-8 text.

## 6. Unwritable values

A writer MUST return an error with code `unwritable`, naming the part by path ([results.md](results.md)), and never write
a near equivalent, for:

- an array that holds an array or a map;
- a map entry whose value is a map;
- a measurement whose magnitude is `0` and whose unit begins with `x`, because its text can read as a hexadecimal literal.

When the outermost map is written as an element's property record (the writer is told), a record property MAY hold a map
whose entries are scalars or arrays. A range with no ends cannot be constructed ([values.md](values.md) section 3).

## 7. Examples

| Value | Text |
|---|---|
| `integer` 16 | `16` |
| `decimal` 1.0 | `1.0` |
| `measurement` of `integer` 5 and `kg` | `5kg` |
| `measurement` of `decimal` 5.0 and `kg` | `5.0kg` |
| `string` `a-b` | `a-b` |
| `string` `a b` | `"a b"` |
| `string` with a line feed | `"line one\nline two"` |
| `map` {b: 1, a: 2} | `{a: 2, b: 1}` |
| `tagged` `point` and `30 10` | ``point`30 10` `` |
