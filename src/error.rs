use std::fmt;

/// Classification of a parsing failure, independent of its human-readable text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// No non-whitespace content was supplied.
    EmptyInput,
    /// No level-two release heading was found.
    NoReleases,
    /// A release heading could not be interpreted without guessing.
    MalformedHeading,
    /// The document uses an unsupported structural arrangement.
    UnsupportedStructure,
    /// The input exceeds a documented size or nesting limit.
    ResourceLimit,
}

/// A structured parsing failure with an optional one-based source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Machine-readable error classification.
    pub kind: ErrorKind,
    /// One-based source line, when the failure has a specific location.
    pub line: Option<usize>,
    /// Actionable description of the failure.
    pub message: String,
}

impl ParseError {
    pub(crate) fn new(kind: ErrorKind, line: Option<usize>, message: impl Into<String>) -> Self {
        Self {
            kind,
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(line) = self.line {
            write!(f, "line {line}: ")?;
        }
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}
