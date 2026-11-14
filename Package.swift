// swift-tools-version:5.9
import PackageDescription

// The Swift package of gram-value (ADR-007 2.8). SwiftPM reads the root of the repository, so this manifest
// points at swift/Sources. The library has no dependencies; the conformance runner and unit tests are a
// separate package in swift/Tests so that users of the library do not fetch tree-sitter.
let package = Package(
    name: "GramValue",
    platforms: [.macOS(.v13), .iOS(.v16), .tvOS(.v16), .watchOS(.v9)],
    products: [
        .library(name: "GramValue", targets: ["GramValue"]),
    ],
    targets: [
        .target(name: "GramValue", path: "swift/Sources/GramValue"),
    ]
)
