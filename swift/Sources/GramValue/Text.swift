/// Whether the outermost map is an ordinary value or an element's property record.
public enum Context: Sendable {
    /// An ordinary value: map entries hold scalars or arrays.
    case value
    /// An element's property record: a property may also hold a map of scalars and arrays.
    case record
}

private func escape(_ s: String, quote: Unicode.Scalar) -> String {
    var out = String.UnicodeScalarView()
    for c in s.unicodeScalars {
        switch c {
        case "\\": out.append(contentsOf: "\\\\".unicodeScalars)
        case "\u{8}": out.append(contentsOf: "\\b".unicodeScalars)
        case "\u{c}": out.append(contentsOf: "\\f".unicodeScalars)
        case "\n": out.append(contentsOf: "\\n".unicodeScalars)
        case "\r": out.append(contentsOf: "\\r".unicodeScalars)
        case "\t": out.append(contentsOf: "\\t".unicodeScalars)
        default:
            if c == quote { out.append("\\") }
            out.append(c)
        }
    }
    return String(out)
}

/// A string in canonical text: the bare word when it can be, else double-quoted.
public func stringText(_ s: String) -> String {
    isBareWord(s) ? s : "\"" + escape(s, quote: "\"") + "\""
}

private enum ScalarText {
    case text(String)
    case reason(String)
}

private func scalarText(_ v: Value) -> ScalarText {
    switch v {
    case .integer(let n): return .text(String(n))
    case .bigInteger(let s), .bigDecimal(let s): return .text(s)
    case .decimal(let f): return .text(floatText(f))
    case .boolean(let b): return .text(b ? "true" : "false")
    case .string(let s): return .text(stringText(s))
    case .tagged(let tag, let content): return .text(tag + "`" + escape(content, quote: "`") + "`")
    case .measurement(let magnitude, let unit):
        switch scalarText(magnitude) {
        case .reason(let r): return .reason(r)
        case .text(let mag):
            if mag == "0" && unit.hasPrefix("x") {
                return .reason("a measurement of 0 with a unit beginning x can read as a hexadecimal literal")
            }
            return .text(mag + unit)
        }
    case .range(let lower, let upper):
        var lo: String?, hi: String?
        if let lower {
            switch scalarText(lower) {
            case .reason(let r): return .reason(r)
            case .text(let t): lo = t
            }
        }
        if let upper {
            switch scalarText(upper) {
            case .reason(let r): return .reason(r)
            case .text(let t): hi = t
            }
        }
        switch (lo, hi) {
        case (let l?, let h?): return .text("\(l)..\(h)")
        case (let l?, nil): return .text("\(l)...")
        case (nil, let h?): return .text("...\(h)")
        case (nil, nil): return .reason("a range with no ends")
        }
    case .array, .map: return .reason("not a scalar")
    }
}

private func unwritable(_ path: [PathSegment], _ what: String) -> Diagnostic {
    Diagnostic(code: "unwritable", message: what, path: path)
}

private func writeArray(_ items: [Value], _ path: [PathSegment], _ errs: inout [Diagnostic]) -> String {
    var parts: [String] = []
    for (i, item) in items.enumerated() {
        let p = path + [.index(i)]
        switch item {
        case .array: errs.append(unwritable(p, "an array cannot hold an array"))
        case .map: errs.append(unwritable(p, "an array cannot hold a map"))
        default:
            switch scalarText(item) {
            case .text(let t): parts.append(t)
            case .reason(let r): errs.append(unwritable(p, r))
            }
        }
    }
    return "[" + parts.joined(separator: ", ") + "]"
}

private func writeMap(_ entries: [Value.Entry], nestedOK: Bool, _ path: [PathSegment], _ errs: inout [Diagnostic]) -> String {
    var parts: [String] = []
    for e in entries {
        let p = path + [.key(e.key)]
        var text: String?
        switch e.value {
        case .array(let items): text = writeArray(items, p, &errs)
        case .map(let inner):
            if nestedOK {
                text = writeMap(inner, nestedOK: false, p, &errs)
            } else {
                errs.append(unwritable(p, "a map entry cannot hold a map"))
            }
        default:
            switch scalarText(e.value) {
            case .text(let t): text = t
            case .reason(let r): errs.append(unwritable(p, r))
            }
        }
        if let text { parts.append("\(stringText(e.key)): \(text)") }
    }
    return "{" + parts.joined(separator: ", ") + "}"
}

extension Value {
    /// The canonical text of a value. A part the grammar has no text for is reported with code `unwritable` and
    /// its path; the thrown `Diagnostics` holds every such part.
    public func toText(context: Context = .value) throws -> String {
        var errs: [Diagnostic] = []
        let text: String
        switch self {
        case .array(let items): text = writeArray(items, [], &errs)
        case .map(let entries): text = writeMap(entries, nestedOK: context == .record, [], &errs)
        default:
            switch scalarText(self) {
            case .text(let t): text = t
            case .reason(let r):
                errs.append(unwritable([], r))
                text = ""
            }
        }
        if !errs.isEmpty { throw Diagnostics(errs) }
        return text
    }
}
