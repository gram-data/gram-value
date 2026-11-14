module github.com/gram-data/gram-value/go

go 1.23

require (
	github.com/tree-sitter/go-tree-sitter v0.25.0
	github.com/tree-sitter/tree-sitter-gram v0.7.0
)

require github.com/mattn/go-pointer v0.0.1 // indirect

replace github.com/tree-sitter/tree-sitter-gram => github.com/gram-data/tree-sitter-gram v0.7.0
