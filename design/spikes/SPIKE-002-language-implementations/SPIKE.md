---
id: SPIKE-002
title: Native Implementations Versus a Shared Core, per Language
status: done
rfcs: [RFC-001]
created: 2026-11-14
---

# SPIKE-002: Native Implementations Versus a Shared Core, per Language

## 1. Question

RFC-001 sections 5.7 and 7.11: for each first-tier language, is a native implementation of the value model a better experience than a binding to a shared Rust core, and what does a user pay to cross the boundary? Crossing a language boundary is expected to be costly, WebAssembly most of all, so the spike measures it on simple operations a consumer would do all the time: create 1,000 values, add them to a list, map the list to new values, and read the result.

## 2. Method

A prototype value model of ten kinds (`scripts/core`, Rust) stands in for the shared core; the benchmark uses eight scalar kinds in rotation, collected into an array. Each language gets the same operations: **create** 1,000 values, **collect** them into a list, **map** the list to a new list (integers plus one, decimals doubled, booleans negated, strings and tagged content suffixed, ranges shifted, measurements doubled), **read** every element in the host (a checksum over all payloads, so every value is touched), and **text**, writing the list as gram text. Every candidate computes the same checksum, which is compared across candidates.

Candidates:

| Language | Candidate | What it is |
|---|---|---|
| Rust | native | the core's own enum; no boundary |
| TypeScript | native | plain objects, discriminated union |
| TypeScript | wasm handles, callback map | the Rust core compiled with `wasm-bindgen`; JavaScript holds opaque handles; the map callback runs in JavaScript, so every field read and every new value crosses the boundary |
| TypeScript | wasm handles, builtin map | the same handles; the map runs inside the core (one crossing), a lower bound for any handle design |
| TypeScript | wasm JSON | the pattern-rs approach: JSON text in, JSON text out, objects on both sides |
| Python | native | `dataclass(slots=True)` with `match` |
| Python | PyO3 handles (callback and builtin map) | the core as an extension module; Python holds objects wrapping Rust values |
| Python | PyO3 JSON | JSON text in and out |

Each is timed at n = 1,000 (median of 200 runs after 3 warm-up runs) and n = 100,000 (median of 10 to 12 runs). A second micro-benchmark times single calls across each boundary (`scripts/ts/crossing.ts`, `scripts/python/crossing.py`). Outputs are `bench-*-output.txt` and `*-crossing-output.txt`. Build and run commands are in section 4.

Machine: Apple M5 Max, macOS 26.7.3, Rust 1.96.0, Node 24.14.1, Python 3.14.5, `wasm-bindgen` 0.2.128 (the module optimized with `wasm-opt -O3`), PyO3 0.30. Browsers, bundlers, Node native addons, Go, Swift and Haskell were not measured.

## 3. Time-box

One day.

## 4. Notes

2026-11-14
- Rebuild: `cd scripts && cargo build --release -p wasm-bench --target wasm32-unknown-unknown`, then `wasm-bindgen --target nodejs --out-dir ts/pkg target/wasm32-unknown-unknown/release/wasm_bench.wasm` and `wasm-opt -O3` on the result (with the feature flags the toolchain needs); `cargo build --release -p pybench` and copy `target/release/libpybench.dylib` to `python/pybench.so`. Run `node ts/bench.ts` (Node 24 strips the types), `python3 python/bench.py`, `cargo run --release -p rust-bench`.
- The checksum agrees across all candidates at n = 1,000 (78,596,525). At n = 100,000 it differs between languages only because the run counts differ.
- The wasm handle benchmark frees every handle (`.free()`); without that the handles leak (see findings).
- A 64-bit integer crosses to JavaScript as a `BigInt`, which the handle API pays on every integer; the JSON candidate sends a number, so a 64-bit value beyond 2^53 would be damaged there.

## 5. Findings

### 5.1 The same pipeline, per value

Pipeline time is create + collect + map + read (the text step is reported separately). Ratio is against the native candidate in the same language.

| Candidate | n = 1,000 | per value | ratio | n = 100,000 | per value | ratio |
|---|---|---|---|---|---|---|
| Rust native | 0.042 ms | 0.042 us | | 2.3 ms | 0.023 us | |
| TypeScript native | 0.024 ms | 0.024 us | 1x | 3.0 ms | 0.030 us | 1x |
| TypeScript, wasm handles, callback map | 0.613 ms | 0.613 us | 25x | 67.1 ms | 0.671 us | 22x |
| TypeScript, wasm handles, builtin map | 0.291 ms | 0.291 us | 12x | 32.1 ms | 0.321 us | 11x |
| TypeScript, wasm JSON | 0.378 ms | 0.378 us | 16x | 38.2 ms | 0.382 us | 13x |
| Python native | 0.521 ms | 0.521 us | 1x | 51.1 ms | 0.511 us | 1x |
| Python, PyO3 handles, callback map | 0.425 ms | 0.425 us | 0.8x | 44.4 ms | 0.444 us | 0.9x |
| Python, PyO3 handles, builtin map | 0.253 ms | 0.253 us | 0.5x | 27.7 ms | 0.277 us | 0.5x |
| Python, PyO3 JSON | 1.108 ms | 1.108 us | 2.1x | 124.7 ms | 1.247 us | 2.4x |

