import Foundation
import GramValue
import SwiftTreeSitter
import Testing

func checkVocab(_ vocab: [String: Set<String>], _ group: String, _ name: String, _ id: String) throws {
    try check(vocab[group]?.contains(name) == true, "case \(id): \"\(name)\" is not in the vocabulary group \(group)")
}

func textOf(_ v: Value) throws -> String {
    do { return try v.toText() } catch { throw Failure("unwritable: \(error)") }
}

func loadValue(fromText text: String) throws -> Value {
    let src = "(x {value: \(text)})"
    let root = try parse(src)
    try check(!root.hasError, "canonical text \(text) does not parse")
    let s = Source(src)
    var found: Value?
    var failure: Error?
    s.walk(root) { n in
        if n.nodeType == "record_property", found == nil {
            do { found = try s.lower(n.child(byFieldName: "value")!) } catch { failure = error }
        }
    }
    if let failure { throw failure }
    return found!
}

let tagForNative = ["Date": "date", "Time": "time", "DateTime": "datetime", "Duration": "duration", "Point": "point", "UUID": "uuid", "Week": "week", "Embedding": "embedding"]

func nativeTypeName(_ n: Any) -> String {
    switch n {
    case is Int64: return "Int64"
    case is Double: return "Double"
    case is Bool: return "Bool"
    case is String: return "String"
    case is GramTagged: return "Tagged"
    case is GramRange: return "Range"
    case is GramMeasurement: return "Measurement"
    case is [Any]: return "Array"
    case is [String: Any]: return "Dictionary"
    default: return "\(type(of: n))"
    }
}

func runMapping(_ c: Case, _ vocab: [String: Set<String>]) throws {
    for k in c.props.keys where k != "name" && k != "value" { try checkVocab(vocab, "Language", k, c.id) }
    guard case .map? = c.props["swift"], case .map(let entries)? = c.props["swift"] else { return }
    func word(_ k: String) -> String? {
        for e in entries where e.key == k { if case .string(let s) = e.value { return s } }
        return nil
    }
    let cls = word("class")!, native = word("native")!
    try checkVocab(vocab, "Class", cls, c.id)
    try checkVocab(vocab, "SwiftNative", native, c.id)
    var wantLoss: [String] = []
    for e in entries where e.key == "loss" {
        if case .array(let items) = e.value {
            for i in items { if case .string(let s) = i { wantLoss.append(s); try checkVocab(vocab, "Loss", s, c.id) } }
        }
    }
    try check((cls == "lossy") == !wantLoss.isEmpty, "loss kinds are present exactly when the class is lossy")
    // Swift never reports a loss (ADR-006): every entry is exact or unsupported.
    try check(cls != "lossy", "Swift has no lossy entries")

    if cls == "unsupported" {
        try check(native == "none", "an unsupported entry has native none")
        do {
            _ = try c.value.toNative()
            throw Failure("the conversion should fail")
        } catch let d as Diagnostics {
            try check(d.items.first?.code == "unsupported", "code \(d.items.first?.code ?? "?")")
        }
        return
    }
    let back: Value
    if let tag = tagForNative[native] {
        guard let i = try interpret(c.value) else { throw Failure("a known tag interprets") }
        try check(i.tag == tag, "the typed value is for a different tag")
        back = i.toValue()
    } else {
        let n = try c.value.toNative()
        try check(nativeTypeName(n) == native, "native type \(nativeTypeName(n)), want \(native)")
        back = try Value.fromNative(n)
    }
    let backText = try textOf(back)
    try check(backText == word("roundtrip"), "round trip through \(native): \(backText)")
    try check(back == c.value, "exact means the round trip gives an equal value")
}

func runCase(_ c: Case, _ vocab: [String: Set<String>]) throws {
    switch c.label {
    case "ValueCase":
        let kind = try c.str("kind")
        try checkVocab(vocab, "Kind", kind, c.id)
        try check(c.value.kind.description == kind, "wrong kind \(c.value.kind)")
    case "TextCase":
        let got = try textOf(c.value)
        let canonical = try c.str("canonical")
        try check(got == canonical, "canonical text \(got)")
        let again = try loadValue(fromText: got)
        try check(again == c.value, "canonical text \(got) reads back as a different value")
    case "TagCase":
        let outcome = try c.str("outcome")
        try checkVocab(vocab, "Outcome", outcome, c.id)
        guard case .tagged(let tag, _) = c.value else { throw Failure("a TagCase value is a tagged string") }
        try checkVocab(vocab, "Tag", tag, c.id)
        if outcome == "accepted" {
            guard let i = try? interpret(c.value) else { throw Failure("expected accepted") }
            let interpreted = try c.str("interpreted")
            try check(i.normalContent == interpreted, "normal content \(i.normalContent)")
        } else {
            do {
                _ = try interpret(c.value)
                throw Failure("expected rejected")
            } catch let d as Diagnostics {
                let want = try c.str("diagnostic")
                try checkVocab(vocab, "Diagnostic", want, c.id)
                try check(d.items.first?.code == want, "diagnostic code \(d.items.first?.code ?? "?") (\(d.items.first?.message ?? ""))")
            }
        }
    case "MappingCase": try runMapping(c, vocab)
    default: throw Failure("unknown case kind \(c.label)")
    }
}

@Test func corpus() throws {
    let version = try String(contentsOf: corpusDir.appendingPathComponent("GRAMMAR_VERSION"), encoding: .utf8)
    #expect(version.trimmingCharacters(in: .whitespacesAndNewlines) == grammarVersion)
    let vocab = try vocabulary()
    let skips = try loadSkips()
    var seen = Set<String>()
    var counts: [String: Int] = [:]
    var failures: [String] = []
    var results = ""
    for file in ["values.gram", "text.gram", "tags.gram", "mappings.gram"] {
        for c in try loadNodes(corpusDir.appendingPathComponent(file)) {
            try check(!seen.contains(c.id), "case id \(c.id) is used twice")
            seen.insert(c.id)
            // A mapping case with no entry for this language does not apply to it (ADR-001 2.4).
            if c.label == "MappingCase", c.props["swift"] == nil {
                counts["not-applicable", default: 0] += 1
                results += "(\(c.id):Result {language: swift, status: not-applicable})\n"
                continue
            }
            var problem: String?
            do { try runCase(c, vocab) } catch let e { problem = "\(e)" }
            var status = "passed"
            var extra = ""
            switch (problem, skips[c.id]) {
            case (nil, nil): break
            case (nil, _?):
                status = "failed"
                failures.append("\(c.id): unexpected pass of a skipped case")
            case (_?, let reason?):
                status = "skipped"
                extra = ", reason: \"\(reason.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\""))\""
            case (let e?, nil):
                status = "failed"
                failures.append("\(c.id): \(e)")
            }
            counts[status, default: 0] += 1
            results += "(\(c.id):Result {language: swift, status: \(status)\(extra)})\n"
        }
    }
    for id in skips.keys where !seen.contains(id) { failures.append("skips.gram names an unknown case: \(id)") }
    let outDir = testsDir.appendingPathComponent("target")
    try FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
    try results.write(to: outDir.appendingPathComponent("conformance-results.gram"), atomically: true, encoding: .utf8)
    #expect(failures.isEmpty, "\(failures.count) failed of \(seen.count):\n\(failures.joined(separator: "\n"))")
    #expect(seen.count > 100, "the corpus should hold many cases, found \(seen.count)")
    print("conformance: \(counts)")
}
