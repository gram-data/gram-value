/// A gram value. The set of kinds is closed (ADR-002).
///
/// The cases are the eleven kinds. Build values that need validation (a tag, a unit, a map's keys, a range's
/// ends, the digits of a big number) with the `make...` functions, which throw `Diagnostics`; the payload cases
/// of `integer`, `boolean`, `string` and `array` cannot be invalid. Equality, hashing and order compare content
/// part by part, with strings compared by their UTF-8 bytes (ADR-002 2.5, 2.6).
public indirect enum Value: Sendable {
    case integer(Int64)
    /// An integer outside the 64-bit range: a sign and decimal digits without leading zeros.
    case bigInteger(String)
    case decimal(Double)
    /// A decimal a 64-bit float cannot hold exactly: sign, digits, a point and digits.
    case bigDecimal(String)
    case boolean(Bool)
    case string(String)
    case tagged(tag: String, content: String)
    case range(lower: Value?, upper: Value?)
    case measurement(magnitude: Value, unit: String)
    case array([Value])
    /// Entries in ascending order of the keys' UTF-8 bytes.
    case map([Entry])

    /// A map entry.
    public struct Entry: Sendable {
        public let key: String
        public let value: Value
        public init(_ key: String, _ value: Value) {
            self.key = key
            self.value = value
        }
    }
}

/// The kind of a value, in kind order.
public enum Kind: Int, Sendable, CaseIterable, Comparable, CustomStringConvertible {
    case integer, bigInteger, decimal, bigDecimal, boolean, string, tagged, range, measurement, array, map

    public static func < (a: Kind, b: Kind) -> Bool { a.rawValue < b.rawValue }

    /// The kind's name as the conformance vocabulary spells it.
    public var description: String {
        switch self {
        case .integer: return "integer"
        case .bigInteger: return "big_integer"
        case .decimal: return "decimal"
        case .bigDecimal: return "big_decimal"
        case .boolean: return "boolean"
        case .string: return "string"
        case .tagged: return "tagged"
        case .range: return "range"
        case .measurement: return "measurement"
        case .array: return "array"
        case .map: return "map"
        }
    }
}

extension Value {
    public var kind: Kind {
        switch self {
        case .integer: return .integer
        case .bigInteger: return .bigInteger
        case .decimal: return .decimal
        case .bigDecimal: return .bigDecimal
        case .boolean: return .boolean
        case .string: return .string
        case .tagged: return .tagged
        case .range: return .range
        case .measurement: return .measurement
        case .array: return .array
        case .map: return .map
        }
    }

    /// True for every kind except array and map.
    public var isScalar: Bool { kind != .array && kind != .map }

    /// For an array, whether all elements have one kind (true when empty); false for other kinds.
    public var isHomogeneousArray: Bool {
        guard case .array(let items) = self else { return false }
        return items.dropFirst().allSatisfy { $0.kind == items[0].kind }
    }

    var isNumeric: Bool {
        switch kind {
        case .integer, .bigInteger, .decimal, .bigDecimal: return true
        default: return false
        }
    }
}

// MARK: - Symbols and number text

private func isAsciiLetter(_ b: UInt8) -> Bool { (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A) }
private func isAsciiDigit(_ b: UInt8) -> Bool { b >= 0x30 && b <= 0x39 }

/// True when `s` matches the symbol rule `[a-zA-Z_][0-9a-zA-Z_.\-@]*`.
public func isSymbol(_ s: String) -> Bool {
    var first = true
    for b in s.utf8 {
        if first {
            guard isAsciiLetter(b) || b == 0x5F else { return false }
            first = false
        } else {
            guard isAsciiLetter(b) || isAsciiDigit(b) || b == 0x5F || b == 0x2E || b == 0x2D || b == 0x40 else {
                return false
            }
        }
    }
    return !first
}

/// True when `s` is a symbol other than `true` and `false`.
public func isBareWord(_ s: String) -> Bool { isSymbol(s) && s != "true" && s != "false" }

func digitsOnly(_ s: Substring) -> Bool { !s.isEmpty && s.utf8.allSatisfy(isAsciiDigit) }

