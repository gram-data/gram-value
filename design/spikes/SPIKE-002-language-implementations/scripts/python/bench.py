"""SPIKE-002: Python candidates. Native dataclasses, and the Rust core as a PyO3 extension with objects and with JSON.

Run: python3 bench.py   (after `cargo build --release -p pybench` and copying
target/release/libpybench.dylib to python/pybench.so; see ../../SPIKE.md)
"""
import json
import statistics
import sys
import time
from dataclasses import dataclass
from typing import Optional

sys.path.insert(0, __file__.rsplit("/", 1)[0])
import pybench  # noqa: E402


@dataclass(slots=True)
class Integer:
    value: int


@dataclass(slots=True)
class Decimal:
    value: float


@dataclass(slots=True)
class Boolean:
    value: bool


@dataclass(slots=True)
class String:
    value: str


@dataclass(slots=True)
class Symbol:
    value: str


@dataclass(slots=True)
class Tagged:
    tag: str
    content: str


@dataclass(slots=True)
class Range:
    lower: Optional[float]
    upper: Optional[float]


@dataclass(slots=True)
class Measurement:
    unit: str
    value: float


@dataclass(slots=True)
class Array:
    items: list


def gen(i):
    k = i % 8
    if k == 0:
        return Integer(i)
    if k == 1:
        return Decimal(i * 0.5)
    if k == 2:
        return Boolean(i % 2 == 0)
    if k == 3:
        return String(f"s{i}")
    if k == 4:
        return Symbol(f"sym{i}")
    if k == 5:
        return Tagged("md", f"# {i}")
    if k == 6:
        return Range(float(i), float(i) + 10.0)
    return Measurement("kg", float(i))


def map_one(v):
    match v:
        case Integer(x):
            return Integer(x + 1)
        case Decimal(x):
            return Decimal(x * 2.0)
        case Boolean(x):
            return Boolean(not x)
        case String(x):
            return String(x + "!")
        case Symbol(x):
            return Symbol(x)
        case Tagged(tag, c):
            return Tagged(tag, c + "!")
        case Range(lo, up):
            return Range(None if lo is None else lo + 1.0, None if up is None else up + 1.0)
        case Measurement(u, x):
            return Measurement(u, x * 2.0)
        case Array(items):
            return Array([map_one(x) for x in items])


def checksum(v):
    match v:
        case Integer(x) | Decimal(x) | Measurement(_, x):
            return float(x)
        case Boolean(x):
            return 1.0 if x else 0.0
        case String(x) | Symbol(x):
            return float(len(x))
        case Tagged(_, c):
            return float(len(c))
        case Range(lo, up):
            return (lo or 0.0) + (up or 0.0)
        case Array(items):
            return sum(checksum(x) for x in items)


def num(f):
    return f"{f:.1f}" if f == int(f) else repr(f)


def to_gram(v):
    match v:
        case Integer(x):
            return str(x)
        case Decimal(x):
            return num(x)
        case Boolean(x):
            return "true" if x else "false"
        case String(x):
            return '"' + x.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'
        case Symbol(x):
            return x
        case Tagged(tag, c):
            return f"{tag}`{c}`"
        case Range(lo, up):
            return f"{int(lo)}..{int(up)}" if lo is not None and up is not None else "..."
        case Measurement(u, x):
            return f"{int(x)}{u}"
        case Array(items):
            return "[" + ", ".join(to_gram(x) for x in items) + "]"


# --- the PyO3 handle API ---------------------------------------------------------------
PV, PL = pybench.PValue, pybench.PList


def gen_p(i):
    k = i % 8
    if k == 0:
        return PV.integer(i)
    if k == 1:
        return PV.decimal(i * 0.5)
    if k == 2:
        return PV.boolean(i % 2 == 0)
    if k == 3:
        return PV.string(f"s{i}")
    if k == 4:
        return PV.symbol(f"sym{i}")
    if k == 5:
        return PV.tagged("md", f"# {i}")
    if k == 6:
        return PV.range(float(i), float(i) + 10.0)
    return PV.measurement("kg", float(i))


def map_one_p(v):
    k = v.kind()
    if k == 0:
        return PV.integer(int(v.number()) + 1)
    if k == 1:
        return PV.decimal(v.number() * 2.0)
    if k == 2:
        return PV.boolean(v.number() == 0)
    if k == 3:
        return PV.string(v.text() + "!")
    if k == 4:
        return PV.symbol(v.text())
    if k == 5:
        return PV.tagged(v.text(), v.text2() + "!")
    if k == 6:
        return PV.range(v.number() + 1.0, v.number2() + 1.0)
    return PV.measurement(v.text(), v.number() * 2.0)


