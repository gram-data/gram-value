package gramvalue

import (
	"fmt"
	"strconv"
	"strings"
)

// PathSegment is one step of a path into a value: a map key or an array index.
type PathSegment struct {
	key     string
	index   int
	isIndex bool
}

// Key is the path segment for a map key.
func Key(k string) PathSegment { return PathSegment{key: k} }

// Index is the path segment for an array index.
func Index(i int) PathSegment { return PathSegment{index: i, isIndex: true} }

// IsIndex reports whether the segment is an array index.
func (s PathSegment) IsIndex() bool { return s.isIndex }

// String is the segment's key or index as text.
func (s PathSegment) String() string {
	if s.isIndex {
		return strconv.Itoa(s.index)
	}
	return s.key
}

// A Diagnostic is a problem found by an operation.
type Diagnostic struct {
	// Code is a lower-case word with hyphens, such as "duplicate-key".
	Code string
	// Message is text for a person; it is not part of the contract.
	Message string
	// Path is the part of the value concerned; empty for the whole value.
	Path []PathSegment
}

// Pointer is the diagnostic's path as a JSON Pointer (RFC 6901): "/items/0/name", empty for the whole value.
func (d Diagnostic) Pointer() string { return PathToPointer(d.Path) }

// String formats the diagnostic as "code: message (at /path)".
func (d Diagnostic) String() string {
	if len(d.Path) == 0 {
		return d.Code + ": " + d.Message
	}
	return fmt.Sprintf("%s: %s (at %s)", d.Code, d.Message, d.Pointer())
}

// PathToPointer writes a path as a JSON Pointer (RFC 6901).
func PathToPointer(path []PathSegment) string {
	var b strings.Builder
	for _, s := range path {
		b.WriteByte('/')
		text := strings.ReplaceAll(s.String(), "~", "~0")
		b.WriteString(strings.ReplaceAll(text, "/", "~1"))
	}
	return b.String()
}

// Diagnostics holds every problem found by a failed operation. It is the error those operations return.
type Diagnostics []Diagnostic

// Error joins the diagnostics.
func (ds Diagnostics) Error() string {
	parts := make([]string, len(ds))
	for i, d := range ds {
		parts[i] = d.String()
	}
	return strings.Join(parts, "; ")
}

func diag(code, message string) Diagnostics {
	return Diagnostics{{Code: code, Message: message}}
}

// under puts a segment in front of every path.
func under(seg PathSegment, ds Diagnostics) Diagnostics {
	out := make(Diagnostics, len(ds))
	for i, d := range ds {
		out[i] = Diagnostic{Code: d.Code, Message: d.Message, Path: append([]PathSegment{seg}, d.Path...)}
	}
	return out
}

func at(path []PathSegment, d Diagnostics) Diagnostics {
	out := make(Diagnostics, len(d))
	for i, x := range d {
		out[i] = Diagnostic{Code: x.Code, Message: x.Message, Path: append([]PathSegment(nil), path...)}
	}
	return out
}
