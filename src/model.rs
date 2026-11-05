use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// An owned, serializable changelog independent of its source format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Changelog {
    /// Source format, retained so validation uses the correct conventions.
    /// Defaults to Keep a Changelog when reading older serialized models.
    #[serde(default)]
    pub format: crate::Format,
    /// Title and introductory Markdown.
    pub metadata: Metadata,
    /// All releases, including pending releases, in original document order.
    pub releases: Vec<Release>,
    /// Document-wide reference definitions in source order.
    /// Independently decoded bodies keep their definitions in [`Release::links`].
    pub links: Vec<LinkDefinition>,
    /// Exact input, including whitespace, duplicate definitions, and line endings.
    /// Defaults to empty when deserializing CLI exports, which omit this field.
    /// Spans still refer to the original input when the source copy is absent.
    #[serde(default)]
    pub source: String,
}

/// Document-level information; arbitrary metadata stays in the introduction.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Metadata {
    /// Plain text of the optional level-one title.
    pub title: Option<String>,
    /// Introductory source text before the first release, excluding a Markdown title.
    pub description: String,
}

/// A release identifier is opaque: neither `SemVer` nor chronological sorting is imposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Release {
    /// Opaque identifier; `None` represents pending or unversioned content.
    pub version: Option<String>,
    /// Optional calendar date. Use validation to check calendar correctness.
    /// Timestamp-based formats retain the original timestamp in attributes.
    pub date: Option<String>,
    /// Source location of a date stored outside the release heading, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_span: Option<Span>,
    /// Whether the source explicitly marks this release as withdrawn.
    pub yanked: bool,
    /// Original heading text, or decoded release title from a structured export.
    pub heading: String,
    /// Resolved link on the release heading, if present.
    pub link: Option<String>,
    /// Ordered groups, including unnamed groups for uncategorized content.
    pub sections: Vec<Section>,
    /// Source location of the release heading.
    pub span: Span,
    /// Format-specific attributes such as package, bump, author, or timestamp.
    /// Empty for ordinary Keep a Changelog releases.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
    /// References local to an independently decoded body, overriding document links.
    /// Empty for formats whose releases share one Markdown document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkDefinition>,
}

/// Semantic release state, independent of whether an identifier is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReleaseStatus {
    /// Pending changes or a news fragment.
    Pending,
    /// An explicitly marked draft, possibly with a version identifier.
    Draft,
    /// A versioned, published release; includes prereleases and withdrawn releases.
    Published,
    /// A published date/author record without an invented version identifier.
    DatedRecord,
    /// Notes whose publication state is unknown.
    Unversioned,
}

impl Changelog {
    /// Create an empty document of the given format, without source text.
    ///
    /// ```
    /// use logloom::{Changelog, Format, Release};
    /// let mut changelog = Changelog::new(Format::GenericMarkdown);
    /// changelog.releases.push(Release::new(Some("preview".into())));
    /// assert_eq!(changelog.releases.len(), 1);
    /// ```
    #[must_use]
    pub fn new(format: crate::Format) -> Self {
        Self {
            format,
            metadata: Metadata::default(),
            releases: Vec::new(),
            links: Vec::new(),
            source: String::new(),
        }
    }
}

impl Release {
    /// Create a release with no date, content, attributes, or source location.
    /// A missing identifier starts as pending; attributes can refine its state.
    #[must_use]
    pub fn new(version: Option<String>) -> Self {
        Self {
            version,
            date: None,
            date_span: None,
            yanked: false,
            heading: String::new(),
            link: None,
            sections: Vec::new(),
            span: Span::default(),
            attributes: BTreeMap::new(),
            links: Vec::new(),
        }
    }

    /// Classify this release using its current identifier and format attributes.
    ///
    /// ```
    /// use logloom::{Release, ReleaseStatus};
    /// let mut release = Release::new(Some("v1".into()));
    /// assert_eq!(release.status(), ReleaseStatus::Published);
    /// release.attributes.insert("draft".into(), "true".into());
    /// assert_eq!(release.status(), ReleaseStatus::Draft);
    /// ```
    #[must_use]
    pub fn status(&self) -> ReleaseStatus {
        if self
            .attributes
            .get("draft")
            .is_some_and(|value| value == "true")
        {
            ReleaseStatus::Draft
        } else if self
            .attributes
            .get("record-kind")
            .is_some_and(|value| value == "dated-change")
        {
            ReleaseStatus::DatedRecord
        } else if self.version.is_some() {
            ReleaseStatus::Published
        } else if self
            .attributes
            .get("version-status")
            .is_some_and(|value| value == "unknown")
        {
            ReleaseStatus::Unversioned
        } else {
            ReleaseStatus::Pending
        }
    }

    /// Whether published selectors should include this release or date/author record.
    #[must_use]
    pub fn is_published(&self) -> bool {
        matches!(
            self.status(),
            ReleaseStatus::Published | ReleaseStatus::DatedRecord
        )
    }

