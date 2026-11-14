// Mapping between Value and JavaScript values (ADR-006).

import {
  diagnostic,
  fail,
  ok,
  under,
  type Diagnostic,
  type PathSegment,
  type Result,
} from "./diagnostic.ts";
import * as v from "./value.ts";
import type { Value } from "./value.ts";

/** A tagged string: a tag and a content string. */
export class Tagged {
  readonly tag: string;
  readonly content: string;
  constructor(tag: string, content: string) {
    this.tag = tag;
    this.content = content;
  }
}

/** A magnitude and a unit. */
export class Measurement {
  readonly magnitude: number | bigint;
  readonly unit: string;
  constructor(magnitude: number | bigint, unit: string) {
    this.magnitude = magnitude;
    this.unit = unit;
  }
}

/** A range: inclusive, with an optional lower end and an optional upper end. */
export class Range {
  readonly lower?: number | bigint | Measurement;
  readonly upper?: number | bigint | Measurement;
  constructor(lower?: number | bigint | Measurement, upper?: number | bigint | Measurement) {
    if (lower !== undefined) this.lower = lower;
    if (upper !== undefined) this.upper = upper;
  }
}

/** A JavaScript value a gram value maps to. */
export type Native =
  | number
  | bigint
  | boolean
  | string
  | Tagged
  | Range
  | Measurement
  | readonly Native[]
  | { [key: string]: Native };

const SAFE_MAX = BigInt(Number.MAX_SAFE_INTEGER);

function numericToNative(value: v.NumericValue): Result<number | bigint> {
  switch (value.kind) {
    case "integer":
    case "big_integer": {
      const n = value.value;
      return ok(n >= -SAFE_MAX && n <= SAFE_MAX ? Number(n) : n);
    }
    case "decimal":
      return ok(
        value.value,
        Number.isInteger(value.value)
          ? [diagnostic("kind", "a whole decimal arrives as an integer-valued number")]
          : [],
      );
    case "big_decimal": {
      const n = Number(value.digits);
      if (!Number.isFinite(n)) {
        return fail(diagnostic("out-of-range", "the decimal is beyond the range of a number"));
      }
      return ok(n, [diagnostic("precision", "digits beyond a 64-bit float are lost")]);
    }
  }
}

function endToNative(
  value: v.NumericValue | v.MeasurementValue | undefined,
  key: string,
  losses: Diagnostic[],
  errs: Diagnostic[],
): number | bigint | Measurement | undefined {
  if (value === undefined) return undefined;
  const r = value.kind === "measurement" ? measurementToNative(value) : numericToNative(value);
  if (!r.ok) {
    errs.push(...under(key, r.diagnostics));
    return undefined;
  }
  losses.push(...under(key, r.diagnostics));
  return r.value;
}

function measurementToNative(m: v.MeasurementValue): Result<Measurement> {
  const r = numericToNative(m.magnitude);
  if (!r.ok) return fail(...under("magnitude", r.diagnostics));
  return ok(new Measurement(r.value, m.unit), under("magnitude", r.diagnostics));
}

/** Convert a value to JavaScript values, with the losses incurred (ADR-006 2.3, 2.4). */
export function toNative(value: Value): Result<Native> {
  switch (value.kind) {
    case "integer":
    case "big_integer":
    case "decimal":
    case "big_decimal":
      return numericToNative(value);
    case "boolean":
    case "string":
      return ok(value.value);
    case "tagged":
      return ok(new Tagged(value.tag, value.content));
    case "measurement":
      return measurementToNative(value);
    case "range": {
      const losses: Diagnostic[] = [];
      const errs: Diagnostic[] = [];
      const lower = endToNative(value.lower, "lower", losses, errs);
      const upper = endToNative(value.upper, "upper", losses, errs);
      return errs.length > 0 ? fail(...errs) : ok(new Range(lower, upper), losses);
    }
    case "array": {
      const out: Native[] = [];
      const losses: Diagnostic[] = [];
      const errs: Diagnostic[] = [];
      value.items.forEach((item, i) => {
        const r = toNative(item);
        if (r.ok) {
          out.push(r.value);
          losses.push(...under(i, r.diagnostics));
        } else errs.push(...under(i, r.diagnostics));
      });
      return errs.length > 0 ? fail(...errs) : ok(out, losses);
    }
    case "map": {
      const out: { [key: string]: Native } = Object.create(null) as { [key: string]: Native };
      const losses: Diagnostic[] = [];
      const errs: Diagnostic[] = [];
      for (const [k, item] of value.entries) {
        const r = toNative(item);
        if (r.ok) {
          out[k] = r.value;
          losses.push(...under(k, r.diagnostics));
        } else errs.push(...under(k, r.diagnostics));
      }
      return errs.length > 0 ? fail(...errs) : ok(out, losses);
    }
  }
}

