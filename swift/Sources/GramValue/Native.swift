/// The Swift value of a tagged string: a tag and a content string.
public struct GramTagged: Sendable, Hashable {
    public let tag: String
    public let content: String
    public init(tag: String, content: String) {
        self.tag = tag
        self.content = content
    }
}

/// The Swift value of a measurement. `magnitude` is an `Int64` or a `Double`.
public struct GramMeasurement: @unchecked Sendable {
    public let magnitude: Any
    public let unit: String
    public init(magnitude: Any, unit: String) {
        self.magnitude = magnitude
        self.unit = unit
    }
}

/// The Swift value of a range, inclusive. `lower` and `upper` are nil, an `Int64`, a `Double` or a
/// `GramMeasurement`.
public struct GramRange: @unchecked Sendable {
    public let lower: Any?
    public let upper: Any?
    public init(lower: Any?, upper: Any?) {
        self.lower = lower
        self.upper = upper
    }
}

private func numericToNative(_ v: Value) throws -> Any {
    switch v {
    case .integer(let n): return n
    case .decimal(let f): return f
    case .bigInteger:
        throw Diagnostics(code: "unsupported", message: "a big integer has no Swift standard library type; keep the Value")
    default:
        throw Diagnostics(code: "unsupported", message: "a big decimal has no Swift standard library type; keep the Value")
    }
}

extension Value {
    /// The Swift value (ADR-006): `Int64`, `Double`, `Bool`, `String`, `GramTagged`, `GramRange`,
    /// `GramMeasurement`, `[Any]` and `[String: Any]`. A big integer or big decimal has no native value and
    /// fails with code `unsupported`. Every entry is exact or unsupported, so a conversion never reports a loss.
    /// The thrown `Diagnostics` holds every problem found.
    public func toNative() throws -> Any {
        switch self {
        case .integer, .bigInteger, .decimal, .bigDecimal: return try numericToNative(self)
        case .boolean(let b): return b
        case .string(let s): return s
        case .tagged(let tag, let content): return GramTagged(tag: tag, content: content)
        case .measurement(let magnitude, let unit):
            do {
                return GramMeasurement(magnitude: try numericToNative(magnitude), unit: unit)
            } catch let d as Diagnostics {
                throw d.under(.key("magnitude"))
            }
        case .range(let lower, let upper):
            var errs: [Diagnostic] = []
            var ends: [Any?] = [nil, nil]
            for (i, (key, end)) in [("lower", lower), ("upper", upper)].enumerated() {
                guard let end else { continue }
                do {
                    ends[i] = end.kind == .measurement ? try end.toNative() : try numericToNative(end)
                } catch let d as Diagnostics {
                    errs += d.under(.key(key)).items
                }
            }
            if !errs.isEmpty { throw Diagnostics(errs) }
            return GramRange(lower: ends[0], upper: ends[1])
        case .array(let items):
            var out: [Any] = []
            var errs: [Diagnostic] = []
            for (i, item) in items.enumerated() {
                do { out.append(try item.toNative()) } catch let d as Diagnostics { errs += d.under(.index(i)).items }
            }
            if !errs.isEmpty { throw Diagnostics(errs) }
            return out
        case .map(let entries):
            var out: [String: Any] = [:]
            var errs: [Diagnostic] = []
            for e in entries {
                do { out[e.key] = try e.value.toNative() } catch let d as Diagnostics { errs += d.under(.key(e.key)).items }
            }
            if !errs.isEmpty { throw Diagnostics(errs) }
            // Swift compares strings by canonical equivalence: two keys that differ only in normalization
            // would collapse into one entry, so refuse instead of dropping one.
            if out.count != entries.count {
                throw Diagnostics(code: "unsupported", message: "two keys are canonically equivalent Strings, which a Swift dictionary cannot hold apart")
            }
            return out
        }
    }

    /// Convert a Swift value to a gram value. It accepts `Bool`, `String`, the integer types (a `UInt64` past
    /// the signed range becomes a big integer), finite `Double`, `GramTagged`, `GramRange`, `GramMeasurement`,
    /// arrays and string-keyed dictionaries. Anything else (nil, NaN, an infinity, `Float` (convert it
    /// explicitly), a non-string key, a closure, an object) throws `Diagnostics` listing every problem, with
    /// code `unsupported` or `not-finite` (ADR-006 2.6).
    public static func fromNative(_ x: Any) throws -> Value {
        var errs: [Diagnostic] = []
        let v = convertNative(x, [], &errs)
        if !errs.isEmpty { throw Diagnostics(errs) }
        return v ?? .boolean(false)
    }
}