/// `-?(0|[1-9][0-9]*)`, and not `-0`.
func isIntegerText(_ s: String) -> Bool {
    let d = s.hasPrefix("-") ? s.dropFirst() : Substring(s)
    return digitsOnly(d) && (d == "0" || !d.hasPrefix("0")) && s != "-0"
}

/// `-?(0|[1-9][0-9]*)\.[0-9]+`
func isDecimalText(_ s: String) -> Bool {
    let d = s.hasPrefix("-") ? s.dropFirst() : Substring(s)
    let parts = d.split(separator: ".", omittingEmptySubsequences: false)
    guard parts.count == 2 else { return false }
    return digitsOnly(parts[0]) && (parts[0] == "0" || !parts[0].hasPrefix("0")) && digitsOnly(parts[1])
}

/// Drop trailing zeros after the point, keeping at least one digit.
func normalizeDecimal(_ s: String) -> String {
    let parts = s.split(separator: ".", omittingEmptySubsequences: false)
    var frac = Substring(parts[1])
    while frac.hasSuffix("0") { frac = frac.dropLast() }
    return "\(parts[0]).\(frac.isEmpty ? "0" : String(frac))"
}

/// The shortest positional text of a finite float, with at least one digit after the point.
public func floatText(_ f: Double) -> String {
    // Swift's description is the shortest round-trip text, but uses an exponent for large and small values.
    var s = f.description
    if f == 0 { return f.sign == .minus ? "-0.0" : "0.0" }
    if let e = s.firstIndex(where: { $0 == "e" || $0 == "E" }) {
        let mantissa = String(s[s.startIndex..<e])
        let exp = Int(s[s.index(after: e)...]) ?? 0
        s = expand(mantissa: mantissa, exponent: exp)
    }
    return s.contains(".") ? s : s + ".0"
}

private func expand(mantissa: String, exponent: Int) -> String {
    let negative = mantissa.hasPrefix("-")
    let body = negative ? String(mantissa.dropFirst()) : mantissa
    let parts = body.split(separator: ".", omittingEmptySubsequences: false)
    let intPart = String(parts[0])
    let frac = parts.count > 1 ? String(parts[1]) : ""
    let digits = intPart + frac
    let point = intPart.count + exponent
    var out: String
    if point <= 0 {
        out = "0." + String(repeating: "0", count: -point) + digits
    } else if point >= digits.count {
        out = digits + String(repeating: "0", count: point - digits.count)
    } else {
        let i = digits.index(digits.startIndex, offsetBy: point)
        out = String(digits[..<i]) + "." + String(digits[i...])
    }
    return negative ? "-" + out : out
}

// MARK: - Constructors

private let int64MinText = "-9223372036854775808"

extension Value {
    /// An integer from `-?(0|[1-9][0-9]*)`: an `integer` if it fits in 64 bits, else a `bigInteger`.
    public static func makeInteger(fromDigits text: String) throws -> Value {
        guard isIntegerText(text) else {
            throw Diagnostics(code: "invalid-integer", message: "\"\(text)\" is not an integer without leading zeros")
        }
        if let n = Int64(text) { return .integer(n) }
        return .bigInteger(text)
    }

    /// A `bigInteger`; the digits must be outside the 64-bit range.
    public static func makeBigInteger(_ text: String) throws -> Value {
        guard isIntegerText(text) else {
            throw Diagnostics(code: "invalid-integer", message: "\"\(text)\" is not an integer without leading zeros")
        }
        guard Int64(text) == nil else {
            throw Diagnostics(code: "invalid-integer", message: "\(text) fits in 64 bits, so it is an integer")
        }
        return .bigInteger(text)
    }

    /// A `decimal`. `NaN` and the infinities are rejected (`not-finite`).
    public static func makeDecimal(_ f: Double) throws -> Value {
        guard f.isFinite else { throw Diagnostics(code: "not-finite", message: "a decimal must be finite") }
        return .decimal(f)
    }

