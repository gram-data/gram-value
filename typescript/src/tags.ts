// Interpretation of the eleven known tags (spec/tags.md, ADR-004, ADR-008).

import { diagnostic, fail, ok, type Result } from "./diagnostic.ts";
import { floatText, tagged, type Value } from "./value.ts";

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

export type EmbeddingDtype = "f16" | "bf16" | "f32" | "f64";

export const MAX_EMBEDDING_DIMENSION = 1_048_576;

function dtypeWidth(dtype: EmbeddingDtype): number {
  switch (dtype) {
    case "f16":
    case "bf16":
      return 2;
    case "f32":
      return 4;
    case "f64":
      return 8;
  }
}

function dtypeStr(dtype: EmbeddingDtype): "F16" | "BF16" | "F32" | "F64" {
  switch (dtype) {
    case "f16":
      return "F16";
    case "bf16":
      return "BF16";
    case "f32":
      return "F32";
    case "f64":
      return "F64";
  }
}

/** A rank-one dense floating-point tensor carried by an `embedding` tag (ADR-008). */
export class Embedding {
  readonly dtype: EmbeddingDtype;
  readonly dimension: number;
  readonly bytes: Uint8Array;

  private constructor(dtype: EmbeddingDtype, dimension: number, bytes: Uint8Array) {
    this.dtype = dtype;
    this.dimension = dimension;
    this.bytes = bytes;
  }

  static create(dtype: EmbeddingDtype, dimension: number, bytes: Uint8Array): Result<Embedding> {
    if (!Number.isInteger(dimension) || dimension <= 0 || dimension > MAX_EMBEDDING_DIMENSION) {
      return fail(
        diagnostic(
          "out-of-range",
          `embedding: dimension ${dimension} must be between 1 and ${MAX_EMBEDDING_DIMENSION}`,
        ),
      );
    }
    const expected = dimension * dtypeWidth(dtype);
    if (bytes.byteLength !== expected) {
      return fail(
        diagnostic(
          "invalid-form",
          `embedding: expected ${expected} payload bytes for ${dimension} ${dtype} elements, got ${bytes.byteLength}`,
        ),
      );
    }
    return ok(new Embedding(dtype, dimension, new Uint8Array(bytes)));
  }

  equals(other: Embedding): boolean {
    if (
      this.dtype !== other.dtype ||
      this.dimension !== other.dimension ||
      this.bytes.byteLength !== other.bytes.byteLength
    ) {
      return false;
    }
    for (let i = 0; i < this.bytes.byteLength; i++) {
      if (this.bytes[i] !== other.bytes[i]) return false;
    }
    return true;
  }

  toNormalContent(): string {
    return encodeEmbeddingNormal(this.dtype, this.dimension, this.bytes);
  }

  toValue(): Value {
    const r = tagged("embedding", this.toNormalContent());
    if (!r.ok) throw new Error("embedding is a valid symbol");
    return r.value;
  }
}

export type Typed =
  | { readonly type: "date"; readonly date: DateParts }
  | { readonly type: "time"; readonly time: TimeParts }
  | { readonly type: "datetime"; readonly dateTime: DateTimeParts }
  | { readonly type: "duration"; readonly duration: DurationParts }
  | { readonly type: "point"; readonly point: PointParts }
  | { readonly type: "uuid"; readonly bytes: Uint8Array }
  | { readonly type: "week"; readonly week: WeekParts }
  | { readonly type: "embedding"; readonly embedding: Embedding };

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
  "embedding",
] as const;
export type KnownTag = (typeof KNOWN_TAGS)[number];

class Fault extends Error {
  readonly code: "invalid-form" | "out-of-range" | "unsupported";
  constructor(code: "invalid-form" | "out-of-range" | "unsupported", message: string) {
    super(message);
    this.code = code;
  }
}
const form = (m: string) => new Fault("invalid-form", m);
const range = (m: string) => new Fault("out-of-range", m);
const unsupportedDtype = (m: string) => new Fault("unsupported", m);

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
    case "embedding": {
      const [embedding, normalContent] = parseEmbedding(content);
      return { tag: name, typed: { type: "embedding", embedding }, normalContent };
    }
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
  if (f === 0) return "0";
  const t = floatText(f);
  return t.endsWith(".0") ? t.slice(0, -2) : t;
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

