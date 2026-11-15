import Foundation

/// The eleven tags whose content has a standard form (ADR-004, ADR-008).
public let knownTags = ["date", "time", "localtime", "datetime", "localdatetime", "duration", "point", "uuid", "week", "weekDate", "embedding"]

public struct GramDate: Sendable, Hashable {
    public let year: Int64
    public let month: Int
    public let day: Int
}

public struct GramTime: Sendable, Hashable {
    public let hour: Int
    public let minute: Int
    public let second: Int
    public let nanosecond: Int
    /// The offset east of UTC in minutes, when written.
    public let offsetMinutes: Int?
}

public struct GramDateTime: Sendable, Hashable {
    public let date: GramDate
    public let time: GramTime
    /// The zone name in square brackets, when written.
    public let zone: String?
}

public struct GramSeconds: Sendable, Hashable {
    public let negative: Bool
    public let whole: UInt64
    public let nanos: Int
}

/// An ISO 8601 duration with per-number signs; a nil component was not written.
public struct GramDuration: Sendable, Hashable {
    public var years, months, days, hours, minutes: Int64?
    public var seconds: GramSeconds?
}

public struct GramPoint: Sendable, Hashable {
    public let srid: UInt64?
    public let x: Double
    public let y: Double
    public let z: Double?
}

public struct GramUUID: Sendable, Hashable {
    public let bytes: [UInt8]
}

/// An ISO week of a year; `day` is set for a `weekDate`.
public struct GramWeek: Sendable, Hashable {
    public let year: Int
    public let week: Int
    public let day: Int?
}

public enum GramEmbeddingDtype: String, Sendable, Hashable {
    case f16 = "F16"
    case bf16 = "BF16"
    case f32 = "F32"
    case f64 = "F64"

    public var width: Int {
        switch self {
        case .f16, .bf16: return 2
        case .f32: return 4
        case .f64: return 8
        }
    }
}

public let maxEmbeddingDimension = 1_048_576

/// A rank-one dense floating-point tensor carried by an `embedding` tag (ADR-008).
public struct GramEmbedding: Sendable, Hashable {
    public let dtype: GramEmbeddingDtype
    public let dimension: Int
    public let bytes: Data

    public init(dtype: GramEmbeddingDtype, dimension: Int, bytes: Data) throws {
        guard dimension >= 1, dimension <= maxEmbeddingDimension else {
            throw Diagnostics(code: "out-of-range", message: "embedding: dimension \(dimension) must be between 1 and \(maxEmbeddingDimension)")
        }
        let expected = dimension * dtype.width
        guard bytes.count == expected else {
            throw Diagnostics(code: "invalid-form", message: "embedding: expected \(expected) payload bytes, got \(bytes.count)")
        }
        self.dtype = dtype
        self.dimension = dimension
        self.bytes = bytes
    }

    init(validated dtype: GramEmbeddingDtype, dimension: Int, bytes: Data) {
        self.dtype = dtype
        self.dimension = dimension
        self.bytes = bytes
    }

    public func toNormalContent() -> String {
        encodeEmbeddingNormal(dtype, dimension, bytes)
    }

    public func toValue() -> Value {
        .tagged(tag: "embedding", content: toNormalContent())
    }
}

/// What a known tag's content means.
public enum Typed: Sendable, Hashable {
    case date(GramDate)
    case time(GramTime)
    case dateTime(GramDateTime)
    case duration(GramDuration)
    case point(GramPoint)
    case uuid(GramUUID)
    case week(GramWeek)
    case embedding(GramEmbedding)
}

/// A successful interpretation of a known tag.
public struct Interpretation: Sendable, Hashable {
    public let tag: String
    public let typed: Typed
    /// The normal content string (spec/tags.md section 4).
    public let normalContent: String

    /// The tagged value with the normal content string.
    public func toValue() -> Value { .tagged(tag: tag, content: normalContent) }
}

private struct Fault: Error {
    let code: String
    let message: String
}

