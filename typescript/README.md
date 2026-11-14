# gram-value for TypeScript

The data values of the [gram](https://github.com/gram-data/tree-sitter-gram) notation, in plain TypeScript with no
dependencies: the eleven value kinds, equality and order, canonical gram text, interpretation of the ten known tags,
and the mapping to JavaScript values. The rules are in [`spec/`](https://github.com/gram-data/gram-value/tree/main/spec) and the decisions in
[`design/`](https://github.com/gram-data/gram-value/tree/main/design/adr); the mapping table is
[`docs/mappings/typescript.md`](https://github.com/gram-data/gram-value/blob/main/docs/mappings/typescript.md).

```ts
import { integer, measurement, toText, toNative, unwrap } from "@gram-data/value";

const m = unwrap(measurement(integer(5), "kg"));
unwrap(toText(m)); // "5kg"
unwrap(toNative(integer(2n ** 53n))); // 9007199254740992n: a bigint, because a number would round it
```

Constructors and conversions that can find a problem return a result, `{ ok, value, diagnostics }`. A conversion to
JavaScript values never fails because of a loss: it returns the value and a list of losses (`precision`, `kind`), and
`failOnLoss` turns any loss into a failure. This package does not parse gram text; use tree-sitter-gram for that.

## Development

The source is in the [gram-value repository](https://github.com/gram-data/gram-value) (`typescript/`):

```sh
npm ci
npm run typecheck
npm test
```