// ---- embedding (ADR-008) --------------------------------------------------------------------

const MAX_BASE64_CHARS = 11_190_284;
const MAX_DECODED_BYTES = 8_392_712;
const MAX_HEADER_BYTES = 4_096;
const B64_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_LOOKUP = (() => {
  const t = new Int8Array(128).fill(-1);
  for (let i = 0; i < B64_ALPHABET.length; i++) t[B64_ALPHABET.charCodeAt(i)] = i;
  return t;
})();

function b64Val(code: number): number {
  return code < 128 ? B64_LOOKUP[code]! : -1;
}

function decodeStrictBase64(content: string): Uint8Array {
  // 1. Base64 syntax (invalid-form)
  if (content.length === 0) throw form("empty Base64 content");
  if (content.length % 4 !== 0) throw form("Base64 length must be a multiple of 4");
  const padCount = content.endsWith("==") ? 2 : content.endsWith("=") ? 1 : 0;
  const dataEnd = content.length - padCount;
  for (let i = 0; i < dataEnd; i++) {
    if (b64Val(content.charCodeAt(i)) < 0) {
      throw form(`invalid Base64 character at index ${i}`);
    }
  }
  if (padCount === 1) {
    const v3 = b64Val(content.charCodeAt(content.length - 2));
    if ((v3 & 0x03) !== 0) throw form("non-canonical Base64 pad bits");
  } else if (padCount === 2) {
    const v2 = b64Val(content.charCodeAt(content.length - 3));
    if ((v2 & 0x0f) !== 0) throw form("non-canonical Base64 pad bits");
  }

  // 2. Decoded-size bound (out-of-range before allocation)
  const decodedLen = (content.length / 4) * 3 - padCount;
  if (content.length > MAX_BASE64_CHARS || decodedLen > MAX_DECODED_BYTES) {
    throw range(`embedding exceeds maximum decoded size of ${MAX_DECODED_BYTES} bytes`);
  }

  const out = new Uint8Array(decodedLen);
  let pos = 0;
  for (let i = 0; i < content.length; i += 4) {
    const a = b64Val(content.charCodeAt(i));
    const b = b64Val(content.charCodeAt(i + 1));
    out[pos++] = (a << 2) | (b >> 4);
    if (content.charCodeAt(i + 2) !== 61) {
      const c = b64Val(content.charCodeAt(i + 2));
      out[pos++] = ((b << 4) | (c >> 2)) & 0xff;
      if (content.charCodeAt(i + 3) !== 61) {
        const d = b64Val(content.charCodeAt(i + 3));
        out[pos++] = ((c << 6) | d) & 0xff;
      }
    }
  }
  return out;
}

function encodeBase64(bytes: Uint8Array): string {
  const parts: string[] = [];
  for (let i = 0; i < bytes.byteLength; i += 3) {
    const b0 = bytes[i]!;
    const has1 = i + 1 < bytes.byteLength;
    const has2 = i + 2 < bytes.byteLength;
    const b1 = has1 ? bytes[i + 1]! : 0;
    const b2 = has2 ? bytes[i + 2]! : 0;
    parts.push(
      B64_ALPHABET[b0 >> 2]!,
      B64_ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)]!,
      has1 ? B64_ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)]! : "=",
      has2 ? B64_ALPHABET[b2 & 0x3f]! : "=",
    );
  }
  return parts.join("");
}

const UTF8_ENCODER = new TextEncoder();
const UTF8_DECODER = new TextDecoder("utf-8", { fatal: true });

function encodeEmbeddingNormal(dtype: EmbeddingDtype, dimension: number, payload: Uint8Array): string {
  const json = `{"embedding":{"dtype":"${dtypeStr(dtype)}","shape":[${dimension}],"data_offsets":[0,${payload.byteLength}]}}`;
  const jsonBytes = UTF8_ENCODER.encode(json);
  const pad = (8 - (jsonBytes.byteLength % 8)) % 8;
  const headerLen = jsonBytes.byteLength + pad;
  const buf = new Uint8Array(8 + headerLen + payload.byteLength);
  const view = new DataView(buf.buffer);
  view.setBigUint64(0, BigInt(headerLen), true);
  buf.set(jsonBytes, 8);
  buf.fill(0x20, 8 + jsonBytes.byteLength, 8 + headerLen);
  buf.set(payload, 8 + headerLen);
  return encodeBase64(buf);
}

