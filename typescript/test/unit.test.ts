import assert from "node:assert/strict";
import { test } from "node:test";
import {
  array,
  boolean,
  compare,
  decimal,
  equals,
  fail,
  failOnLoss,
  fromNative,
  integer,
  isHomogeneousArray,
  map,
  measurement,
  pathToPointer,
  range,
  string,
  tagged,
  toNative,
  toText,
  unwrap,
  bigInteger,
  bigDecimal,
  integerFromDigits,
  Measurement,
} from "../src/index.ts";

const code = (r: { ok: boolean; diagnostics: readonly { code: string }[] }) => r.diagnostics[0]?.code;

test("equality distinguishes kinds and the spelling of numbers", () => {
  assert.ok(!equals(integer(1), unwrap(decimal(1))));
  assert.ok(!equals(unwrap(decimal(-0)), unwrap(decimal(0))));
  assert.ok(compare(unwrap(decimal(-0)), unwrap(decimal(0))) < 0);
  assert.ok(!equals(unwrap(measurement(integer(5), "kg")), unwrap(measurement(unwrap(decimal(5)), "kg"))));
});

test("maps are equal whatever the entry order, and keys sort by code point", () => {
  const a = unwrap(map([["a", integer(1)], ["b", integer(2)]]));
  const b = unwrap(map([["b", integer(2)], ["a", integer(1)]]));
  assert.ok(equals(a, b));
  const astral = unwrap(map([["\u{1F600}", integer(1)], ["￿", integer(2)]]));
  assert.ok(astral.kind === "map");
  assert.deepEqual([...astral.entries.keys()], ["￿", "\u{1F600}"]);
});

test("constructors report errors with codes", () => {
  assert.equal(code(decimal(Number.NaN)), "not-finite");
  assert.equal(code(tagged("true", "x")), "invalid-tag");
  assert.equal(code(tagged("a b", "x")), "invalid-tag");
  assert.equal(code(range()), "no-ends");
  assert.equal(code(range(unwrap(string("x")))), "invalid-range-end");
  assert.equal(code(measurement(integer(1), "k g")), "invalid-unit");
  assert.equal(code(bigInteger(5n)), "invalid-integer");
  assert.equal(code(bigDecimal("0.1")), "invalid-decimal");
  assert.equal(code(integerFromDigits("007")), "invalid-integer");
  assert.equal(code(string("\uD800")), "invalid-string");
  const dup = map([["a", integer(1)], ["a", integer(2)]]);
  assert.equal(code(dup), "duplicate-key");
  assert.deepEqual(dup.diagnostics[0]?.path, ["a"]);
});

test("integers pick their kind by range", () => {
  assert.equal(integer(2n ** 63n - 1n).kind, "integer");
  assert.equal(integer(2n ** 63n).kind, "big_integer");
  assert.ok(compare(integer(2n ** 63n - 1n), integer(2n ** 63n)) < 0);
  assert.ok(isHomogeneousArray(array([integer(1), integer(2)])));
  assert.ok(!isHomogeneousArray(array([integer(1), boolean(true)])));
});

test("unwritable parts are reported with paths", () => {
  const r = toText(array([integer(1), array([integer(2)])]));
  assert.ok(!r.ok);
  assert.equal(r.diagnostics[0]?.code, "unwritable");
  assert.deepEqual(r.diagnostics[0]?.path, [1]);
  assert.ok(!toText(unwrap(measurement(integer(0), "xFF"))).ok);
});

test("a record may hold a map one level deep", () => {
  const inner = unwrap(map([["x", integer(1)]]));
  const rec = unwrap(map([["k", inner]]));
  assert.ok(!toText(rec).ok);
  assert.equal(unwrap(toText(rec, "record")), "{k: {x: 1}}");
  assert.ok(!toText(unwrap(map([["k", rec]])), "record").ok);
});

test("decimals never use an exponent", () => {
  assert.equal(unwrap(toText(unwrap(decimal(1e21)))), "1000000000000000000000.0");
  assert.equal(unwrap(toText(unwrap(decimal(1e-7)))), "0.0000001");
  assert.equal(unwrap(toText(unwrap(decimal(1.5e-10)))), "0.00000000015");
});

test("toNative: integers are numbers when safe and bigints otherwise", () => {
  assert.equal(unwrap(toNative(integer(42))), 42);
  assert.equal(unwrap(toNative(integer(2n ** 53n))), 2n ** 53n);
  assert.equal(unwrap(toNative(integer(2n ** 53n - 1n))), Number.MAX_SAFE_INTEGER);
});

test("toNative: losses are recorded with their paths and never fail the conversion", () => {
  const value = array([unwrap(decimal(1)), unwrap(decimal(1.5))]);
  const r = toNative(value);
  assert.ok(r.ok);
  assert.deepEqual(r.diagnostics.map((d) => [d.code, d.path]), [["kind", [0]]]);
  assert.ok(!failOnLoss(r).ok);
  assert.equal(pathToPointer(["a/b", 0, "c~d"]), "/a~1b/0/c~0d");
});

test("toNative: maps have a null prototype", () => {
  const r = toNative(unwrap(map([["__proto__", integer(1)]])));
  assert.ok(r.ok);
  assert.equal(Object.getPrototypeOf(r.value), null);
  assert.equal((r.value as Record<string, unknown>)["__proto__"], 1);
});

test("fromNative: unwritable values fail with every problem listed", () => {
  const r = fromNative({ name: "x", nick: null, score: Number.NaN, f: () => 1 });
  assert.ok(!r.ok);
  assert.deepEqual(
    r.diagnostics.map((d) => [d.code, d.path]),
    [["unsupported", ["nick"]], ["not-finite", ["score"]], ["unsupported", ["f"]]],
  );
  assert.equal(fromNative(new Date()).ok, false);
  assert.equal(fromNative(new Map([[1, 2]])).ok, false);
  assert.deepEqual(fail().diagnostics, []);
});

test("fromNative: numbers and bigints", () => {
  assert.equal(unwrap(fromNative(3)).kind, "integer");
  assert.equal(unwrap(fromNative(3.5)).kind, "decimal");
  assert.equal(unwrap(fromNative(2n ** 70n)).kind, "big_integer");
  const m = unwrap(fromNative(new Measurement(5, "kg")));
  assert.equal(unwrap(toText(m)), "5kg");
});
