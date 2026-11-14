import Foundation
import GramValue
import Testing

private func code(_ body: () throws -> Void) -> String? {
    do { try body() } catch let d as Diagnostics { return d.items.first?.code } catch { return "other" }
    return nil
}

@Test func equalityDistinguishesKindsAndNumberSpelling() throws {
    #expect(Value.integer(1) != .decimal(1.0))
    #expect(Value.decimal(-0.0) != .decimal(0.0))
    #expect(Value.decimal(-0.0) < .decimal(0.0))
    let a = try Value.makeMeasurement(.integer(5), unit: "kg")
    let b = try Value.makeMeasurement(.decimal(5.0), unit: "kg")
    #expect(a != b)
}

@Test func stringsCompareByBytesNotCanonicalEquivalence() throws {
    // "é" precomposed and decomposed are equal Swift Strings but different gram strings (ADR-002).
    let precomposed = "\u{e9}", decomposed = "e\u{301}"
    #expect(precomposed == decomposed)
    #expect(Value.string(precomposed) != Value.string(decomposed))
    let m = try Value.makeMap([Value.Entry(precomposed, .integer(1)), Value.Entry(decomposed, .integer(2))])
    guard case .map(let entries) = m else { Issue.record("not a map"); return }
    #expect(entries.count == 2)
    #expect(code { _ = try m.toNative() } == "unsupported")
}

@Test func mapsAreEqualWhateverTheEntryOrderAndHashAgrees() throws {
    let a = try Value.makeMap([Value.Entry("a", .integer(1)), Value.Entry("b", .integer(2))])
    let b = try Value.makeMap([Value.Entry("b", .integer(2)), Value.Entry("a", .integer(1))])
    #expect(a == b)
    #expect(a.hashValue == b.hashValue)
    #expect(Set([Value.decimal(0.0), Value.decimal(-0.0)]).count == 2)
}

@Test func constructorsReportErrorsWithCodes() throws {
    #expect(code { _ = try Value.makeDecimal(.nan) } == "not-finite")
    #expect(code { _ = try Value.makeTagged("true", "x") } == "invalid-tag")
    #expect(code { _ = try Value.makeTagged("a b", "x") } == "invalid-tag")
    #expect(code { _ = try Value.makeRange(lower: nil, upper: nil) } == "no-ends")
    #expect(code { _ = try Value.makeRange(lower: .string("x"), upper: nil) } == "invalid-range-end")
    #expect(code { _ = try Value.makeMeasurement(.integer(1), unit: "k g") } == "invalid-unit")
    #expect(code { _ = try Value.makeBigInteger("5") } == "invalid-integer")
    #expect(code { _ = try Value.makeBigDecimal("0.1") } == "invalid-decimal")
    #expect(code { _ = try Value.makeInteger(fromDigits: "007") } == "invalid-integer")
    do {
        _ = try Value.makeMap([Value.Entry("a", .integer(1)), Value.Entry("a", .integer(2))])
        Issue.record("expected duplicate-key")
    } catch let d as Diagnostics {
        #expect(d.items.first?.code == "duplicate-key")
        #expect(d.items.first?.pointer == "/a")
    }
}

@Test func integersPickTheirKindByRange() throws {
    #expect(try Value.makeInteger(fromDigits: "9223372036854775807").kind == .integer)
    #expect(try Value.makeInteger(fromDigits: "9223372036854775808").kind == .bigInteger)
    #expect(try Value.makeInteger(fromDigits: "9223372036854775807") < Value.makeInteger(fromDigits: "9223372036854775808"))
    #expect(Value.array([.integer(1), .integer(2)]).isHomogeneousArray)
    #expect(!Value.array([.integer(1), .boolean(true)]).isHomogeneousArray)
}

@Test func unwritablePartsAreReportedWithPaths() throws {
    do {
        _ = try Value.array([.integer(1), .array([.integer(2)])]).toText()
        Issue.record("expected unwritable")
    } catch let d as Diagnostics {
        #expect(d.items.first?.code == "unwritable")
        #expect(d.items.first?.pointer == "/1")
    }
    #expect(code { _ = try Value.makeMeasurement(.integer(0), unit: "xFF").toText() } == "unwritable")
}

@Test func aRecordMayHoldAMapOneLevelDeep() throws {
    let inner = try Value.makeMap([Value.Entry("x", .integer(1))])
    let rec = try Value.makeMap([Value.Entry("k", inner)])
    #expect(code { _ = try rec.toText() } == "unwritable")
    #expect(try rec.toText(context: .record) == "{k: {x: 1}}")
    let deep = try Value.makeMap([Value.Entry("k", rec)])
    #expect(code { _ = try deep.toText(context: .record) } == "unwritable")
}

@Test func decimalsNeverUseAnExponent() throws {
    #expect(try Value.decimal(1e21).toText() == "1000000000000000000000.0")
    #expect(try Value.decimal(1e-7).toText() == "0.0000001")
    #expect(try Value.decimal(1.5e-10).toText() == "0.00000000015")
}

@Test func toNativeBigNumbersAreUnsupportedWithPaths() throws {
    let big = try Value.makeDecimal(fromText: "0.10000000000000000001")
    do {
        _ = try Value.array([.integer(1), big]).toNative()
        Issue.record("expected unsupported")
    } catch let d as Diagnostics {
        #expect(d.items.first?.code == "unsupported")
        #expect(d.items.first?.pointer == "/1")
    }
    #expect(try Value.integer(42).toNative() as? Int64 == 42)
}

@Test func fromNativeUnwritableValuesFailWithEveryProblemListed() throws {
    let input: [String: Any] = ["name": "x", "nick": Optional<Int>.none as Any, "score": Double.nan, "f": { 1 }]
    do {
        _ = try Value.fromNative(input)
        Issue.record("expected failure")
    } catch let d as Diagnostics {
        let got = Dictionary(uniqueKeysWithValues: d.items.map { ($0.pointer, $0.code) })
        #expect(got == ["/nick": "unsupported", "/score": "not-finite", "/f": "unsupported"])
    }
    #expect(code { _ = try Value.fromNative(Float(1)) } == "unsupported")
    #expect(code { _ = try Value.fromNative([1: 2]) } == "unsupported")
}

@Test func fromNativeNumbersAndTypedCollections() throws {
    #expect(try Value.fromNative(UInt64.max).kind == .bigInteger)
    #expect(try Value.fromNative(3).kind == .integer)
    #expect(try Value.fromNative(true).kind == .boolean)
    #expect(try Value.fromNative(["a", "b c"]).toText() == "[a, \"b c\"]")
    #expect(try Value.fromNative(GramMeasurement(magnitude: Int64(5), unit: "kg")).toText() == "5kg")
}
