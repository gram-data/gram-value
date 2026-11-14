# gram-value for Go

The data values of the [gram](https://github.com/gram-data/tree-sitter-gram) notation, in plain Go with no
dependencies: the eleven value kinds, equality and order, canonical gram text, interpretation of the ten known
tags, and the mapping to Go values. The rules are in [`spec/`](../spec/README.md) and the decisions in
[`design/`](../design/adr/README.md); the mapping table is [`docs/mappings/go.md`](../docs/mappings/go.md).

```go
import gv "github.com/gram-data/gram-value/go"

m, _ := gv.NewMeasurement(gv.NewInteger(5), "kg")
text, _ := gv.ToText(m, gv.ContextValue) // "5kg"
```

Operations that can find a problem return an `error` that is a `gv.Diagnostics` value holding every problem found,
each with its path (`errors.As(err, &diagnostics)`). Every Go entry of the mapping is exact or unsupported, so no
conversion reports a loss; a `big_decimal` has no Go standard library type and fails `ToNative` with code
`unsupported` (keep the `BigDecimalValue`). This package does not parse gram text; use tree-sitter-gram for that.

## Development

```sh
go test ./...                       # unit tests of the library
cd conformance && go test ./...     # the conformance runner (a separate module)
```

The library module has no dependencies. The conformance runner is a separate module, `go/conformance`, because
`go mod tidy` in a consumer also looks at the imports of a dependency's tests, and a test-only dependency would
then break it. The runner reads the corpus with tree-sitter-gram's Go binding, which needs a C compiler (cgo). The
binding's module declares the path `github.com/tree-sitter/tree-sitter-gram` while the repository is
`gram-data/tree-sitter-gram`, so the runner's `go.mod` has a `replace` line that points the declared path at the real
repository.

Go has no package registry: a release is a tag, `go/vX.Y.Z`, which the Go module proxy serves.
