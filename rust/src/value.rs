//! The eleven kinds of value and their constructors (spec/values.md).

use crate::diagnostic::{Diagnostic, PathSegment};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// A gram value. The set of kinds is closed (ADR-002).
///
/// Variants are declared in kind order, so the derived ordering is the total order of
/// spec/values.md section 6. Equality and hashing compare content part by part.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Value {
    /// A signed 64-bit integer.
    Integer(i64),
    /// An integer outside the 64-bit range.
    BigInteger(BigInteger),
    /// A finite 64-bit float that writes back as the same number.
    Decimal(Decimal),
    /// A decimal a 64-bit float cannot hold exactly.
    BigDecimal(BigDecimal),
    /// True or false.
    Boolean(bool),
    /// Text.
    String(String),
    /// A tag and a content string.
    Tagged(Tagged),
    /// A range with an optional lower and an optional upper end.
    Range(Range),
    /// A magnitude and a unit.
    Measurement(Measurement),
    /// An ordered sequence of values.
    Array(Vec<Value>),
    /// An unordered set of entries with unique string keys.
    Map(Map),
}

/// The kind of a value, in kind order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// See [`Value::Integer`].
    Integer,
    /// See [`Value::BigInteger`].
    BigInteger,
    /// See [`Value::Decimal`].
    Decimal,
    /// See [`Value::BigDecimal`].
    BigDecimal,
    /// See [`Value::Boolean`].
    Boolean,
    /// See [`Value::String`].
    String,
    /// See [`Value::Tagged`].
    Tagged,
    /// See [`Value::Range`].
    Range,
    /// See [`Value::Measurement`].
    Measurement,
    /// See [`Value::Array`].
    Array,
    /// See [`Value::Map`].
    Map,
}

impl Kind {
    /// The kind's name as the conformance vocabulary spells it.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Integer => "integer",
            Kind::BigInteger => "big_integer",
            Kind::Decimal => "decimal",
            Kind::BigDecimal => "big_decimal",
            Kind::Boolean => "boolean",
            Kind::String => "string",
            Kind::Tagged => "tagged",
            Kind::Range => "range",
            Kind::Measurement => "measurement",
            Kind::Array => "array",
            Kind::Map => "map",
        }
    }
}

/// A finite float. Equality and order use the bit pattern's total order, so `-0.0` differs from `0.0`.
#[derive(Debug, Clone, Copy)]
pub struct Decimal(f64);

impl Decimal {
    /// The float.
    pub fn get(self) -> f64 {
        self.0
    }
}
impl PartialEq for Decimal {
    fn eq(&self, o: &Self) -> bool {
        self.0.to_bits() == o.0.to_bits()
    }
}
impl Eq for Decimal {}
impl std::hash::Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.0.to_bits().hash(h)
    }
}
impl PartialOrd for Decimal {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Decimal {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// The exact digits of an integer outside the 64-bit range.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BigInteger(String);

impl BigInteger {
    /// The sign and decimal digits.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl PartialOrd for BigInteger {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for BigInteger {
    fn cmp(&self, o: &Self) -> Ordering {
        let (an, ad) = split_sign(&self.0);
        let (bn, bd) = split_sign(&o.0);
        match (an, bn) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (neg, _) => {
                let m = ad.len().cmp(&bd.len()).then_with(|| ad.cmp(bd));
                if neg {
                    m.reverse()
                } else {
                    m
                }
            }
        }
    }
}

/// The exact digits of a decimal a 64-bit float cannot hold.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BigDecimal(String);

impl BigDecimal {
    /// The sign, digits, point and digits.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl PartialOrd for BigDecimal {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for BigDecimal {
    fn cmp(&self, o: &Self) -> Ordering {
        let (an, ad) = split_sign(&self.0);
        let (bn, bd) = split_sign(&o.0);
        match (an, bn) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (neg, _) => {
                let (ai, af) = ad.split_once('.').unwrap_or((ad, ""));
                let (bi, bf) = bd.split_once('.').unwrap_or((bd, ""));
                let m = ai
                    .len()
                    .cmp(&bi.len())
                    .then_with(|| ai.cmp(bi))
                    .then_with(|| af.cmp(bf));
                if neg {
                    m.reverse()
                } else {
                    m
                }
            }
        }
    }
}

