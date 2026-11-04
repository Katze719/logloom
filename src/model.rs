use serde::{Deserialize, Serialize};

/// An owned, serializable changelog independent of its source format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changelog {
    /// Title and introductory Markdown.
    pub metadata: Metadata,
    /// All releases, including pending releases, in original document order.
    pub releases: Vec<Release>,
    /// Effective reference-style link definitions in source order.
    pub links: Vec<LinkDefinition>,
    /// Exact input, including whitespace, duplicate definitions, and line endings.
    pub source: String,
}

/// Document-level information; arbitrary metadata stays in the introduction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// Plain text of the optional level-one title.
    pub title: Option<String>,
    /// Markdown before the first release, excluding the title heading itself.
    pub description: String,
}

/// A release identifier is opaque: neither `SemVer` nor chronological sorting is imposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    /// Opaque identifier; `None` represents pending (unreleased) changes.
    pub version: Option<String>,
    /// Original date text. Use validation to check calendar correctness.
    pub date: Option<String>,
    /// Whether the source explicitly marks this release as withdrawn.
    pub yanked: bool,
    /// Exact Markdown heading, excluding surrounding whitespace.
    pub heading: String,
    /// Resolved link on the release heading, if present.
    pub link: Option<String>,
    /// Ordered groups, including unnamed groups for uncategorized content.
    pub sections: Vec<Section>,
    /// Source location of the release heading.
    pub span: Span,
}

/// A group of ordered entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// `None` for content before the first category in a release.
    pub category: Option<Category>,
    /// Exact heading Markdown; absent for an unnamed group.
    pub heading: Option<String>,
    /// Entries in their original order, retaining nested content.
    pub entries: Vec<Entry>,
    /// Source location of the heading, or first entry for an unnamed group.
    pub span: Span,
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

/// A top-level list item or other Markdown block within a section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Exact source Markdown, trimmed only at the block boundaries.
    /// List markers, nested lists, code, and reference labels are retained.
    pub markdown: String,
    /// Whether this entry originated as a list item or another Markdown block.
    pub kind: EntryKind,
    /// Original source location.
    pub span: Span,
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
}

/// One effective `CommonMark` link definition; duplicate definitions remain in `source`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Half-open UTF-8 byte offsets in the original source, plus a one-based line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// Inclusive starting byte offset.
    pub start: usize,
    /// Exclusive ending byte offset.
    pub end: usize,
    /// One-based line containing the starting offset.
    pub line: usize,
}
