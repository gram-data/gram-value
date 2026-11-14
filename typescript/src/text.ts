// Canonical gram text (spec/text.md, ADR-003).

import { diagnostic, fail, ok, type Diagnostic, type PathSegment, type Result } from "./diagnostic.ts";
import { floatText, isBareWord, type MapValue, type Value } from "./value.ts";

/** Whether the outermost map is an ordinary value or an element's property record. */
export type Context = "value" | "record";

function escapeInto(s: string, quote: string): string {
  let out = "";
  for (const c of s) {
    switch (c) {
      case "\\":
        out += "\\\\";
        break;
      case "\b":
        out += "\\b";
        break;
      case "\f":
        out += "\\f";
        break;
      case "\n":
        out += "\\n";
        break;
      case "\r":
        out += "\\r";
        break;
      case "\t":
        out += "\\t";
        break;
      default:
        out += c === quote ? `\\${c}` : c;
    }
  }
  return out;
}

/** A string in canonical text: the bare word when it can be, else double-quoted. */
export function stringText(s: string): string {
  return isBareWord(s) ? s : `"${escapeInto(s, '"')}"`;
}

function unwritable(path: readonly PathSegment[], what: string): Diagnostic {
  return diagnostic("unwritable", what, [...path]);
}

/** Text of a scalar, or the reason it has none. */
function scalarText(v: Value): { text: string } | { reason: string } {
  switch (v.kind) {
    case "integer":
    case "big_integer":
      return { text: v.value.toString() };
    case "decimal":
      return { text: floatText(v.value) };
    case "big_decimal":
      return { text: v.digits };
    case "boolean":
      return { text: String(v.value) };
    case "string":
      return { text: stringText(v.value) };
    case "tagged":
      return { text: `${v.tag}\`${escapeInto(v.content, "`")}\`` };
    case "measurement": {
      const mag = scalarText(v.magnitude);
      if ("reason" in mag) return mag;
      if (mag.text === "0" && v.unit.startsWith("x")) {
        return {
          reason: "a measurement of 0 with a unit beginning x can read as a hexadecimal literal",
        };
      }
      return { text: `${mag.text}${v.unit}` };
    }
    case "range": {
      const end = (e: Value | undefined) => (e === undefined ? undefined : scalarText(e));
      const l = end(v.lower);
      const u = end(v.upper);
      for (const e of [l, u]) if (e !== undefined && "reason" in e) return e;
      const lt = l && "text" in l ? l.text : undefined;
      const ut = u && "text" in u ? u.text : undefined;
      if (lt !== undefined && ut !== undefined) return { text: `${lt}..${ut}` };
      if (lt !== undefined) return { text: `${lt}...` };
      if (ut !== undefined) return { text: `...${ut}` };
      return { reason: "a range with no ends" };
    }
    case "array":
    case "map":
      return { reason: "not a scalar" };
  }
}

function writeArray(
  items: readonly Value[],
  path: PathSegment[],
  errs: Diagnostic[],
): string {
  const parts: string[] = [];
  items.forEach((item, i) => {
    path.push(i);
    if (item.kind === "array") errs.push(unwritable(path, "an array cannot hold an array"));
    else if (item.kind === "map") errs.push(unwritable(path, "an array cannot hold a map"));
    else {
      const t = scalarText(item);
      if ("text" in t) parts.push(t.text);
      else errs.push(unwritable(path, t.reason));
    }
    path.pop();
  });
  return `[${parts.join(", ")}]`;
}

function writeMap(
  map: MapValue,
  nestedOk: boolean,
  path: PathSegment[],
  errs: Diagnostic[],
): string {
  const parts: string[] = [];
  for (const [k, v] of map.entries) {
    path.push(k);
    let text: string | undefined;
    if (v.kind === "array") text = writeArray(v.items, path, errs);
    else if (v.kind === "map") {
      if (nestedOk) text = writeMap(v, false, path, errs);
      else errs.push(unwritable(path, "a map entry cannot hold a map"));
    } else {
      const t = scalarText(v);
      if ("text" in t) text = t.text;
      else errs.push(unwritable(path, t.reason));
    }
    if (text !== undefined) parts.push(`${stringText(k)}: ${text}`);
    path.pop();
  }
  return `{${parts.join(", ")}}`;
}

/** The canonical text of a value, or an `unwritable` diagnostic for each part the grammar has no text for. */
export function toText(value: Value, context: Context = "value"): Result<string> {
  const errs: Diagnostic[] = [];
  const path: PathSegment[] = [];
  let text: string;
  if (value.kind === "array") text = writeArray(value.items, path, errs);
  else if (value.kind === "map") text = writeMap(value, context === "record", path, errs);
  else {
    const t = scalarText(value);
    if ("text" in t) text = t.text;
    else {
      errs.push(unwritable(path, t.reason));
      text = "";
    }
  }
  return errs.length === 0 ? ok(text) : fail(...errs);
}