fn split_sign(s: &str) -> (bool, &str) {
    match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    }
}

/// A tag and its content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tagged {
    tag: String,
    content: String,
}

impl Tagged {
    /// The tag, a symbol other than `true` and `false`.
    pub fn tag(&self) -> &str {
        &self.tag
    }
    /// The content string, as written.
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// A range: inclusive, with an optional lower end and an optional upper end.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Range {
    lower: Option<Box<Value>>,
    upper: Option<Box<Value>>,
}

impl Range {
    /// The lower end, if any.
    pub fn lower(&self) -> Option<&Value> {
        self.lower.as_deref()
    }
    /// The upper end, if any.
    pub fn upper(&self) -> Option<&Value> {
        self.upper.as_deref()
    }
}

/// A magnitude and a unit.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Measurement {
    magnitude: Box<Value>,
    unit: String,
}

impl Measurement {
    /// The magnitude: an integer, big integer, decimal or big decimal.
    pub fn magnitude(&self) -> &Value {
        &self.magnitude
    }
    /// The unit, ASCII letters.
    pub fn unit(&self) -> &str {
        &self.unit
    }
}

/// An unordered set of entries with unique string keys; iteration is in key (UTF-8 byte) order.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Map(BTreeMap<String, Value>);

impl Map {
    /// The value of a key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }
    /// The entries in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }
    /// The number of entries.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// True when there are no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn is_symbol(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '@'))
}

