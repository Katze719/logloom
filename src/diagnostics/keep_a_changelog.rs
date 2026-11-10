use logloom::{DiagnosticCode, ErrorKind};

pub const fn parse_details(kind: ErrorKind) -> (&'static str, &'static str) {
    match kind {
        ErrorKind::EmptyInput => (
            "empty-input",
            "Keep a Changelog requires a release heading, for example: ## [Unreleased].",
        ),
        ErrorKind::NoReleases => (
            "no-releases",
            "Keep a Changelog uses top-level level-two Markdown headings for releases, for example: ## [1.0.0] - 2026-11-04.",
        ),
        ErrorKind::MalformedHeading => (
            "malformed-heading",
            "For Keep a Changelog, use ## [version] - YYYY-MM-DD or ## [Unreleased]; balance brackets and give ### categories a name.",
        ),
        ErrorKind::UnsupportedStructure => (
            "unsupported-structure",
            "For Keep a Changelog, place one optional # title before all ## releases, and put ### categories inside a release.",
        ),
        _ => (
            "parse",
            "Check the supported Keep a Changelog syntax in the README.",
        ),
    }
}

pub const fn validation_details(code: DiagnosticCode) -> (&'static str, &'static str) {
    match code {
        DiagnosticCode::MissingTitle => (
            "missing-title",
            "Add a level-one title at the start, for example: # Changelog.",
        ),
        DiagnosticCode::MissingDate => (
            "missing-date",
            "Append a release date to the heading, for example: ## [1.0.0] - 2026-11-04.",
        ),
        DiagnosticCode::InvalidDate => (
            "invalid-date",
            "Use YYYY-MM-DD with a real Gregorian calendar date (years 0001–9999); February 29 requires a leap year.",
        ),
        DiagnosticCode::DatedPendingRelease => (
            "dated-pending-release",
            "Remove the date from Unreleased; assign a date when publishing the release.",
        ),
        DiagnosticCode::DuplicateRelease => (
            "duplicate-release",
            "Merge entries for the same release or give each published release a unique identifier; keep at most one Unreleased section.",
        ),
        DiagnosticCode::PendingReleaseOrder => (
            "pending-release-order",
            "Move the Unreleased section above all published releases.",
        ),
        DiagnosticCode::ReleaseDateOrder => (
            "release-date-order",
            "Order published releases newest first; check whether a date was entered incorrectly.",
        ),
        DiagnosticCode::CustomCategory => (
            "custom-category",
            "Standard categories are Added, Changed, Deprecated, Removed, Fixed, and Security. Custom categories are preserved.",
        ),
        DiagnosticCode::UncategorizedContent => (
            "uncategorized-content",
            "Place entries under a ### category, or under #### categories inside a ### component. Components may have introductory Markdown before their first category. Uncategorized content is preserved.",
        ),
        _ => (
            "validation",
            "Check the changelog conventions described in the README.",
        ),
    }
}
