"""Runs the language-neutral conformance corpus (ADR-001) against this package.

The corpus is gram text. Each case's ``value`` is read with tree-sitter-gram and lowered to a
``Value`` by the functions below, which live in test code and are not shipped (ADR-001 2.3).
"""

from __future__ import annotations

import json
from collections.abc import Iterator
from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path

import tree_sitter_gram
from tree_sitter import Language, Node, Parser

import gram_value as gv
from gram_value import Value

CORPUS = Path(__file__).resolve().parents[2] / "conformance"
GRAMMAR_VERSION = "0.7.0"

_parser = Parser(Language(tree_sitter_gram.language()))


def parse(src: str) -> Node:
    return _parser.parse(src.encode("utf-8")).root_node


def text(n: Node) -> str:
    assert n.text is not None
    return n.text.decode("utf-8")


def walk(root: Node) -> Iterator[Node]:
    stack = [root]
    while stack:
        n = stack.pop()
        yield n
        stack.extend(n.children)


# ---- lowering -------------------------------------------------------------------------------

_TABLE = {"b": "\b", "f": "\f", "n": "\n", "r": "\r", "t": "\t", "/": "/", "\\": "\\"}


def content(n: Node) -> str:
    c = n.child_by_field_name("content")
    return text(c) if c is not None else ""


def unescape(s: str, quote: str) -> str:
    out: list[str] = []
    it = iter(s)
    for c in it:
        if c != "\\":
            out.append(c)
            continue
        e = next(it)
        if e in _TABLE:
            out.append(_TABLE[e])
        elif e == quote:
            out.append(e)
        else:
            raise AssertionError(f"escape the grammar does not allow: {e}")
    return "".join(out)


def string_value(n: Node) -> str:
    full = text(n)
    return content(n) if full.startswith("```") else unescape(content(n), full[0])


def lower_number_text(t: str) -> Value:
    if "." in t:
        return gv.decimal_from_text(t).unwrap()
    return gv.integer_from_digits("0" if t == "-0" else t).unwrap()


def lower_numeric(n: Node) -> Value:
    t = text(n)
    if n.type in ("integer", "decimal"):
        return lower_number_text(t)
    if n.type == "hexadecimal":
        return gv.integer(int(t, 16))
    if n.type == "octal":
        return gv.integer(int(t, 8))
    if n.type == "measurement":
        split = next(i for i, c in enumerate(t) if c.isalpha())
        return gv.measurement(lower_number_text(t[:split]), t[split:]).unwrap()
    raise AssertionError(f"not a numeric literal: {n.type}")


def key_name(n: Node) -> str:
    if n.type == "quoted_name":
        return unescape(content(n), "`")
    if n.type == "double_quoted_name":
        return unescape(content(n), '"')
    return text(n)


def named(n: Node) -> list[Node]:
    return [c for c in n.named_children if c.type not in ("comment", "doc_comment")]


def field(n: Node, name: str) -> Node:
    c = n.child_by_field_name(name)
    assert c is not None, f"missing field {name} on {n.type}"
    return c


def lower(n: Node) -> Value:
    t = n.type
    if t in ("integer", "decimal", "hexadecimal", "octal", "measurement"):
        return lower_numeric(n)
    if t == "boolean_literal":
        return gv.boolean(text(n) == "true")
    if t == "symbol":
        return gv.string(text(n)).unwrap()
    if t == "string_literal":
        return gv.string(string_value(n)).unwrap()
    if t == "tagged_string":
        body = content(n) if text(n).startswith("```") else unescape(content(n), "`")
        return gv.tagged(text(field(n, "tag")), body).unwrap()
    if t == "range":
        lo, hi = n.child_by_field_name("lower"), n.child_by_field_name("upper")
        return gv.range_(
            lower_numeric(lo) if lo is not None else None,
            lower_numeric(hi) if hi is not None else None,
        ).unwrap()
    if t == "array":
        return gv.array(lower(c) for c in named(n))
    if t == "map":
        return gv.map_((key_name(field(e, "key")), lower(field(e, "value"))) for e in named(n)).unwrap()
    raise AssertionError(f"cannot lower node kind {t}")


# ---- cases ----------------------------------------------------------------------------------


@dataclass
class Case:
    id: str
    label: str
    props: dict[str, Value]
    value: Value