/// True when `s` matches the symbol rule and is not `true` or `false`.
pub(crate) fn is_bare_word(s: &str) -> bool {
    is_symbol(s) && s != "true" && s != "false"
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `-?(0|[1-9]\d*)`
fn is_integer_text(s: &str) -> bool {
    let (_, d) = split_sign(s);
    is_digits(d) && (d == "0" || !d.starts_with('0')) && s != "-0"
}

/// `-?(0|[1-9]\d*)\.\d+`
fn is_decimal_text(s: &str) -> bool {
    let (_, d) = split_sign(s);
    match d.split_once('.') {
        Some((i, f)) => is_digits(i) && (i == "0" || !i.starts_with('0')) && is_digits(f),
        None => false,
    }
}

/// Drop trailing zeros after the point, keeping at least one digit.
fn normalize_decimal(s: &str) -> String {
    let (int, frac) = s.split_once('.').expect("decimal text has a point");
    let frac = frac.trim_end_matches('0');
    format!("{int}.{}", if frac.is_empty() { "0" } else { frac })
}

/// The shortest positional text of a float, with at least one digit after the point.
pub(crate) fn float_text(f: f64) -> String {
    let t = format!("{f}");
    if t.contains('.') {
        t
    } else {
        format!("{t}.0")
    }
}

impl Value {
    /// The kind of this value.
    pub fn kind(&self) -> Kind {
        match self {
            Value::Integer(_) => Kind::Integer,
            Value::BigInteger(_) => Kind::BigInteger,
            Value::Decimal(_) => Kind::Decimal,
            Value::BigDecimal(_) => Kind::BigDecimal,
            Value::Boolean(_) => Kind::Boolean,
            Value::String(_) => Kind::String,
            Value::Tagged(_) => Kind::Tagged,
            Value::Range(_) => Kind::Range,
            Value::Measurement(_) => Kind::Measurement,
            Value::Array(_) => Kind::Array,
            Value::Map(_) => Kind::Map,
        }
    }

    /// True for every kind except array and map.
    pub fn is_scalar(&self) -> bool {
        !matches!(self, Value::Array(_) | Value::Map(_))
    }

    /// For an array, whether all elements have one kind (true for an empty array); false for other kinds.
    pub fn is_homogeneous_array(&self) -> bool {
        match self {
            Value::Array(items) => items.windows(2).all(|w| w[0].kind() == w[1].kind()),
            _ => false,
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(
            self,
            Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_) | Value::BigDecimal(_)
        )
    }

    /// An `integer`.
    pub fn integer(n: i64) -> Value {
        Value::Integer(n)
    }

    /// An integer from `-?(0|[1-9]\d*)`: an `integer` if it fits in 64 bits, else a `big_integer`.
    pub fn integer_from_digits(text: &str) -> Result<Value, Diagnostic> {
        if !is_integer_text(text) {
            return Err(Diagnostic::new(
                "invalid-integer",
                format!("{text:?} is not an integer without leading zeros"),
            ));
        }
        Ok(match text.parse::<i64>() {
            Ok(n) => Value::Integer(n),
            Err(_) => Value::BigInteger(BigInteger(text.to_string())),
        })
    }

    /// A `big_integer`. The digits must be outside the 64-bit range.
    pub fn big_integer(text: &str) -> Result<Value, Diagnostic> {
        match Value::integer_from_digits(text)? {
            v @ Value::BigInteger(_) => Ok(v),
            _ => Err(Diagnostic::new(
                "invalid-integer",
                format!("{text} fits in 64 bits, so it is an integer"),
            )),
        }
    }

    /// A `decimal`. `NaN` and the infinities are rejected (`not-finite`).
    pub fn decimal(f: f64) -> Result<Value, Diagnostic> {
        if f.is_finite() {
            Ok(Value::Decimal(Decimal(f)))
        } else {
            Err(Diagnostic::new("not-finite", "a decimal must be finite"))
        }
    }

    /// A decimal from `-?(0|[1-9]\d*)\.\d+`: a `decimal` when the nearest 64-bit float writes back as the
    /// same number (trailing zeros after the point ignored), else a `big_decimal`.
    pub fn decimal_from_text(text: &str) -> Result<Value, Diagnostic> {
        if !is_decimal_text(text) {
            return Err(Diagnostic::new(
                "invalid-decimal",
                format!("{text:?} is not a decimal"),
            ));
        }
        let normalized = normalize_decimal(text);
        let f: f64 = normalized.parse().map_err(|_| {
            Diagnostic::new("invalid-decimal", format!("{text:?} is not a decimal"))
        })?;
        if f.is_finite() && float_text(f) == normalized {
            Ok(Value::Decimal(Decimal(f)))
        } else {
            Ok(Value::BigDecimal(BigDecimal(normalized)))
        }
    }

    /// A `big_decimal`. The text must be normalized (no trailing zeros after the point beyond one digit)
    /// and must not be a number a `decimal` can hold.
    pub fn big_decimal(text: &str) -> Result<Value, Diagnostic> {
        if is_decimal_text(text) && normalize_decimal(text) == text {
            if let Value::BigDecimal(_) = Value::decimal_from_text(text)? {
                return Ok(Value::BigDecimal(BigDecimal(text.to_string())));
            }
        }
        Err(Diagnostic::new(
            "invalid-decimal",
            format!("{text:?} is not a normalized decimal beyond a 64-bit float"),
        ))
    }

    /// A `boolean`.
    pub fn boolean(b: bool) -> Value {
        Value::Boolean(b)
    }

    /// A `string`.
    pub fn string(s: impl Into<String>) -> Value {
        Value::String(s.into())
    }

    /// A `tagged` value. The tag must be a symbol other than `true` and `false` (`invalid-tag`).
    pub fn tagged(tag: &str, content: impl Into<String>) -> Result<Value, Diagnostic> {
        if !is_bare_word(tag) {
            return Err(Diagnostic::new(
                "invalid-tag",
                format!("{tag:?} is not a symbol other than true or false"),
            ));
        }
        Ok(Value::Tagged(Tagged {
            tag: tag.to_string(),
            content: content.into(),
        }))
    }

    /// A `range`. Each end must be numeric or a measurement (`invalid-range-end`) and at least one must be present (`no-ends`).
    pub fn range(lower: Option<Value>, upper: Option<Value>) -> Result<Value, Diagnostic> {
        if lower.is_none() && upper.is_none() {
            return Err(Diagnostic::new("no-ends", "a range needs at least one end"));
        }
        for end in lower.iter().chain(upper.iter()) {
            if !end.is_numeric() && !matches!(end, Value::Measurement(_)) {
                return Err(Diagnostic::new(
                    "invalid-range-end",
                    format!(
                        "a range end must be numeric or a measurement, not {}",
                        end.kind().name()
                    ),
                ));
            }
        }
        Ok(Value::Range(Range {
            lower: lower.map(Box::new),
            upper: upper.map(Box::new),
        }))
    }

    /// A `measurement`. The magnitude must be numeric and the unit ASCII letters (`invalid-unit`).
    pub fn measurement(magnitude: Value, unit: &str) -> Result<Value, Diagnostic> {
        if !magnitude.is_numeric() {
            return Err(Diagnostic::new(
                "invalid-range-end",
                format!(
                    "a magnitude must be numeric, not {}",
                    magnitude.kind().name()
                ),
            ));
        }
        if unit.is_empty() || !unit.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(Diagnostic::new(
                "invalid-unit",
                format!("{unit:?} is not one or more ASCII letters"),
            ));
        }
        Ok(Value::Measurement(Measurement {
            magnitude: Box::new(magnitude),
            unit: unit.to_string(),
        }))
    }

    /// An `array`.
    pub fn array(items: impl IntoIterator<Item = Value>) -> Value {
        Value::Array(items.into_iter().collect())
    }

    /// A `map`. A repeated key is an error (`duplicate-key`) that names the key.
    pub fn map<K: Into<String>>(
        entries: impl IntoIterator<Item = (K, Value)>,
    ) -> Result<Value, Diagnostic> {
        let mut map = BTreeMap::new();
        for (k, v) in entries {
            let k: String = k.into();
            if map.contains_key(&k) {
                return Err(
                    Diagnostic::new("duplicate-key", format!("the key {k:?} is repeated"))
                        .at(vec![PathSegment::Key(k)]),
                );
            }
            map.insert(k, v);
        }
        Ok(Value::Map(Map(map)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_distinguishes_kinds_and_spelling_of_numbers() {
        let i = Value::integer(1);
        let d = Value::decimal(1.0).unwrap();
        assert_ne!(i, d);
        assert_ne!(Value::decimal(-0.0).unwrap(), Value::decimal(0.0).unwrap());
        assert!(Value::decimal(-0.0).unwrap() < Value::decimal(0.0).unwrap());
    }

    #[test]
    fn maps_are_equal_whatever_the_entry_order() {
        let a = Value::map([("a", Value::integer(1)), ("b", Value::integer(2))]).unwrap();
        let b = Value::map([("b", Value::integer(2)), ("a", Value::integer(1))]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn constructors_report_errors_with_codes() {
        assert_eq!(Value::decimal(f64::NAN).unwrap_err().code, "not-finite");
        assert_eq!(Value::tagged("true", "x").unwrap_err().code, "invalid-tag");
        assert_eq!(Value::tagged("a b", "x").unwrap_err().code, "invalid-tag");
        assert_eq!(Value::range(None, None).unwrap_err().code, "no-ends");
        assert_eq!(
            Value::range(Some(Value::string("x")), None)
                .unwrap_err()
                .code,
            "invalid-range-end"
        );
        assert_eq!(
            Value::measurement(Value::integer(1), "k g")
                .unwrap_err()
                .code,
            "invalid-unit"
        );
        assert_eq!(Value::big_integer("5").unwrap_err().code, "invalid-integer");
        assert_eq!(
            Value::big_decimal("0.1").unwrap_err().code,
            "invalid-decimal"
        );
        let dup = Value::map([("a", Value::integer(1)), ("a", Value::integer(2))]).unwrap_err();
        assert_eq!(dup.code, "duplicate-key");
        assert_eq!(dup.path, vec![PathSegment::Key("a".into())]);
    }

    #[test]
    fn big_numbers_order_numerically_within_kind() {
        let a = Value::integer_from_digits("9223372036854775808").unwrap();
        let b = Value::integer_from_digits("92233720368547758080").unwrap();
        let n = Value::integer_from_digits("-9223372036854775809").unwrap();
        assert!(n < a && a < b);
        assert!(
            Value::integer(i64::MAX) < a,
            "kind order puts every big_integer after every integer"
        );
    }

    #[test]
    fn homogeneous_arrays() {
        assert!(Value::array([]).is_homogeneous_array());
        assert!(Value::array([Value::integer(1), Value::integer(2)]).is_homogeneous_array());
        assert!(!Value::array([Value::integer(1), Value::string("a")]).is_homogeneous_array());
    }
}
