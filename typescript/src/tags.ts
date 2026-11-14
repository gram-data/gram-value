// Interpretation of the ten known tags (spec/tags.md, ADR-004).

import { diagnostic, fail, ok, type Result } from "./diagnostic.ts";
import { tagged, type Value } from "./value.ts";

export interface DateParts {
  readonly year: number;
  readonly month: number;
  readonly day: number;
}
export interface TimeParts {
  readonly hour: number;
  readonly minute: number;
  readonly second: number;
  readonly nanosecond: number;
  /** Minutes east of UTC, when an offset is written. */
  readonly offsetMinutes?: number;
}
export interface DateTimeParts {
  readonly date: DateParts;
  readonly time: TimeParts;
  readonly zone?: string;
}
export interface Seconds {
  readonly negative: boolean;
  readonly whole: bigint;
  readonly nanos: number;
}
export interface DurationParts {
  readonly years?: bigint;
  readonly months?: bigint;
  readonly days?: bigint;
  readonly hours?: bigint;
  readonly minutes?: bigint;
  readonly seconds?: Seconds;
}
export interface PointParts {
  readonly srid?: bigint;
  readonly x: number;
  readonly y: number;
  readonly z?: number;
}
export interface WeekParts {
  readonly year: number;
  readonly week: number;
  readonly day?: number;
}

export type Typed =
  | { readonly type: "date"; readonly date: DateParts }
  | { readonly type: "time"; readonly time: TimeParts }
  | { readonly type: "datetime"; readonly dateTime: DateTimeParts }
  | { readonly type: "duration"; readonly duration: DurationParts }
  | { readonly type: "point"; readonly point: PointParts }
  | { readonly type: "uuid"; readonly bytes: Uint8Array }
  | { readonly type: "week"; readonly week: WeekParts };

/** A successful interpretation. */
export interface Interpretation {
  readonly tag: KnownTag;
  readonly typed: Typed;
  /** The normal content string (spec/tags.md section 4). */
  readonly normalContent: string;
}

export const KNOWN_TAGS = [
  "date",
  "time",
  "localtime",
  "datetime",
  "localdatetime",
  "duration",
  "point",
  "uuid",
  "week",
  "weekDate",
] as const;
export type KnownTag = (typeof KNOWN_TAGS)[number];

class Fault extends Error {
  readonly code: "invalid-form" | "out-of-range";
  constructor(code: "invalid-form" | "out-of-range", message: string) {
    super(message);
    this.code = code;
  }
}
const form = (m: string) => new Fault("invalid-form", m);
const range = (m: string) => new Fault("out-of-range", m);

/** Interpret a tagged value. Returns `ok(null)` for a value that is not tagged or whose tag is unknown. */
export function interpret(value: Value): Result<Interpretation | null> {
  return value.kind === "tagged" ? interpretTag(value.tag, value.content) : ok(null);
}

/** Interpret a tag and content. Returns `ok(null)` for an unknown tag. */
export function interpretTag(tag: string, content: string): Result<Interpretation | null> {
  const name = (KNOWN_TAGS as readonly string[]).includes(tag) ? (tag as KnownTag) : undefined;
  if (name === undefined) return ok(null);
  try {
    return ok(run(name, content));
  } catch (e) {
    if (e instanceof Fault) return fail(diagnostic(e.code, `${name}: ${e.message}`));
    throw e;
  }
}

/** The tagged value with this interpretation's normal content string. */
export function interpretationToValue(i: Interpretation): Value {
  const r = tagged(i.tag, i.normalContent);
  if (!r.ok) throw new Error("a known tag is a symbol");
  return r.value;
}

function run(name: KnownTag, content: string): Interpretation {
  switch (name) {
    case "date": {
      const [date, used] = parseDatePrefix(content);
      if (used !== content.length) throw form("unexpected text after the date");
      return { tag: name, typed: { type: "date", date }, normalContent: content };
    }
    case "time":
    case "localtime": {
      const time = parseTime(content, name === "time");
      return { tag: name, typed: { type: "time", time }, normalContent: timeNormal(time) };
    }
    case "datetime":
    case "localdatetime": {
      const [dateTime, normal] = parseDateTime(content, name === "datetime");
      return { tag: name, typed: { type: "datetime", dateTime }, normalContent: normal };
    }
    case "duration":
      return { tag: name, typed: { type: "duration", duration: parseDuration(content) }, normalContent: content };
    case "point": {
      const [point, normal] = parsePoint(content);
      return { tag: name, typed: { type: "point", point }, normalContent: normal };
    }
    case "uuid":
      return { tag: name, typed: { type: "uuid", bytes: parseUuid(content) }, normalContent: content.toLowerCase() };
    case "week":
      return { tag: name, typed: { type: "week", week: parseWeek(content, false) }, normalContent: content };
    case "weekDate":
      return { tag: name, typed: { type: "week", week: parseWeek(content, true) }, normalContent: content };
  }
}

const digits = (s: string) => /^\d+$/.test(s);

function twoDigits(s: string, what: string): number {
  if (!/^\d\d$/.test(s)) throw form(`${what} must be two digits`);
  return Number(s);
}

