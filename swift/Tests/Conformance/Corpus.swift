// Runs the language-neutral conformance corpus (ADR-001) against this package.
//
// The corpus is gram text. Each case's `value` is read with tree-sitter-gram and lowered to a `Value` by the
// functions below, which live in test code and are not shipped (ADR-001 2.3). SwiftTreeSitter parses UTF-16
// text, so node text is taken from `Node.range` (an NSRange over UTF-16 code units), not from byte ranges.

import Foundation
import GramValue
import SwiftTreeSitter
import Testing
import TreeSitterGram

let grammarVersion = "0.7.0"

struct Failure: Error, CustomStringConvertible {
    let description: String
    init(_ d: String) { description = d }
}

func check(_ cond: Bool, _ message: @autoclosure () -> String) throws {
    if !cond { throw Failure(message()) }
}

// MARK: - locations

let testsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
let corpusDir = testsDir.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("conformance")

// MARK: - parsing and lowering

func parse(_ src: String) throws -> Node {
    let parser = Parser()
    try parser.setLanguage(Language(language: tree_sitter_gram()))
    guard let tree = parser.parse(src), let root = tree.rootNode else { throw Failure("parse failed") }
    return root
}

struct Source {
    let src: String
    let ns: NSString
    init(_ s: String) {
        src = s
        ns = s as NSString
    }

    func text(_ n: Node) -> String { ns.substring(with: n.range) }

    func content(_ n: Node) -> String {
        n.child(byFieldName: "content").map(text) ?? ""
    }

    func named(_ n: Node) -> [Node] {
        (0..<n.namedChildCount).compactMap { n.namedChild(at: $0) }.filter { $0.nodeType != "comment" && $0.nodeType != "doc_comment" }
    }

    func children(_ n: Node) -> [Node] {
        (0..<n.childCount).compactMap { n.child(at: $0) }
    }

    func walk(_ n: Node, _ visit: (Node) -> Void) {
        visit(n)
        for c in children(n) { walk(c, visit) }
    }
}

func unescape(_ s: String, quote: Character) throws -> String {
    var out = ""
    var it = s.makeIterator()
    while let c = it.next() {
        guard c == "\\" else {
            out.append(c)
            continue
        }
        guard let e = it.next() else { throw Failure("dangling escape") }
        switch e {
        case "b": out.append("\u{8}")
        case "f": out.append("\u{c}")
        case "n": out.append("\n")
        case "r": out.append("\r")
        case "t": out.append("\t")
        case "/", "\\": out.append(e)
        default:
            guard e == quote else { throw Failure("escape the grammar does not allow: \(e)") }
            out.append(e)
        }
    }
    return out
}

extension Source {
    func stringValue(_ n: Node) throws -> String {
        let full = text(n)
        if full.hasPrefix("```") { return content(n) }
        return try unescape(content(n), quote: full.first!)
    }

    func lowerNumberText(_ t: String) throws -> Value {
        if t.contains(".") { return try Value.makeDecimal(fromText: t) }
        return try Value.makeInteger(fromDigits: t == "-0" ? "0" : t)
    }

    func lowerNumeric(_ n: Node) throws -> Value {
        let t = text(n)
        switch n.nodeType {
        case "integer", "decimal": return try lowerNumberText(t)
        case "hexadecimal": return try radix(String(t.dropFirst(2)), 16)
        case "octal": return try radix(String(t.dropFirst(1)), 8)
        case "measurement":
            let split = t.firstIndex { $0.isLetter }!
            return try Value.makeMeasurement(try lowerNumberText(String(t[..<split])), unit: String(t[split...]))
        default: throw Failure("not a numeric literal: \(n.nodeType ?? "?")")
        }
    }

    /// Digits in `base` as an integer or big integer, for literals of any size.
    func radix(_ digits: String, _ base: Int) throws -> Value {
        var dec: [Int] = [0]  // little-endian decimal digits
        for ch in digits {
            guard var carry = ch.hexDigitValue else { throw Failure("digit") }
            for i in dec.indices {
                let v = dec[i] * base + carry
                dec[i] = v % 10
                carry = v / 10
            }
            while carry > 0 {
                dec.append(carry % 10)
                carry /= 10
            }
        }
        return try Value.makeInteger(fromDigits: String(dec.reversed().map { Character(String($0)) }))
    }

