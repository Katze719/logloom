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

mod detection;
mod error;
mod model;
mod parsers;
mod validation;

pub use detection::{DetectionError, detect, detect_named};
pub use error::{ErrorKind, ParseError};
pub use model::{
    Category, Changelog, Entry, EntryKind, LinkDefinition, Metadata, Release, ReleaseStatus,
    Section, Span,
};
pub use validation::{Diagnostic, DiagnosticCode, Severity, validate, validate_for_format};

/// Supported input formats. More formats may be added in future releases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Format {
    /// Keep a Changelog 1.1.0, including common 1.0.0 conventions.
    #[default]
    KeepAChangelog,
    /// Rendered Conventional Changelog histories (including Angular-style headings).
    ConventionalChangelog,
    /// Custom Markdown histories using release headings, or an unversioned body.
    GenericMarkdown,
    /// Saved Markdown release notes or GitHub REST release JSON.
    #[serde(rename = "github-release-notes")]
    GitHubReleaseNotes,
    /// Changesets package changelogs and individual YAML-frontmatter changesets.
    Changesets,
    /// Towncrier Markdown/reStructuredText histories and individual news fragments.
    Towncrier,
    /// Debian package changelog records with maintainer trailers.
    DebianChangelog,
    /// GNU-style NEWS release histories with outline headings.
    GnuNews,
    /// Traditional date-and-author GNU `ChangeLog` records.
    #[serde(rename = "gnu-changelog")]
    GnuChangeLog,
}

impl Format {
    /// Stable input-format name used by the CLI and serialized models.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KeepAChangelog => "keep-a-changelog",
            Self::ConventionalChangelog => "conventional-changelog",
            Self::GenericMarkdown => "generic-markdown",
            Self::GitHubReleaseNotes => "github-release-notes",
            Self::Changesets => "changesets",
            Self::Towncrier => "towncrier",
            Self::DebianChangelog => "debian-changelog",
            Self::GnuNews => "gnu-news",
            Self::GnuChangeLog => "gnu-changelog",
        }
    }
}

/// Maximum UTF-8 input size (16 MiB), enforced before Markdown parsing.
pub const MAX_INPUT_BYTES: usize = 16_777_216;

/// Parse an in-memory document, retaining its original content and ordering.
///
/// Dates and other conventions are checked separately by [`validate`]. A missing
/// title or date is accepted. Release-heading rules depend on the selected format.
/// Markdown nesting is limited to 128 containers, including inline formatting.
///
/// # Errors
///
/// Returns a [`ParseError`] for empty input, missing releases, malformed release
/// headings or structured data, unsupported document structure, or resource limits.
pub fn parse(input: &str, format: Format) -> Result<Changelog, ParseError> {
    parse_internal(input, format, None)
}

/// Parse with an optional source filename's semantic information, without I/O.
///
/// Towncrier fragment names such as `123.bugfix.rst` select fragment parsing and
/// supply the issue and category absent from the body. Other formats ignore `name`.
/// Ordinary history filenames such as `NEWS.rst` do not select fragment mode.
/// Use [`parse`] when the name is unavailable; unnamed fragments retain their
/// content without inventing a type. A named `misc` fragment may have an empty body.
///
/// # Errors
///
/// Returns the same errors as [`parse`], and a malformed-input error for an invalid
/// Towncrier fragment name. `name` is interpreted as a filename, not opened.
pub fn parse_named(input: &str, format: Format, name: &str) -> Result<Changelog, ParseError> {
    parse_internal(input, format, Some(name))
}

fn parse_internal(
    input: &str,
    format: Format,
    name: Option<&str>,
) -> Result<Changelog, ParseError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(ParseError::new(
            ErrorKind::ResourceLimit,
            None,
            "input exceeds the 16 MiB limit",
        ));
    }
    if input.trim().is_empty() && !(format == Format::Towncrier && name.is_some()) {
        return Err(ParseError::new(
            ErrorKind::EmptyInput,
            None,
            "expected nonempty changelog or release-note content",
        ));
    }
    let mut changelog = match format {
        Format::KeepAChangelog => parsers::keep_a_changelog::parse(input),
        Format::ConventionalChangelog | Format::GenericMarkdown => {
            parsers::histories::parse(input, format)
        }
        Format::GitHubReleaseNotes => parsers::github::parse(input),
        Format::Changesets => parsers::changesets::parse(input),
        Format::Towncrier => parsers::towncrier::parse(input, name),
        Format::DebianChangelog => parsers::debian::parse(input),
        Format::GnuNews | Format::GnuChangeLog => parsers::gnu::parse(input, format),
    }?;
    changelog.format = format;
    Ok(changelog)
}
