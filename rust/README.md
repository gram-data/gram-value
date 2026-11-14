# gram-value for Rust

The data values of the [gram](https://github.com/gram-data/tree-sitter-gram) notation, with no dependencies: the
eleven value kinds, equality and order, canonical gram text, interpretation of the ten known tags, and conversion
to and from Rust's own types with `From` and `TryFrom`. The rules are in
[`spec/`](https://github.com/gram-data/gram-value/tree/main/spec) and the decisions in
[`design/`](https://github.com/gram-data/gram-value/tree/main/design/adr).

```rust
use gram_value::Value;

let m = Value::measurement(Value::integer(5), "kg").unwrap();
assert_eq!(m.to_text().unwrap(), "5kg");
```

This crate does not parse gram text; use tree-sitter-gram for that.