Writing the list as gram text (n = 1,000): Rust 0.077 ms, TypeScript native 0.030 ms, wasm 0.054 ms (1.8x slower than native TypeScript); Python native 0.230 ms, PyO3 0.051 ms (4.5x faster than native Python).

### 5.2 One call across the boundary

| Operation | Native | Across the boundary |
|---|---|---|
| TypeScript: read a number from a value | 0.4 ns | 2.5 ns (wasm handle) |
| TypeScript: read a string from a value | not measured separately | 68.5 ns (the string is copied out) |
| TypeScript: create an integer value | 0.5 ns | 120 ns (handle plus `BigInt`, and free) |
| TypeScript: create a decimal value | | 109 ns (handle and free) |
| Python: read a number from a value | 12.4 ns | 18.4 ns (PyO3) |
| Python: read a string from a value | | 44.7 ns |
| Python: create an integer value | 35.2 ns | 48.3 ns |

The wasm package loads in 0.8 ms and the PyO3 module imports in 0.6 ms. The prototype wasm module, with the JSON dependency, is 221 KB, 182 KB optimized and 70 KB gzipped, and grows with a real core. JSON text for 1,000 values is 37 KB (about 37 bytes each).

### 5.3 What the numbers and the build say

1. **TypeScript: the shared core in WebAssembly is 11 to 25 times slower per value than native objects**, even when the work inside the core is a single call (12x, the best a handle design can do here). The JSON route is 13 to 16 times slower. Absolute costs are small, below one microsecond a value, so 1,000 values cost well under a millisecond; at 100,000 values the gap is 3 ms against 32 to 67 ms.
2. **The slowness is the boundary, not the work.** Reading a number across is cheap (2.5 ns). What costs is creating and freeing handles (110 to 120 ns each), copying strings (68 ns), and calling back into JavaScript for a general map (the callback map is 2x the single-crossing map).
3. **Wasm handles also change the programming model**: a 64-bit integer is a `BigInt`; handles must be freed by hand (or by an opt-in finalizer) or they leak; every element read is a call; a callback map is slower than a bulk operation inside the core, which a general consumer cannot always use; and a browser needs asynchronous initialization and bundler support, which this spike did not measure.
4. **Python: the boundary is cheap.** A PyO3 call costs 13 to 48 ns, close to a Python attribute read or an object creation, so the core as handles is as fast or faster than native dataclasses (0.5x to 0.9x the pipeline time), and much faster for work done in the core (text writing 4.5x). The JSON route is the slow one (2.1x to 2.4x native).
5. **The cost of Python's native route is Python itself.** Native Python is 22x slower than native TypeScript on this workload, so a shared core is competitive there.
6. **Distribution differs from speed.** A PyO3 module must be shipped as a compiled wheel for every operating system, architecture and Python version (or one `abi3` wheel per platform); a native Python model is one pure-Python wheel. A WebAssembly module is one file but has the browser concerns above. Neither was built for release here.
7. **Logic that must match across languages is the cost of going native.** Text writing and the known-tag validators (about 800 lines of Rust in relate-gram's `typed/`) would be ported to each native language; the conformance corpus and perhaps a generator (RFC-001 6.7) are what keep those ports in step.

### 5.4 Not measured

Browsers and bundlers; Node native addons (an N-API route) as a third TypeScript candidate; memory use; the real model with maps and known tags; parse cost (not this project's); wheel and npm packaging effort; Go, Swift and Haskell (next tier); how the results change on another machine or Node and Python versions.

## 6. Conclusion

The data do not support one Rust core for every language. They support deciding per language:

- **Rust:** the core, natively.
- **TypeScript:** a native implementation. A WebAssembly core is 11 to 25 times slower per value and adds `BigInt`, manual freeing and browser initialization burdens, confirming the expectation that the experience is poor. The native model's weakness is semantic (a JavaScript number holds 53 bits), which belongs in the mapping tables (RFC-001 3.4).
- **Python:** open. A PyO3 core is as fast or faster than a native model and gives identical logic to the Rust tier, so the choice depends on the packaging burden (platform wheels) that this spike did not measure, and on how much shared logic (text writing, tag validation) the Python tier would otherwise duplicate.

Consequences for RFC-001: parity cannot rely on shared code in TypeScript, so it rests on the conformance corpus (5.6), and a generator or shared test vectors (6.7) matter more. Text for RFC-001 question 7.11: "Answered for TypeScript by SPIKE-002: a native implementation (WebAssembly was 11 to 25 times slower per value and adds BigInt, freeing and initialization costs). Python remains open between native and a PyO3 core, which is as fast as native and faster for work done in the core; the deciding factor is wheel distribution."