def load_cases(file: str) -> list[Case]:
    return load_nodes(CORPUS / file)


def load_skips() -> dict[str, str]:
    """The package's skip list (ADR-001 2.6): case id to reason."""
    skips: dict[str, str] = {}
    for node in load_nodes(Path(__file__).resolve().parents[1] / "skips.gram"):
        assert node.label == "Skip", f"{node.id}: a skips file holds Skip nodes"
        skips[prop_str(node, "case")] = prop_str(node, "reason")
    return skips


def load_nodes(path: Path) -> list[Case]:
    root = parse(path.read_text(encoding="utf-8"))
    assert not root.has_error, f"{path} has a syntax error"
    cases: list[Case] = []
    for n in walk(root):
        if n.type != "node_pattern":
            continue
        ident = n.child_by_field_name("identifier")
        labels = n.child_by_field_name("labels")
        record = n.child_by_field_name("record")
        if ident is None or labels is None or record is None:
            continue
        props = {key_name(field(p, "key")): lower(field(p, "value")) for p in named(record)}
        cases.append(Case(text(ident), text(named(labels)[0]), props, props.get("value", gv.boolean(False))))
    return sorted(cases, key=lambda c: c.id)


def vocabulary() -> dict[str, set[str]]:
    root = parse((CORPUS / "vocabulary.gram").read_text(encoding="utf-8"))
    assert not root.has_error, "vocabulary.gram has a syntax error"
    vocab: dict[str, set[str]] = {}
    for n in walk(root):
        if n.type != "node_pattern":
            continue
        ident, labels = n.child_by_field_name("identifier"), n.child_by_field_name("labels")
        if ident is None or labels is None:
            continue
        for lab in named(labels):
            vocab.setdefault(text(lab), set()).add(text(ident))
    return vocab


VOCAB = vocabulary()


def check_vocab(group: str, name: str, case_id: str) -> None:
    assert name in VOCAB.get(group, set()), f"case {case_id}: {name!r} is not in the vocabulary group {group}"


def prop_str(c: Case, key: str) -> str:
    v = c.props.get(key)
    assert isinstance(v, gv.StringValue), f"case {c.id}: property {key} should be a string"
    return v.value


def text_of(value: Value) -> str:
    r = gv.to_text(value)
    assert r.ok, f"unwritable: {r.diagnostics}"
    return r.value


def load_value_from_text(t: str) -> Value:
    root = parse(f"(x {{value: {t}}})")
    assert not root.has_error, f"canonical text {t} does not parse"
    for n in walk(root):
        if n.type == "record_property":
            return lower(field(n, "value"))
    raise AssertionError("no value")


TAG_FOR_NATIVE = {
    "Date": "date",
    "Time": "time",
    "DateTime": "datetime",
    "Duration": "duration",
    "Point": "point",
    "Uuid": "uuid",
    "Week": "week",
}


def native_type_name(n: object) -> str:
    # bool is checked before int: bool is an int subclass.
    for typ, name in (
        (bool, "bool"),
        (int, "int"),
        (float, "float"),
        (Decimal, "Decimal"),
        (str, "str"),
        (gv.Tagged, "Tagged"),
        (gv.Range, "Range"),
        (gv.Measurement, "Measurement"),
        (list, "list"),
        (dict, "dict"),
    ):
        if isinstance(n, typ):
            return name
    raise AssertionError(f"unexpected native type {type(n)}")