function unsupported(path: readonly PathSegment[], what: string): Diagnostic {
  return diagnostic("unsupported", what, path);
}

function numberToValue(n: number | bigint, path: PathSegment[]): Result<Value> {
  if (typeof n === "bigint") return ok(v.integer(n));
  if (!Number.isFinite(n)) return fail(diagnostic("not-finite", "NaN and infinities cannot be written", path));
  return Number.isInteger(n) ? ok(v.integer(n)) : v.decimal(n);
}

function endToValue(
  end: number | bigint | Measurement | undefined,
  key: string,
  path: PathSegment[],
  errs: Diagnostic[],
): Value | undefined {
  if (end === undefined) return undefined;
  const r = fromNativeAt(end, [...path, key]);
  if (!r.ok) {
    errs.push(...r.diagnostics);
    return undefined;
  }
  return r.value;
}

function fromNativeAt(x: unknown, path: PathSegment[]): Result<Value> {
  const withPath = <T>(r: Result<T>): Result<T> =>
    r.ok ? r : { ok: false, diagnostics: r.diagnostics.map((d) => ({ ...d, path })) };
  if (typeof x === "number" || typeof x === "bigint") return numberToValue(x, path);
  if (typeof x === "boolean") return ok(v.boolean(x));
  if (typeof x === "string") return withPath(v.string(x));
  if (x instanceof Tagged) return withPath(v.tagged(x.tag, x.content));
  if (x instanceof Measurement) {
    const mag = numberToValue(x.magnitude, [...path, "magnitude"]);
    if (!mag.ok) return mag;
    return withPath(v.measurement(mag.value, x.unit));
  }
  if (x instanceof Range) {
    const errs: Diagnostic[] = [];
    const lower = endToValue(x.lower, "lower", path, errs);
    const upper = endToValue(x.upper, "upper", path, errs);
    return errs.length > 0 ? fail(...errs) : withPath(v.range(lower, upper));
  }
  if (Array.isArray(x)) {
    const items: Value[] = [];
    const errs: Diagnostic[] = [];
    x.forEach((item: unknown, i) => {
      const r = fromNativeAt(item, [...path, i]);
      if (r.ok) items.push(r.value);
      else errs.push(...r.diagnostics);
    });
    return errs.length > 0 ? fail(...errs) : ok(v.array(items));
  }
  if (x instanceof Map) {
    const errs: Diagnostic[] = [];
    const entries: [string, Value][] = [];
    for (const [k, item] of x as Map<unknown, unknown>) {
      if (typeof k !== "string") {
        errs.push(unsupported(path, "a map key must be a string"));
        continue;
      }
      const r = fromNativeAt(item, [...path, k]);
      if (r.ok) entries.push([k, r.value]);
      else errs.push(...r.diagnostics);
    }
    return errs.length > 0 ? fail(...errs) : v.map(entries);
  }
  if (typeof x === "object" && x !== null) {
    const proto: unknown = Object.getPrototypeOf(x);
    if (proto !== null && proto !== Object.prototype) {
      return fail(unsupported(path, "only plain objects and null-prototype objects are maps"));
    }
    const errs: Diagnostic[] = [];
    const entries: [string, Value][] = [];
    for (const [k, item] of Object.entries(x)) {
      const r = fromNativeAt(item, [...path, k]);
      if (r.ok) entries.push([k, r.value]);
      else errs.push(...r.diagnostics);
    }
    return errs.length > 0 ? fail(...errs) : v.map(entries);
  }
  const what = x === null ? "null" : x === undefined ? "undefined" : typeof x;
  return fail(unsupported(path, `${what} has no gram counterpart`));
}

/** Convert a JavaScript value to a gram value. Anything unwritable fails with every problem listed (ADR-006 2.6). */
export function fromNative(x: unknown): Result<Value> {
  return fromNativeAt(x, []);
}
