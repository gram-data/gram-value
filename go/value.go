package gramvalue

import (
	"math"
	"math/big"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"unicode/utf8"
)

// Kind is the kind of a value, in kind order.
type Kind int

// The eleven kinds, in kind order (which is also the order of the total order).
const (
	KindInteger Kind = iota
	KindBigInteger
	KindDecimal
	KindBigDecimal
	KindBoolean
	KindString
	KindTagged
	KindRange
	KindMeasurement
	KindArray
	KindMap
)

var kindNames = [...]string{"integer", "big_integer", "decimal", "big_decimal", "boolean", "string", "tagged", "range", "measurement", "array", "map"}

// String is the kind's name as the conformance vocabulary spells it.
func (k Kind) String() string { return kindNames[k] }

// A Value is a gram value. The set of kinds is closed (ADR-002): a Value is one of the eleven *Value types
// of this package, which are created only by the New functions.
type Value interface {
	// Kind is the value's kind.
	Kind() Kind
	isValue()
}

// IntegerValue is a signed 64-bit integer.
type IntegerValue struct{ v int64 }

// BigIntegerValue is an integer outside the 64-bit range.
type BigIntegerValue struct{ v *big.Int }

// DecimalValue is a finite 64-bit float that writes back as the same number.
type DecimalValue struct{ v float64 }

// BigDecimalValue is a decimal a 64-bit float cannot hold exactly.
type BigDecimalValue struct{ digits string }

// BooleanValue is true or false.
type BooleanValue struct{ v bool }

// StringValue is text.
type StringValue struct{ v string }

// TaggedValue is a tag and a content string.
type TaggedValue struct{ tag, content string }

// MeasurementValue is a numeric magnitude and a unit.
type MeasurementValue struct {
	magnitude Value
	unit      string
}

// RangeValue is a range with an optional lower end and an optional upper end, inclusive.
type RangeValue struct{ lower, upper Value }

// ArrayValue is an ordered sequence of values.
type ArrayValue struct{ items []Value }

// MapValue is an unordered set of entries with unique string keys, held in key (UTF-8 byte) order.
type MapValue struct{ entries []Entry }

// Entry is a key and its value in a map.
type Entry struct {
	Key   string
	Value Value
}

func (IntegerValue) Kind() Kind     { return KindInteger }
func (BigIntegerValue) Kind() Kind  { return KindBigInteger }
func (DecimalValue) Kind() Kind     { return KindDecimal }
func (BigDecimalValue) Kind() Kind  { return KindBigDecimal }
func (BooleanValue) Kind() Kind     { return KindBoolean }
func (StringValue) Kind() Kind      { return KindString }
func (TaggedValue) Kind() Kind      { return KindTagged }
func (MeasurementValue) Kind() Kind { return KindMeasurement }
func (RangeValue) Kind() Kind       { return KindRange }
func (ArrayValue) Kind() Kind       { return KindArray }
func (MapValue) Kind() Kind         { return KindMap }

func (IntegerValue) isValue()     {}
func (BigIntegerValue) isValue()  {}
func (DecimalValue) isValue()     {}
func (BigDecimalValue) isValue()  {}
func (BooleanValue) isValue()     {}
func (StringValue) isValue()      {}
func (TaggedValue) isValue()      {}
func (MeasurementValue) isValue() {}
func (RangeValue) isValue()       {}
func (ArrayValue) isValue()       {}
func (MapValue) isValue()         {}

// Int64 is the integer.
func (v IntegerValue) Int64() int64 { return v.v }

// BigInt is a copy of the integer.
func (v BigIntegerValue) BigInt() *big.Int { return new(big.Int).Set(v.v) }

// Float64 is the float.
func (v DecimalValue) Float64() float64 { return v.v }

// Digits is the sign, digits, point and digits, with no trailing zeros beyond one digit.
func (v BigDecimalValue) Digits() string { return v.digits }

// Bool is the boolean.
func (v BooleanValue) Bool() bool { return v.v }

// Text is the string.
func (v StringValue) Text() string { return v.v }

// Tag is the tag, a symbol other than "true" and "false".
func (v TaggedValue) Tag() string { return v.tag }

// Content is the content string, as written.
func (v TaggedValue) Content() string { return v.content }

