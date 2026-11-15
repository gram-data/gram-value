//! Conversions between [`Value`] and Rust's own types (ADR-006 2.7).
//!
//! `From` builds a value; `TryFrom<&Value>` extracts a native type. Every Rust entry is exact or
//! unsupported, so a conversion never reports a loss. A failure is a [`Diagnostics`] holding every
//! problem found, each with its path.

use crate::diagnostic::{Diagnostic, PathSegment};
use crate::tags::{interpret_tag, Embedding, Interpretation, Typed};
use crate::value::{BigDecimal, BigInteger, Measurement, Range, Tagged, Value};
use std::collections::BTreeMap;
use std::fmt;

/// Every problem found by a failed conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    /// The problems, in the order found.
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.0
    }

    /// The problems, consumed.
    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.0
    }

    fn one(code: &'static str, message: String) -> Self {
        Diagnostics(vec![Diagnostic::new(code, message)])
    }

    /// Put `segment` in front of every path (the conversion of a part of a larger value).
    fn under(mut self, segment: PathSegment) -> Self {
        for d in &mut self.0 {
            d.path.insert(0, segment.clone());
        }
        self
    }
}

impl From<Diagnostic> for Diagnostics {
    fn from(d: Diagnostic) -> Self {
        Diagnostics(vec![d])
    }
}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, d) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostics {}

/// A type that can be extracted from a [`Value`]. It is the bound for the elements of `Vec<T>`
/// and `BTreeMap<String, T>`.
pub trait FromValue: Sized {
    /// Extract `Self`, or report every problem.
    fn from_value(value: &Value) -> Result<Self, Diagnostics>;
}

fn mismatch(want: &str, got: &Value) -> Diagnostics {
    Diagnostics::one(
        "type-mismatch",
        format!("expected {want}, found {}", got.kind().name()),
    )
}

fn out_of_range(want: &str, got: &Value) -> Diagnostics {
    Diagnostics::one(
        "out-of-range",
        format!("the {} does not fit {want}", got.kind().name()),
    )
}

macro_rules! native {
    ($ty:ty, |$v:ident| $body:expr) => {
        impl FromValue for $ty {
            fn from_value($v: &Value) -> Result<Self, Diagnostics> {
                $body
            }
        }
        impl TryFrom<&Value> for $ty {
            type Error = Diagnostics;
            fn try_from(v: &Value) -> Result<Self, Diagnostics> {
                <$ty as FromValue>::from_value(v)
            }
        }
    };
}

macro_rules! signed_unsigned {
    ($($ty:ty),*) => {$(
        native!($ty, |v| match v {
            Value::Integer(n) => <$ty>::try_from(*n).map_err(|_| out_of_range(stringify!($ty), v)),
            Value::BigInteger(b) => b.as_str().parse::<$ty>().map_err(|_| out_of_range(stringify!($ty), v)),
            other => Err(mismatch("an integer", other)),
        });
    )*};
}

signed_unsigned!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

native!(f64, |v| match v {
    Value::Decimal(d) => Ok(d.get()),
    Value::BigDecimal(_) => Err(out_of_range("f64", v)),
    other => Err(mismatch("a decimal", other)),
});
native!(bool, |v| match v {
    Value::Boolean(b) => Ok(*b),
    other => Err(mismatch("a boolean", other)),
});
native!(String, |v| match v {
    Value::String(s) => Ok(s.clone()),
    other => Err(mismatch("a string", other)),
});
native!(Tagged, |v| match v {
    Value::Tagged(t) => Ok(t.clone()),
    other => Err(mismatch("a tagged string", other)),
});
native!(Range, |v| match v {
    Value::Range(r) => Ok(r.clone()),
    other => Err(mismatch("a range", other)),
});
native!(Measurement, |v| match v {
    Value::Measurement(m) => Ok(m.clone()),
    other => Err(mismatch("a measurement", other)),
});
native!(BigInteger, |v| match v {
    Value::BigInteger(b) => Ok(b.clone()),
    other => Err(mismatch("a big integer", other)),
});
native!(BigDecimal, |v| match v {
    Value::BigDecimal(b) => Ok(b.clone()),
    other => Err(mismatch("a big decimal", other)),
});
native!(Embedding, |v| match v {
    Value::Tagged(t) if t.tag() == "embedding" => match interpret_tag("embedding", t.content()) {
        Ok(Some(Interpretation {
            typed: Typed::Embedding(emb),
            ..
        })) => Ok(emb),
        Ok(_) => unreachable!("embedding tag always produces Typed::Embedding"),
        Err(d) => Err(Diagnostics::from(d)),
    },
    other => Err(mismatch("an embedding", other)),
});
native!(Value, |v| Ok(v.clone()));

impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(v: &Value) -> Result<Self, Diagnostics> {
        let Value::Array(items) = v else {
            return Err(mismatch("an array", v));
        };
        let mut out = Vec::with_capacity(items.len());
        let mut errs = Vec::new();
        for (i, item) in items.iter().enumerate() {
            match T::from_value(item) {
                Ok(t) => out.push(t),
                Err(e) => errs.extend(e.under(PathSegment::Index(i))),
            }
        }
        if errs.is_empty() {
            Ok(out)
        } else {
            Err(Diagnostics(errs))
        }
    }
}

impl<T: FromValue> TryFrom<&Value> for Vec<T> {
    type Error = Diagnostics;
    fn try_from(v: &Value) -> Result<Self, Diagnostics> {
        <Vec<T> as FromValue>::from_value(v)
    }
}