const isLeap = (y: number) => (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0;

function daysInMonth(year: number, month: number): number {
  if ([1, 3, 5, 7, 8, 10, 12].includes(month)) return 31;
  if ([4, 6, 9, 11].includes(month)) return 30;
  return isLeap(year) ? 29 : 28;
}

function parseDatePrefix(s: string): [DateParts, number] {
  const m = /^(-?\d{4,})-(\d\d)-(\d\d)/.exec(s);
  if (!m) {
    if (!/^-?\d{4,}/.test(s)) throw form("the year needs at least four digits");
    throw form("expected YYYY-MM-DD");
  }
  const year = Number(m[1]);
  if (!Number.isSafeInteger(year)) throw range("the year is too large");
  const month = twoDigits(m[2] ?? "", "the month");
  const day = twoDigits(m[3] ?? "", "the day");
  if (month < 1 || month > 12) throw range(`month ${month} does not exist`);
  if (day === 0 || day > daysInMonth(year, month)) {
    throw range(`day ${day} does not exist in month ${month} of ${year}`);
  }
  return [{ year, month, day }, m[0].length];
}

function parseOffset(s: string): number {
  if (s === "Z") return 0;
  const m = /^([+-])(\d\d):(\d\d)$/.exec(s);
  if (!m) throw form("an offset is Z, +hh:mm or -hh:mm");
  const h = Number(m[2]);
  const mi = Number(m[3]);
  if (h > 23 || mi > 59) throw range("the offset is out of range");
  return (m[1] === "-" ? -1 : 1) * (h * 60 + mi);
}

function parseTime(s: string, offsetAllowed: boolean): TimeParts {
  const split = s.search(/[Z+-]/);
  const clock = split < 0 ? s : s.slice(0, split);
  const off = split < 0 ? "" : s.slice(split);
  let offsetMinutes: number | undefined;
  if (off !== "") {
    if (!offsetAllowed) throw form("this tag does not allow an offset");
    offsetMinutes = parseOffset(off);
  }
  if (clock.length < 5 || clock[2] !== ":") throw form("expected hh:mm");
  const hour = twoDigits(clock.slice(0, 2), "the hour");
  const minute = twoDigits(clock.slice(3, 5), "the minute");
  let second = 0;
  let nanosecond = 0;
  if (clock.length > 5) {
    if (clock[5] !== ":" || clock.length < 8) throw form("expected hh:mm or hh:mm:ss");
    second = twoDigits(clock.slice(6, 8), "the second");
    if (clock.length > 8) {
      if (clock[8] !== ".") throw form("expected a fraction after the seconds");
      const frac = clock.slice(9);
      if (frac.length < 1 || frac.length > 9 || !digits(frac)) throw form("a fraction has 1 to 9 digits");
      nanosecond = Number(frac.padEnd(9, "0"));
    }
  }
  if (hour > 23 || minute > 59 || second > 59) throw range("the time is out of range");
  const t: TimeParts = { hour, minute, second, nanosecond };
  return offsetMinutes === undefined ? t : { ...t, offsetMinutes };
}

function offsetText(minutes: number): string {
  if (minutes === 0) return "Z";
  const a = Math.abs(minutes);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${minutes < 0 ? "-" : "+"}${p(Math.floor(a / 60))}:${p(a % 60)}`;
}

function timeNormal(t: TimeParts): string {
  const p = (n: number) => String(n).padStart(2, "0");
  let s = `${p(t.hour)}:${p(t.minute)}:${p(t.second)}`;
  if (t.nanosecond !== 0) s += "." + String(t.nanosecond).padStart(9, "0").replace(/0+$/, "");
  if (t.offsetMinutes !== undefined) s += offsetText(t.offsetMinutes);
  return s;
}

function parseDateTime(s: string, offsetAllowed: boolean): [DateTimeParts, string] {
  const [date, used] = parseDatePrefix(s);
  if (s[used] !== "T") throw form("expected T after the date");
  const rest = s.slice(used + 1);
  const open = rest.indexOf("[");
  let clock = rest;
  let zone: string | undefined;
  if (open >= 0) {
    if (!rest.endsWith("]")) throw form("a zone name is written [name]");
    clock = rest.slice(0, open);
    zone = rest.slice(open + 1, -1);
  }
  const time = parseTime(clock, offsetAllowed);
  if (zone !== undefined) {
    if (time.offsetMinutes === undefined) throw form("a zone name needs an offset");
    if (!/^[A-Za-z0-9/_+-]+$/.test(zone)) throw form("a zone name is letters, digits and / _ + -");
  }
  const normal = `${s.slice(0, used)}T${timeNormal(time)}${zone !== undefined ? `[${zone}]` : ""}`;
  return [zone === undefined ? { date, time } : { date, time, zone }, normal];
}

function parseSigned(num: string): bigint {
  if (!/^-?\d+$/.test(num)) throw form("a duration component is a number");
  return BigInt(num);
}

function parseDuration(s: string): DurationParts {
  if (!s.startsWith("P")) throw form("a duration starts with P");
  let rest = s.slice(1);
  const d: { -readonly [K in keyof DurationParts]: DurationParts[K] } = {};
  let inTime = false;
  let orderDate = 0;
  let orderTime = 0;
  let any = false;
  let timeComponents = 0;
  while (rest.length > 0) {
    if (rest.startsWith("T")) {
      if (inTime) throw form("T appears once");
      inTime = true;
      rest = rest.slice(1);
      if (rest.length === 0) throw form("T needs at least one component");
      continue;
    }
    const end = rest.search(/[A-Z]/);
    if (end < 0) throw form("a component needs a designator");
    const num = rest.slice(0, end);
    const designator = rest[end];
    rest = rest.slice(end + 1);
    any = true;
    if (inTime) timeComponents += 1;
    if (!inTime && designator === "Y" && orderDate < 1) {
      d.years = parseSigned(num);
      orderDate = 1;
    } else if (!inTime && designator === "M" && orderDate < 2) {
      d.months = parseSigned(num);
      orderDate = 2;
    } else if (!inTime && designator === "D" && orderDate < 3) {
      d.days = parseSigned(num);
      orderDate = 3;
    } else if (inTime && designator === "H" && orderTime < 1) {
      d.hours = parseSigned(num);
      orderTime = 1;
    } else if (inTime && designator === "M" && orderTime < 2) {
      d.minutes = parseSigned(num);
      orderTime = 2;
    } else if (inTime && designator === "S" && orderTime < 3) {
      const m = /^(-?)(\d+)(?:\.(\d{1,9}))?$/.exec(num);
      if (!m) throw form("seconds are digits with an optional fraction of 1 to 9 digits");
      d.seconds = {
        negative: m[1] === "-",
        whole: BigInt(m[2] ?? "0"),
        nanos: m[3] === undefined ? 0 : Number(m[3].padEnd(9, "0")),
      };
      orderTime = 3;
    } else {
      throw form("components are in the order Y M D, then T, H M S");
    }
  }
  if (!any) throw form("a duration has at least one component");
  if (inTime && timeComponents === 0) throw form("T needs at least one component");
  return d;
}

const NUMBER = /^-?\d+(\.\d+)?$/;

function numberText(f: number): string {
  return f === 0 ? "0" : String(f);
}

function parsePoint(s: string): [PointParts, string] {
  let srid: bigint | undefined;
  let coords = s;
  if (s.startsWith("SRID=")) {
    const semi = s.indexOf(";");
    if (semi < 0) throw form("SRID=n; needs the semicolon");
    const n = s.slice(5, semi);
    if (!digits(n) || BigInt(n) === 0n) throw form("the SRID is a positive integer");
    srid = BigInt(n);
    coords = s.slice(semi + 1);
  }
  const parts = coords.split(" ");
  if (!(parts.length === 2 || parts.length === 3) || !parts.every((p) => NUMBER.test(p))) {
    throw form("a point is two or three numbers separated by single spaces");
  }
  const nums = parts.map(Number);
  if (nums.some((n) => !Number.isFinite(n))) throw range("a coordinate is out of range");
  const normal = (srid !== undefined ? `SRID=${srid};` : "") + nums.map(numberText).join(" ");
  const point: PointParts = {
    ...(srid !== undefined ? { srid } : {}),
    x: nums[0] as number,
    y: nums[1] as number,
    ...(nums[2] !== undefined ? { z: nums[2] } : {}),
  };
  return [point, normal];
}

function parseUuid(s: string): Uint8Array {
  if (!/^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/.test(s)) {
    throw form("a UUID is 8-4-4-4-12 hexadecimal digits");
  }
  const hex = s.replaceAll("-", "");
  const out = new Uint8Array(16);
  for (let i = 0; i < 16; i++) out[i] = parseInt(hex.slice(2 * i, 2 * i + 2), 16);
  return out;
}

/** Day of the week of 1 January, 0 = Monday (proleptic Gregorian). */
function jan1Weekday(year: number): number {
  const y = year - 1;
  const days = y * 365 + Math.floor(y / 4) - Math.floor(y / 100) + Math.floor(y / 400);
  return ((days % 7) + 7) % 7;
}

function has53Weeks(year: number): boolean {
  const w = jan1Weekday(year);
  return w === 3 || (isLeap(year) && w === 2);
}

function parseWeek(s: string, withDay: boolean): WeekParts {
  const re = withDay ? /^(\d{4})W(\d\d)-(\d)$/ : /^(\d{4})W(\d\d)$/;
  const m = re.exec(s);
  if (!m) throw form(withDay ? "expected YYYYWww-D" : "expected YYYYWww");
  const year = Number(m[1]);
  const week = Number(m[2]);
  const max = has53Weeks(year) ? 53 : 52;
  if (week === 0 || week > max) throw range(`${year} has ${max} ISO weeks, not week ${week}`);
  if (withDay) {
    const day = Number(m[3]);
    if (day < 1 || day > 7) throw range("the weekday is 1 to 7");
    return { year, week, day };
  }
  return { year, week };
}
