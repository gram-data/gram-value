//! Diagnostics: the problems an operation reports (spec/results.md).

use std::fmt;

/// One step of a path into a value: a map key or an array index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    /// A key of a map.
    Key(String),
    /// An index into an array.
    Index(usize),
}

/// A problem found by an operation: a code from the vocabulary, a message for a
/// person, and the path to the part of the value concerned (empty for the whole value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// A lower-case word with hyphens, such as `duplicate-key`.
    pub code: &'static str,
    /// Text for a person; not part of the contract.
    pub message: String,
    /// The part of the value concerned.
    pub path: Vec<PathSegment>,
}

impl Diagnostic {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            message: message.into(),
            path: Vec::new(),
        }
    }

    pub(crate) fn at(mut self, path: Vec<PathSegment>) -> Self {
        self.path = path;
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)?;
        if !self.path.is_empty() {
            write!(f, " (at ")?;
            for (i, seg) in self.path.iter().enumerate() {
                match seg {
                    PathSegment::Key(k) => write!(f, "{}{k:?}", if i == 0 { "" } else { "." })?,
                    PathSegment::Index(n) => write!(f, "[{n}]")?,
                }
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}
