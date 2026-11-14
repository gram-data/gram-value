//! SPIKE-002: the Rust core behind a WebAssembly boundary. JavaScript holds opaque handles.
use bench_core::{checksum, from_json, gen_list, map_one, to_gram, to_json, Value};
use wasm_bindgen::convert::TryFromJsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WValue {
    inner: Value,
}

#[wasm_bindgen]
impl WValue {
    pub fn integer(v: i64) -> WValue { WValue { inner: Value::Integer { value: v } } }
    pub fn decimal(v: f64) -> WValue { WValue { inner: Value::Decimal { value: v } } }
    pub fn boolean(v: bool) -> WValue { WValue { inner: Value::Boolean { value: v } } }
    pub fn string(v: String) -> WValue { WValue { inner: Value::String { value: v } } }
    pub fn symbol(v: String) -> WValue { WValue { inner: Value::Symbol { value: v } } }
    pub fn tagged(tag: String, content: String) -> WValue { WValue { inner: Value::Tagged { tag, content } } }
    pub fn range(lower: f64, upper: f64) -> WValue { WValue { inner: Value::Range { lower: Some(lower), upper: Some(upper) } } }
    pub fn measurement(unit: String, value: f64) -> WValue { WValue { inner: Value::Measurement { unit, value } } }

    pub fn kind(&self) -> u8 {
        match &self.inner {
            Value::Integer { .. } => 0, Value::Decimal { .. } => 1, Value::Boolean { .. } => 2, Value::String { .. } => 3,
            Value::Symbol { .. } => 4, Value::Tagged { .. } => 5, Value::Range { .. } => 6, Value::Measurement { .. } => 7,
            Value::Array { .. } => 8, Value::Map { .. } => 9,
        }
    }
    /// The number the value carries (the integer, decimal, boolean, range lower bound or measurement magnitude).
    pub fn number(&self) -> f64 {
        match &self.inner {
            Value::Integer { value } => *value as f64, Value::Decimal { value } => *value,
            Value::Boolean { value } => f64::from(u8::from(*value)), Value::Range { lower, .. } => lower.unwrap_or(0.0),
            Value::Measurement { value, .. } => *value, _ => 0.0,
        }
    }
    pub fn number2(&self) -> f64 {
        match &self.inner { Value::Range { upper, .. } => upper.unwrap_or(0.0), _ => 0.0 }
    }
    /// The first text the value carries (string, symbol, tag, or unit).
    pub fn text(&self) -> String {
        match &self.inner {
            Value::String { value } | Value::Symbol { value } => value.clone(),
            Value::Tagged { tag, .. } => tag.clone(), Value::Measurement { unit, .. } => unit.clone(), _ => String::new(),
        }
    }
    pub fn text2(&self) -> String {
        match &self.inner { Value::Tagged { content, .. } => content.clone(), _ => String::new() }
    }
}

#[wasm_bindgen]
pub struct WList {
    items: Vec<Value>,
}

#[wasm_bindgen]
impl WList {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WList { WList { items: Vec::new() } }
    pub fn push(&mut self, v: &WValue) { self.items.push(v.inner.clone()); }
    pub fn len(&self) -> usize { self.items.len() }
    pub fn get(&self, i: usize) -> WValue { WValue { inner: self.items[i].clone() } }
    /// A general map: the callback runs in JavaScript for each element.
    pub fn map_with(&self, f: &js_sys::Function) -> Result<WList, JsValue> {
        let mut out = Vec::with_capacity(self.items.len());
        for item in &self.items {
            let r = f.call1(&JsValue::NULL, &JsValue::from(WValue { inner: item.clone() }))?;
            let w = WValue::try_from_js_value(r).map_err(|_| JsValue::from_str("callback must return a WValue"))?;
            out.push(w.inner);
        }
        Ok(WList { items: out })
    }
    /// The benchmark's own transformation, run entirely inside the core (one crossing).
    pub fn map_builtin(&self) -> WList { WList { items: self.items.iter().map(map_one).collect() } }
    pub fn to_gram(&self) -> String { to_gram(&Value::Array { items: self.items.clone() }) }
    pub fn checksum(&self) -> f64 { self.items.iter().map(checksum).sum() }
}

/// The JSON boundary: one string in, one string out.
#[wasm_bindgen]
pub fn map_json(s: &str) -> String { to_json(&map_one(&from_json(s))) }

/// Build the benchmark list inside the core and return it as JSON (a lower bound for creation).
#[wasm_bindgen]
pub fn gen_list_json(n: u32) -> String { to_json(&gen_list(n)) }