const U64_MAX = (1n << 64n) - 1n;

class HeaderParser {
  private readonly s: string;
  private pos = 0;

  constructor(s: string) {
    this.s = s;
  }

  private skipWs(): void {
    while (this.pos < this.s.length) {
      const c = this.s.charCodeAt(this.pos);
      if (c === 0x20 || c === 0x09 || c === 0x0a || c === 0x0d) this.pos++;
      else break;
    }
  }

  private expectChar(want: string): void {
    this.skipWs();
    if (this.pos >= this.s.length || this.s[this.pos] !== want) {
      throw form(`expected '${want}' in JSON header`);
    }
    this.pos++;
  }

  private parseHex4(): number {
    if (this.pos + 4 > this.s.length) throw form("short \\uXXXX escape in JSON string");
    const hex = this.s.slice(this.pos, this.pos + 4);
    if (!/^[0-9a-fA-F]{4}$/.test(hex)) throw form("invalid \\uXXXX escape in JSON string");
    this.pos += 4;
    return Number.parseInt(hex, 16);
  }

  private parseString(): string {
    this.skipWs();
    if (this.pos >= this.s.length || this.s[this.pos] !== '"') throw form("expected JSON string");
    this.pos++;
    let out = "";
    while (this.pos < this.s.length) {
      const c = this.s.charCodeAt(this.pos);
      if (c === 0x22) {
        this.pos++;
        return out;
      }
      if (c < 0x20) throw form("unescaped control character in JSON string");
      if (c === 0x5c) {
        this.pos++;
        if (this.pos >= this.s.length) throw form("unterminated JSON escape");
        const esc = this.s[this.pos++];
        switch (esc) {
          case '"':
            out += '"';
            break;
          case "\\":
            out += "\\";
            break;
          case "/":
            out += "/";
            break;
          case "b":
            out += "\b";
            break;
          case "f":
            out += "\f";
            break;
          case "n":
            out += "\n";
            break;
          case "r":
            out += "\r";
            break;
          case "t":
            out += "\t";
            break;
          case "u": {
            const code = this.parseHex4();
            if (code >= 0xd800 && code <= 0xdbff) {
              if (this.s.slice(this.pos, this.pos + 2) !== "\\u") {
                throw form("unpaired UTF-16 high surrogate in JSON string");
              }
              this.pos += 2;
              const low = this.parseHex4();
              if (low < 0xdc00 || low > 0xdfff) {
                throw form("invalid UTF-16 low surrogate in JSON string");
              }
              const cp = 0x10000 + (((code - 0xd800) << 10) | (low - 0xdc00));
              out += String.fromCodePoint(cp);
            } else if (code >= 0xdc00 && code <= 0xdfff) {
              throw form("unpaired UTF-16 low surrogate in JSON string");
            } else {
              out += String.fromCharCode(code);
            }
            break;
          }
          default:
            throw form("invalid JSON escape sequence");
        }
      } else {
        const cp = this.s.codePointAt(this.pos)!;
        const ch = String.fromCodePoint(cp);
        out += ch;
        this.pos += ch.length;
      }
    }
    throw form("unterminated JSON string");
  }

  private parsePlainUintToken(): bigint {
    this.skipWs();
    const start = this.pos;
    while (this.pos < this.s.length) {
      const c = this.s.charCodeAt(this.pos);
      if (c >= 0x30 && c <= 0x39) this.pos++;
      else break;
    }
    if (this.pos === start) throw form("expected plain base-10 integer");
    if (this.pos - start > 1 && this.s[start] === "0") throw form("leading zero in JSON integer");
    const next = this.s[this.pos];
    if (next === "." || next === "e" || next === "E") throw form("non-integer JSON number");
    return BigInt(this.s.slice(start, this.pos));
  }

  private parseShape(): bigint {
    this.expectChar("[");
    const n = this.parsePlainUintToken();
    this.expectChar("]");
    return n;
  }

  private parseDataOffsets(): [bigint, bigint] {
    this.expectChar("[");
    const a = this.parsePlainUintToken();
    this.expectChar(",");
    const b = this.parsePlainUintToken();
    this.expectChar("]");
    if (a > U64_MAX || b > U64_MAX) throw form("data_offsets exceeds 64 unsigned bits");
    return [a, b];
  }