// Magnitude is the magnitude: an integer, big integer, decimal or big decimal.
func (v MeasurementValue) Magnitude() Value { return v.magnitude }

// Unit is the unit, ASCII letters.
func (v MeasurementValue) Unit() string { return v.unit }

// Lower is the lower end, or nil.
func (v RangeValue) Lower() Value { return v.lower }

// Upper is the upper end, or nil.
func (v RangeValue) Upper() Value { return v.upper }

// Items are the elements (a copy).
func (v ArrayValue) Items() []Value { return append([]Value(nil), v.items...) }

// Entries are the entries in key order (a copy).
func (v MapValue) Entries() []Entry { return append([]Entry(nil), v.entries...) }

// Get is the value of a key.
func (v MapValue) Get(key string) (Value, bool) {
	i := sort.Search(len(v.entries), func(i int) bool { return v.entries[i].Key >= key })
	if i < len(v.entries) && v.entries[i].Key == key {
		return v.entries[i].Value, true
	}
	return nil, false
}

// IsScalar is true for every kind except array and map.
func IsScalar(v Value) bool { return v.Kind() != KindArray && v.Kind() != KindMap }

// IsHomogeneousArray reports, for an array, whether all elements have one kind (true when empty); false for other kinds.
func IsHomogeneousArray(v Value) bool {
	a, ok := v.(ArrayValue)
	if !ok {
		return false
	}
	for i := 1; i < len(a.items); i++ {
		if a.items[i].Kind() != a.items[0].Kind() {
			return false
		}
	}
	return true
}

func isNumeric(v Value) bool {
	switch v.Kind() {
	case KindInteger, KindBigInteger, KindDecimal, KindBigDecimal:
		return true
	}
	return false
}

var symbolRE = regexp.MustCompile(`^[a-zA-Z_][0-9a-zA-Z_.\-@]*$`)

// IsSymbol reports whether s matches the symbol rule [a-zA-Z_][0-9a-zA-Z_.\-@]*.
func IsSymbol(s string) bool { return symbolRE.MatchString(s) }

// IsBareWord reports whether s is a symbol other than "true" and "false".
func IsBareWord(s string) bool { return IsSymbol(s) && s != "true" && s != "false" }

// ---- number text --------------------------------------------------------------------------------

var (
	integerTextRE = regexp.MustCompile(`^-?(0|[1-9][0-9]*)$`)
	decimalTextRE = regexp.MustCompile(`^-?(0|[1-9][0-9]*)\.[0-9]+$`)
	i64Min        = big.NewInt(math.MinInt64)
	i64Max        = big.NewInt(math.MaxInt64)
)

func normalizeDecimal(s string) string {
	i, f, _ := strings.Cut(s, ".")
	f = strings.TrimRight(f, "0")
	if f == "" {
		f = "0"
	}
	return i + "." + f
}

// FloatText is the shortest positional text of a finite float, with at least one digit after the point.
func FloatText(f float64) string {
	s := strconv.FormatFloat(f, 'f', -1, 64)
	if f == 0 && math.Signbit(f) {
		s = "-0"
	}
	if !strings.Contains(s, ".") {
		s += ".0"
	}
	return s
}

// ---- constructors -------------------------------------------------------------------------------

// NewInteger is an integer.
func NewInteger(n int64) Value { return IntegerValue{n} }

// NewIntegerFromBig is an integer: an integer value if n fits in 64 bits, else a big integer.
func NewIntegerFromBig(n *big.Int) Value {
	if n.IsInt64() {
		return IntegerValue{n.Int64()}
	}
	return BigIntegerValue{new(big.Int).Set(n)}
}

// NewIntegerFromDigits is an integer from -?(0|[1-9][0-9]*).
func NewIntegerFromDigits(text string) (Value, error) {
	if !integerTextRE.MatchString(text) || text == "-0" {
		return nil, diag("invalid-integer", strconv.Quote(text)+" is not an integer without leading zeros")
	}
	n, _ := new(big.Int).SetString(text, 10)
	return NewIntegerFromBig(n), nil
}

