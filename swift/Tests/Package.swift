// swift-tools-version:5.9
import PackageDescription

// The conformance runner and unit tests of the Swift package (ADR-001, ADR-007 2.8). This is a separate package
// so that users of the GramValue library do not fetch tree-sitter. Run `swift test` in this directory.
let package = Package(
    name: "GramValueTests",
    platforms: [.macOS(.v13)],
    dependencies: [
        .package(path: "../.."),
        .package(url: "https://github.com/gram-data/tree-sitter-gram", exact: "0.7.0"),
        .package(url: "https://github.com/ChimeHQ/SwiftTreeSitter", from: "0.25.0"),
    ],
    targets: [
        .testTarget(
            name: "ConformanceTests",
            dependencies: [
                .product(name: "GramValue", package: "gram-value"),
                .product(name: "SwiftTreeSitter", package: "SwiftTreeSitter"),
                .product(name: "TreeSitterGram", package: "tree-sitter-gram"),
            ],
            path: "Conformance"
        ),
    ]
)
