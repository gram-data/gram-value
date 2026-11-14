package gramvalue

import (
	"strconv"
	"strings"
)

// Context says whether the outermost map is an ordinary value or an element's property record.
type Context int

const (
	// ContextValue is an ordinary value: map entries hold scalars or arrays.
	ContextValue Context = iota
	// ContextRecord is an element's property record: a property may also hold a map of scalars and arrays.
	ContextRecord
)

func escape(s string, quote rune) string {
	var b strings.Builder
	for _, c := range s {
		switch c {
		case '\\':
			b.WriteString(`\\`)
		case '\b':
			b.WriteString(`\b`)
		case '\f':
			b.WriteString(`\f`)
		case '\n':
			b.WriteString(`\n`)
		case '\r':
			b.WriteString(`\r`)
		case '\t':
			b.WriteString(`\t`)
		default:
			if c == quote {
				b.WriteByte('\\')
			}
			b.WriteRune(c)
		}
	}
	return b.String()
}

// StringText is a string in canonical text: the bare word when it can be, else double-quoted.
func StringText(s string) string {
	if IsBareWord(s) {
		return s
	}
	return `"` + escape(s, '"') + `"`
}

// scalarText is the text of a scalar, or the reason it has none.
func scalarText(v Value) (text, reason string) {
	switch x := v.(type) {
	case IntegerValue:
		return strconv.FormatInt(x.v, 10), ""
	case BigIntegerValue:
		return x.v.String(), ""
	case DecimalValue:
		return FloatText(x.v), ""
	case BigDecimalValue:
		return x.digits, ""
	case BooleanValue:
		if x.v {
			return "true", ""
		}
		return "false", ""
	case StringValue:
		return StringText(x.v), ""
	case TaggedValue:
		return x.tag + "`" + escape(x.content, '`') + "`", ""
	case MeasurementValue:
		mag, why := scalarText(x.magnitude)
		if why != "" {
			return "", why
		}
		if mag == "0" && strings.HasPrefix(x.unit, "x") {
			return "", "a measurement of 0 with a unit beginning x can read as a hexadecimal literal"
		}
		return mag + x.unit, ""
	case RangeValue:
		var lo, hi string
		for i, end := range []Value{x.lower, x.upper} {
			if end == nil {
				continue
			}
			t, why := scalarText(end)
			if why != "" {
				return "", why
			}
			if i == 0 {
				lo = t
			} else {
				hi = t
			}
		}
		switch {
		case x.lower != nil && x.upper != nil:
			return lo + ".." + hi, ""
		case x.lower != nil:
			return lo + "...", ""
		case x.upper != nil:
			return "..." + hi, ""
		}
		return "", "a range with no ends"
	}
	return "", "not a scalar"
}

func unwritable(path []PathSegment, what string) Diagnostics {
	return at(path, diag("unwritable", what))
}

func writeArray(items []Value, path []PathSegment, errs *Diagnostics) string {
	parts := make([]string, 0, len(items))
	for i, item := range items {
		p := append(append([]PathSegment(nil), path...), Index(i))
		switch item.Kind() {
		case KindArray:
			*errs = append(*errs, unwritable(p, "an array cannot hold an array")...)
		case KindMap:
			*errs = append(*errs, unwritable(p, "an array cannot hold a map")...)
		default:
			if t, why := scalarText(item); why != "" {
				*errs = append(*errs, unwritable(p, why)...)
			} else {
				parts = append(parts, t)
			}
		}
	}
	return "[" + strings.Join(parts, ", ") + "]"
}

func writeMap(m MapValue, nestedOK bool, path []PathSegment, errs *Diagnostics) string {
	parts := make([]string, 0, len(m.entries))
	for _, e := range m.entries {
		p := append(append([]PathSegment(nil), path...), Key(e.Key))
		var text string
		ok := true
		switch x := e.Value.(type) {
		case ArrayValue:
			text = writeArray(x.items, p, errs)
		case MapValue:
			if nestedOK {
				text = writeMap(x, false, p, errs)
			} else {
				*errs = append(*errs, unwritable(p, "a map entry cannot hold a map")...)
				ok = false
			}
		default:
			t, why := scalarText(e.Value)
			if why != "" {
				*errs = append(*errs, unwritable(p, why)...)
				ok = false
			}
			text = t
		}
		if ok {
			parts = append(parts, StringText(e.Key)+": "+text)
		}
	}
	return "{" + strings.Join(parts, ", ") + "}"
}

// ToText is the canonical text of a value. A part the grammar has no text for is reported with code
// "unwritable" and its path; the error holds every such part.
func ToText(v Value, context Context) (string, error) {
	var errs Diagnostics
	var text string
	switch x := v.(type) {
	case ArrayValue:
		text = writeArray(x.items, nil, &errs)
	case MapValue:
		text = writeMap(x, context == ContextRecord, nil, &errs)
	default:
		t, why := scalarText(v)
		if why != "" {
			errs = append(errs, unwritable(nil, why)...)
		}
		text = t
	}
	if len(errs) > 0 {
		return "", errs
	}
	return text, nil
}