private func unsupported(_ path: [PathSegment], _ what: String) -> Diagnostic {
    Diagnostic(code: "unsupported", message: what, path: path)
}

private func numberFromNative(_ x: Any, _ path: [PathSegment], _ errs: inout [Diagnostic]) -> Value? {
    switch x {
    case let n as Int64: return .integer(n)
    case let n as Int: return .integer(Int64(n))
    case let n as Double:
        guard n.isFinite else {
            errs.append(Diagnostic(code: "not-finite", message: "NaN and infinities cannot be written", path: path))
            return nil
        }
        return .decimal(n)
    default:
        errs.append(unsupported(path, "a number must be an integer type or a finite Double"))
        return nil
    }
}

private func convertNative(_ x: Any, _ path: [PathSegment], _ errs: inout [Diagnostic]) -> Value? {
    // Optionals arrive boxed in Any: nil is unsupported, and a present one is unwrapped.
    let mirror = Mirror(reflecting: x)
    if mirror.displayStyle == .optional {
        if let child = mirror.children.first { return convertNative(child.value, path, &errs) }
        errs.append(unsupported(path, "nil has no gram counterpart"))
        return nil
    }
    switch x {
    case let b as Bool: return .boolean(b)
    case let n as Int8: return .integer(Int64(n))
    case let n as Int16: return .integer(Int64(n))
    case let n as Int32: return .integer(Int64(n))
    case let n as Int64: return .integer(n)
    case let n as Int: return .integer(Int64(n))
    case let n as UInt8: return .integer(Int64(n))
    case let n as UInt16: return .integer(Int64(n))
    case let n as UInt32: return .integer(Int64(n))
    case let n as UInt64: return unsignedValue(n)
    case let n as UInt: return unsignedValue(UInt64(n))
    case let f as Double: return numberFromNative(f, path, &errs)
    case let s as String: return .string(s)
    case let t as GramTagged:
        do { return try Value.makeTagged(t.tag, t.content) } catch let d as Diagnostics { errs += d.at(path).items; return nil } catch { return nil }
    case let e as GramEmbedding:
        return e.toValue()
    case let m as GramMeasurement:
        guard let mag = numberFromNative(m.magnitude, path + [.key("magnitude")], &errs) else { return nil }
        do { return try Value.makeMeasurement(mag, unit: m.unit) } catch let d as Diagnostics { errs += d.at(path).items; return nil } catch { return nil }
    case let r as GramRange:
        var ends: [Value?] = [nil, nil]
        var failed = false
        for (i, (key, end)) in [("lower", r.lower), ("upper", r.upper)].enumerated() {
            guard let end else { continue }
            if let v = convertNative(end, path + [.key(key)], &errs) { ends[i] = v } else { failed = true }
        }
        if failed { return nil }
        do { return try Value.makeRange(lower: ends[0], upper: ends[1]) } catch let d as Diagnostics { errs += d.at(path).items; return nil } catch { return nil }
    default: break
    }
    switch mirror.displayStyle {
    case .collection:
        var items: [Value] = []
        var failed = false
        for (i, child) in mirror.children.enumerated() {
            if let v = convertNative(child.value, path + [.index(i)], &errs) { items.append(v) } else { failed = true }
        }
        return failed ? nil : .array(items)
    case .dictionary:
        var entries: [Value.Entry] = []
        var failed = false
        for child in mirror.children {
            let pair = Mirror(reflecting: child.value).children.map { $0.value }
            guard pair.count == 2, let key = pair[0] as? String else {
                errs.append(unsupported(path, "a map key must be a String"))
                failed = true
                continue
            }
            if let v = convertNative(pair[1], path + [.key(key)], &errs) { entries.append(Value.Entry(key, v)) } else { failed = true }
        }
        if failed { return nil }
        do { return try Value.makeMap(entries) } catch let d as Diagnostics { errs += d.at(path).items; return nil } catch { return nil }
    default:
        errs.append(unsupported(path, "\(type(of: x)) has no gram counterpart"))
        return nil
    }
}

private func unsignedValue(_ n: UInt64) -> Value {
    n <= UInt64(Int64.max) ? .integer(Int64(n)) : .bigInteger(String(n))
}