  parseHeader(): { dtype: string; shapeDim: bigint; offsets: [bigint, bigint] } {
    this.expectChar("{");
    const topKey = this.parseString();
    if (topKey !== "embedding") throw form(`unexpected top-level key "${topKey}"`);
    this.expectChar(":");
    this.expectChar("{");

    let dtype: string | undefined;
    let shapeDim: bigint | undefined;
    let offsets: [bigint, bigint] | undefined;

    for (let idx = 0; idx < 3; idx++) {
      if (idx > 0) this.expectChar(",");
      const key = this.parseString();
      this.expectChar(":");
      if (key === "dtype") {
        if (dtype !== undefined) throw form('duplicate key "dtype"');
        dtype = this.parseString();
      } else if (key === "shape") {
        if (shapeDim !== undefined) throw form('duplicate key "shape"');
        shapeDim = this.parseShape();
      } else if (key === "data_offsets") {
        if (offsets !== undefined) throw form('duplicate key "data_offsets"');
        offsets = this.parseDataOffsets();
      } else {
        throw form(`unexpected tensor key "${key}"`);
      }
    }
    this.expectChar("}");
    this.expectChar("}");
    this.skipWs();
    if (this.pos !== this.s.length) throw form("trailing bytes in SafeTensors JSON header");
    return { dtype: dtype!, shapeDim: shapeDim!, offsets: offsets! };
  }
}

function parseEmbedding(content: string): [Embedding, string] {
  // Steps 1 & 2: Base64 syntax + decoded-size bound before allocation.
  const decoded = decodeStrictBase64(content);

  // Step 3: SafeTensors container & JSON profile (invalid-form).
  if (decoded.byteLength < 8) throw form("SafeTensors buffer shorter than 8 bytes");
  const view = new DataView(decoded.buffer, decoded.byteOffset, decoded.byteLength);
  const headerLenBig = view.getBigUint64(0, true);
  const remaining = BigInt(decoded.byteLength - 8);
  if (headerLenBig > BigInt(MAX_HEADER_BYTES) || headerLenBig > remaining) {
    throw form("SafeTensors header length exceeds 4,096 or buffer");
  }
  const headerLen = Number(headerLenBig);
  let headerStr: string;
  try {
    headerStr = UTF8_DECODER.decode(decoded.subarray(8, 8 + headerLen));
  } catch {
    throw form("SafeTensors header is not valid UTF-8");
  }
  const { dtype: dtypeRaw, shapeDim, offsets } = new HeaderParser(headerStr).parseHeader();

  // Step 4: dtype (unsupported).
  let dtype: EmbeddingDtype;
  switch (dtypeRaw) {
    case "F16":
      dtype = "f16";
      break;
    case "BF16":
      dtype = "bf16";
      break;
    case "F32":
      dtype = "f32";
      break;
    case "F64":
      dtype = "f64";
      break;
    default:
      throw unsupportedDtype(`unsupported embedding dtype "${dtypeRaw}"`);
  }

  // Step 5: dimension range (out-of-range).
  if (shapeDim > U64_MAX) throw range("embedding dimension exceeds 64 unsigned bits");
  if (shapeDim === 0n || shapeDim > BigInt(MAX_EMBEDDING_DIMENSION)) {
    throw range(`embedding dimension ${shapeDim} must be between 1 and ${MAX_EMBEDDING_DIMENSION}`);
  }
  const dimension = Number(shapeDim);

  // Step 6: stated and actual byte lengths (invalid-form).
  const expectedBytes = dimension * dtypeWidth(dtype);
  if (offsets[0] !== 0n || offsets[1] !== BigInt(expectedBytes)) {
    throw form(`data_offsets [${offsets[0]}, ${offsets[1]}] does not match [0, ${expectedBytes}]`);
  }
  const payloadStart = 8 + headerLen;
  if (decoded.byteLength !== payloadStart + expectedBytes) {
    throw form(
      `payload length ${decoded.byteLength - payloadStart} does not match expected ${expectedBytes}`,
    );
  }

  const payload = decoded.slice(payloadStart);
  const embRes = Embedding.create(dtype, dimension, payload);
  if (!embRes.ok) throw form("invalid embedding payload");
  const normal = encodeEmbeddingNormal(dtype, dimension, payload);
  return [embRes.value, normal];
}
