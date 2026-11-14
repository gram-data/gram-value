//! The data values of the gram notation.
//!
//! `gram-value` models every value gram can write (integers, decimals, booleans, strings, tagged strings,
//! ranges, measurements, arrays and maps), compares them, writes each as canonical gram text, and interprets
//! the ten known tags. It does not parse: [tree-sitter-gram](https://github.com/gram-data/tree-sitter-gram)
//! does. The rules are in `spec/` of the repository; the decisions are ADR-002 to ADR-005 under `design/adr/`.
//!
//! ```
//! use gram_value::Value;
//!
//! let m = Value::measurement(Value::integer(5), "kg").unwrap();
//! assert_eq!(m.to_text().unwrap(), "5kg");
//! assert_ne!(m, Value::measurement(Value::decimal(5.0).unwrap(), "kg").unwrap());
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod diagnostic;
pub mod tags;
mod text;
mod value;

pub use diagnostic::{Diagnostic, PathSegment};
pub use text::{string_text, Context};
pub use value::{BigDecimal, BigInteger, Decimal, Kind, Map, Measurement, Range, Tagged, Value};
