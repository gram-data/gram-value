// SPIKE-002: TypeScript candidates. Native objects, the Rust core behind WebAssembly with handles, and with JSON.
// Run: node bench.ts   (after building ../ts/pkg with wasm-bindgen --target nodejs; see ../../SPIKE.md)
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const wasm = require("./pkg/wasm_bench.js");

type Value =
  | { type: "integer"; value: number }
  | { type: "decimal"; value: number }
  | { type: "boolean"; value: boolean }
  | { type: "string"; value: string }
  | { type: "symbol"; value: string }
  | { type: "tagged"; tag: string; content: string }
  | { type: "range"; lower: number | null; upper: number | null }
  | { type: "measurement"; unit: string; value: number }
  | { type: "array"; items: Value[] };

function gen(i: number): Value {
  switch (i % 8) {
    case 0: return { type: "integer", value: i };
    case 1: return { type: "decimal", value: i * 0.5 };
    case 2: return { type: "boolean", value: i % 2 === 0 };
    case 3: return { type: "string", value: `s${i}` };
    case 4: return { type: "symbol", value: `sym${i}` };
    case 5: return { type: "tagged", tag: "md", content: `# ${i}` };
    case 6: return { type: "range", lower: i, upper: i + 10 };
    default: return { type: "measurement", unit: "kg", value: i };
  }
}

function mapOne(v: Value): Value {
  switch (v.type) {
    case "integer": return { type: "integer", value: v.value + 1 };
    case "decimal": return { type: "decimal", value: v.value * 2 };
    case "boolean": return { type: "boolean", value: !v.value };
    case "string": return { type: "string", value: v.value + "!" };
    case "symbol": return { type: "symbol", value: v.value };
    case "tagged": return { type: "tagged", tag: v.tag, content: v.content + "!" };
    case "range": return { type: "range", lower: v.lower === null ? null : v.lower + 1, upper: v.upper === null ? null : v.upper + 1 };
    case "measurement": return { type: "measurement", unit: v.unit, value: v.value * 2 };
    case "array": return { type: "array", items: v.items.map(mapOne) };
  }
}

function checksum(v: Value): number {
  switch (v.type) {
    case "integer": case "decimal": case "measurement": return v.value;
    case "boolean": return v.value ? 1 : 0;
    case "string": case "symbol": return v.value.length;
    case "tagged": return v.content.length;
    case "range": return (v.lower ?? 0) + (v.upper ?? 0);
    case "array": return v.items.reduce((a, x) => a + checksum(x), 0);
  }
}

const num = (f: number) => (Number.isInteger(f) ? `${f}.0` : `${f}`);
function toGram(v: Value): string {
  switch (v.type) {
    case "integer": return `${v.value}`;
    case "decimal": return num(v.value);
    case "boolean": return `${v.value}`;
    case "string": return `"${v.value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n")}"`;
    case "symbol": return v.value;
    case "tagged": return `${v.tag}\`${v.content}\``;
    case "range": return v.lower !== null && v.upper !== null ? `${Math.trunc(v.lower)}..${Math.trunc(v.upper)}` : "...";
    case "measurement": return `${Math.trunc(v.value)}${v.unit}`;
    case "array": return `[${v.items.map(toGram).join(", ")}]`;
  }
}

// --- the wasm handle API -------------------------------------------------------------------
const { WValue, WList } = wasm;
function genW(i: number) {
  switch (i % 8) {
    case 0: return WValue.integer(BigInt(i));
    case 1: return WValue.decimal(i * 0.5);
    case 2: return WValue.boolean(i % 2 === 0);
    case 3: return WValue.string(`s${i}`);
    case 4: return WValue.symbol(`sym${i}`);
    case 5: return WValue.tagged("md", `# ${i}`);
    case 6: return WValue.range(i, i + 10);
    default: return WValue.measurement("kg", i);
  }
}
// A general map written in JavaScript against the handle API: every field read and every
// new value is a boundary crossing.
function mapOneW(v: any) {
  const k = v.kind();
  let out;
  switch (k) {
    case 0: out = WValue.integer(BigInt(v.number() + 1)); break;
    case 1: out = WValue.decimal(v.number() * 2); break;
    case 2: out = WValue.boolean(v.number() === 0); break;
    case 3: out = WValue.string(v.text() + "!"); break;
    case 4: out = WValue.symbol(v.text()); break;
    case 5: out = WValue.tagged(v.text(), v.text2() + "!"); break;
    case 6: out = WValue.range(v.number() + 1, v.number2() + 1); break;
    default: out = WValue.measurement(v.text(), v.number() * 2); break;
  }
  v.free();
  return out;
}
function readW(v: any): number {
  const k = v.kind();
  switch (k) {
    case 3: case 4: return v.text().length;
    case 5: return v.text2().length;
    case 6: return v.number() + v.number2();
    default: return v.number();
  }
}

