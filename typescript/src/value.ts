// The eleven kinds of value, their constructors, equality and order (spec/values.md, ADR-002).

import { diagnostic, fail, ok, type Result } from "./diagnostic.ts";

export interface IntegerValue {
  readonly kind: "integer";
  readonly value: bigint;
}
export interface BigIntegerValue {
  readonly kind: "big_integer";
  readonly value: bigint;
}
export interface DecimalValue {
  readonly kind: "decimal";
  readonly value: number;
}
export interface BigDecimalValue {
  readonly kind: "big_decimal";
  /** Sign, digits, point and digits, with no trailing zeros beyond one digit. */
  readonly digits: string;
}
export interface BooleanValue {
  readonly kind: "boolean";
  readonly value: boolean;
}
export interface StringValue {
  readonly kind: "string";
  readonly value: string;
}
export interface TaggedValue {
  readonly kind: "tagged";
  readonly tag: string;
  readonly content: string;
}
export type NumericValue = IntegerValue | BigIntegerValue | DecimalValue | BigDecimalValue;
export interface MeasurementValue {
  readonly kind: "measurement";
  readonly magnitude: NumericValue;
  readonly unit: string;
}
export interface RangeValue {
  readonly kind: "range";
  readonly lower?: NumericValue | MeasurementValue;
  readonly upper?: NumericValue | MeasurementValue;
}
export interface ArrayValue {
  readonly kind: "array";
  readonly items: readonly Value[];
}
export interface MapValue {
  readonly kind: "map";
  /** Entries in ascending order of the keys' UTF-8 bytes. */
  readonly entries: ReadonlyMap<string, Value>;
}

/** A gram value. The set of kinds is closed (ADR-002). */
export type Value =
  | IntegerValue
  | BigIntegerValue
  | DecimalValue
  | BigDecimalValue
  | BooleanValue
  | StringValue
  | TaggedValue
  | RangeValue
  | MeasurementValue
  | ArrayValue
  | MapValue;

export type Kind = Value["kind"];

/** The kinds in kind order, which is also the order of the total order. */
export const KINDS: readonly Kind[] = [
  "integer",
  "big_integer",
  "decimal",
  "big_decimal",
  "boolean",
  "string",
  "tagged",
  "range",
  "measurement",
  "array",
  "map",
];

const I64_MIN = -(2n ** 63n);
const I64_MAX = 2n ** 63n - 1n;

export function isScalar(v: Value): boolean {
  return v.kind !== "array" && v.kind !== "map";
}

/** For an array, whether all elements have one kind (true for an empty array); false for other kinds. */
export function isHomogeneousArray(v: Value): boolean {
  return v.kind === "array" && v.items.every((x, i) => i === 0 || x.kind === v.items[0]?.kind);
}

export function isNumeric(v: Value): v is NumericValue {
  return (
    v.kind === "integer" ||
    v.kind === "big_integer" ||
    v.kind === "decimal" ||
    v.kind === "big_decimal"
  );
}

/** True when `s` matches the symbol rule `[a-zA-Z_][0-9a-zA-Z_.\-@]*`. */
export function isSymbol(s: string): boolean {
  return /^[a-zA-Z_][0-9a-zA-Z_.\-@]*$/.test(s);
}

/** True when `s` is a symbol other than `true` and `false`. */
export function isBareWord(s: string): boolean {
  return isSymbol(s) && s !== "true" && s !== "false";
}

function hasLoneSurrogate(s: string): boolean {
  return !s.isWellFormed();
}

/** Compare strings by UTF-8 bytes, which is the order of their code points. */
export function compareUtf8(a: string, b: string): number {
  const ai = a[Symbol.iterator]();
  const bi = b[Symbol.iterator]();
  for (;;) {
    const x = ai.next();
    const y = bi.next();
    if (x.done && y.done) return 0;
    if (x.done) return -1;
    if (y.done) return 1;
    const cx = x.value.codePointAt(0) ?? 0;
    const cy = y.value.codePointAt(0) ?? 0;
    if (cx !== cy) return cx < cy ? -1 : 1;
  }
}

// ---- number text ------------------------------------------------------------------------------

const INTEGER_TEXT = /^-?(0|[1-9]\d*)$/;
const DECIMAL_TEXT = /^-?(0|[1-9]\d*)\.\d+$/;

/** Drop trailing zeros after the point, keeping at least one digit. */
function normalizeDecimal(s: string): string {
  const [int = "", frac = ""] = s.split(".");
  const f = frac.replace(/0+$/, "");
  return `${int}.${f === "" ? "0" : f}`;
}

/** The shortest positional text of a finite float, with at least one digit after the point. */
export function floatText(n: number): string {
  if (Object.is(n, -0)) return "-0.0";
  let s = String(n);
  if (s.includes("e")) s = expandExponent(s);
  return s.includes(".") ? s : `${s}.0`;
}

