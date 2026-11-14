// SPIKE-002: the cost of one call across the WebAssembly boundary, against the same work natively.
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const t0 = performance.now();
const wasm = require("./pkg/wasm_bench.js");
console.log(`load + instantiate the wasm package: ${(performance.now() - t0).toFixed(2)} ms`);
const { WValue } = wasm;
const N = 1_000_000;
function time(label: string, n: number, f: () => number) {
  f(); // warm up
  const best: number[] = [];
  for (let r = 0; r < 7; r++) { const t = performance.now(); const s = f(); best.push(performance.now() - t); if (s === -1) console.log(s); }
  best.sort((a, b) => a - b);
  console.log(`${label}: ${((best[3] * 1e6) / n).toFixed(1)} ns per call`);
}
const native = { type: "integer", value: 42 } as { type: string; value: number };
time("native property read (obj.value)", N, () => { let s = 0; for (let i = 0; i < N; i++) s += native.value; return s; });
const h = WValue.integer(42n);
time("wasm handle read: v.number()", N, () => { let s = 0; for (let i = 0; i < N; i++) s += h.number(); return s; });
time("wasm handle read: v.kind()", N, () => { let s = 0; for (let i = 0; i < N; i++) s += h.kind(); return s; });
const hs = WValue.string("hello world");
time("wasm handle read of a string: v.text()", N, () => { let s = 0; for (let i = 0; i < N; i++) s += hs.text().length; return s; });
const M = 200_000;
time("native create an integer object", M, () => { let s = 0; for (let i = 0; i < M; i++) s += ({ type: "integer", value: i }).value; return s; });
time("wasm create an integer handle and free it", M, () => { let s = 0; for (let i = 0; i < M; i++) { const v = WValue.integer(BigInt(i)); s += 1; v.free(); } return s; });
time("wasm create a decimal handle and free it", M, () => { let s = 0; for (let i = 0; i < M; i++) { const v = WValue.decimal(i * 0.5); s += 1; v.free(); } return s; });
