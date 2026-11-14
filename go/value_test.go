package gramvalue_test

import (
	"errors"
	"math"
	"math/big"
	"testing"

	gv "github.com/gram-data/gram-value/go"
)

func mk(t *testing.T) func(gv.Value, error) gv.Value {
	return func(v gv.Value, err error) gv.Value {
		t.Helper()
		if err != nil {
			t.Fatal(err)
		}
		return v
	}
}

func code(t *testing.T, err error) string {
	t.Helper()
	var ds gv.Diagnostics
	if !errors.As(err, &ds) || len(ds) == 0 {
		t.Fatalf("expected Diagnostics, got %v", err)
	}
	return ds[0].Code
}

func dec(t *testing.T, f float64) gv.Value {
	return mk(t)(gv.NewDecimal(f))
}

func TestEqualityDistinguishesKindsAndNumberSpelling(t *testing.T) {
	if gv.Equal(gv.NewInteger(1), dec(t, 1)) {
		t.Error("1 equals 1.0")
	}
	if gv.Equal(dec(t, math.Copysign(0, -1)), dec(t, 0)) {
		t.Error("-0.0 equals 0.0")
	}
	if gv.Compare(dec(t, math.Copysign(0, -1)), dec(t, 0)) >= 0 {
		t.Error("-0.0 should sort before 0.0")
	}
	a := mk(t)(gv.NewMeasurement(gv.NewInteger(5), "kg"))
	b := mk(t)(gv.NewMeasurement(dec(t, 5), "kg"))
	if gv.Equal(a, b) {
		t.Error("5kg equals 5.0kg")
	}
}

func TestMapsAreEqualWhateverTheEntryOrder(t *testing.T) {
	a := mk(t)(gv.NewMap(gv.Entry{Key: "a", Value: gv.NewInteger(1)}, gv.Entry{Key: "b", Value: gv.NewInteger(2)}))
	b := mk(t)(gv.NewMap(gv.Entry{Key: "b", Value: gv.NewInteger(2)}, gv.Entry{Key: "a", Value: gv.NewInteger(1)}))
	if !gv.Equal(a, b) {
		t.Error("maps with the same entries differ")
	}
	v, found := a.(gv.MapValue).Get("b")
	if !found || !gv.Equal(v, gv.NewInteger(2)) {
		t.Error("Get failed")
	}
}

func TestConstructorsReportErrorsWithCodes(t *testing.T) {
	check := func(want string, err error) {
		t.Helper()
		if got := code(t, err); got != want {
			t.Errorf("code %s, want %s", got, want)
		}
	}
	_, err := gv.NewDecimal(math.NaN())
	check("not-finite", err)
	_, err = gv.NewTagged("true", "x")
	check("invalid-tag", err)
	_, err = gv.NewTagged("a b", "x")
	check("invalid-tag", err)
	_, err = gv.NewRange(nil, nil)
	check("no-ends", err)
	s := mk(t)(gv.NewString("x"))
	_, err = gv.NewRange(s, nil)
	check("invalid-range-end", err)
	_, err = gv.NewMeasurement(gv.NewInteger(1), "k g")
	check("invalid-unit", err)
	_, err = gv.NewBigInteger(big.NewInt(5))
	check("invalid-integer", err)
	_, err = gv.NewBigDecimal("0.1")
	check("invalid-decimal", err)
	_, err = gv.NewIntegerFromDigits("007")
	check("invalid-integer", err)
	_, err = gv.NewString("\xff")
	check("invalid-string", err)
	_, err = gv.NewMap(gv.Entry{Key: "a", Value: gv.NewInteger(1)}, gv.Entry{Key: "a", Value: gv.NewInteger(2)})
	check("duplicate-key", err)
	var ds gv.Diagnostics
	errors.As(err, &ds)
	if ds[0].Pointer() != "/a" {
		t.Errorf("pointer %q", ds[0].Pointer())
	}
}

func TestIntegersPickTheirKindByRange(t *testing.T) {
	max := new(big.Int).SetInt64(math.MaxInt64)
	if gv.NewIntegerFromBig(max).Kind() != gv.KindInteger {
		t.Error("max int64 should be an integer")
	}
	over := new(big.Int).Add(max, big.NewInt(1))
	if gv.NewIntegerFromBig(over).Kind() != gv.KindBigInteger {
		t.Error("2^63 should be a big integer")
	}
	if gv.Compare(gv.NewIntegerFromBig(max), gv.NewIntegerFromBig(over)) >= 0 {
		t.Error("order")
	}
	if !gv.IsHomogeneousArray(gv.NewArray(gv.NewInteger(1), gv.NewInteger(2))) {
		t.Error("homogeneous")
	}
	if gv.IsHomogeneousArray(gv.NewArray(gv.NewInteger(1), gv.NewBoolean(true))) {
		t.Error("mixed")
	}
}