    /// Whether this release is pending, including explicitly marked drafts.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        matches!(self.status(), ReleaseStatus::Pending | ReleaseStatus::Draft)
    }

    /// Borrow the format's package name, when present.
    #[must_use]
    pub fn package(&self) -> Option<&str> {
        self.attributes.get("package").map(String::as_str)
    }

    /// Whether the input explicitly marks this as a prerelease.
    #[must_use]
    pub fn is_prerelease(&self) -> bool {
        self.attributes
            .get("prerelease")
            .is_some_and(|value| value == "true")
    }
}

/// A group of ordered entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Section {
    /// Optional component name shared by this category's entries.
    /// Absent for release-wide categories and older serialized models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    /// `None` for uncategorized content at release or component level.
    pub category: Option<Category>,
    /// Exact heading Markdown; absent for an unnamed group.
    pub heading: Option<String>,
    /// Entries in their original order, retaining nested content.
    pub entries: Vec<Entry>,
    /// Source location of the heading, or first entry for an unnamed group.
    pub span: Span,
}

impl Section {
    /// Create an empty group without a component, heading, or source location.
    #[must_use]
    pub fn new(category: Option<Category>) -> Self {
        Self {
            component: None,
            category,
            heading: None,
            entries: Vec::new(),
            span: Span::default(),
        }
    }
}

/// A normalized category with a lossless fallback for other names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Category {
    /// New functionality.
    Added,
    /// Changes to existing functionality.
    Changed,
    /// Functionality scheduled for removal.
    Deprecated,
    /// Removed functionality.
    Removed,
    /// Corrections to existing behavior.
    Fixed,
    /// Security-related changes.
    Security,
    /// A category outside the standard vocabulary, preserving its plain text.
    Custom(String),
}

impl Category {
    /// Return the normalized display name, preserving custom category names.
    ///
    /// ```
    /// use logloom::Category;
    /// assert_eq!(Category::Fixed.as_str(), "Fixed");
    /// assert_eq!(Category::Custom("Performance".into()).as_str(), "Performance");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Added => "Added",
            Self::Changed => "Changed",
            Self::Deprecated => "Deprecated",
            Self::Removed => "Removed",
            Self::Fixed => "Fixed",
            Self::Security => "Security",
            Self::Custom(name) => name,
        }
    }

    pub(crate) fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "added" => Self::Added,
            "changed" => Self::Changed,
            "deprecated" => Self::Deprecated,
            "removed" => Self::Removed,
            "fixed" => Self::Fixed,
            "security" => Self::Security,
            _ => Self::Custom(name.to_owned()),
        }
    }
}

/// A top-level list item, Markdown block, or raw text block within a section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Entry {
    /// Original content, trimmed only at block boundaries; may be Markdown or text.
    /// List markers, nested lists, code, and reference labels are retained.
    pub markdown: String,
    /// Whether this entry originated as a list item, Markdown block, or raw text.
    pub kind: EntryKind,
    /// Original source location.
    pub span: Span,
}

impl Entry {
    /// Create an entry without a source location; content is kept as supplied.
    #[must_use]
    pub fn new(markdown: String, kind: EntryKind) -> Self {
        Self {
            markdown,
            kind,
            span: Span::default(),
        }
    }
}

/// Structural origin of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EntryKind {
    /// One complete item from a top-level ordered or unordered list.
    ListItem,
    /// Another Markdown block (paragraph, code, quote, table, HTML, or subheading).
    Markdown,
    /// Plain text or reStructuredText retained without Markdown interpretation.
    Text,
}

/// One effective `CommonMark` link definition; duplicate definitions remain in `source`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LinkDefinition {
    /// Reference label as reported by the Markdown parser.
    pub label: String,
    /// Resolved destination.
    pub destination: String,
    /// Optional reference title.
    pub title: Option<String>,
    /// Source location of the definition.
    pub span: Span,
}

impl LinkDefinition {
    /// Create a reference without a title or source location.
    #[must_use]
    pub fn new(label: String, destination: String) -> Self {
        Self {
            label,
            destination,
            title: None,
            span: Span::default(),
        }
    }
}

/// Half-open UTF-8 byte offsets in the original source, plus a one-based line.
/// The default is an empty range on line one for newly constructed content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Span {
    /// Inclusive starting byte offset.
    pub start: usize,
    /// Exclusive ending byte offset.
    pub end: usize,
    /// One-based line containing the starting offset.
    pub line: usize,
}

impl Span {
    /// Construct original UTF-8 byte offsets and a one-based line.
    /// Callers must ensure offsets are valid for their source; this performs no I/O.
    #[must_use]
    pub const fn new(start: usize, end: usize, line: usize) -> Self {
        Self { start, end, line }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::new(0, 0, 1)
    }
}