type Segs = Record<string, number>;
const now = () => performance.now();
function median(a: number[]) { const s = [...a].sort((x, y) => x - y); return s[Math.floor(s.length / 2)]; }

function bench(name: string, n: number, runs: number, once: (n: number) => Segs) {
  const acc: Record<string, number[]> = {};
  let sink = 0;
  for (let r = 0; r < runs + 3; r++) {
    const segs = once(n);
    sink += segs.sink ?? 0;
    if (r >= 3) for (const [k, v] of Object.entries(segs)) if (k !== "sink") (acc[k] ??= []).push(v);
  }
  const m: Record<string, number> = {};
  for (const [k, v] of Object.entries(acc)) m[k] = median(v);
  const pipe = (m.create ?? 0) + (m.collect ?? 0) + (m.map ?? 0) + (m.read ?? 0);
  const fmt = Object.entries(m).map(([k, v]) => `${k} ${v.toFixed(4)} ms`).join(", ");
  console.log(`${name} n=${n}: ${fmt}, pipeline(create+collect+map+read) ${pipe.toFixed(4)} ms [${((pipe * 1000) / n).toFixed(3)} us/value] (sink ${Math.round(sink)})`);
}

function nativeRun(n: number): Segs {
  let t = now();
  const vals: Value[] = [];
  for (let i = 0; i < n; i++) vals.push(gen(i));
  const create = now() - t;
  t = now();
  const list: Value = { type: "array", items: vals };
  const collect = now() - t;
  t = now();
  const mapped = mapOne(list);
  const map = now() - t;
  t = now();
  const s = checksum(mapped);
  const read = now() - t;
  t = now();
  const txt = toGram(mapped);
  const text = now() - t;
  return { create, collect, map, read, text, sink: s + txt.length };
}

function wasmHandlesRun(n: number, general: boolean): Segs {
  let t = now();
  const vals: any[] = [];
  for (let i = 0; i < n; i++) vals.push(genW(i));
  const create = now() - t;
  t = now();
  const list = new WList();
  for (const v of vals) { list.push(v); v.free(); }
  const collect = now() - t;
  t = now();
  const mapped = general ? list.map_with(mapOneW) : list.map_builtin();
  const map = now() - t;
  t = now();
  let s = 0;
  const len = mapped.len();
  for (let i = 0; i < len; i++) { const v = mapped.get(i); s += readW(v); v.free(); }
  const read = now() - t;
  t = now();
  const txt = mapped.to_gram();
  const text = now() - t;
  list.free(); mapped.free();
  return { create, collect, map, read, text, sink: s + txt.length };
}

function wasmJsonRun(n: number): Segs {
  let t = now();
  const vals: Value[] = [];
  for (let i = 0; i < n; i++) vals.push(gen(i));
  const create = now() - t;
  t = now();
  const json = JSON.stringify({ type: "array", items: vals });
  const collect = now() - t;
  t = now();
  const out = wasm.map_json(json);
  const map = now() - t;
  t = now();
  const mapped = JSON.parse(out) as Value;
  const s = checksum(mapped);
  const read = now() - t;
  t = now();
  const txt = toGram(mapped);
  const text = now() - t;
  return { create, collect, map, read, text, sink: s + txt.length };
}

for (const [n, runs] of [[1000, 200], [100000, 12]] as const) {
  bench("ts-native", n, runs, nativeRun);
  bench("ts-wasm-handles (callback map)", n, runs, (k) => wasmHandlesRun(k, true));
  bench("ts-wasm-handles (builtin map)", n, runs, (k) => wasmHandlesRun(k, false));
  bench("ts-wasm-json", n, runs, wasmJsonRun);
}
