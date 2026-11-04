//! Parse existing changelogs without filesystem access.
//!
//! Parsing preserves source order and Markdown. Convention checks are a separate
//! operation: invalid dates, for example, remain available for inspection.
//!
//! ```
//! use logloom::{parse, validate, Format};
//!
//! let changelog = parse("# Changelog\n\n## [Unreleased]\n\n### Added\n- A library.\n",
//!                       Format::KeepAChangelog)?;
//! assert_eq!(changelog.releases.len(), 1);
//! assert!(validate(&changelog).is_empty());
//! let json = serde_json::to_string(&changelog)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod error;
mod model;
mod parsers;
mod validation;

pub use error::{ErrorKind, ParseError};
pub use model::{
    Category, Changelog, Entry, EntryKind, LinkDefinition, Metadata, Release, Section, Span,
};
pub use validation::{Diagnostic, DiagnosticCode, Severity, validate};

/// Supported input formats. More formats may be added in future releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Format {
    /// Keep a Changelog 1.1.0, including common 1.0.0 conventions.
    KeepAChangelog,
}

/// Maximum UTF-8 input size (16 MiB), enforced before Markdown parsing.
pub const MAX_INPUT_BYTES: usize = 16_777_216;

/// Parse an in-memory document, retaining its original Markdown and ordering.
///
/// Dates and other conventions are checked separately by [`validate`]. A missing
/// title or date is accepted. At least one level-two release heading is required.
/// Markdown nesting is limited to 128 containers, including inline formatting.
///
/// # Errors
///
/// Returns a [`ParseError`] for empty input, missing releases, malformed release
/// headings, unsupported document structure, or resource limits.
pub fn parse(input: &str, format: Format) -> Result<Changelog, ParseError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(ParseError::new(
            ErrorKind::ResourceLimit,
            None,
            "input exceeds the 16 MiB limit",
        ));
    }
    if input.trim().is_empty() {
        return Err(ParseError::new(
            ErrorKind::EmptyInput,
            None,
            "expected a changelog with a release heading",
        ));
    }
    match format {
        Format::KeepAChangelog => parsers::keep_a_changelog::parse(input),
    }
}
