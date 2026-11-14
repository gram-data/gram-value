// Runs the language-neutral conformance corpus (ADR-001) against this package.
//
// The corpus is gram text. Each case's `value` is read with tree-sitter-gram and lowered to a
// `Value` by the functions below, which live in test code and are not shipped (ADR-001 2.3).

import assert from "node:assert/strict";
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import Parser from "tree-sitter";
import Gram from "@gram-data/tree-sitter-gram";
import {
  fromNative,
  interpret,
  interpretationToValue,
  toNative,
  toText,
  Measurement,
  Range,
  Tagged,
  unwrap,
  equals,
  type Value,
  type Native,
} from "../src/index.ts";
import * as gv from "../src/value.ts";

type Node = Parser.SyntaxNode;

const GRAMMAR_VERSION = "0.7.0";
const here = dirname(fileURLToPath(import.meta.url));
const corpus = (f: string) => join(here, "..", "..", "conformance", f);

const parser = new Parser();
parser.setLanguage(Gram as Parser.Language);
const parse = (src: string) => parser.parse(src);

// ---- lowering -------------------------------------------------------------------------------

function content(n: Node): string {
  return n.childForFieldName("content")?.text ?? "";
}

function unescape(s: string, quote: string): string {
  let out = "";
  for (let i = 0; i < s.length; i++) {
    const c = s[i] as string;
    if (c !== "\\") {
      out += c;
      continue;
    }
    const e = s[++i];
    const table: Record<string, string> = { b: "\b", f: "\f", n: "\n", r: "\r", t: "\t", "/": "/", "\\": "\\" };
    if (e !== undefined && e in table) out += table[e];
    else if (e === quote) out += e;
    else throw new Error(`escape the grammar does not allow: ${e}`);
  }
  return out;
}

function stringValue(n: Node): string {
  const full = n.text;
  return full.startsWith("```") ? content(n) : unescape(content(n), full[0] as string);
}

function lowerNumberText(t: string): Value {
  return unwrap(t.includes(".") ? gv.decimalFromText(t) : gv.integerFromDigits(t === "-0" ? "0" : t));
}

function lowerNumeric(n: Node): Value {
  const t = n.text;
  switch (n.type) {
    case "integer":
    case "decimal":
      return lowerNumberText(t);
    case "hexadecimal":
      return gv.integer(BigInt(t));
    case "octal":
      return gv.integer(BigInt("0o" + t.slice(1)));
    case "measurement": {
      const split = t.search(/[a-zA-Z]/);
      return unwrap(gv.measurement(lowerNumberText(t.slice(0, split)), t.slice(split)));
    }
    default:
      throw new Error(`not a numeric literal: ${n.type}`);
  }
}

function keyName(n: Node): string {
  if (n.type === "quoted_name") return unescape(content(n), "`");
  if (n.type === "double_quoted_name") return unescape(content(n), '"');
  return n.text;
}

const named = (n: Node) => n.namedChildren.filter((c) => c.type !== "comment" && c.type !== "doc_comment");

function lower(n: Node): Value {
  switch (n.type) {
    case "integer":
    case "decimal":
    case "hexadecimal":
    case "octal":
    case "measurement":
      return lowerNumeric(n);
    case "boolean_literal":
      return gv.boolean(n.text === "true");
    case "symbol":
      return unwrap(gv.string(n.text));
    case "string_literal":
      return unwrap(gv.string(stringValue(n)));
    case "tagged_string": {
      const tag = (n.childForFieldName("tag") as Node).text;
      const body = n.text.startsWith("```") ? content(n) : unescape(content(n), "`");
      return unwrap(gv.tagged(tag, body));
    }
    case "range": {
      const end = (f: string) => {
        const e = n.childForFieldName(f);
        return e ? lowerNumeric(e) : undefined;
      };
      return unwrap(gv.range(end("lower"), end("upper")));
    }
    case "array":
      return gv.array(named(n).map(lower));
    case "map":
      return unwrap(
        gv.map(
          named(n).map((e) => [
            keyName(e.childForFieldName("key") as Node),
            lower(e.childForFieldName("value") as Node),
          ] as const),
        ),
      );
    default:
      throw new Error(`cannot lower node kind ${n.type}`);
  }
}

// ---- cases ----------------------------------------------------------------------------------

interface Case {
  id: string;
  label: string;
  props: Map<string, Value>;
  value: Value;
}

function walk(root: Node, visit: (n: Node) => void): void {
  const stack = [root];
  for (let n = stack.pop(); n !== undefined; n = stack.pop()) {
    visit(n);
    stack.push(...n.children);
  }
}

function loadCases(file: string): Case[] {
  return loadNodes(corpus(file));
}

