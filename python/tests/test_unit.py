from __future__ import annotations

from decimal import Decimal

import pytest

import gram_value as gv
from gram_value import Value


def code(r: gv.Result[Value] | gv.Result[str] | gv.Result[object]) -> str | None:
    return r.diagnostics[0].code if r.diagnostics else None


def test_equality_distinguishes_kinds_and_spelling_of_numbers() -> None:
    one, one_point = gv.integer(1), gv.decimal(1.0).unwrap()
    assert one != one_point
    assert gv.decimal(-0.0).unwrap() != gv.decimal(0.0).unwrap()
    assert gv.decimal(-0.0).unwrap() < gv.decimal(0.0).unwrap()
    kg5 = gv.measurement(gv.integer(5), "kg").unwrap()
    kg5_point = gv.measurement(gv.decimal(5.0).unwrap(), "kg").unwrap()
    assert kg5 != kg5_point


def test_hash_agrees_with_equality() -> None:
    a = gv.map_([("a", gv.integer(1)), ("b", gv.integer(2))]).unwrap()
    b = gv.map_([("b", gv.integer(2)), ("a", gv.integer(1))]).unwrap()
    assert a == b and hash(a) == hash(b)
    assert len({gv.decimal(0.0).unwrap(), gv.decimal(-0.0).unwrap()}) == 2


def test_maps_sort_keys_by_code_point() -> None:
    m = gv.map_([("\U0001f600", gv.integer(1)), ("￿", gv.integer(2))]).unwrap()
    assert isinstance(m, gv.MapValue)
    assert [k for k, _ in m.entries] == ["￿", "\U0001f600"]


def test_constructors_report_errors_with_codes() -> None:
    assert code(gv.decimal(float("nan"))) == "not-finite"
    assert code(gv.tagged("true", "x")) == "invalid-tag"
    assert code(gv.tagged("a b", "x")) == "invalid-tag"
    assert code(gv.range_()) == "no-ends"
    assert code(gv.range_(gv.string("x").unwrap())) == "invalid-range-end"
    assert code(gv.measurement(gv.integer(1), "k g")) == "invalid-unit"
    assert code(gv.big_integer(5)) == "invalid-integer"
    assert code(gv.big_decimal("0.1")) == "invalid-decimal"
    assert code(gv.integer_from_digits("007")) == "invalid-integer"
    assert code(gv.string("\ud800")) == "invalid-string"
    dup = gv.map_([("a", gv.integer(1)), ("a", gv.integer(2))])
    assert code(dup) == "duplicate-key" and dup.diagnostics[0].path == ("a",)


def test_integers_pick_their_kind_by_range() -> None:
    assert gv.integer(2**63 - 1).kind == "integer"
    assert gv.integer(2**63).kind == "big_integer"
    assert gv.integer(2**63 - 1) < gv.integer(2**63)
    with pytest.raises(TypeError):
        gv.integer(True)
    assert gv.is_homogeneous_array(gv.array([gv.integer(1), gv.integer(2)]))
    assert not gv.is_homogeneous_array(gv.array([gv.integer(1), gv.boolean(True)]))


def test_unwritable_parts_are_reported_with_paths() -> None:
    r = gv.to_text(gv.array([gv.integer(1), gv.array([gv.integer(2)])]))
    assert not r.ok and r.diagnostics[0].code == "unwritable" and r.diagnostics[0].path == (1,)
    assert not gv.to_text(gv.measurement(gv.integer(0), "xFF").unwrap()).ok


def test_a_record_may_hold_a_map_one_level_deep() -> None:
    inner = gv.map_([("x", gv.integer(1))]).unwrap()
    rec = gv.map_([("k", inner)]).unwrap()
    assert not gv.to_text(rec).ok
    assert gv.to_text(rec, "record").unwrap() == "{k: {x: 1}}"
    assert not gv.to_text(gv.map_([("k", rec)]).unwrap(), "record").ok


def test_decimals_never_use_an_exponent() -> None:
    assert gv.to_text(gv.decimal(1e21).unwrap()).unwrap() == "1000000000000000000000.0"
    assert gv.to_text(gv.decimal(1e-7).unwrap()).unwrap() == "0.0000001"
    assert gv.to_text(gv.decimal(1.5e-10).unwrap()).unwrap() == "0.00000000015"


def test_every_python_entry_is_exact() -> None:
    assert gv.to_native(gv.decimal(1.0).unwrap()).unwrap() == 1.0
    assert isinstance(gv.to_native(gv.decimal(1.0).unwrap()).unwrap(), float)
    big = gv.decimal_from_text("0.10000000000000000001").unwrap()
    assert gv.to_native(big).unwrap() == Decimal("0.10000000000000000001")
    assert gv.to_native(gv.integer(2**70)).unwrap() == 2**70
    assert gv.to_native(gv.integer(1)).diagnostics == ()


def test_maps_become_dicts_in_key_order() -> None:
    m = gv.map_([("b", gv.integer(1)), ("a", gv.integer(2))]).unwrap()
    assert list(gv.to_native(m).unwrap()) == ["a", "b"]


def test_a_bool_is_not_an_integer() -> None:
    assert gv.from_native(True).unwrap().kind == "boolean"
    assert gv.from_native(1).unwrap().kind == "integer"


def test_from_native_unwritable_values_fail_with_every_problem_listed() -> None:
    r = gv.from_native({"name": "x", "nick": None, "score": float("nan"), "f": lambda: 1})
    assert not r.ok
    assert [(d.code, d.path) for d in r.diagnostics] == [
        ("unsupported", ("nick",)),
        ("not-finite", ("score",)),
        ("unsupported", ("f",)),
    ]
    assert not gv.from_native({1: 2}).ok
    assert not gv.from_native(Decimal("NaN")).ok


def test_from_native_numbers() -> None:
    assert gv.from_native(3.5).unwrap().kind == "decimal"
    assert gv.from_native(2**70).unwrap().kind == "big_integer"
    assert gv.from_native(Decimal("1E+2")).unwrap() == gv.decimal(100.0).unwrap()
    m = gv.from_native(gv.Measurement(5, "kg")).unwrap()
    assert gv.to_text(m).unwrap() == "5kg"


def test_losses_helpers() -> None:
    r = gv.ok(1, [gv.diagnostic("kind", "x")])
    assert not gv.fail_on_loss(r).ok
    assert gv.path_to_pointer(["a/b", 0, "c~d"]) == "/a~1b/0/c~0d"
    with pytest.raises(gv.GramValueError):
        gv.decimal(float("inf")).unwrap()
