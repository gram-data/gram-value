//! Canonical gram text (spec/text.md).

use crate::diagnostic::{Diagnostic, PathSegment};
use crate::value::{float_text, is_bare_word, Value};

/// Whether the outermost map is an ordinary value or an element's property record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// An ordinary value: map entries hold scalars or arrays.
    Value,
    /// An element's property record: a property may also hold a map of scalars and arrays.
    Record,
}

/// A string in canonical text: the bare word when it can be, else double-quoted.
pub fn string_text(s: &str) -> String {
    if is_bare_word(s) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    escape_into(s, '"', &mut out);
    out.push('"');
    out
}

fn escape_into(s: &str, quote: char, out: &mut String) {
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
}

impl Value {
    /// The canonical text of this value, or an `unwritable` diagnostic for each part the grammar has no text for.
    pub fn to_text(&self) -> Result<String, Vec<Diagnostic>> {
        self.to_text_in(Context::Value)
    }

    /// The canonical text, with the outermost map written as an element's record when `context` says so.
    pub fn to_text_in(&self, context: Context) -> Result<String, Vec<Diagnostic>> {
        let mut out = String::new();
        let mut errs = Vec::new();
        write_value(self, context, &mut Vec::new(), &mut out, &mut errs);
        if errs.is_empty() {
            Ok(out)
        } else {
            Err(errs)
        }
    }
}

fn unwritable(path: &[PathSegment], what: &str) -> Diagnostic {
    Diagnostic::new("unwritable", what.to_string()).at(path.to_vec())
}

/// Text of a scalar, or the reason it has none.
fn scalar_text(v: &Value) -> Result<String, String> {
    Ok(match v {
        Value::Integer(n) => n.to_string(),
        Value::BigInteger(b) => b.as_str().to_string(),
        Value::Decimal(d) => float_text(d.get()),
        Value::BigDecimal(b) => b.as_str().to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::String(s) => string_text(s),
        Value::Tagged(t) => {
            let mut out = String::new();
            out.push_str(t.tag());
            out.push('`');
            escape_into(t.content(), '`', &mut out);
            out.push('`');
            out
        }
        Value::Measurement(m) => {
            let mag = scalar_text(m.magnitude())?;
            if mag == "0" && m.unit().starts_with('x') {
                return Err(
                    "a measurement of 0 with a unit beginning x can read as a hexadecimal literal"
                        .into(),
                );
            }
            format!("{mag}{}", m.unit())
        }
        Value::Range(r) => match (r.lower(), r.upper()) {
            (Some(l), Some(u)) => format!("{}..{}", scalar_text(l)?, scalar_text(u)?),
            (Some(l), None) => format!("{}...", scalar_text(l)?),
            (None, Some(u)) => format!("...{}", scalar_text(u)?),
            (None, None) => return Err("a range with no ends".into()),
        },
        Value::Array(_) | Value::Map(_) => return Err("not a scalar".into()),
    })
}

fn write_array(
    items: &[Value],
    path: &mut Vec<PathSegment>,
    out: &mut String,
    errs: &mut Vec<Diagnostic>,
) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        path.push(PathSegment::Index(i));
        match item {
            Value::Array(_) => errs.push(unwritable(path, "an array cannot hold an array")),
            Value::Map(_) => errs.push(unwritable(path, "an array cannot hold a map")),
            scalar => match scalar_text(scalar) {
                Ok(t) => out.push_str(&t),
                Err(why) => errs.push(unwritable(path, &why)),
            },
        }
        path.pop();
    }
    out.push(']');
}

fn write_map(
    map: &crate::value::Map,
    nested_ok: bool,
    path: &mut Vec<PathSegment>,
    out: &mut String,
    errs: &mut Vec<Diagnostic>,
) {
    out.push('{');
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&string_text(k));
        out.push_str(": ");
        path.push(PathSegment::Key(k.clone()));
        match v {
            Value::Array(items) => write_array(items, path, out, errs),
            Value::Map(inner) if nested_ok => write_map(inner, false, path, out, errs),
            Value::Map(_) => errs.push(unwritable(path, "a map entry cannot hold a map")),
            scalar => match scalar_text(scalar) {
                Ok(t) => out.push_str(&t),
                Err(why) => errs.push(unwritable(path, &why)),
            },
        }
        path.pop();
    }
    out.push('}');
}

fn write_value(
    v: &Value,
    context: Context,
    path: &mut Vec<PathSegment>,
    out: &mut String,
    errs: &mut Vec<Diagnostic>,
) {
    match v {
        Value::Array(items) => write_array(items, path, out, errs),
        Value::Map(map) => write_map(map, context == Context::Record, path, out, errs),
        scalar => match scalar_text(scalar) {
            Ok(t) => out.push_str(&t),
            Err(why) => errs.push(unwritable(path, &why)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwritable_parts_are_reported_with_paths() {
        let v = Value::array([Value::integer(1), Value::array([Value::integer(2)])]);
        let errs = v.to_text().unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, "unwritable");
        assert_eq!(errs[0].path, vec![PathSegment::Index(1)]);

        let x = Value::measurement(Value::integer(0), "xFF").unwrap();
        assert_eq!(x.to_text().unwrap_err()[0].code, "unwritable");
    }

    #[test]
    fn a_record_may_hold_a_map_one_level_deep() {
        let inner = Value::map([("x", Value::integer(1))]).unwrap();
        let rec = Value::map([("k", inner.clone())]).unwrap();
        assert!(rec.to_text().is_err());
        assert_eq!(rec.to_text_in(Context::Record).unwrap(), "{k: {x: 1}}");
        let deep = Value::map([("k", Value::map([("j", inner)]).unwrap())]).unwrap();
        assert!(deep.to_text_in(Context::Record).is_err());
    }

    #[test]
    fn decimals_never_use_an_exponent() {
        assert_eq!(
            Value::decimal(1e21).unwrap().to_text().unwrap(),
            "1000000000000000000000.0"
        );
        assert_eq!(
            Value::decimal(1e-7).unwrap().to_text().unwrap(),
            "0.0000001"
        );
    }
}
