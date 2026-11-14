// Package gramvalue models the data values of the gram notation.
//
// It models every value gram can write (integers, decimals, booleans, strings, tagged strings, ranges,
// measurements, arrays and maps), compares them, writes each as canonical gram text, interprets the ten
// known tags, and maps values to Go values. It does not parse: tree-sitter-gram does. The rules are in
// spec/ of the repository; the decisions are ADR-002 to ADR-007 under design/adr/.
//
// Operations that can find problems return an error that is a [Diagnostics] value holding every problem
// found, each with its path. Every Go mapping entry is exact or unsupported, so no conversion reports a loss.
package gramvalue
