use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{Category, Changelog};

/// Importance of a convention diagnostic. Parsing itself has already succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Accepted but unconventional content.
    Warning,
    /// Content with invalid semantics, such as an impossible calendar date.
    Error,
}

/// Stable machine-readable convention diagnostic codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// No document title is present.
    MissingTitle,
    /// A published release has no date.
    MissingDate,
    /// A date is not a valid Gregorian YYYY-MM-DD value.
    InvalidDate,
    /// Pending changes have a date attached.
    DatedPendingRelease,
    /// A release identifier occurs more than once.
    DuplicateRelease,
    /// Pending changes appear after published releases.
    PendingReleaseOrder,
    /// Published release dates are not in descending order.
    ReleaseDateOrder,
    /// A group uses an unknown category name.
    CustomCategory,
    /// A release contains content outside a named category.
    UncategorizedContent,
}

/// A convention check result with a one-based source line when available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity of the finding.
    pub severity: Severity,
    /// Machine-readable classification.
    pub code: DiagnosticCode,
    /// Original source line, when available.
    pub line: Option<usize>,
    /// Human-readable explanation.
    pub message: String,
}

/// Check conventions without mutating, sorting, or rejecting the parsed content.
///
/// Empty releases and categories are accepted. Version identifiers are opaque.
/// Dates use four-digit years 0001–9999 and the Gregorian calendar. Unknown
/// categories, missing dates, and unusual ordering produce warnings; invalid
/// dates and duplicate identifiers produce errors. This checks the unified
/// model, rather than attempting to validate every Markdown convention.
#[must_use]
pub fn validate(changelog: &Changelog) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut report = |severity, code, line, message: &str| {
        diagnostics.push(Diagnostic {
            severity,
            code,
            line,
            message: message.to_owned(),
        });
    };
    if changelog.metadata.title.is_none() {
        report(
            Severity::Warning,
            DiagnosticCode::MissingTitle,
            None,
            "document has no title",
        );
    }
    let mut versions = HashSet::new();
    let mut previous_date: Option<&str> = None;
    let mut published_seen = false;
    for release in &changelog.releases {
        let line = Some(release.span.line);
        if !versions.insert(release.version.as_deref()) {
            report(
                Severity::Error,
                DiagnosticCode::DuplicateRelease,
                line,
                "duplicate release identifier",
            );
        }
        if release.version.is_none() && published_seen {
            report(
                Severity::Warning,
                DiagnosticCode::PendingReleaseOrder,
                line,
                "pending changes usually precede published releases",
            );
        }
        published_seen |= release.version.is_some();
        match &release.date {
            Some(date) => {
                if !valid_date(date) {
                    report(
                        Severity::Error,
                        DiagnosticCode::InvalidDate,
                        line,
                        "expected a real calendar date in YYYY-MM-DD format",
                    );
                } else if release.version.is_some() {
                    if previous_date.is_some_and(|previous| previous < date.as_str()) {
                        report(
                            Severity::Warning,
                            DiagnosticCode::ReleaseDateOrder,
                            line,
                            "release dates usually appear newest first",
                        );
                    }
                    previous_date = Some(date);
                }
                if release.version.is_none() {
                    report(
                        Severity::Warning,
                        DiagnosticCode::DatedPendingRelease,
                        line,
                        "pending changes usually have no release date",
                    );
                }
            }
            None if release.version.is_some() => report(
                Severity::Warning,
                DiagnosticCode::MissingDate,
                line,
                "published release has no date",
            ),
            None => {}
        }
        for section in &release.sections {
            match &section.category {
                Some(Category::Custom(_)) => report(
                    Severity::Warning,
                    DiagnosticCode::CustomCategory,
                    Some(section.span.line),
                    "category is outside the standard vocabulary",
                ),
                None if !section.entries.is_empty() => report(
                    Severity::Warning,
                    DiagnosticCode::UncategorizedContent,
                    Some(section.span.line),
                    "content appears outside a named category",
                ),
                _ => {}
            }
        }
    }
    diagnostics
}

fn valid_date(date: &str) -> bool {
    if date.len() != 10
        || !date
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'-')
    {
        return false;
    }
    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) =
        (year.parse::<u16>(), month.parse::<u8>(), day.parse::<u8>())
    else {
        return false;
    };
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let max_day = match month {
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return false,
    };
    year != 0 && day != 0 && day <= max_day
}
