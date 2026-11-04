use std::fmt;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::parsers::{
    histories,
    markdown::{self, Block, Heading},
};
use crate::{EntryKind, ErrorKind, Format, MAX_INPUT_BYTES, ParseError, parse};

/// A conservative format detector's failure, separate from convention validation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DetectionError {
    /// No sufficiently specific signature was found; choose a format explicitly.
    Unrecognized,
    /// Several signatures are plausible; no candidate was selected.
    Ambiguous {
        /// Plausible input formats, in stable detector order.
        candidates: Vec<Format>,
    },
    /// Input could not be safely inspected (for example size, nesting, or JSON syntax).
    InvalidInput(ParseError),
}

impl fmt::Display for DetectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unrecognized => {
                f.write_str("no sufficiently specific changelog format signature found")
            }
            Self::Ambiguous { candidates } => {
                f.write_str("ambiguous changelog format: ")?;
                for (index, format) in candidates.iter().enumerate() {
                    if index != 0 {
                        f.write_str(", ")?;
                    }
                    f.write_str(format.as_str())?;
                }
                Ok(())
            }
            Self::InvalidInput(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for DetectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidInput(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ParseError> for DetectionError {
    fn from(error: ParseError) -> Self {
        Self::InvalidInput(error)
    }
}

/// Detect a format from specific content signatures without I/O or validation.
///
/// Generic Markdown is not a catch-all. Overlapping or weak signatures require an
/// explicit format. Detection does not guarantee parsing or semantic validity.
///
/// ```
/// use logloom::{detect, Format};
/// assert_eq!(detect("# Changelog\n\n## [Unreleased]\n")?, Format::KeepAChangelog);
/// # Ok::<(), logloom::DetectionError>(())
/// ```
///
/// # Errors
///
/// Returns [`DetectionError::Unrecognized`] or [`DetectionError::Ambiguous`] when
/// guessing would be necessary, or [`DetectionError::InvalidInput`] for empty,
/// oversized, excessively nested, or malformed JSON input.
pub fn detect(input: &str) -> Result<Format, DetectionError> {
    detect_internal(input, None)
}

/// Detect with a filename hint for numeric or anonymous Towncrier fragments.
///
/// Names such as `123.bugfix.rst` and `+feature.feature.md` contribute a Towncrier
/// signature. A normal filename or extension alone never identifies a format.
/// The name is not opened. Conflicting content and filename signatures are errors.
///
/// # Errors
///
/// Returns the same errors as [`detect`]. An empty named Towncrier fragment can
/// be identified; parsing still checks whether its type permits an empty body.
pub fn detect_named(input: &str, name: &str) -> Result<Format, DetectionError> {
    detect_internal(input, Some(name))
}

fn detect_internal(input: &str, name: Option<&str>) -> Result<Format, DetectionError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(ParseError::new(
            ErrorKind::ResourceLimit,
            None,
            "input exceeds the 16 MiB limit",
        )
        .into());
    }
    let mut candidates = Vec::new();
    if name.is_some_and(fragment_name) {
        candidates.push(Format::Towncrier);
    }
    if input.trim().is_empty() {
        if candidates.is_empty() {
            return Err(ParseError::new(
                ErrorKind::EmptyInput,
                None,
                "expected nonempty changelog content or a Towncrier fragment filename",
            )
            .into());
        }
        return finish(&candidates);
    }
    let trimmed = input.trim_start();
    if trimmed.starts_with('{')
        || trimmed
            .strip_prefix('[')
            .is_some_and(|rest| rest.trim_start().starts_with(['{', '[', ']']))
    {
        let value: serde_json::Value = serde_json::from_str(input).map_err(|error| {
            ParseError::new(
                ErrorKind::MalformedInput,
                Some(error.line()),
                format!("cannot inspect JSON: {error}"),
            )
        })?;
        if github_json(&value) {
            candidates.push(Format::GitHubReleaseNotes);
        }
        return finish(&candidates);
    }
    if input
        .lines()
        .next()
        .is_some_and(|line| line.trim() == "---")
        && parse(input, Format::Changesets).is_ok_and(|model| {
            model.releases.iter().any(|release| {
                release
                    .attributes
                    .get("fragment")
                    .is_some_and(|kind| kind == "changesets")
            })
        })
    {
        candidates.push(Format::Changesets);
    }
    let first = input
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    if debian(first) {
        candidates.push(Format::DebianChangelog);
    }
    if dated_author(first) {
        candidates.push(Format::GnuChangeLog);
    }
    let scanned = markdown::scan(input)?;
    markdown_candidates(input, &scanned.blocks, &mut candidates);
    finish(&candidates)
}

fn finish(candidates: &[Format]) -> Result<Format, DetectionError> {
    let order = [
        Format::KeepAChangelog,
        Format::ConventionalChangelog,
        Format::GenericMarkdown,
        Format::GitHubReleaseNotes,
        Format::Changesets,
        Format::Towncrier,
        Format::DebianChangelog,
        Format::GnuNews,
        Format::GnuChangeLog,
    ];
    let candidates: Vec<_> = order
        .into_iter()
        .filter(|format| candidates.contains(format))
        .collect();
    match candidates.as_slice() {
        [] => Err(DetectionError::Unrecognized),
        [format] => Ok(*format),
        _ => Err(DetectionError::Ambiguous { candidates }),
    }
}

fn github_json(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Array(items) => items.iter().any(github_object),
        _ => github_object(value),
    }
}

fn github_object(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object.contains_key("tag_name") || object.contains_key("tagName")
        }
        _ => false,
    }
}