    func keyName(_ n: Node) throws -> String {
        switch n.nodeType {
        case "quoted_name": return try unescape(content(n), quote: "`")
        case "double_quoted_name": return try unescape(content(n), quote: "\"")
        default: return text(n)
        }
    }

    func lower(_ n: Node) throws -> Value {
        switch n.nodeType {
        case "integer", "decimal", "hexadecimal", "octal", "measurement": return try lowerNumeric(n)
        case "boolean_literal": return .boolean(text(n) == "true")
        case "symbol": return .string(text(n))
        case "string_literal": return .string(try stringValue(n))
        case "tagged_string":
            let body = text(n).hasPrefix("```") ? content(n) : try unescape(content(n), quote: "`")
            return try Value.makeTagged(text(n.child(byFieldName: "tag")!), body)
        case "range":
            let lo = try n.child(byFieldName: "lower").map(lowerNumeric)
            let hi = try n.child(byFieldName: "upper").map(lowerNumeric)
            return try Value.makeRange(lower: lo, upper: hi)
        case "array": return .array(try named(n).map(lower))
        case "map":
            return try Value.makeMap(try named(n).map {
                Value.Entry(try keyName($0.child(byFieldName: "key")!), try lower($0.child(byFieldName: "value")!))
            })
        default: throw Failure("cannot lower node kind \(n.nodeType ?? "?")")
        }
    }
}

// MARK: - cases

struct Case {
    let id: String
    let label: String
    let props: [String: Value]
    var value: Value { props["value"] ?? .boolean(false) }

    func str(_ key: String) throws -> String {
        guard case .string(let s)? = props[key] else { throw Failure("case \(id): property \(key) should be a string") }
        return s
    }
}

func loadNodes(_ url: URL) throws -> [Case] {
    let srcText = try String(contentsOf: url, encoding: .utf8)
    let root = try parse(srcText)
    try check(!root.hasError, "\(url.path) has a syntax error")
    let s = Source(srcText)
    var cases: [Case] = []
    var errors: [Error] = []
    s.walk(root) { n in
        guard n.nodeType == "node_pattern", let id = n.child(byFieldName: "identifier"),
            let labels = n.child(byFieldName: "labels"), let record = n.child(byFieldName: "record")
        else { return }
        do {
            var props: [String: Value] = [:]
            for p in s.named(record) {
                props[try s.keyName(p.child(byFieldName: "key")!)] = try s.lower(p.child(byFieldName: "value")!)
            }
            cases.append(Case(id: s.text(id), label: s.text(s.named(labels)[0]), props: props))
        } catch { errors.append(error) }
    }
    if let e = errors.first { throw e }
    return cases.sorted { $0.id.utf8.lexicographicallyPrecedes($1.id.utf8) }
}

func vocabulary() throws -> [String: Set<String>] {
    let srcText = try String(contentsOf: corpusDir.appendingPathComponent("vocabulary.gram"), encoding: .utf8)
    let root = try parse(srcText)
    try check(!root.hasError, "vocabulary.gram has a syntax error")
    let s = Source(srcText)
    var vocab: [String: Set<String>] = [:]
    s.walk(root) { n in
        guard n.nodeType == "node_pattern", let id = n.child(byFieldName: "identifier"),
            let labels = n.child(byFieldName: "labels")
        else { return }
        for l in s.named(labels) { vocab[s.text(l), default: []].insert(s.text(id)) }
    }
    return vocab
}

func loadSkips() throws -> [String: String] {
    var skips: [String: String] = [:]
    for node in try loadNodes(testsDir.appendingPathComponent("skips.gram")) {
        try check(node.label == "Skip", "\(node.id): a skips file holds Skip nodes")
        skips[try node.str("case")] = try node.str("reason")
    }
    return skips
}
