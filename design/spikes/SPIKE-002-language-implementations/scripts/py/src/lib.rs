//! SPIKE-002: the Rust core behind a Python extension module (PyO3). Python holds objects that wrap Rust values.
use bench_core::{checksum, from_json, map_one, to_gram, to_json, Value};
use pyo3::prelude::*;

#[pyclass]
struct PValue {
    inner: Value,
}

#[pymethods]
impl PValue {
    #[staticmethod]
    fn integer(v: i64) -> PValue { PValue { inner: Value::Integer { value: v } } }
    #[staticmethod]
    fn decimal(v: f64) -> PValue { PValue { inner: Value::Decimal { value: v } } }
    #[staticmethod]
    fn boolean(v: bool) -> PValue { PValue { inner: Value::Boolean { value: v } } }
    #[staticmethod]
    fn string(v: String) -> PValue { PValue { inner: Value::String { value: v } } }
    #[staticmethod]
    fn symbol(v: String) -> PValue { PValue { inner: Value::Symbol { value: v } } }
    #[staticmethod]
    fn tagged(tag: String, content: String) -> PValue { PValue { inner: Value::Tagged { tag, content } } }
    #[staticmethod]
    fn range(lower: f64, upper: f64) -> PValue { PValue { inner: Value::Range { lower: Some(lower), upper: Some(upper) } } }
    #[staticmethod]
    fn measurement(unit: String, value: f64) -> PValue { PValue { inner: Value::Measurement { unit, value } } }

    fn kind(&self) -> u8 {
        match &self.inner {
            Value::Integer { .. } => 0, Value::Decimal { .. } => 1, Value::Boolean { .. } => 2, Value::String { .. } => 3,
            Value::Symbol { .. } => 4, Value::Tagged { .. } => 5, Value::Range { .. } => 6, Value::Measurement { .. } => 7,
            Value::Array { .. } => 8, Value::Map { .. } => 9,
        }
    }
    fn number(&self) -> f64 {
        match &self.inner {
            Value::Integer { value } => *value as f64, Value::Decimal { value } => *value,
            Value::Boolean { value } => f64::from(u8::from(*value)), Value::Range { lower, .. } => lower.unwrap_or(0.0),
            Value::Measurement { value, .. } => *value, _ => 0.0,
        }
    }
    fn number2(&self) -> f64 {
        match &self.inner { Value::Range { upper, .. } => upper.unwrap_or(0.0), _ => 0.0 }
    }
    fn text(&self) -> String {
        match &self.inner {
            Value::String { value } | Value::Symbol { value } => value.clone(),
            Value::Tagged { tag, .. } => tag.clone(), Value::Measurement { unit, .. } => unit.clone(), _ => String::new(),
        }
    }
    fn text2(&self) -> String {
        match &self.inner { Value::Tagged { content, .. } => content.clone(), _ => String::new() }
    }
}

#[pyclass]
struct PList {
    items: Vec<Value>,
}

#[pymethods]
impl PList {
    #[new]
    fn new() -> PList { PList { items: Vec::new() } }
    fn push(&mut self, v: PyRef<'_, PValue>) { self.items.push(v.inner.clone()); }
    fn __len__(&self) -> usize { self.items.len() }
    fn get(&self, i: usize) -> PValue { PValue { inner: self.items[i].clone() } }
    /// A general map: the callable runs in Python for each element.
    fn map_with(&self, py: Python<'_>, f: &Bound<'_, PyAny>) -> PyResult<PList> {
        let mut out = Vec::with_capacity(self.items.len());
        for item in &self.items {
            let arg = Py::new(py, PValue { inner: item.clone() })?;
            let r = f.call1((arg,))?;
            let pv: PyRef<'_, PValue> = r.extract()?;
            out.push(pv.inner.clone());
        }
        Ok(PList { items: out })
    }
    fn map_builtin(&self) -> PList { PList { items: self.items.iter().map(map_one).collect() } }
    fn to_gram(&self) -> String { to_gram(&Value::Array { items: self.items.clone() }) }
    fn checksum(&self) -> f64 { self.items.iter().map(checksum).sum() }
}

#[pyfunction]
fn map_json(s: &str) -> String { to_json(&map_one(&from_json(s))) }

#[pymodule]
fn pybench(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PValue>()?;
    m.add_class::<PList>()?;
    m.add_function(wrap_pyfunction!(map_json, m)?)?;
    Ok(())
}
