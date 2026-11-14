module github.com/gram-data/gram-value/go/conformance

go 1.23

require (
	github.com/gram-data/gram-value/go v0.0.0
	github.com/tree-sitter/go-tree-sitter v0.25.0
	github.com/tree-sitter/tree-sitter-gram v0.7.0
)

require github.com/mattn/go-pointer v0.0.1 // indirect

// The library is the parent directory. This module holds the conformance runner, so that the library module
// itself has no dependencies (and `go mod tidy` in a consumer never looks at tree-sitter).
replace github.com/gram-data/gram-value/go => ../

// tree-sitter-gram's Go binding declares the module path github.com/tree-sitter/tree-sitter-gram while the
// repository is gram-data/tree-sitter-gram; this points the declared path at the real repository.
replace github.com/tree-sitter/tree-sitter-gram => github.com/gram-data/tree-sitter-gram v0.7.0
