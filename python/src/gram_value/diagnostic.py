"""Diagnostics and results (spec/results.md, ADR-005)."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass, field
from typing import Generic, TypeVar

T = TypeVar("T")

PathSegment = str | int
"""One step of a path into a value: a map key (str) or an array index (int)."""


@dataclass(frozen=True)
class Diagnostic:
    """A problem found by an operation."""

    code: str
    """A lower-case word with hyphens, such as ``duplicate-key``."""
    message: str
    """Text for a person; not part of the contract."""
    path: tuple[PathSegment, ...] = ()
    """The part of the value concerned; empty for the whole value."""

    def __str__(self) -> str:
        at = f" (at {path_to_pointer(self.path)})" if self.path else ""
        return f"{self.code}: {self.message}{at}"


@dataclass(frozen=True)
class Result(Generic[T]):
    """The result of an operation that can find problems.

    A successful result (``ok``) may still carry diagnostics: for a conversion they are losses.
    """

    ok: bool
    diagnostics: tuple[Diagnostic, ...] = ()
    _value: T | None = field(default=None, repr=True)

    @property
    def value(self) -> T:
        """The value of a successful result; raises :class:`GramValueError` for a failed one."""
        if not self.ok:
            raise GramValueError(self.diagnostics)
        return self._value  # type: ignore[return-value]

    def unwrap(self) -> T:
        """The value of a successful result; raises :class:`GramValueError` for a failed one."""
        return self.value


class GramValueError(Exception):
    """Raised when the value of a failed result is read."""

    def __init__(self, diagnostics: Sequence[Diagnostic]) -> None:
        super().__init__("; ".join(str(d) for d in diagnostics))
        self.diagnostics = tuple(diagnostics)


def ok(value: T, diagnostics: Sequence[Diagnostic] = ()) -> Result[T]:
    return Result(True, tuple(diagnostics), value)


def fail(*diagnostics: Diagnostic) -> Result[T]:
    return Result(False, tuple(diagnostics))


def diagnostic(code: str, message: str, path: Sequence[PathSegment] = ()) -> Diagnostic:
    return Diagnostic(code, message, tuple(path))


def path_to_pointer(path: Sequence[PathSegment]) -> str:
    """A path as a JSON Pointer (RFC 6901): ``/items/0/name``, empty for the whole value."""
    return "".join("/" + str(s).replace("~", "~0").replace("/", "~1") for s in path)


def fail_on_loss(result: Result[T]) -> Result[T]:
    """Turn any loss into a failure, for a consumer that wants strictness (ADR-006 2.3)."""
    if result.ok and result.diagnostics:
        return Result(False, result.diagnostics)
    return result


def under(segment: PathSegment, diagnostics: Sequence[Diagnostic]) -> list[Diagnostic]:
    """Prefix every path of ``diagnostics`` with ``segment``."""
    return [Diagnostic(d.code, d.message, (segment, *d.path)) for d in diagnostics]