    /// A decimal from `-?(0|[1-9][0-9]*)\.[0-9]+`: a `decimal` when the nearest float writes back as the same
    /// number (trailing zeros after the point ignored), else a `bigDecimal`.
    public static func makeDecimal(fromText text: String) throws -> Value {
        guard isDecimalText(text) else {
            throw Diagnostics(code: "invalid-decimal", message: "\"\(text)\" is not a decimal")
        }
        let normalized = normalizeDecimal(text)
        if let f = Double(normalized), f.isFinite, floatText(f) == normalized { return .decimal(f) }
        return .bigDecimal(normalized)
    }

    /// A `bigDecimal`. The text must be normalized and beyond what a float holds.
    public static func makeBigDecimal(_ text: String) throws -> Value {
        if isDecimalText(text), normalizeDecimal(text) == text, case .bigDecimal = try makeDecimal(fromText: text) {
            return .bigDecimal(text)
        }
        throw Diagnostics(code: "invalid-decimal", message: "\"\(text)\" is not a normalized decimal beyond a 64-bit float")
    }

    /// A `tagged` value. The tag must be a symbol other than `true` and `false` (`invalid-tag`).
    public static func makeTagged(_ tag: String, _ content: String) throws -> Value {
        guard isBareWord(tag) else {
            throw Diagnostics(code: "invalid-tag", message: "\"\(tag)\" is not a symbol other than true or false")
        }
        return .tagged(tag: tag, content: content)
    }

    /// A `range`. Each end must be numeric or a measurement (`invalid-range-end`); at least one end is needed
    /// (`no-ends`).
    public static func makeRange(lower: Value?, upper: Value?) throws -> Value {
        if lower == nil && upper == nil {
            throw Diagnostics(code: "no-ends", message: "a range needs at least one end")
        }
        for end in [lower, upper] {
            if let end, !end.isNumeric, end.kind != .measurement {
                throw Diagnostics(code: "invalid-range-end", message: "a range end must be numeric or a measurement, not \(end.kind)")
            }
        }
        return .range(lower: lower, upper: upper)
    }

    /// A `measurement`. The magnitude must be numeric and the unit ASCII letters (`invalid-unit`).
    public static func makeMeasurement(_ magnitude: Value, unit: String) throws -> Value {
        guard magnitude.isNumeric else {
            throw Diagnostics(code: "invalid-range-end", message: "a magnitude must be numeric, not \(magnitude.kind)")
        }
        guard !unit.isEmpty, unit.utf8.allSatisfy(isAsciiLetter) else {
            throw Diagnostics(code: "invalid-unit", message: "\"\(unit)\" is not one or more ASCII letters")
        }
        return .measurement(magnitude: magnitude, unit: unit)
    }

    /// A `map`. A repeated key is an error (`duplicate-key`) that names the key.
    public static func makeMap(_ entries: [Entry]) throws -> Value {
        let sorted = entries.sorted { compareBytes($0.key, $1.key) < 0 }
        for i in sorted.indices.dropFirst() where bytesEqual(sorted[i].key, sorted[i - 1].key) {
            throw Diagnostics(code: "duplicate-key", message: "the key \"\(sorted[i].key)\" is repeated", path: [.key(sorted[i].key)])
        }
        return .map(sorted)
    }
}

// MARK: - Equality, hashing and order

/// Strings compare by their UTF-8 bytes, not by Swift's canonical equivalence.
func bytesEqual(_ a: String, _ b: String) -> Bool { a.utf8.elementsEqual(b.utf8) }

func compareBytes(_ a: String, _ b: String) -> Int {
    var i = a.utf8.makeIterator()
    var j = b.utf8.makeIterator()
    while true {
        switch (i.next(), j.next()) {
        case (nil, nil): return 0
        case (nil, _): return -1
        case (_, nil): return 1
        case (let x?, let y?): if x != y { return x < y ? -1 : 1 }
        }
    }
}

private func cmp<T: Comparable>(_ a: T, _ b: T) -> Int { a < b ? -1 : (a > b ? 1 : 0) }

private func compareDigits(_ a: String, _ b: String) -> Int {
    let na = a.hasPrefix("-"), nb = b.hasPrefix("-")
    if na != nb { return na ? -1 : 1 }
    func split(_ s: String) -> (String, String) {
        let body = s.hasPrefix("-") ? String(s.dropFirst()) : s
        let parts = body.split(separator: ".", omittingEmptySubsequences: false)
        return (String(parts[0]), parts.count > 1 ? String(parts[1]) : "")
    }
    let (ai, af) = split(a), (bi, bf) = split(b)
    var m = cmp(ai.count, bi.count)
    if m == 0 { m = compareBytes(ai, bi) }
    if m == 0 { m = compareBytes(af, bf) }
    return na ? -m : m
}