private func form(_ m: String) -> Fault { Fault(code: "invalid-form", message: m) }
private func range(_ m: String) -> Fault { Fault(code: "out-of-range", message: m) }
private func unsupported(_ m: String) -> Fault { Fault(code: "unsupported", message: m) }

/// Interpret a tagged value. Returns nil for a value that is not tagged or whose tag is unknown; throws
/// `Diagnostics` (code `invalid-form` or `out-of-range`) for a known tag with bad content.
public func interpret(_ value: Value) throws -> Interpretation? {
    guard case .tagged(let tag, let content) = value else { return nil }
    return try interpretTag(tag, content)
}

/// Interpret a tag and content. Returns nil for an unknown tag.
public func interpretTag(_ tag: String, _ content: String) throws -> Interpretation? {
    guard knownTags.contains(tag) else { return nil }
    do {
        return try run(tag, Array(content.utf8), content)
    } catch let f as Fault {
        throw Diagnostics(code: f.code, message: "\(tag): \(f.message)")
    }
}

private func run(_ tag: String, _ b: [UInt8], _ content: String) throws -> Interpretation {
    switch tag {
    case "date":
        let (d, used) = try parseDatePrefix(b)
        guard used == b.count else { throw form("unexpected text after the date") }
        return Interpretation(tag: tag, typed: .date(d), normalContent: content)
    case "time", "localtime":
        let t = try parseTime(b[...], offsetAllowed: tag == "time")
        return Interpretation(tag: tag, typed: .time(t), normalContent: timeNormal(t))
    case "datetime", "localdatetime":
        let (dt, normal) = try parseDateTime(b, content, offsetAllowed: tag == "datetime")
        return Interpretation(tag: tag, typed: .dateTime(dt), normalContent: normal)
    case "duration":
        return Interpretation(tag: tag, typed: .duration(try parseDuration(b)), normalContent: content)
    case "point":
        let (p, normal) = try parsePoint(b)
        return Interpretation(tag: tag, typed: .point(p), normalContent: normal)
    case "uuid":
        return Interpretation(tag: tag, typed: .uuid(try parseUUID(b)), normalContent: content.lowercased())
    case "week":
        return Interpretation(tag: tag, typed: .week(try parseWeek(b, withDay: false)), normalContent: content)
    case "embedding":
        let (emb, normal) = try parseEmbedding(b)
        return Interpretation(tag: tag, typed: .embedding(emb), normalContent: normal)
    default:
        return Interpretation(tag: tag, typed: .week(try parseWeek(b, withDay: true)), normalContent: content)
    }
}

// MARK: - helpers on ASCII bytes

private func isDigit(_ c: UInt8) -> Bool { c >= 0x30 && c <= 0x39 }
private func isHex(_ c: UInt8) -> Bool { isDigit(c) || (c >= 0x41 && c <= 0x46) || (c >= 0x61 && c <= 0x66) }
private func num(_ b: ArraySlice<UInt8>) -> Int { b.reduce(0) { $0 * 10 + Int($1 - 0x30) } }
private func text(_ b: ArraySlice<UInt8>) -> String { String(decoding: b, as: UTF8.self) }
private func allDigits(_ b: ArraySlice<UInt8>) -> Bool { !b.isEmpty && b.allSatisfy(isDigit) }

private func twoDigits(_ b: ArraySlice<UInt8>, _ what: String) throws -> Int {
    guard b.count == 2, allDigits(b) else { throw form("\(what) must be two digits") }
    return num(b)
}