fn fragment_name(name: &str) -> bool {
    let filename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let mut parts = filename.split('.');
    let (Some(issue), Some(kind)) = (parts.next(), parts.next()) else {
        return false;
    };
    !issue.is_empty()
        && kind.starts_with(char::is_alphabetic)
        && (issue.bytes().all(|byte| byte.is_ascii_digit())
            || (issue.starts_with('+') && issue.len() > 1))
        && !matches!(kind.to_ascii_lowercase().as_str(), "md" | "rst" | "txt")
}

fn debian(line: &str) -> bool {
    line.split_once(" (").is_some_and(|(package, rest)| {
        !package.is_empty()
            && !package.contains(char::is_whitespace)
            && rest.contains(") ")
            && rest.contains("; urgency=")
    })
}

fn dated_author(line: &str) -> bool {
    line.get(..10)
        .zip(line.get(10..))
        .is_some_and(|(date, author)| {
            date.bytes().enumerate().all(|(index, byte)| {
                if matches!(index, 4 | 7) {
                    byte == b'-'
                } else {
                    byte.is_ascii_digit()
                }
            }) && author.starts_with(char::is_whitespace)
                && !author.trim().is_empty()
        })
}

fn markdown_candidates(input: &str, blocks: &[Block], candidates: &mut Vec<Format>) {
    let visible = visible_markers(input);
    let headings: Vec<_> = blocks
        .iter()
        .filter_map(|block| match block {
            Block::Heading(heading) => Some(heading),
            Block::Entry(..) => None,
        })
        .collect();
    let numeric_parenthesized = headings
        .iter()
        .any(|heading| heading.text.contains(" (") && histories::looks_like_release(&heading.text));
    let conventional_categories = headings.iter().any(|heading| {
        matches!(
            heading.text.to_ascii_lowercase().as_str(),
            "features"
                | "bug fixes"
                | "performance improvements"
                | "⚠ breaking changes"
                | "breaking changes"
        )
    });
    for heading in &headings {
        heading_candidates(heading, candidates);
    }
    if numeric_parenthesized && conventional_categories {
        candidates.push(Format::ConventionalChangelog);
    }
    let github_heading = headings.iter().any(|heading| {
        matches!(
            heading.text.to_ascii_lowercase().as_str(),
            "what's changed" | "new contributors"
        )
    });
    if github_heading {
        candidates.push(Format::GitHubReleaseNotes);
        if !visible.github_url {
            candidates.push(Format::GenericMarkdown);
        }
    }
    for block in blocks {
        if let Block::Entry(range, EntryKind::ListItem) = block
            && let Some(text) = input.get(range.clone())
            && (text.starts_with("* Noteworthy changes in release ")
                || text.starts_with("* Changes in version ")
                || text.starts_with("* Changes in release ")
                || text.starts_with("* Version "))
        {
            candidates.push(Format::GnuNews);
        }
    }
    if input.starts_with("GNU ") && visible.gnu_version {
        candidates.push(Format::GnuNews);
    }
    if candidates.is_empty() {
        if numeric_parenthesized {
            candidates.extend([
                Format::ConventionalChangelog,
                Format::Towncrier,
                Format::GenericMarkdown,
            ]);
        } else if !headings.is_empty()
            || blocks
                .iter()
                .any(|block| matches!(block, Block::Entry(_, EntryKind::ListItem)))
        {
            candidates.extend([
                Format::GenericMarkdown,
                Format::GitHubReleaseNotes,
                Format::Towncrier,
            ]);
        }
    }
}

#[derive(Default)]
struct VisibleMarkers {
    github_url: bool,
    gnu_version: bool,
}

fn visible_markers(input: &str) -> VisibleMarkers {
    let mut markers = VisibleMarkers::default();
    let mut excluded: usize = 0;
    // scan() has already enforced the shared nesting limit. This pass only
    // inspects visible text and resolved URLs, ignoring literal code and quotes.
    for (event, range) in Parser::new(input).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::BlockQuote(_)) => {
                excluded = excluded.saturating_add(1);
            }
            Event::End(TagEnd::CodeBlock | TagEnd::BlockQuote(_)) => {
                excluded = excluded.saturating_sub(1);
            }
            Event::Text(text) if excluded == 0 => {
                markers.github_url |= text.contains("github.com/");
                markers.gnu_version |= text.starts_with("Version ")
                    && (range.start == 0
                        || input
                            .get(..range.start)
                            .is_some_and(|prefix| prefix.ends_with('\n')));
            }
            Event::Start(Tag::Link { dest_url, .. }) if excluded == 0 => {
                markers.github_url |= dest_url.contains("github.com/");
            }
            _ => {}
        }
    }
    markers
}

fn heading_candidates(heading: &Heading, candidates: &mut Vec<Format>) {
    let text = heading.text.trim();
    if heading.level == HeadingLevel::H2
        && !text.contains(" (")
        && (text.starts_with('[')
            || text.eq_ignore_ascii_case("unreleased")
            || text
                .rsplit_once(" - ")
                .is_some_and(|(name, _)| !name.contains(char::is_whitespace)))
    {
        candidates.push(Format::KeepAChangelog);
    }
    if heading.level == HeadingLevel::H3
        && matches!(text, "Major Changes" | "Minor Changes" | "Patch Changes")
    {
        candidates.push(Format::Changesets);
    }
    if let Some((name, _)) = text.rsplit_once(" (")
        && name.contains(char::is_whitespace)
        && let Some(version) = name.split_whitespace().last()
        && histories::looks_like_release(version)
    {
        candidates.push(Format::Towncrier);
    }
    if heading.level == HeadingLevel::H2
        && let Some((name, _)) = text.rsplit_once(" - ")
        && name.contains(char::is_whitespace)
        && !name.starts_with('[')
    {
        candidates.push(Format::GenericMarkdown);
    }
}
