#!/usr/bin/env python3
"""Generate docs/mappings/<language>.md from conformance/mappings.gram (ADR-006 2.9).

Each MappingCase must be written on one line, as in the existing file. Run with --check to fail
when a generated file is stale (used by CI).
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CASE = re.compile(r'^\((?P<id>[\w-]+):MappingCase \{name: (?P<name>"(?:[^"\\]|\\.)*"), value: (?P<rest>.*)\}\)$')
LANGS = {"rust": "Rust", "typescript": "TypeScript", "python": "Python", "go": "Go"}


def unquote(s):
    return re.sub(r'\\(.)', r'\1', s[1:-1])


def scan_value(s, i):
    """Return (text, next index) of one value starting at s[i]."""
    if s[i] == '"':
        j = i + 1
        while s[j] != '"':
            j += 2 if s[j] == '\\' else 1
        return s[i:j + 1], j + 1
    if s[i] == '[':
        j = s.index(']', i)
        return s[i:j + 1], j + 1
    j = i
    while j < len(s) and s[j] not in ',}':
        j += 1
    return s[i:j], j


def parse_entry(body):
    out, i = {}, 0
    while i < len(body):
        m = re.compile(r'\s*(\w+):\s*').match(body, i)
        if not m:
            break
        key = m.group(1)
        val, i = scan_value(body, m.end())
        out[key] = val.strip()
        while i < len(body) and body[i] in ', ':
            i += 1
    return out


def parse_case(line):
    m = CASE.match(line)
    if not m:
        raise SystemExit(f"cannot read MappingCase line: {line[:80]}")
    rest = m.group("rest")
    # The value is everything up to the first ", <language>: {".
    lang_at = re.search(r', (rust|typescript|python|go): \{', rest)
    value = rest[:lang_at.start()]
    entries, i = {}, lang_at.start()
    while True:
        lm = re.compile(r',\s*(rust|typescript|python|go): \{').match(rest, i)
        if not lm:
            break
        depth, j, in_str = 1, lm.end(), False
        while depth:
            c = rest[j]
            if in_str:
                if c == '\\':
                    j += 1
                elif c == '"':
                    in_str = False
            elif c == '"':
                in_str = True
            elif c == '{':
                depth += 1
            elif c == '}':
                depth -= 1
            j += 1
        entries[lm.group(1)] = parse_entry(rest[lm.end():j - 1])
        i = j
    return m.group("id"), unquote(m.group("name")), value, entries


def cell(s):
    return s.replace("|", "\\|")


def render(lang, cases):
    title = LANGS[lang]
    lines = [
        f"# {title} mapping",
        "",
        "Generated from `conformance/mappings.gram` by `scripts/gen-mapping-docs.py`; do not edit.",
        "Each row is a conformance case: the gram value, the entry class, the native type, the loss kinds",
        "and the gram text the native value writes back as (ADR-006).",
        "",
        "| Case | Value | Class | Native | Loss | Round trip |",
        "|---|---|---|---|---|---|",
    ]
    for cid, name, value, entries in cases:
        e = entries.get(lang)
        if not e:
            continue
        loss = e.get("loss", "[]").strip("[]").strip() or "none"
        rt = unquote(e["roundtrip"]) if "roundtrip" in e else "none"
        lines.append(
            f"| {cid}: {cell(name)} | `{cell(value)}` | {e['class']} | {e['native']} | {loss} | `{cell(rt)}` |"
            if rt != "none"
            else f"| {cid}: {cell(name)} | `{cell(value)}` | {e['class']} | {e['native']} | {loss} | none |"
        )
    return "\n".join(lines) + "\n"


def main():
    check = "--check" in sys.argv
    cases = []
    for line in (ROOT / "conformance" / "mappings.gram").read_text().splitlines():
        if line.startswith("(") and ":MappingCase" in line:
            cases.append(parse_case(line))
    stale = []
    for lang in LANGS:
        if not any(lang in c[3] for c in cases):
            continue
        out = ROOT / "docs" / "mappings" / f"{lang}.md"
        text = render(lang, cases)
        if check:
            if not out.exists() or out.read_text() != text:
                stale.append(str(out.relative_to(ROOT)))
        else:
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_text(text)
    if stale:
        print("stale generated files (run scripts/gen-mapping-docs.py): " + ", ".join(stale))
        sys.exit(1)


main()