private func isLeap(_ y: Int64) -> Bool { (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 }

private func daysInMonth(_ year: Int64, _ month: Int) -> Int {
    switch month {
    case 1, 3, 5, 7, 8, 10, 12: return 31
    case 4, 6, 9, 11: return 30
    default: return isLeap(year) ? 29 : 28
    }
}

private func parseDatePrefix(_ b: [UInt8]) throws -> (GramDate, Int) {
    var i = 0
    if i < b.count, b[i] == 0x2D { i += 1 }
    let yearStart = i
    while i < b.count, isDigit(b[i]) { i += 1 }
    guard i - yearStart >= 4 else { throw form("the year needs at least four digits") }
    guard i + 6 <= b.count, b[i] == 0x2D, b[i + 3] == 0x2D,
        allDigits(b[(i + 1)..<(i + 3)]), allDigits(b[(i + 4)..<(i + 6)])
    else { throw form("expected YYYY-MM-DD") }
    guard let year = Int64(text(b[0..<i])) else { throw range("the year is too large") }
    let month = num(b[(i + 1)..<(i + 3)])
    let day = num(b[(i + 4)..<(i + 6)])
    guard (1...12).contains(month) else { throw range("month \(month) does not exist") }
    guard day >= 1, day <= daysInMonth(year, month) else {
        throw range("day \(day) does not exist in month \(month) of \(year)")
    }
    return (GramDate(year: year, month: month, day: day), i + 6)
}

private func parseOffset(_ b: ArraySlice<UInt8>) throws -> Int {
    if text(b) == "Z" { return 0 }
    let a = Array(b)
    guard a.count == 6, a[0] == 0x2B || a[0] == 0x2D, a[3] == 0x3A,
        allDigits(b[(b.startIndex + 1)..<(b.startIndex + 3)]), allDigits(b[(b.startIndex + 4)..<(b.startIndex + 6)])
    else { throw form("an offset is Z, +hh:mm or -hh:mm") }
    let h = Int(a[1] - 0x30) * 10 + Int(a[2] - 0x30)
    let m = Int(a[4] - 0x30) * 10 + Int(a[5] - 0x30)
    guard h <= 23, m <= 59 else { throw range("the offset is out of range") }
    return a[0] == 0x2D ? -(h * 60 + m) : h * 60 + m
}

private func parseTime(_ b: ArraySlice<UInt8>, offsetAllowed: Bool) throws -> GramTime {
    var clock = b
    var offset: Int?
    if let i = b.firstIndex(where: { $0 == 0x5A || $0 == 0x2B || $0 == 0x2D }) {
        guard offsetAllowed else { throw form("this tag does not allow an offset") }
        offset = try parseOffset(b[i...])
        clock = b[..<i]
    }
    let c = Array(clock)
    guard c.count >= 5, c[2] == 0x3A else { throw form("expected hh:mm") }
    let hour = try twoDigits(clock[clock.startIndex..<(clock.startIndex + 2)], "the hour")
    let minute = try twoDigits(clock[(clock.startIndex + 3)..<(clock.startIndex + 5)], "the minute")
    var second = 0, nano = 0
    if c.count > 5 {
        guard c[5] == 0x3A, c.count >= 8 else { throw form("expected hh:mm or hh:mm:ss") }
        second = try twoDigits(clock[(clock.startIndex + 6)..<(clock.startIndex + 8)], "the second")
        if c.count > 8 {
            guard c[8] == 0x2E else { throw form("expected a fraction after the seconds") }
            let frac = clock[(clock.startIndex + 9)...]
            guard (1...9).contains(frac.count), allDigits(frac) else { throw form("a fraction has 1 to 9 digits") }
            nano = num(frac) * Int(pow10(9 - frac.count))
        }
    }
    guard hour <= 23, minute <= 59, second <= 59 else { throw range("the time is out of range") }
    return GramTime(hour: hour, minute: minute, second: second, nanosecond: nano, offsetMinutes: offset)
}

private func pow10(_ n: Int) -> Int { (0..<n).reduce(1) { acc, _ in acc * 10 } }

private func pad(_ n: Int, _ width: Int) -> String {
    let s = String(n)
    return String(repeating: "0", count: max(0, width - s.count)) + s
}

private func offsetText(_ minutes: Int) -> String {
    if minutes == 0 { return "Z" }
    let a = abs(minutes)
    return "\(minutes < 0 ? "-" : "+")\(pad(a / 60, 2)):\(pad(a % 60, 2))"
}

private func timeNormal(_ t: GramTime) -> String {
    var s = "\(pad(t.hour, 2)):\(pad(t.minute, 2)):\(pad(t.second, 2))"
    if t.nanosecond != 0 {
        var frac = pad(t.nanosecond, 9)
        while frac.hasSuffix("0") { frac.removeLast() }
        s += "." + frac
    }
    if let o = t.offsetMinutes { s += offsetText(o) }
    return s
}

private func isZoneByte(_ c: UInt8) -> Bool {
    isDigit(c) || (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) || c == 0x2F || c == 0x5F || c == 0x2B || c == 0x2D
}

private func parseDateTime(_ b: [UInt8], _ content: String, offsetAllowed: Bool) throws -> (GramDateTime, String) {
    let (date, used) = try parseDatePrefix(b)
    guard used < b.count, b[used] == 0x54 else { throw form("expected T after the date") }
    let rest = b[(used + 1)...]
    var clock = rest
    var zone: String?
    if let open = rest.firstIndex(of: 0x5B) {
        guard rest.last == 0x5D else { throw form("a zone name is written [name]") }
        clock = rest[..<open]
        zone = text(rest[(open + 1)..<(rest.endIndex - 1)])
    }
    let time = try parseTime(clock, offsetAllowed: offsetAllowed)
    var normal = text(b[0..<used]) + "T" + timeNormal(time)
    if let zone {
        guard time.offsetMinutes != nil else { throw form("a zone name needs an offset") }
        guard !zone.isEmpty, zone.utf8.allSatisfy(isZoneByte) else {
            throw form("a zone name is letters, digits and / _ + -")
        }
        normal += "[" + zone + "]"
    }
    return (GramDateTime(date: date, time: time, zone: zone), normal)
}

private func parseSigned(_ b: ArraySlice<UInt8>) throws -> Int64 {
    let digits = b.first == 0x2D ? b.dropFirst() : b
    guard allDigits(digits) else { throw form("a duration component is a number") }
    guard let n = Int64(text(b)) else { throw range("a duration component is too large") }
    return n
}

private func parseDuration(_ b: [UInt8]) throws -> GramDuration {
    guard b.first == 0x50 else { throw form("a duration starts with P") }
    var rest = b[1...]
    var d = GramDuration()
    var inTime = false, any = false
    var orderDate = 0, orderTime = 0, timeComponents = 0
    while !rest.isEmpty {
        if rest.first == 0x54 {
            guard !inTime else { throw form("T appears once") }
            inTime = true
            rest = rest.dropFirst()
            guard !rest.isEmpty else { throw form("T needs at least one component") }
            continue
        }
        guard let end = rest.firstIndex(where: { $0 >= 0x41 && $0 <= 0x5A }) else {
            throw form("a component needs a designator")
        }
        let number = rest[rest.startIndex..<end]
        let designator = rest[end]
        rest = rest[(end + 1)...]
        any = true
        if inTime { timeComponents += 1 }
        switch (inTime, designator) {
        case (false, 0x59) where orderDate < 1: d.years = try parseSigned(number); orderDate = 1
        case (false, 0x4D) where orderDate < 2: d.months = try parseSigned(number); orderDate = 2
        case (false, 0x44) where orderDate < 3: d.days = try parseSigned(number); orderDate = 3
        case (true, 0x48) where orderTime < 1: d.hours = try parseSigned(number); orderTime = 1
        case (true, 0x4D) where orderTime < 2: d.minutes = try parseSigned(number); orderTime = 2
        case (true, 0x53) where orderTime < 3:
            let negative = number.first == 0x2D
            let body = negative ? number.dropFirst() : number
            let parts = body.split(separator: 0x2E, omittingEmptySubsequences: false)
            guard parts.count <= 2, allDigits(parts[0]), parts.count == 1 || (1...9).contains(parts[1].count) && allDigits(parts[1])
            else { throw form("seconds are digits with an optional fraction of 1 to 9 digits") }
            guard let whole = UInt64(text(parts[0])) else { throw range("the seconds are too large") }
            let nanos = parts.count == 2 ? num(parts[1]) * pow10(9 - parts[1].count) : 0
            d.seconds = GramSeconds(negative: negative, whole: whole, nanos: nanos)
            orderTime = 3
        default: throw form("components are in the order Y M D, then T, H M S")
        }
    }
    guard any else { throw form("a duration has at least one component") }
    guard !inTime || timeComponents > 0 else { throw form("T needs at least one component") }
    return d
}

private func isNumber(_ b: ArraySlice<UInt8>) -> Bool {
    let body = b.first == 0x2D ? b.dropFirst() : b
    let parts = body.split(separator: 0x2E, omittingEmptySubsequences: false)
    return (parts.count == 1 || parts.count == 2) && parts.allSatisfy { allDigits($0) }
}

private func numberText(_ f: Double) -> String {
    if f == 0 { return "0" }
    let t = floatText(f)
    return t.hasSuffix(".0") ? String(t.dropLast(2)) : t
}

private func parsePoint(_ b: [UInt8]) throws -> (GramPoint, String) {
    var srid: UInt64?
    var coords = b[...]
    if text(b[...]).hasPrefix("SRID=") {
        guard let semi = b.firstIndex(of: 0x3B) else { throw form("SRID=n; needs the semicolon") }
        let n = b[5..<semi]
        guard allDigits(n), let v = UInt64(text(n)), v > 0 else { throw form("the SRID is a positive integer") }
        srid = v
        coords = b[(semi + 1)...]
    }
    let parts = coords.split(separator: 0x20, omittingEmptySubsequences: false)
    guard parts.count == 2 || parts.count == 3, parts.allSatisfy(isNumber) else {
        throw form("a point is two or three numbers separated by single spaces")
    }
    let nums = try parts.map { p -> Double in
        guard let f = Double(text(p)), f.isFinite else { throw range("a coordinate is out of range") }
        return f
    }
    var normal = srid.map { "SRID=\($0);" } ?? ""
    normal += nums.map(numberText).joined(separator: " ")
    return (GramPoint(srid: srid, x: nums[0], y: nums[1], z: nums.count == 3 ? nums[2] : nil), normal)
}

private func parseUUID(_ b: [UInt8]) throws -> GramUUID {
    guard b.count == 36, [8, 13, 18, 23].allSatisfy({ b[$0] == 0x2D }) else {
        throw form("a UUID is 8-4-4-4-12 hexadecimal digits")
    }
    let hex = b.filter { $0 != 0x2D }
    guard hex.count == 32, hex.allSatisfy(isHex) else { throw form("a UUID is 8-4-4-4-12 hexadecimal digits") }
    func nibble(_ c: UInt8) -> UInt8 { isDigit(c) ? c - 0x30 : (c | 0x20) - 0x61 + 10 }
    return GramUUID(bytes: stride(from: 0, to: 32, by: 2).map { nibble(hex[$0]) << 4 | nibble(hex[$0 + 1]) })
}

/// Day of the week of 1 January, 0 = Monday (proleptic Gregorian).
private func jan1Weekday(_ year: Int64) -> Int64 {
    let y = year - 1
    let d = y * 365 + y / 4 - y / 100 + y / 400
    return ((d % 7) + 7) % 7
}

private func has53Weeks(_ year: Int64) -> Bool {
    let w = jan1Weekday(year)
    return w == 3 || (isLeap(year) && w == 2)
}

private func parseWeek(_ b: [UInt8], withDay: Bool) throws -> GramWeek {
    let expected = withDay ? 9 : 7
    guard b.count == expected, allDigits(b[0..<4]), b[4] == 0x57, allDigits(b[5..<7]),
        !withDay || (b[7] == 0x2D && isDigit(b[8]))
    else { throw form(withDay ? "expected YYYYWww-D" : "expected YYYYWww") }
    let year = num(b[0..<4])
    let week = num(b[5..<7])
    let maxWeek = has53Weeks(Int64(year)) ? 53 : 52
    guard week >= 1, week <= maxWeek else { throw range("\(year) has \(maxWeek) ISO weeks, not week \(week)") }
    var day: Int?
    if withDay {
        day = Int(b[8] - 0x30)
        guard (1...7).contains(day!) else { throw range("the weekday is 1 to 7") }
    }
    return GramWeek(year: year, week: week, day: day)
}

// MARK: - embedding (ADR-008)

private let maxBase64Chars = 11_190_284
private let maxDecodedBytes = 8_392_712
private let maxHeaderBytes = 4_096

private func b64Val(_ c: UInt8) -> Int {
    switch c {
    case 0x41...0x5A: return Int(c - 0x41)
    case 0x61...0x7A: return Int(c - 0x61) + 26
    case 0x30...0x39: return Int(c - 0x30) + 52
    case 0x2B: return 62
    case 0x2F: return 63
    default: return -1
    }
}

private func decodeStrictBase64(_ raw: [UInt8]) throws -> [UInt8] {
    guard !raw.isEmpty else { throw form("empty Base64 content") }
    guard raw.count % 4 == 0 else { throw form("Base64 length must be a multiple of 4") }
    let padCount: Int
    if raw.count >= 2 && raw[raw.count - 2] == 0x3D && raw[raw.count - 1] == 0x3D {
        padCount = 2
    } else if raw[raw.count - 1] == 0x3D {
        padCount = 1
    } else {
        padCount = 0
    }
    let dataEnd = raw.count - padCount
    for i in 0..<dataEnd {
        if b64Val(raw[i]) < 0 {
            throw form("invalid Base64 byte at index \(i)")
        }
    }
    if padCount == 1 {
        if (b64Val(raw[raw.count - 2]) & 0x03) != 0 {
            throw form("non-canonical Base64 pad bits")
        }
    } else if padCount == 2 {
        if (b64Val(raw[raw.count - 3]) & 0x0F) != 0 {
            throw form("non-canonical Base64 pad bits")
        }
    }

    let decodedLen = (raw.count / 4) * 3 - padCount
    if raw.count > maxBase64Chars || decodedLen > maxDecodedBytes {
        throw range("embedding exceeds maximum decoded size of \(maxDecodedBytes) bytes")
    }

    var out: [UInt8] = []
    out.reserveCapacity(decodedLen)
    var i = 0
    while i < raw.count {
        let a = b64Val(raw[i])
        let b = b64Val(raw[i + 1])
        out.append(UInt8((a << 2) | (b >> 4)))
        if raw[i + 2] != 0x3D {
            let c = b64Val(raw[i + 2])
            out.append(UInt8(((b << 4) | (c >> 2)) & 0xFF))
            if raw[i + 3] != 0x3D {
                let d = b64Val(raw[i + 3])
                out.append(UInt8(((c << 6) | d) & 0xFF))
            }
        }
        i += 4
    }
    return out
}

private func encodeEmbeddingNormal(_ dtype: GramEmbeddingDtype, _ dimension: Int, _ payload: Data) -> String {
    let json = "{\"embedding\":{\"dtype\":\"\(dtype.rawValue)\",\"shape\":[\(dimension)],\"data_offsets\":[0,\(payload.count)]}}"
    let jsonBytes = Array(json.utf8)
    let pad = (8 - (jsonBytes.count % 8)) % 8
    let headerLen = jsonBytes.count + pad
    var buf = Data(capacity: 8 + headerLen + payload.count)
    var leLen = UInt64(headerLen).littleEndian
    withUnsafeBytes(of: &leLen) { buf.append(contentsOf: $0) }
    buf.append(contentsOf: jsonBytes)
    if pad > 0 {
        buf.append(contentsOf: repeatElement(UInt8(0x20), count: pad))
    }
    buf.append(payload)
    return buf.base64EncodedString()
}

private struct HeaderParser {
    let bytes: [UInt8]
    var pos: Int = 0

    mutating func skipWS() {
        while pos < bytes.count {
            let c = bytes[pos]
            if c == 0x20 || c == 0x09 || c == 0x0A || c == 0x0D {
                pos += 1
            } else {
                break
            }
        }
    }

    mutating func expect(_ want: UInt8) throws {
        skipWS()
        guard pos < bytes.count, bytes[pos] == want else {
            throw form("expected '\(Character(UnicodeScalar(want)))' in JSON header")
        }
        pos += 1
    }

    mutating func parseHex4() throws -> UInt16 {
        guard pos + 4 <= bytes.count else { throw form("short \\uXXXX escape in JSON string") }
        let slice = String(decoding: bytes[pos..<(pos + 4)], as: UTF8.self)
        guard let v = UInt16(slice, radix: 16) else { throw form("invalid \\uXXXX escape in JSON string") }
        pos += 4
        return v
    }

    mutating func parseString() throws -> String {
        skipWS()
        guard pos < bytes.count, bytes[pos] == 0x22 else { throw form("expected JSON string") }
        pos += 1
        var scalars: [UnicodeScalar] = []
        while pos < bytes.count {
            let b = bytes[pos]
            if b == 0x22 {
                pos += 1
                var s = String()
                s.unicodeScalars.append(contentsOf: scalars)
                return s
            }
            if b < 0x20 {
                throw form("unescaped control character in JSON string")
            }
            if b == 0x5C {
                pos += 1
                guard pos < bytes.count else { throw form("unterminated JSON escape") }
                let esc = bytes[pos]
                pos += 1
                switch esc {
                case 0x22: scalars.append("\"")
                case 0x5C: scalars.append("\\")
                case 0x2F: scalars.append("/")
                case 0x62: scalars.append("\u{0008}")
                case 0x66: scalars.append("\u{000C}")
                case 0x6E: scalars.append("\n")
                case 0x72: scalars.append("\r")
                case 0x74: scalars.append("\t")
                case 0x75:
                    let code = try parseHex4()
                    if (0xD800...0xDBFF).contains(code) {
                        guard pos + 2 <= bytes.count, bytes[pos] == 0x5C, bytes[pos + 1] == 0x75 else {
                            throw form("unpaired UTF-16 high surrogate in JSON string")
                        }
                        pos += 2
                        let low = try parseHex4()
                        guard (0xDC00...0xDFFF).contains(low) else {
                            throw form("invalid UTF-16 low surrogate in JSON string")
                        }
                        let cp = 0x10000 + ((UInt32(code - 0xD800) << 10) | UInt32(low - 0xDC00))
                        guard let scalar = UnicodeScalar(cp) else { throw form("invalid Unicode scalar") }
                        scalars.append(scalar)
                    } else if (0xDC00...0xDFFF).contains(code) {
                        throw form("unpaired UTF-16 low surrogate in JSON string")
                    } else {
                        guard let scalar = UnicodeScalar(UInt32(code)) else { throw form("invalid Unicode scalar") }
                        scalars.append(scalar)
                    }
                default:
                    throw form("invalid JSON escape sequence")
                }
            } else {
                var iter = UTF8()
                var slice = bytes[pos...].makeIterator()
                switch iter.decode(&slice) {
                case .scalarValue(let scalar):
                    scalars.append(scalar)
                    pos += UTF8.width(scalar)
                case .emptyInput, .error:
                    throw form("invalid UTF-8 in JSON string")
                }
            }
        }
        throw form("unterminated JSON string")
    }

    mutating func parsePlainUintToken() throws -> String {
        skipWS()
        let start = pos
        while pos < bytes.count, isDigit(bytes[pos]) {
            pos += 1
        }
        guard pos > start else { throw form("expected plain base-10 integer") }
        if pos - start > 1 && bytes[start] == 0x30 {
            throw form("leading zero in JSON integer")
        }
        if pos < bytes.count && (bytes[pos] == 0x2E || bytes[pos] == 0x65 || bytes[pos] == 0x45) {
            throw form("non-integer JSON number")
        }
        return String(decoding: bytes[start..<pos], as: UTF8.self)
    }

    mutating func parseShape() throws -> (UInt64, Bool) {
        try expect(0x5B)
        let tok = try parsePlainUintToken()
        try expect(0x5D)
        if let n = UInt64(tok) {
            return (n, false)
        }
        return (0, true)
    }

    mutating func parseDataOffsets() throws -> (UInt64, UInt64) {
        try expect(0x5B)
        let t0 = try parsePlainUintToken()
        try expect(0x2C)
        let t1 = try parsePlainUintToken()
        try expect(0x5D)
        guard let a = UInt64(t0), let b = UInt64(t1) else {
            throw form("data_offsets exceeds 64 unsigned bits")
        }
        return (a, b)
    }

    mutating func parseHeader() throws -> (String, UInt64, Bool, UInt64, UInt64) {
        try expect(0x7B)
        let topKey = try parseString()
        guard topKey == "embedding" else { throw form("unexpected top-level key \"\(topKey)\"") }
        try expect(0x3A)
        try expect(0x7B)

        var dtype: String?
        var shapeVal: (UInt64, Bool)?
        var offsets: (UInt64, UInt64)?

        for idx in 0..<3 {
            if idx > 0 { try expect(0x2C) }
            let key = try parseString()
            try expect(0x3A)
            switch key {
            case "dtype":
                if dtype != nil { throw form("duplicate key \"dtype\"") }
                dtype = try parseString()
            case "shape":
                if shapeVal != nil { throw form("duplicate key \"shape\"") }
                shapeVal = try parseShape()
            case "data_offsets":
                if offsets != nil { throw form("duplicate key \"data_offsets\"") }
                offsets = try parseDataOffsets()
            default:
                throw form("unexpected tensor key \"\(key)\"")
            }
        }
        try expect(0x7D)
        try expect(0x7D)
        skipWS()
        guard pos == bytes.count else { throw form("trailing bytes in SafeTensors JSON header") }
        return (dtype!, shapeVal!.0, shapeVal!.1, offsets!.0, offsets!.1)
    }
}

private func parseEmbedding(_ b: [UInt8]) throws -> (GramEmbedding, String) {
    let decoded = try decodeStrictBase64(b)
    guard decoded.count >= 8 else { throw form("SafeTensors buffer shorter than 8 bytes") }
    var headerLenU64: UInt64 = 0
    for i in 0..<8 {
        headerLenU64 |= UInt64(decoded[i]) << (UInt64(i) * 8)
    }
    let remaining = UInt64(decoded.count - 8)
    guard headerLenU64 <= UInt64(maxHeaderBytes), headerLenU64 <= remaining else {
        throw form("SafeTensors header length exceeds 4,096 or buffer")
    }
    let headerLen = Int(headerLenU64)
    let headerBytes = Array(decoded[8..<(8 + headerLen)])
    guard String(bytes: headerBytes, encoding: .utf8) != nil else {
        throw form("SafeTensors header is not valid UTF-8")
    }
    var parser = HeaderParser(bytes: headerBytes)
    let (dtypeRaw, shapeDim, shapeOverflow, off0, off1) = try parser.parseHeader()

    guard let dtype = GramEmbeddingDtype(rawValue: dtypeRaw) else {
        throw unsupported("unsupported embedding dtype \"\(dtypeRaw)\"")
    }
    if shapeOverflow {
        throw range("embedding dimension exceeds 64 unsigned bits")
    }
    guard shapeDim >= 1, shapeDim <= UInt64(maxEmbeddingDimension) else {
        throw range("embedding dimension \(shapeDim) must be between 1 and \(maxEmbeddingDimension)")
    }
    let dimension = Int(shapeDim)
    let expectedBytes = dimension * dtype.width
    guard off0 == 0, off1 == UInt64(expectedBytes) else {
        throw form("data_offsets [\(off0), \(off1)] does not match [0, \(expectedBytes)]")
    }
    let payloadStart = 8 + headerLen
    guard decoded.count == payloadStart + expectedBytes else {
        throw form("payload length \(decoded.count - payloadStart) does not match expected \(expectedBytes)")
    }
    let payload = Data(decoded[payloadStart...])
    let emb = GramEmbedding(validated: dtype, dimension: dimension, bytes: payload)
    let normal = encodeEmbeddingNormal(dtype, dimension, payload)
    return (emb, normal)
}