def read_p(v):
    k = v.kind()
    if k in (3, 4):
        return float(len(v.text()))
    if k == 5:
        return float(len(v.text2()))
    if k == 6:
        return v.number() + v.number2()
    return v.number()


# --- JSON shape (same as the Rust core's serde output) -----------------------------------
def to_json_obj(v):
    match v:
        case Integer(x):
            return {"type": "integer", "value": x}
        case Decimal(x):
            return {"type": "decimal", "value": x}
        case Boolean(x):
            return {"type": "boolean", "value": x}
        case String(x):
            return {"type": "string", "value": x}
        case Symbol(x):
            return {"type": "symbol", "value": x}
        case Tagged(tag, c):
            return {"type": "tagged", "tag": tag, "content": c}
        case Range(lo, up):
            return {"type": "range", "lower": lo, "upper": up}
        case Measurement(u, x):
            return {"type": "measurement", "unit": u, "value": x}


def from_json_obj(d):
    t = d["type"]
    if t == "integer":
        return Integer(d["value"])
    if t == "decimal":
        return Decimal(d["value"])
    if t == "boolean":
        return Boolean(d["value"])
    if t == "string":
        return String(d["value"])
    if t == "symbol":
        return Symbol(d["value"])
    if t == "tagged":
        return Tagged(d["tag"], d["content"])
    if t == "range":
        return Range(d["lower"], d["upper"])
    if t == "measurement":
        return Measurement(d["unit"], d["value"])
    if t == "array":
        return Array([from_json_obj(x) for x in d["items"]])


now = time.perf_counter


def native_run(n):
    t = now()
    vals = [gen(i) for i in range(n)]
    create = now() - t
    t = now()
    lst = Array(vals)
    collect = now() - t
    t = now()
    mapped = map_one(lst)
    mp = now() - t
    t = now()
    s = checksum(mapped)
    read = now() - t
    t = now()
    txt = to_gram(mapped)
    text = now() - t
    return dict(create=create, collect=collect, map=mp, read=read, text=text), s + len(txt)


def pyo3_run(n, general):
    t = now()
    vals = [gen_p(i) for i in range(n)]
    create = now() - t
    t = now()
    lst = PL()
    for v in vals:
        lst.push(v)
    collect = now() - t
    t = now()
    mapped = lst.map_with(map_one_p) if general else lst.map_builtin()
    mp = now() - t
    t = now()
    s = 0.0
    for i in range(len(mapped)):
        s += read_p(mapped.get(i))
    read = now() - t
    t = now()
    txt = mapped.to_gram()
    text = now() - t
    return dict(create=create, collect=collect, map=mp, read=read, text=text), s + len(txt)


def pyo3_json_run(n):
    t = now()
    vals = [gen(i) for i in range(n)]
    create = now() - t
    t = now()
    js = json.dumps({"type": "array", "items": [to_json_obj(v) for v in vals]})
    collect = now() - t
    t = now()
    out = pybench.map_json(js)
    mp = now() - t
    t = now()
    mapped = from_json_obj(json.loads(out))
    s = checksum(mapped)
    read = now() - t
    t = now()
    txt = to_gram(mapped)
    text = now() - t
    return dict(create=create, collect=collect, map=mp, read=read, text=text), s + len(txt)


def bench(name, n, runs, once):
    acc = {}
    sink = 0.0
    for r in range(runs + 3):
        segs, s = once(n)
        sink += s
        if r >= 3:
            for k, v in segs.items():
                acc.setdefault(k, []).append(v * 1000.0)
    m = {k: statistics.median(v) for k, v in acc.items()}
    pipe = m["create"] + m["collect"] + m["map"] + m["read"]
    fmt = ", ".join(f"{k} {v:.4f} ms" for k, v in m.items())
    print(f"{name} n={n}: {fmt}, pipeline(create+collect+map+read) {pipe:.4f} ms [{pipe * 1000 / n:.3f} us/value] (sink {round(sink)})")


for n, runs in ((1000, 200), (100000, 10)):
    bench("py-native", n, runs, native_run)
    bench("py-pyo3-handles (callback map)", n, runs, lambda k: pyo3_run(k, True))
    bench("py-pyo3-handles (builtin map)", n, runs, lambda k: pyo3_run(k, False))
    bench("py-pyo3-json", n, runs, pyo3_json_run)