impl<T: FromValue> FromValue for BTreeMap<String, T> {
    fn from_value(v: &Value) -> Result<Self, Diagnostics> {
        let Value::Map(map) = v else {
            return Err(mismatch("a map", v));
        };
        let mut out = BTreeMap::new();
        let mut errs = Vec::new();
        for (k, item) in map.iter() {
            match T::from_value(item) {
                Ok(t) => {
                    out.insert(k.clone(), t);
                }
                Err(e) => errs.extend(e.under(PathSegment::Key(k.clone()))),
            }
        }
        if errs.is_empty() {
            Ok(out)
        } else {
            Err(Diagnostics(errs))
        }
    }
}

impl<T: FromValue> TryFrom<&Value> for BTreeMap<String, T> {
    type Error = Diagnostics;
    fn try_from(v: &Value) -> Result<Self, Diagnostics> {
        <BTreeMap<String, T> as FromValue>::from_value(v)
    }
}

// ---- building values ------------------------------------------------------------------------

macro_rules! from_small_int {
    ($($ty:ty),*) => {$(
        impl From<$ty> for Value {
            fn from(n: $ty) -> Value {
                Value::Integer(i64::from(n))
            }
        }
    )*};
}
from_small_int!(i8, i16, i32, i64, u8, u16, u32);

macro_rules! from_wide_int {
    ($($ty:ty),*) => {$(
        impl From<$ty> for Value {
            fn from(n: $ty) -> Value {
                Value::integer_from_digits(&n.to_string()).expect("an integer's digits are valid")
            }
        }
    )*};
}
from_wide_int!(i128, isize, u64, u128, usize);

impl TryFrom<f64> for Value {
    type Error = Diagnostics;
    /// A `decimal`; `NaN` and the infinities fail with `not-finite`.
    fn try_from(f: f64) -> Result<Value, Diagnostics> {
        Value::decimal(f).map_err(Diagnostics::from)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Value {
        Value::Boolean(b)
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::String(s.to_string())
    }
}
impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::String(s)
    }
}
impl From<Tagged> for Value {
    fn from(t: Tagged) -> Value {
        Value::Tagged(t)
    }
}
impl From<Range> for Value {
    fn from(r: Range) -> Value {
        Value::Range(r)
    }
}
impl From<Measurement> for Value {
    fn from(m: Measurement) -> Value {
        Value::Measurement(m)
    }
}
impl From<BigInteger> for Value {
    fn from(b: BigInteger) -> Value {
        Value::BigInteger(b)
    }
}
impl From<BigDecimal> for Value {
    fn from(b: BigDecimal) -> Value {
        Value::BigDecimal(b)
    }
}
impl From<Embedding> for Value {
    fn from(e: Embedding) -> Value {
        e.to_value()
    }
}
impl From<&Embedding> for Value {
    fn from(e: &Embedding) -> Value {
        e.to_value()
    }
}
impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(items: Vec<T>) -> Value {
        Value::Array(items.into_iter().map(Into::into).collect())
    }
}
impl<T: Into<Value>> From<BTreeMap<String, T>> for Value {
    fn from(entries: BTreeMap<String, T>) -> Value {
        Value::map(entries.into_iter().map(|(k, v)| (k, v.into())))
            .expect("a BTreeMap has unique keys")
    }
}

impl Interpretation {
    /// The tagged value with this interpretation's normal content string (spec/tags.md section 4).
    pub fn to_value(&self) -> Value {
        Value::tagged(self.tag, self.normal_content.clone()).expect("a known tag is a symbol")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_extract_to_any_type_that_holds_them() {
        let v = Value::integer(300);
        assert_eq!(i64::try_from(&v).unwrap(), 300);
        assert_eq!(u16::try_from(&v).unwrap(), 300);
        assert_eq!(
            u8::try_from(&v).unwrap_err().as_slice()[0].code,
            "out-of-range"
        );
        let big = Value::from(u64::MAX);
        assert!(matches!(big, Value::BigInteger(_)));
        assert_eq!(u64::try_from(&big).unwrap(), u64::MAX);
        assert_eq!(
            i64::try_from(&big).unwrap_err().as_slice()[0].code,
            "out-of-range"
        );
    }

    #[test]
    fn a_wrong_kind_is_a_type_mismatch() {
        let v = Value::string("x");
        assert_eq!(
            i64::try_from(&v).unwrap_err().as_slice()[0].code,
            "type-mismatch"
        );
        assert_eq!(
            f64::try_from(&Value::integer(1)).unwrap_err().as_slice()[0].code,
            "type-mismatch"
        );
    }

    #[test]
    fn arrays_and_maps_report_every_failing_part_with_its_path() {
        let v = Value::array([Value::integer(1), Value::string("a"), Value::boolean(true)]);
        let err = Vec::<i64>::try_from(&v).unwrap_err();
        let paths: Vec<_> = err.as_slice().iter().map(|d| d.path.clone()).collect();
        assert_eq!(
            paths,
            vec![vec![PathSegment::Index(1)], vec![PathSegment::Index(2)]]
        );

        let m = Value::map([("a", Value::integer(1)), ("b", Value::string("x"))]).unwrap();
        let err = BTreeMap::<String, i64>::try_from(&m).unwrap_err();
        assert_eq!(err.as_slice()[0].path, vec![PathSegment::Key("b".into())]);
    }

    #[test]
    fn building_values_from_native_types() {
        assert_eq!(Value::from(42i32), Value::integer(42));
        assert_eq!(Value::from(vec![1i64, 2]).to_text().unwrap(), "[1, 2]");
        assert_eq!(
            Value::try_from(f64::NAN).unwrap_err().as_slice()[0].code,
            "not-finite"
        );
        let mut m = BTreeMap::new();
        m.insert("b".to_string(), 1i64);
        m.insert("a".to_string(), 2i64);
        assert_eq!(Value::from(m).to_text().unwrap(), "{a: 2, b: 1}");
    }
}