func TestUnwritablePartsAreReportedWithPaths(t *testing.T) {
	v := gv.NewArray(gv.NewInteger(1), gv.NewArray(gv.NewInteger(2)))
	_, err := gv.ToText(v, gv.ContextValue)
	var ds gv.Diagnostics
	if !errors.As(err, &ds) || ds[0].Code != "unwritable" || ds[0].Pointer() != "/1" {
		t.Errorf("got %v", err)
	}
	x := mk(t)(gv.NewMeasurement(gv.NewInteger(0), "xFF"))
	if _, err := gv.ToText(x, gv.ContextValue); err == nil {
		t.Error("0xFF-looking measurement should be unwritable")
	}
}

func TestARecordMayHoldAMapOneLevelDeep(t *testing.T) {
	inner := mk(t)(gv.NewMap(gv.Entry{Key: "x", Value: gv.NewInteger(1)}))
	rec := mk(t)(gv.NewMap(gv.Entry{Key: "k", Value: inner}))
	if _, err := gv.ToText(rec, gv.ContextValue); err == nil {
		t.Error("a value map cannot hold a map")
	}
	s, err := gv.ToText(rec, gv.ContextRecord)
	if err != nil || s != "{k: {x: 1}}" {
		t.Errorf("got %q %v", s, err)
	}
	deep := mk(t)(gv.NewMap(gv.Entry{Key: "k", Value: rec}))
	if _, err := gv.ToText(deep, gv.ContextRecord); err == nil {
		t.Error("two levels are unwritable")
	}
}

func TestDecimalsNeverUseAnExponent(t *testing.T) {
	for in, want := range map[float64]string{1e21: "1000000000000000000000.0", 1e-7: "0.0000001", 1.5e-10: "0.00000000015"} {
		if got, err := gv.ToText(dec(t, in), gv.ContextValue); err != nil || got != want {
			t.Errorf("%g: got %q %v, want %q", in, got, err, want)
		}
	}
}

func TestToNativeIntegersAndUnsupportedBigDecimal(t *testing.T) {
	n, err := gv.ToNative(gv.NewInteger(42))
	if err != nil || n.(int64) != 42 {
		t.Errorf("got %v %v", n, err)
	}
	big10 := mk(t)(gv.NewDecimalFromText("0.10000000000000000001"))
	_, err = gv.ToNative(gv.NewArray(gv.NewInteger(1), big10))
	var ds gv.Diagnostics
	if !errors.As(err, &ds) || ds[0].Code != "unsupported" || ds[0].Pointer() != "/1" {
		t.Errorf("got %v", err)
	}
}

func TestFromNativeUnwritableValuesFailWithEveryProblemListed(t *testing.T) {
	_, err := gv.FromNative(map[string]any{"name": "x", "nick": nil, "score": math.NaN(), "f": func() {}})
	var ds gv.Diagnostics
	if !errors.As(err, &ds) {
		t.Fatalf("got %v", err)
	}
	got := map[string]string{}
	for _, d := range ds {
		got[d.Pointer()] = d.Code
	}
	want := map[string]string{"/nick": "unsupported", "/score": "not-finite", "/f": "unsupported"}
	if len(got) != len(want) {
		t.Fatalf("got %v", got)
	}
	for k, v := range want {
		if got[k] != v {
			t.Errorf("%s: %s, want %s", k, got[k], v)
		}
	}
	if _, err := gv.FromNative(float32(1)); err == nil {
		t.Error("float32 should be unsupported")
	}
	if _, err := gv.FromNative(map[int]int{1: 2}); err == nil {
		t.Error("a non-string key should be unsupported")
	}
}

func TestFromNativeNumbersAndTypedSlices(t *testing.T) {
	if v, _ := gv.FromNative(uint64(math.MaxUint64)); v.Kind() != gv.KindBigInteger {
		t.Error("uint64 max is a big integer")
	}
	if v, _ := gv.FromNative(3); v.Kind() != gv.KindInteger {
		t.Error("int")
	}
	v, err := gv.FromNative([]string{"a", "b c"})
	if err != nil {
		t.Fatal(err)
	}
	if s, _ := gv.ToText(v, gv.ContextValue); s != `[a, "b c"]` {
		t.Errorf("got %s", s)
	}
	m, _ := gv.FromNative(gv.Measurement{Magnitude: int64(5), Unit: "kg"})
	if s, _ := gv.ToText(m, gv.ContextValue); s != "5kg" {
		t.Errorf("got %s", s)
	}
}
