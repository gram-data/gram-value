/// The ten tags whose content has a standard form (ADR-004).
public let knownTags = ["date", "time", "localtime", "datetime", "localdatetime", "duration", "point", "uuid", "week", "weekDate"]

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

/// What a known tag's content means.
public enum Typed: Sendable, Hashable {
    case date(GramDate)
    case time(GramTime)
    case dateTime(GramDateTime)
    case duration(GramDuration)
    case point(GramPoint)
    case uuid(GramUUID)
    case week(GramWeek)
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