def run_mapping(c: Case) -> None:
    for key in c.props:
        if key not in ("name", "value"):
            check_vocab("Language", key, c.id)
    entry = c.props.get("python")
    if entry is None:
        return
    assert isinstance(entry, gv.MapValue)

    def word(k: str) -> str | None:
        x = entry.get(k)
        return x.value if isinstance(x, gv.StringValue) else None

    cls, native = word("class"), word("native")
    assert cls is not None and native is not None
    check_vocab("Class", cls, c.id)
    check_vocab("PythonNative", native, c.id)
    loss = entry.get("loss")
    want_loss = (
        [x.value for x in loss.items if isinstance(x, gv.StringValue)]
        if isinstance(loss, gv.ArrayValue)
        else []
    )
    for item in want_loss:
        check_vocab("Loss", item, c.id)
    assert (cls == "lossy") == bool(want_loss), "loss kinds are present exactly when the class is lossy"

    if cls == "unsupported":
        assert native == "none"
        assert not gv.to_native(c.value).ok
        return
    losses: list[str] = []
    if native in TAG_FOR_NATIVE:
        r = gv.interpret(c.value)
        assert r.ok and r.value is not None, "a known tag interprets"
        assert r.value.tag == TAG_FOR_NATIVE[native]
        back = r.value.to_value()
    else:
        nr = gv.to_native(c.value)
        assert nr.ok, f"conversion failed: {nr.diagnostics}"
        assert native_type_name(nr.value) == native, "native type"
        losses = sorted({d.code for d in nr.diagnostics})
        back = gv.from_native(nr.value).unwrap()
    assert losses == sorted(want_loss), "loss kinds"
    assert text_of(back) == word("roundtrip"), f"round trip through {native}"
    if cls == "exact":
        assert gv.equals(back, c.value), "exact means the round trip gives an equal value"
    else:
        assert not gv.equals(back, c.value), "lossy means the round trip changes the value"


def run_case(c: Case) -> None:
    if c.label == "ValueCase":
        kind = prop_str(c, "kind")
        check_vocab("Kind", kind, c.id)
        assert c.value.kind == kind, "wrong kind"
    elif c.label == "TextCase":
        got = text_of(c.value)
        assert got == prop_str(c, "canonical"), "canonical text"
        assert gv.equals(load_value_from_text(got), c.value), f"canonical text {got} reads back differently"
    elif c.label == "TagCase":
        outcome = prop_str(c, "outcome")
        check_vocab("Outcome", outcome, c.id)
        assert isinstance(c.value, gv.TaggedValue), "a TagCase value is a tagged string"
        check_vocab("Tag", c.value.tag, c.id)
        r = gv.interpret(c.value)
        if outcome == "accepted":
            assert r.ok and r.value is not None, f"expected accepted, got {r.diagnostics}"
            assert r.value.normal_content == prop_str(c, "interpreted"), "normal content"
        else:
            assert not r.ok, "expected rejected"
            want = prop_str(c, "diagnostic")
            check_vocab("Diagnostic", want, c.id)
            assert r.diagnostics[0].code == want, f"diagnostic code ({r.diagnostics[0].message})"
    elif c.label == "MappingCase":
        run_mapping(c)
    else:
        raise AssertionError(f"unknown case kind {c.label}")


def test_corpus() -> None:
    assert (CORPUS / "GRAMMAR_VERSION").read_text().strip() == GRAMMAR_VERSION
    seen: set[str] = set()
    failures: list[str] = []
    results: list[str] = []
    skips = load_skips()
    counts: dict[str, int] = {}
    for file in ("values.gram", "text.gram", "tags.gram", "mappings.gram"):
        for c in load_cases(file):
            assert c.id not in seen, f"case id {c.id} is used twice"
            seen.add(c.id)
            # A mapping case with no entry for this language does not apply to it (ADR-001 2.4).
            if c.label == "MappingCase" and "python" not in c.props:
                counts["not-applicable"] = counts.get("not-applicable", 0) + 1
                results.append(f"({c.id}:Result {{language: python, status: not-applicable}})")
                continue
            error: str | None = None
            try:
                run_case(c)
            except AssertionError as e:
                error = str(e)
            extra = ""
            reason = skips.get(c.id)
            if error is None and reason is None:
                status = "passed"
            elif error is None:
                status = "failed"
                failures.append(f"{c.id}: unexpected pass of a skipped case")
            elif reason is not None:
                status = "skipped"
                extra = ", reason: " + json.dumps(reason)
            else:
                status = "failed"
                failures.append(f"{c.id}: {error}")
            counts[status] = counts.get(status, 0) + 1
            results.append(f"({c.id}:Result {{language: python, status: {status}{extra}}})")
    for skipped_id in skips:
        assert skipped_id in seen, f"skips.gram names an unknown case: {skipped_id}"
    out = Path(__file__).resolve().parents[1] / "target"
    out.mkdir(exist_ok=True)
    (out / "conformance-results.gram").write_text("\n".join(results) + "\n")
    assert not failures, f"{len(failures)} failed of {len(seen)}:\n" + "\n".join(failures)
    assert len(seen) > 100
    print(f"conformance: {counts}")
