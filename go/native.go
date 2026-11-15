package gramvalue

import (
	"math"
	"math/big"
	"reflect"
)

// Tagged is the Go value of a tagged string: a tag and a content string.
type Tagged struct {
	Tag, Content string
}

// Measurement is the Go value of a measurement. Magnitude is an int64, *big.Int or float64.
type Measurement struct {
	Magnitude any
	Unit      string
}

// Range is the Go value of a range, inclusive. Lower and Upper are nil, an int64, *big.Int, float64 or Measurement.
type Range struct {
	Lower, Upper any
}

func numericToNative(v Value) (any, error) {
	switch x := v.(type) {
	case IntegerValue:
		return x.v, nil
	case BigIntegerValue:
		return new(big.Int).Set(x.v), nil
	case DecimalValue:
		return x.v, nil
	}
	return nil, diag("unsupported", "a big decimal has no Go standard library type; keep the BigDecimalValue")
}

// ToNative converts a value to Go values (ADR-006): int64, *big.Int, float64, bool, string, Tagged, Range,
// Measurement, []any and map[string]any. A big decimal has no native value and fails with code "unsupported".
// Every entry is exact or unsupported, so a conversion never reports a loss. The error holds every problem found.
func ToNative(v Value) (any, error) {
	switch x := v.(type) {
	case IntegerValue, BigIntegerValue, DecimalValue, BigDecimalValue:
		return numericToNative(v)
	case BooleanValue:
		return x.v, nil
	case StringValue:
		return x.v, nil
	case TaggedValue:
		return Tagged{x.tag, x.content}, nil
	case MeasurementValue:
		m, err := numericToNative(x.magnitude)
		if err != nil {
			return nil, under(Key("magnitude"), err.(Diagnostics))
		}
		return Measurement{m, x.unit}, nil
	case RangeValue:
		var errs Diagnostics
		ends := [2]any{}
		for i, e := range []struct {
			key string
			v   Value
		}{{"lower", x.lower}, {"upper", x.upper}} {
			if e.v == nil {
				continue
			}
			var n any
			var err error
			if mv, ok := e.v.(MeasurementValue); ok {
				n, err = ToNative(mv)
			} else {
				n, err = numericToNative(e.v)
			}
			if err != nil {
				errs = append(errs, under(Key(e.key), err.(Diagnostics))...)
				continue
			}
			ends[i] = n
		}
		if len(errs) > 0 {
			return nil, errs
		}
		return Range{ends[0], ends[1]}, nil
	case ArrayValue:
		out := make([]any, 0, len(x.items))
		var errs Diagnostics
		for i, item := range x.items {
			n, err := ToNative(item)
			if err != nil {
				errs = append(errs, under(Index(i), err.(Diagnostics))...)
				continue
			}
			out = append(out, n)
		}
		if len(errs) > 0 {
			return nil, errs
		}
		return out, nil
	case MapValue:
		out := make(map[string]any, len(x.entries))
		var errs Diagnostics
		for _, e := range x.entries {
			n, err := ToNative(e.Value)
			if err != nil {
				errs = append(errs, under(Key(e.Key), err.(Diagnostics))...)
				continue
			}
			out[e.Key] = n
		}
		if len(errs) > 0 {
			return nil, errs
		}
		return out, nil
	}
	panic("unreachable: a Value is one of the eleven types")
}

func unsupported(path []PathSegment, what string) Diagnostics {
	return at(path, diag("unsupported", what))
}

func numberFromNative(x any, path []PathSegment) (Value, Diagnostics) {
	switch n := x.(type) {
	case int64:
		return IntegerValue{n}, nil
	case *big.Int:
		if n == nil {
			return nil, unsupported(path, "a nil *big.Int has no gram counterpart")
		}
		return NewIntegerFromBig(n), nil
	case float64:
		if math.IsNaN(n) || math.IsInf(n, 0) {
			return nil, at(path, diag("not-finite", "NaN and infinities cannot be written"))
		}
		return DecimalValue{n}, nil
	}
	return nil, unsupported(path, "a number must be an integer type, *big.Int or float64")
}