function expandExponent(s: string): string {
  const m = /^(-?)(\d+)(?:\.(\d+))?e([+-]\d+)$/.exec(s);
  if (!m) return s;
  const [, sign = "", int = "", frac = "", exp = "0"] = m;
  const e = Number(exp);
  const digits = int + frac;
  const point = int.length + e;
  if (point <= 0) return `${sign}0.${"0".repeat(-point)}${digits}`;
  if (point >= digits.length) return `${sign}${digits}${"0".repeat(point - digits.length)}`;
  return `${sign}${digits.slice(0, point)}.${digits.slice(point)}`;
}

// ---- constructors -----------------------------------------------------------------------------

/** An integer: an `integer` if it fits in 64 bits, else a `big_integer`. A `number` must be an integer. */
export function integer(n: bigint | number): Value {
  if (typeof n === "number") {
    if (!Number.isInteger(n)) throw new TypeError(`${n} is not an integer`);
    n = BigInt(n);
  }
  return n >= I64_MIN && n <= I64_MAX
    ? { kind: "integer", value: n }
    : { kind: "big_integer", value: n };
}

/** An integer from `-?(0|[1-9]\d*)`. */
export function integerFromDigits(text: string): Result<Value> {
  if (!INTEGER_TEXT.test(text) || text === "-0") {
    return fail(
      diagnostic("invalid-integer", `${JSON.stringify(text)} is not an integer without leading zeros`),
    );
  }
  return ok(integer(BigInt(text)));
}

/** A `big_integer`; the number must be outside the 64-bit range. */
export function bigInteger(n: bigint): Result<Value> {
  if (n >= I64_MIN && n <= I64_MAX) {
    return fail(diagnostic("invalid-integer", `${n} fits in 64 bits, so it is an integer`));
  }
  return ok({ kind: "big_integer", value: n });
}

/** A `decimal`. `NaN` and the infinities are rejected (`not-finite`). */
export function decimal(n: number): Result<Value> {
  if (!Number.isFinite(n)) return fail(diagnostic("not-finite", "a decimal must be finite"));
  return ok({ kind: "decimal", value: n });
}

/** A decimal from `-?(0|[1-9]\d*)\.\d+`: a `decimal` when the nearest float writes back as the same number, else a `big_decimal`. */
export function decimalFromText(text: string): Result<Value> {
  if (!DECIMAL_TEXT.test(text)) {
    return fail(diagnostic("invalid-decimal", `${JSON.stringify(text)} is not a decimal`));
  }
  const normalized = normalizeDecimal(text);
  const f = Number(normalized);
  if (Number.isFinite(f) && floatText(f) === normalized) return ok({ kind: "decimal", value: f });
  return ok({ kind: "big_decimal", digits: normalized });
}

/** A `big_decimal`. The text must be normalized and beyond what a float holds. */
export function bigDecimal(text: string): Result<Value> {
  if (DECIMAL_TEXT.test(text) && normalizeDecimal(text) === text) {
    const r = decimalFromText(text);
    if (r.ok && r.value.kind === "big_decimal") return r;
  }
  return fail(
    diagnostic(
      "invalid-decimal",
      `${JSON.stringify(text)} is not a normalized decimal beyond a 64-bit float`,
    ),
  );
}

export function boolean(b: boolean): Value {
  return { kind: "boolean", value: b };
}

/** A `string`. A string with a lone surrogate is rejected (`invalid-string`). */
export function string(s: string): Result<Value> {
  if (hasLoneSurrogate(s)) {
    return fail(diagnostic("invalid-string", "a string must be valid Unicode"));
  }
  return ok({ kind: "string", value: s });
}

/** A `tagged` value. The tag must be a symbol other than `true` and `false` (`invalid-tag`). */
export function tagged(tag: string, content: string): Result<Value> {
  if (!isBareWord(tag)) {
    return fail(
      diagnostic("invalid-tag", `${JSON.stringify(tag)} is not a symbol other than true or false`),
    );
  }
  if (hasLoneSurrogate(content)) {
    return fail(diagnostic("invalid-string", "the content must be valid Unicode"));
  }
  return ok({ kind: "tagged", tag, content });
}

/** A `range`. Each end must be numeric or a measurement (`invalid-range-end`); at least one end is needed (`no-ends`). */
export function range(lower?: Value, upper?: Value): Result<Value> {
  if (lower === undefined && upper === undefined) {
    return fail(diagnostic("no-ends", "a range needs at least one end"));
  }
  for (const end of [lower, upper]) {
    if (end !== undefined && !isNumeric(end) && end.kind !== "measurement") {
      return fail(
        diagnostic(
          "invalid-range-end",
          `a range end must be numeric or a measurement, not ${end.kind}`,
        ),
      );
    }
  }
  const out: { kind: "range"; lower?: NumericValue | MeasurementValue; upper?: NumericValue | MeasurementValue } = {
    kind: "range",
  };
  if (lower !== undefined) out.lower = lower as NumericValue | MeasurementValue;
  if (upper !== undefined) out.upper = upper as NumericValue | MeasurementValue;
  return ok(out);
}