/** The package's skip list (ADR-001 2.6): case id to reason. */
function loadSkips(): Map<string, string> {
  const skips = new Map<string, string>();
  for (const node of loadNodes(join(here, "..", "skips.gram"))) {
    assert.equal(node.label, "Skip", `${node.id}: a skips file holds Skip nodes`);
    skips.set(propStr(node, "case"), propStr(node, "reason"));
  }
  return skips;
}

function loadNodes(path: string): Case[] {
  const file = path;
  const tree = parse(readFileSync(path, "utf8"));
  assert.ok(!tree.rootNode.hasError, `${file} has a syntax error`);
  const cases: Case[] = [];
  walk(tree.rootNode, (n) => {
    if (n.type !== "node_pattern") return;
    const id = n.childForFieldName("identifier");
    const labels = n.childForFieldName("labels");
    const record = n.childForFieldName("record");
    if (!id || !labels || !record) return;
    const props = new Map<string, Value>();
    for (const p of named(record)) {
      props.set(keyName(p.childForFieldName("key") as Node), lower(p.childForFieldName("value") as Node));
    }
    cases.push({
      id: id.text,
      label: (named(labels)[0] as Node).text,
      props,
      value: props.get("value") as Value,
    });
  });
  return cases.sort((a, b) => (a.id < b.id ? -1 : 1));
}

function vocabulary(): Map<string, Set<string>> {
  const tree = parse(readFileSync(corpus("vocabulary.gram"), "utf8"));
  assert.ok(!tree.rootNode.hasError, "vocabulary.gram has a syntax error");
  const vocab = new Map<string, Set<string>>();
  walk(tree.rootNode, (n) => {
    if (n.type !== "node_pattern") return;
    const id = n.childForFieldName("identifier");
    const labels = n.childForFieldName("labels");
    if (!id || !labels) return;
    for (const l of named(labels)) {
      if (!vocab.has(l.text)) vocab.set(l.text, new Set());
      vocab.get(l.text)?.add(id.text);
    }
  });
  return vocab;
}

const vocab = vocabulary();
function checkVocab(group: string, name: string, id: string): void {
  assert.ok(vocab.get(group)?.has(name), `case ${id}: ${JSON.stringify(name)} is not in the vocabulary group ${group}`);
}

function propStr(c: Case, k: string): string {
  const v = c.props.get(k);
  assert.ok(v?.kind === "string", `case ${c.id}: property ${k} should be a string`);
  return v.value;
}

function textOf(value: Value): string {
  const r = toText(value);
  assert.ok(r.ok, `unwritable: ${JSON.stringify(r.diagnostics)}`);
  return r.value;
}

function loadValueFromText(text: string): Value {
  const src = `(x {value: ${text}})`;
  const tree = parse(src);
  assert.ok(!tree.rootNode.hasError, `canonical text ${text} does not parse`);
  let found: Value | undefined;
  walk(tree.rootNode, (n) => {
    if (n.type === "record_property" && found === undefined) {
      found = lower(n.childForFieldName("value") as Node);
    }
  });
  return found as Value;
}

function nativeTypeName(n: Native): string {
  if (typeof n === "number") return "number";
  if (typeof n === "bigint") return "bigint";
  if (typeof n === "boolean") return "boolean";
  if (typeof n === "string") return "string";
  if (n instanceof Tagged) return "Tagged";
  if (n instanceof Range) return "Range";
  if (n instanceof Measurement) return "Measurement";
  if (Array.isArray(n)) return "Array";
  return "object";
}

const TAG_FOR_NATIVE: Record<string, string> = {
  Date: "date",
  Time: "time",
  DateTime: "datetime",
  Duration: "duration",
  Point: "point",
  Uuid: "uuid",
  Week: "week",
};

