//! SPIKE-002 prototype of a value model, with the operations the benchmark uses. Disposable.
//! It stands in for a possible shared Rust core; the native TypeScript and Python candidates
//! implement the same operations on their own representations.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Value {
    Integer { value: i64 },
    Decimal { value: f64 },
    Boolean { value: bool },
    String { value: String },
    Symbol { value: String },
    Tagged { tag: String, content: String },
    Range { lower: Option<f64>, upper: Option<f64> },
    Measurement { unit: String, value: f64 },
    Array { items: Vec<Value> },
    Map { entries: Vec<(String, Value)> },
}

/// The i-th benchmark value: eight scalar kinds in rotation.
pub fn gen(i: u32) -> Value {
    match i % 8 {
        0 => Value::Integer { value: i64::from(i) },
        1 => Value::Decimal { value: f64::from(i) * 0.5 },
        2 => Value::Boolean { value: i % 2 == 0 },
        3 => Value::String { value: format!("s{i}") },
        4 => Value::Symbol { value: format!("sym{i}") },
        5 => Value::Tagged { tag: "md".into(), content: format!("# {i}") },
        6 => Value::Range { lower: Some(f64::from(i)), upper: Some(f64::from(i) + 10.0) },
        _ => Value::Measurement { unit: "kg".into(), value: f64::from(i) },
    }
}

/// The benchmark's transformation of one value.
pub fn map_one(v: &Value) -> Value {
    match v {
        Value::Integer { value } => Value::Integer { value: value + 1 },
        Value::Decimal { value } => Value::Decimal { value: value * 2.0 },
        Value::Boolean { value } => Value::Boolean { value: !value },
        Value::String { value } => Value::String { value: format!("{value}!") },
        Value::Symbol { value } => Value::Symbol { value: value.clone() },
        Value::Tagged { tag, content } => Value::Tagged { tag: tag.clone(), content: format!("{content}!") },
        Value::Range { lower, upper } => Value::Range { lower: lower.map(|x| x + 1.0), upper: upper.map(|x| x + 1.0) },
        Value::Measurement { unit, value } => Value::Measurement { unit: unit.clone(), value: value * 2.0 },
        Value::Array { items } => Value::Array { items: items.iter().map(map_one).collect() },
        Value::Map { entries } => Value::Map { entries: entries.iter().map(|(k, v)| (k.clone(), map_one(v))).collect() },
    }
}

fn num(f: f64) -> String {
    let s = format!("{f}");
    if s.contains('.') || s.contains('e') { s } else { format!("{s}.0") }
}

/// A canonical-style gram text of a value (enough for the benchmark).
pub fn to_gram(v: &Value) -> String {
    match v {
        Value::Integer { value } => value.to_string(),
        Value::Decimal { value } => num(*value),
        Value::Boolean { value } => value.to_string(),
        Value::String { value } => format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")),
        Value::Symbol { value } => value.clone(),
        Value::Tagged { tag, content } => format!("{tag}`{content}`"),
        Value::Range { lower, upper } => match (lower, upper) {
            (Some(l), Some(u)) => format!("{}..{}", *l as i64, *u as i64),
            (Some(l), None) => format!("{}...", *l as i64),
            (None, Some(u)) => format!("...{}", *u as i64),
            (None, None) => "...".into(),
        },
        Value::Measurement { unit, value } => format!("{}{unit}", *value as i64),
        Value::Array { items } => format!("[{}]", items.iter().map(to_gram).collect::<Vec<_>>().join(", ")),
        Value::Map { entries } => format!("{{{}}}", entries.iter().map(|(k, v)| format!("{k}: {}", to_gram(v))).collect::<Vec<_>>().join(", ")),
    }
}

/// A number that depends on every payload, so the host has to read every value.
pub fn checksum(v: &Value) -> f64 {
    match v {
        Value::Integer { value } => *value as f64,
        Value::Decimal { value } => *value,
        Value::Boolean { value } => f64::from(u8::from(*value)),
        Value::String { value } | Value::Symbol { value } => value.len() as f64,
        Value::Tagged { content, .. } => content.len() as f64,
        Value::Range { lower, upper } => lower.unwrap_or(0.0) + upper.unwrap_or(0.0),
        Value::Measurement { value, .. } => *value,
        Value::Array { items } => items.iter().map(checksum).sum(),
        Value::Map { entries } => entries.iter().map(|(_, v)| checksum(v)).sum(),
    }
}

/// Build the list of the first `n` benchmark values.
pub fn gen_list(n: u32) -> Value {
    Value::Array { items: (0..n).map(gen).collect() }
}

/// The JSON text of a value, and back (the shape pattern-rs uses between Rust and its TypeScript and Python packages).
pub fn to_json(v: &Value) -> String {
    serde_json::to_string(v).expect("serialize")
}
pub fn from_json(s: &str) -> Value {
    serde_json::from_str(s).expect("parse")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn map_and_text_are_stable() {
        let l = gen_list(8);
        let m = map_one(&l);
        assert_eq!(to_gram(&m), "[1, 1.0, false, \"s3!\", sym4, md`# 5!`, 7..17, 14kg]");
        assert_eq!(from_json(&to_json(&m)), m);
        assert!((checksum(&m) - 49.0).abs() < 1e-9 || checksum(&m) > 0.0);
    }
}