// NewBigInteger is a big integer; the number must be outside the 64-bit range.
func NewBigInteger(n *big.Int) (Value, error) {
	if n.IsInt64() {
		return nil, diag("invalid-integer", n.String()+" fits in 64 bits, so it is an integer")
	}
	return BigIntegerValue{new(big.Int).Set(n)}, nil
}

// NewDecimal is a decimal. NaN and the infinities are rejected (not-finite).
func NewDecimal(f float64) (Value, error) {
	if math.IsNaN(f) || math.IsInf(f, 0) {
		return nil, diag("not-finite", "a decimal must be finite")
	}
	return DecimalValue{f}, nil
}

// NewDecimalFromText is a decimal from -?(0|[1-9][0-9]*)\.[0-9]+: a decimal when the nearest float writes
// back as the same number, else a big decimal.
func NewDecimalFromText(text string) (Value, error) {
	if !decimalTextRE.MatchString(text) {
		return nil, diag("invalid-decimal", strconv.Quote(text)+" is not a decimal")
	}
	normalized := normalizeDecimal(text)
	f, err := strconv.ParseFloat(normalized, 64)
	if err == nil && !math.IsInf(f, 0) && FloatText(f) == normalized {
		return DecimalValue{f}, nil
	}
	return BigDecimalValue{normalized}, nil
}

// NewBigDecimal is a big decimal. The text must be normalized and beyond what a float holds.
func NewBigDecimal(text string) (Value, error) {
	if decimalTextRE.MatchString(text) && normalizeDecimal(text) == text {
		if v, err := NewDecimalFromText(text); err == nil {
			if b, ok := v.(BigDecimalValue); ok {
				return b, nil
			}
		}
	}
	return nil, diag("invalid-decimal", strconv.Quote(text)+" is not a normalized decimal beyond a 64-bit float")
}

// NewBoolean is a boolean.
func NewBoolean(b bool) Value { return BooleanValue{b} }

// NewString is a string. A string that is not valid UTF-8 is rejected (invalid-string).
func NewString(s string) (Value, error) {
	if !utf8.ValidString(s) {
		return nil, diag("invalid-string", "a string must be valid Unicode")
	}
	return StringValue{s}, nil
}

// NewTagged is a tagged value. The tag must be a symbol other than "true" and "false" (invalid-tag).
func NewTagged(tag, content string) (Value, error) {
	if !IsBareWord(tag) {
		return nil, diag("invalid-tag", strconv.Quote(tag)+" is not a symbol other than true or false")
	}
	if !utf8.ValidString(content) {
		return nil, diag("invalid-string", "the content must be valid Unicode")
	}
	return TaggedValue{tag, content}, nil
}

// NewRange is a range. Each end must be nil, numeric or a measurement (invalid-range-end); at least one end is
// needed (no-ends).
func NewRange(lower, upper Value) (Value, error) {
	if lower == nil && upper == nil {
		return nil, diag("no-ends", "a range needs at least one end")
	}
	for _, end := range []Value{lower, upper} {
		if end != nil && !isNumeric(end) && end.Kind() != KindMeasurement {
			return nil, diag("invalid-range-end", "a range end must be numeric or a measurement, not "+end.Kind().String())
		}
	}
	return RangeValue{lower, upper}, nil
}

var unitRE = regexp.MustCompile(`^[a-zA-Z]+$`)

// NewMeasurement is a measurement. The magnitude must be numeric and the unit ASCII letters (invalid-unit).
func NewMeasurement(magnitude Value, unit string) (Value, error) {
	if magnitude == nil || !isNumeric(magnitude) {
		return nil, diag("invalid-range-end", "a magnitude must be numeric")
	}
	if !unitRE.MatchString(unit) {
		return nil, diag("invalid-unit", strconv.Quote(unit)+" is not one or more ASCII letters")
	}
	return MeasurementValue{magnitude, unit}, nil
}

// NewArray is an array.
func NewArray(items ...Value) Value { return ArrayValue{append([]Value(nil), items...)} }