func fromNative(x any, path []PathSegment) (Value, Diagnostics) {
	switch n := x.(type) {
	case nil:
		return nil, unsupported(path, "nil has no gram counterpart")
	case bool:
		return BooleanValue{n}, nil
	case int:
		return IntegerValue{int64(n)}, nil
	case int8:
		return IntegerValue{int64(n)}, nil
	case int16:
		return IntegerValue{int64(n)}, nil
	case int32:
		return IntegerValue{int64(n)}, nil
	case int64, *big.Int, float64:
		return numberFromNative(n, path)
	case uint, uint8, uint16, uint32, uint64, uintptr:
		return NewIntegerFromBig(new(big.Int).SetUint64(reflect.ValueOf(n).Uint())), nil
	case string:
		v, err := NewString(n)
		if err != nil {
			return nil, at(path, err.(Diagnostics))
		}
		return v, nil
	case Tagged:
		v, err := NewTagged(n.Tag, n.Content)
		if err != nil {
			return nil, at(path, err.(Diagnostics))
		}
		return v, nil
	case Embedding:
		return n.ToValue(), nil
	case Measurement:
		mag, errs := numberFromNative(n.Magnitude, append(append([]PathSegment(nil), path...), Key("magnitude")))
		if errs != nil {
			return nil, errs
		}
		v, err := NewMeasurement(mag, n.Unit)
		if err != nil {
			return nil, at(path, err.(Diagnostics))
		}
		return v, nil
	case Range:
		var errs Diagnostics
		var ends [2]Value
		for i, e := range []struct {
			key string
			v   any
		}{{"lower", n.Lower}, {"upper", n.Upper}} {
			if e.v == nil {
				continue
			}
			p := append(append([]PathSegment(nil), path...), Key(e.key))
			v, d := fromNative(e.v, p)
			if d != nil {
				errs = append(errs, d...)
				continue
			}
			ends[i] = v
		}
		if len(errs) > 0 {
			return nil, errs
		}
		v, err := NewRange(ends[0], ends[1])
		if err != nil {
			return nil, at(path, err.(Diagnostics))
		}
		return v, nil
	}
	return fromReflect(x, path)
}

// fromReflect handles slices, arrays and string-keyed maps of any element type.
func fromReflect(x any, path []PathSegment) (Value, Diagnostics) {
	rv := reflect.ValueOf(x)
	switch rv.Kind() {
	case reflect.Slice, reflect.Array:
		if rv.Kind() == reflect.Slice && rv.IsNil() {
			return nil, unsupported(path, "a nil slice has no gram counterpart")
		}
		items := make([]Value, 0, rv.Len())
		var errs Diagnostics
		for i := 0; i < rv.Len(); i++ {
			v, d := fromNative(rv.Index(i).Interface(), append(append([]PathSegment(nil), path...), Index(i)))
			if d != nil {
				errs = append(errs, d...)
				continue
			}
			items = append(items, v)
		}
		if len(errs) > 0 {
			return nil, errs
		}
		return ArrayValue{items}, nil
	case reflect.Map:
		if rv.Type().Key().Kind() != reflect.String {
			return nil, unsupported(path, "a map key must be a string")
		}
		if rv.IsNil() {
			return nil, unsupported(path, "a nil map has no gram counterpart")
		}
		var entries []Entry
		var errs Diagnostics
		iter := rv.MapRange()
		for iter.Next() {
			k := iter.Key().String()
			v, d := fromNative(iter.Value().Interface(), append(append([]PathSegment(nil), path...), Key(k)))
			if d != nil {
				errs = append(errs, d...)
				continue
			}
			entries = append(entries, Entry{k, v})
		}
		if len(errs) > 0 {
			sortDiagnostics(errs)
			return nil, errs
		}
		m, err := NewMap(entries...)
		if err != nil {
			return nil, at(path, err.(Diagnostics))
		}
		return m, nil
	}
	return nil, unsupported(path, rv.Kind().String()+" has no gram counterpart")
}

// sortDiagnostics orders diagnostics by path text so that results do not depend on map iteration order.
func sortDiagnostics(ds Diagnostics) {
	for i := 1; i < len(ds); i++ {
		for j := i; j > 0 && ds[j-1].Pointer() > ds[j].Pointer(); j-- {
			ds[j-1], ds[j] = ds[j], ds[j-1]
		}
	}
}

// FromNative converts a Go value to a gram value. It accepts bool, string, the integer types, *big.Int,
// float64 (finite), Tagged, Range, Measurement, slices and arrays, and maps with string keys. Anything else
// (nil, NaN, an infinity, float32, a non-string map key, a function, a struct) fails with every problem
// listed, with code "unsupported" or "not-finite" (ADR-006 2.6).
func FromNative(x any) (Value, error) {
	v, errs := fromNative(x, nil)
	if errs != nil {
		return nil, errs
	}
	return v, nil
}