/** A `measurement`. The magnitude must be numeric and the unit ASCII letters (`invalid-unit`). */
export function measurement(magnitude: Value, unit: string): Result<Value> {
  if (!isNumeric(magnitude)) {
    return fail(
      diagnostic("invalid-range-end", `a magnitude must be numeric, not ${magnitude.kind}`),
    );
  }
  if (!/^[a-zA-Z]+$/.test(unit)) {
    return fail(diagnostic("invalid-unit", `${JSON.stringify(unit)} is not one or more ASCII letters`));
  }
  return ok({ kind: "measurement", magnitude, unit });
}

export function array(items: Iterable<Value>): Value {
  return { kind: "array", items: [...items] };
}

/** A `map`. A repeated key is an error (`duplicate-key`) that names the key. */
export function map(entries: Iterable<readonly [string, Value]>): Result<Value> {
  const seen = new Map<string, Value>();
  for (const [k, v] of entries) {
    if (seen.has(k)) {
      return fail(diagnostic("duplicate-key", `the key ${JSON.stringify(k)} is repeated`, [k]));
    }
    seen.set(k, v);
  }
  const sorted = new Map([...seen].sort(([a], [b]) => compareUtf8(a, b)));
  return ok({ kind: "map", entries: sorted });
}

// ---- equality and order ---------------------------------------------------------------------

/** Equality: same kind and same content, part by part (ADR-002 2.5). */
export function equals(a: Value, b: Value): boolean {
  return compare(a, b) === 0;
}

function cmp<T extends number | bigint | string>(a: T, b: T): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function compareDigits(a: string, b: string): number {
  const neg = (s: string) => s.startsWith("-");
  if (neg(a) !== neg(b)) return neg(a) ? -1 : 1;
  const negative = neg(a);
  const [ai = "", af = ""] = a.replace("-", "").split(".");
  const [bi = "", bf = ""] = b.replace("-", "").split(".");
  const m = cmp(ai.length, bi.length) || cmp(ai, bi) || cmp(af, bf);
  return negative ? -m : m;
}

function compareOptional(a: Value | undefined, b: Value | undefined): number {
  if (a === undefined) return b === undefined ? 0 : -1;
  if (b === undefined) return 1;
  return compare(a, b);
}

/** The total order of spec/values.md section 6. Consistent with {@link equals}. */
export function compare(a: Value, b: Value): number {
  const ka = KINDS.indexOf(a.kind);
  const kb = KINDS.indexOf(b.kind);
  if (ka !== kb) return ka < kb ? -1 : 1;
  switch (a.kind) {
    case "integer":
    case "big_integer":
      return cmp(a.value, (b as IntegerValue | BigIntegerValue).value);
    case "decimal": {
      const x = a.value;
      const y = (b as DecimalValue).value;
      if (Object.is(x, y)) return 0;
      if (x === 0 && y === 0) return Object.is(x, -0) ? -1 : 1;
      return cmp(x, y);
    }
    case "big_decimal":
      return compareDigits(a.digits, (b as BigDecimalValue).digits);
    case "boolean":
      return cmp(Number(a.value), Number((b as BooleanValue).value));
    case "string":
      return compareUtf8(a.value, (b as StringValue).value);
    case "tagged": {
      const t = b as TaggedValue;
      return compareUtf8(a.tag, t.tag) || compareUtf8(a.content, t.content);
    }
    case "range": {
      const r = b as RangeValue;
      return compareOptional(a.lower, r.lower) || compareOptional(a.upper, r.upper);
    }
    case "measurement": {
      const m = b as MeasurementValue;
      return compare(a.magnitude, m.magnitude) || compareUtf8(a.unit, m.unit);
    }
    case "array": {
      const items = (b as ArrayValue).items;
      for (let i = 0; i < Math.min(a.items.length, items.length); i++) {
        const c = compare(a.items[i] as Value, items[i] as Value);
        if (c !== 0) return c;
      }
      return cmp(a.items.length, items.length);
    }
    case "map": {
      const x = [...a.entries];
      const y = [...(b as MapValue).entries];
      for (let i = 0; i < Math.min(x.length, y.length); i++) {
        const [kx, vx] = x[i] as [string, Value];
        const [ky, vy] = y[i] as [string, Value];
        const c = compareUtf8(kx, ky) || compare(vx, vy);
        if (c !== 0) return c;
      }
      return cmp(x.length, y.length);
    }
  }
}
