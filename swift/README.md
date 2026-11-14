# gram-value for Swift

The data values of the [gram](https://github.com/gram-data/tree-sitter-gram) notation, in plain Swift with no
dependencies: the eleven value kinds, equality and order, canonical gram text, interpretation of the ten known
tags, and the mapping to Swift values. The rules are in [`spec/`](../spec/README.md) and the decisions in
[`design/`](../design/adr/README.md); the mapping table is [`docs/mappings/swift.md`](../docs/mappings/swift.md).

Add it with SwiftPM (the package manifest is the `Package.swift` at the root of the repository, like
tree-sitter-gram's):

```swift
.package(url: "https://github.com/gram-data/gram-value", from: "0.1.0")
// target dependency: .product(name: "GramValue", package: "gram-value")
```

```swift
import GramValue

let m = try Value.makeMeasurement(.integer(5), unit: "kg")
try m.toText()   // "5kg"
```

`Value` is an enum of the eleven kinds. Values that need validation (a tag, a unit, a map's keys, a range's ends, the
digits of a big number) are built with the `make...` functions, which throw `Diagnostics`, an error that holds every
problem found with its path. Every Swift entry of the mapping is exact or unsupported, so no conversion reports a
loss; a `bigInteger` or `bigDecimal` has no Swift standard library type and fails `toNative()` with code `unsupported`
(keep the `Value`). Strings compare by their UTF-8 bytes, not by Swift's canonical equivalence, because gram does.
This package does not parse gram text; use tree-sitter-gram for that.

## Development

The conformance runner and unit tests are a separate package, `swift/Tests`, so that users of the library do not
fetch tree-sitter. They use Swift Testing and need Swift 6 (Xcode 16 or a swift.org toolchain):

```sh
swift build                 # the library, from the repository root
cd swift/Tests && swift test
```

On a machine with only the Xcode Command Line Tools, the first `swift test` can fail with "plugin for module
'TestingMacros' not found". Pass the plugin path once:

```sh
swift test -Xswiftc -plugin-path -Xswiftc /Library/Developer/CommandLineTools/usr/lib/swift/host/plugins/testing
```

Swift packages are versioned by repository tags, and SwiftPM only sees plain semantic versions, so the Swift
release tag is the unprefixed `vX.Y.Z` (see [RELEASING.md](../RELEASING.md)). `swift/VERSION` records the version.