private func compareOptional(_ a: Value?, _ b: Value?) -> Int {
    switch (a, b) {
    case (nil, nil): return 0
    case (nil, _): return -1
    case (_, nil): return 1
    case (let x?, let y?): return compare(x, y)
    }
}

/// The total order of spec/values.md section 6: -1, 0 or 1. Consistent with equality.
public func compare(_ a: Value, _ b: Value) -> Int {
    if a.kind != b.kind { return a.kind < b.kind ? -1 : 1 }
    switch (a, b) {
    case (.integer(let x), .integer(let y)): return cmp(x, y)
    case (.bigInteger(let x), .bigInteger(let y)): return compareBigIntegers(x, y)
    case (.decimal(let x), .decimal(let y)):
        if x == 0 && y == 0 { return cmp(y.sign == .minus ? 1 : 0, x.sign == .minus ? 1 : 0) }
        return cmp(x, y)
    case (.bigDecimal(let x), .bigDecimal(let y)): return compareDigits(x, y)
    case (.boolean(let x), .boolean(let y)): return cmp(x ? 1 : 0, y ? 1 : 0)
    case (.string(let x), .string(let y)): return compareBytes(x, y)
    case (.tagged(let t1, let c1), .tagged(let t2, let c2)):
        let c = compareBytes(t1, t2)
        return c != 0 ? c : compareBytes(c1, c2)
    case (.range(let l1, let u1), .range(let l2, let u2)):
        let c = compareOptional(l1, l2)
        return c != 0 ? c : compareOptional(u1, u2)
    case (.measurement(let m1, let u1), .measurement(let m2, let u2)):
        let c = compare(m1, m2)
        return c != 0 ? c : compareBytes(u1, u2)
    case (.array(let x), .array(let y)):
        for (p, q) in zip(x, y) {
            let c = compare(p, q)
            if c != 0 { return c }
        }
        return cmp(x.count, y.count)
    case (.map(let x), .map(let y)):
        for (p, q) in zip(x, y) {
            var c = compareBytes(p.key, q.key)
            if c == 0 { c = compare(p.value, q.value) }
            if c != 0 { return c }
        }
        return cmp(x.count, y.count)
    default:
        preconditionFailure("values of one kind have one case")
    }
}

private func compareBigIntegers(_ a: String, _ b: String) -> Int {
    let na = a.hasPrefix("-"), nb = b.hasPrefix("-")
    if na != nb { return na ? -1 : 1 }
    let da = na ? String(a.dropFirst()) : a
    let db = nb ? String(b.dropFirst()) : b
    var m = cmp(da.count, db.count)
    if m == 0 { m = compareBytes(da, db) }
    return na ? -m : m
}

extension Value: Equatable, Hashable, Comparable {
    public static func == (a: Value, b: Value) -> Bool { compare(a, b) == 0 }
    public static func < (a: Value, b: Value) -> Bool { compare(a, b) < 0 }

    public func hash(into h: inout Hasher) {
        h.combine(kind)
        switch self {
        case .integer(let n): h.combine(n)
        case .bigInteger(let s), .bigDecimal(let s): h.combine(Array(s.utf8))
        case .decimal(let f): h.combine(f.bitPattern)
        case .boolean(let b): h.combine(b)
        case .string(let s): h.combine(Array(s.utf8))
        case .tagged(let t, let c):
            h.combine(Array(t.utf8))
            h.combine(Array(c.utf8))
        case .range(let l, let u):
            h.combine(l)
            h.combine(u)
        case .measurement(let m, let u):
            h.combine(m)
            h.combine(Array(u.utf8))
        case .array(let items): for i in items { h.combine(i) }
        case .map(let entries):
            for e in entries {
                h.combine(Array(e.key.utf8))
                h.combine(e.value)
            }
        }
    }
}
