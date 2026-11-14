#!/usr/bin/env bash
# SPIKE-001 probe. Disposable. Runs one gram input per value kind through relate-gram's `normalize`
# (load into a scratch in-memory database, write back). Usage: relate-gram-probe.sh path/to/relate-gram
set -u
BIN="${1:?path to the relate-gram binary}"
run() {
  local label="$1" text="$2"
  printf '== %s: %s\n' "$label" "${text//$'\n'/\\n}"
  out=$(printf '%s\n' "$text" | "$BIN" normalize - 2>&1) ; code=$?
  printf '  exit %s\n' "$code"
  printf '%s\n' "$out" | sed -n '1,6p' | sed 's/^/  | /'
}
run "integer 0" '(n {k: 0})'
run "integer -7" '(n {k: -7})'
run "integer i64 max" '(n {k: 9223372036854775807})'
run "integer past i64" '(n {k: 9223372036854775808})'
run "hexadecimal" '(n {k: 0x1F})'
run "octal" '(n {k: 010})'
run "decimal" '(n {k: 3.14})'
run "decimal 1.0" '(n {k: 1.0})'
run "decimal negative" '(n {k: -0.5})'
run "measurement 5kg" '(n {k: 5kg})'
run "measurement 5.0kg" '(n {k: 5.0kg})'
run "measurement -5m" '(n {k: -5m})'
run "measurement 3.5kg" '(n {k: 3.5kg})'
run "range 1..5" '(n {k: 1..5})'
run "range open upper" '(n {k: 1...})'
run "range open lower" '(n {k: ...5})'
run "range decimals" '(n {k: 1.5..2.5})'
run "range measurements" '(n {k: 1.5m..2m})'
run "range hex" '(n {k: 0x1..0x10})'
run "boolean" '(n {k: true})'
run "symbol" '(n {k: active})'
run "symbol with punctuation" '(n {k: a.b-c@d})'
run "string double" '(n {k: "dq"})'
run "string single" "(n {k: 'sq'})"
run "string backtick" '(n {k: `bt`})'
run "string escapes" '(n {k: "a\n\t\\\"b"})'
run "string fenced" $'(n {k: ```\nline1\nline2```})'
run "string empty" '(n {k: ""})'
run "tagged backtick" '(n {k: md`# T`})'
run "tagged fenced" $'(n {k: ```md\nbody```})'
run "tagged date" '(n {k: date`2024-01-15`})'
run "array empty" '(n {k: []})'
run "array integers" '(n {k: [1, 2, 3]})'
run "array mixed" '(n {k: [1, "a", true]})'
run "array ranges and measurements" '(n {k: [1..2, 5kg]})'
run "array of arrays" '(n {k: [[1]]})'
run "map" '(n {k: {b: 1, a: 2}})'
run "map with array" '(n {k: {a: [1, 2]}})'
run "map repeated key" '(n {k: {a: 1, a: 2}})'
run "map nested" '(n {k: {a: {b: 1}}})'
run "record repeated key" '(n {k: 1, k: 2})'
run "record key order" '(n {z: 1, y: 2, x: 3, w: 4})'
