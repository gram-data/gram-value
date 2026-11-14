# gram-value for Python

The data values of the [gram](https://github.com/gram-data/tree-sitter-gram) notation, in pure Python with no
dependencies: the eleven value kinds, equality and order, canonical gram text, interpretation of the ten known
tags, and the mapping to Python values. The rules are in [`spec/`](../spec/README.md) and the decisions in
[`design/`](../design/adr/README.md); the mapping table is [`docs/mappings/python.md`](../docs/mappings/python.md).

```python
import gram_value as gv

m = gv.measurement(gv.integer(5), "kg").unwrap()
gv.to_text(m).unwrap()  # "5kg"
gv.to_native(gv.integer(2**70)).unwrap()  # 1180591620717411303424: an int, exact
```

Constructors and conversions that can find a problem return a `Result` with `ok`, `value` and `diagnostics`; reading
`.value` of a failed result raises `GramValueError`. Every Python entry of the mapping is exact, so `to_native`
never reports a loss (`fail_on_loss` is there for symmetry with the other languages). This package does not parse
gram text; use tree-sitter-gram for that.

```sh
uv sync
uv run pytest
uv run mypy
```
