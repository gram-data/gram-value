"""SPIKE-002: the cost of one call into the PyO3 extension, against the same work in Python."""
import sys, time
sys.path.insert(0, __file__.rsplit("/", 1)[0])
t0 = time.perf_counter()
import pybench
print(f"import the extension module: {(time.perf_counter() - t0) * 1000:.2f} ms")
from dataclasses import dataclass


@dataclass(slots=True)
class Integer:
    value: int


def t(label, n, f):
    f()
    runs = []
    for _ in range(7):
        s = time.perf_counter()
        f()
        runs.append(time.perf_counter() - s)
    runs.sort()
    print(f"{label}: {runs[3] * 1e9 / n:.1f} ns per call")


N = 1_000_000
nat = Integer(42)
h = pybench.PValue.integer(42)
hs = pybench.PValue.string("hello world")


def a():
    s = 0
    for _ in range(N):
        s += nat.value
t("native attribute read (obj.value)", N, a)


def b():
    s = 0
    for _ in range(N):
        s += h.number()
t("PyO3 read: v.number()", N, b)


def c():
    s = 0
    for _ in range(N):
        s += h.kind()
t("PyO3 read: v.kind()", N, c)


def d():
    s = 0
    for _ in range(N):
        s += len(hs.text())
t("PyO3 read of a string: v.text()", N, d)

M = 200_000


def e():
    for i in range(M):
        Integer(i)
t("native create a dataclass Integer", M, e)


def f():
    for i in range(M):
        pybench.PValue.integer(i)
t("PyO3 create an Integer handle", M, f)