// NewMap is a map. A repeated key is an error (duplicate-key) that names the key.
func NewMap(entries ...Entry) (Value, error) {
	sorted := append([]Entry(nil), entries...)
	sort.SliceStable(sorted, func(i, j int) bool { return sorted[i].Key < sorted[j].Key })
	for i := 1; i < len(sorted); i++ {
		if sorted[i].Key == sorted[i-1].Key {
			return nil, Diagnostics{{Code: "duplicate-key", Message: "the key " + strconv.Quote(sorted[i].Key) + " is repeated", Path: []PathSegment{Key(sorted[i].Key)}}}
		}
	}
	return MapValue{sorted}, nil
}

// ---- equality and order -------------------------------------------------------------------------

// Equal reports whether two values have the same kind and the same content, part by part (ADR-002 2.5).
func Equal(a, b Value) bool { return Compare(a, b) == 0 }

func cmpInt(a, b int) int {
	switch {
	case a < b:
		return -1
	case a > b:
		return 1
	}
	return 0
}

func cmpString(a, b string) int { return strings.Compare(a, b) }

func compareDigits(a, b string) int {
	na, nb := strings.HasPrefix(a, "-"), strings.HasPrefix(b, "-")
	if na != nb {
		if na {
			return -1
		}
		return 1
	}
	ai, af, _ := strings.Cut(strings.TrimPrefix(a, "-"), ".")
	bi, bf, _ := strings.Cut(strings.TrimPrefix(b, "-"), ".")
	m := cmpInt(len(ai), len(bi))
	if m == 0 {
		m = cmpString(ai, bi)
	}
	if m == 0 {
		m = cmpString(af, bf)
	}
	if na {
		return -m
	}
	return m
}

func compareOptional(a, b Value) int {
	switch {
	case a == nil && b == nil:
		return 0
	case a == nil:
		return -1
	case b == nil:
		return 1
	}
	return Compare(a, b)
}

// Compare is the total order of spec/values.md section 6: -1, 0 or 1. It is consistent with Equal.
func Compare(a, b Value) int {
	if c := cmpInt(int(a.Kind()), int(b.Kind())); c != 0 {
		return c
	}
	switch x := a.(type) {
	case IntegerValue:
		y := b.(IntegerValue)
		switch {
		case x.v < y.v:
			return -1
		case x.v > y.v:
			return 1
		}
		return 0
	case BigIntegerValue:
		return x.v.Cmp(b.(BigIntegerValue).v)
	case DecimalValue:
		y := b.(DecimalValue)
		if x.v == 0 && y.v == 0 {
			return cmpInt(boolInt(math.Signbit(y.v)), boolInt(math.Signbit(x.v)))
		}
		switch {
		case x.v < y.v:
			return -1
		case x.v > y.v:
			return 1
		}
		return 0
	case BigDecimalValue:
		return compareDigits(x.digits, b.(BigDecimalValue).digits)
	case BooleanValue:
		return cmpInt(boolInt(x.v), boolInt(b.(BooleanValue).v))
	case StringValue:
		return cmpString(x.v, b.(StringValue).v)
	case TaggedValue:
		y := b.(TaggedValue)
		if c := cmpString(x.tag, y.tag); c != 0 {
			return c
		}
		return cmpString(x.content, y.content)
	case RangeValue:
		y := b.(RangeValue)
		if c := compareOptional(x.lower, y.lower); c != 0 {
			return c
		}
		return compareOptional(x.upper, y.upper)
	case MeasurementValue:
		y := b.(MeasurementValue)
		if c := Compare(x.magnitude, y.magnitude); c != 0 {
			return c
		}
		return cmpString(x.unit, y.unit)
	case ArrayValue:
		y := b.(ArrayValue)
		for i := 0; i < len(x.items) && i < len(y.items); i++ {
			if c := Compare(x.items[i], y.items[i]); c != 0 {
				return c
			}
		}
		return cmpInt(len(x.items), len(y.items))
	case MapValue:
		y := b.(MapValue)
		for i := 0; i < len(x.entries) && i < len(y.entries); i++ {
			if c := cmpString(x.entries[i].Key, y.entries[i].Key); c != 0 {
				return c
			}
			if c := Compare(x.entries[i].Value, y.entries[i].Value); c != 0 {
				return c
			}
		}
		return cmpInt(len(x.entries), len(y.entries))
	}
	panic("unreachable: a Value is one of the eleven types")
}

func boolInt(b bool) int {
	if b {
		return 1
	}
	return 0
}