function runMapping(c: Case): void {
  for (const key of c.props.keys()) {
    if (key !== "name" && key !== "value") checkVocab("Language", key, c.id);
  }
  const entry = c.props.get("typescript");
  if (entry === undefined) return;
  assert.ok(entry.kind === "map", `case ${c.id}: an entry is a map`);
  const word = (k: string): string | undefined => {
    const x = entry.entries.get(k);
    return x?.kind === "string" ? x.value : undefined;
  };
  const cls = word("class") as string;
  const native = word("native") as string;
  checkVocab("Class", cls, c.id);
  checkVocab("TypeScriptNative", native, c.id);
  const lossEntry = entry.entries.get("loss");
  const wantLoss =
    lossEntry?.kind === "array" ? lossEntry.items.map((x) => (x.kind === "string" ? x.value : "")) : [];
  wantLoss.forEach((l) => checkVocab("Loss", l, c.id));
  assert.equal(cls === "lossy", wantLoss.length > 0, "loss kinds are present exactly when the class is lossy");

  if (cls === "unsupported") {
    assert.equal(native, "none");
    assert.ok(!toNative(c.value).ok, "the conversion should fail");
    return;
  }
  let back: Value;
  let losses: string[] = [];
  if (native in TAG_FOR_NATIVE) {
    const r = interpret(c.value);
    assert.ok(r.ok && r.value !== null, "a known tag interprets");
    assert.equal(r.value.tag, TAG_FOR_NATIVE[native]);
    back = interpretationToValue(r.value);
  } else {
    const r = toNative(c.value);
    assert.ok(r.ok, `conversion failed: ${JSON.stringify(r.diagnostics)}`);
    assert.equal(nativeTypeName(r.value), native, "native type");
    losses = [...new Set(r.diagnostics.map((d) => d.code))].sort();
    back = unwrap(fromNative(r.value));
  }
  assert.deepEqual(losses, [...wantLoss].sort(), "loss kinds");
  const got = textOf(back);
  assert.equal(got, word("roundtrip"), `round trip through ${native}`);
  if (cls === "exact") assert.ok(equals(back, c.value), "exact means the round trip gives an equal value");
  else assert.ok(!equals(back, c.value), "lossy means the round trip changes the value");
}

function runCase(c: Case): void {
  switch (c.label) {
    case "ValueCase": {
      const kind = propStr(c, "kind");
      checkVocab("Kind", kind, c.id);
      assert.equal(c.value.kind, kind, "wrong kind");
      break;
    }
    case "TextCase": {
      const got = textOf(c.value);
      assert.equal(got, propStr(c, "canonical"), "canonical text");
      assert.ok(equals(loadValueFromText(got), c.value), `canonical text ${got} reads back as a different value`);
      break;
    }
    case "TagCase": {
      const outcome = propStr(c, "outcome");
      checkVocab("Outcome", outcome, c.id);
      assert.ok(c.value.kind === "tagged", "a TagCase value is a tagged string");
      checkVocab("Tag", c.value.tag, c.id);
      const r = interpret(c.value);
      if (outcome === "accepted") {
        assert.ok(r.ok && r.value !== null, `expected accepted, got ${JSON.stringify(r.diagnostics)}`);
        assert.equal(r.value.normalContent, propStr(c, "interpreted"), "normal content");
      } else {
        assert.ok(!r.ok, "expected rejected");
        const want = propStr(c, "diagnostic");
        checkVocab("Diagnostic", want, c.id);
        assert.equal(r.diagnostics[0]?.code, want, `diagnostic code (${r.diagnostics[0]?.message})`);
      }
      break;
    }
    case "MappingCase":
      runMapping(c);
      break;
    default:
      assert.fail(`unknown case kind ${c.label}`);
  }
}

test("corpus", () => {
  assert.equal(readFileSync(corpus("GRAMMAR_VERSION"), "utf8").trim(), GRAMMAR_VERSION);
  const seen = new Set<string>();
  const failures: string[] = [];
  const skips = loadSkips();
  const counts = new Map<string, number>();
  let results = "";
  for (const file of ["values.gram", "text.gram", "tags.gram", "mappings.gram"]) {
    for (const c of loadCases(file)) {
      assert.ok(!seen.has(c.id), `case id ${c.id} is used twice`);
      seen.add(c.id);
      // A mapping case with no entry for this language does not apply to it (ADR-001 2.4).
      if (c.label === "MappingCase" && !c.props.has("typescript")) {
        counts.set("not-applicable", (counts.get("not-applicable") ?? 0) + 1);
        results += `(${c.id}:Result {language: typescript, status: not-applicable})\n`;
        continue;
      }
      let status = "passed";
      let extra = "";
      let error: string | undefined;
      try {
        runCase(c);
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      }
      const reason = skips.get(c.id);
      if (error === undefined && reason === undefined) status = "passed";
      else if (error === undefined) {
        status = "failed";
        failures.push(`${c.id}: unexpected pass of a skipped case`);
      } else if (reason !== undefined) {
        status = "skipped";
        extra = `, reason: ${JSON.stringify(reason)}`;
      } else {
        status = "failed";
        failures.push(`${c.id}: ${error}`);
      }
      counts.set(status, (counts.get(status) ?? 0) + 1);
      results += `(${c.id}:Result {language: typescript, status: ${status}${extra}})\n`;
    }
  }
  for (const id of skips.keys()) assert.ok(seen.has(id), `skips.gram names an unknown case: ${id}`);
  mkdirSync(join(here, "..", "target"), { recursive: true });
  writeFileSync(join(here, "..", "target", "conformance-results.gram"), results);
  assert.equal(failures.length, 0, `${failures.length} failed of ${seen.size}:\n${failures.join("\n")}`);
  assert.ok(seen.size > 100);
  console.log(`conformance: ${JSON.stringify(Object.fromEntries(counts))}`);
});
