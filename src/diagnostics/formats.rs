use logloom::{DiagnosticCode, ErrorKind, Format};

pub const fn parse_details(format: Format, kind: ErrorKind) -> (&'static str, &'static str) {
    let id = match kind {
        ErrorKind::EmptyInput => "empty-input",
        ErrorKind::NoReleases => "no-releases",
        ErrorKind::MalformedHeading => "malformed-heading",
        ErrorKind::MalformedInput => "malformed-input",
        ErrorKind::UnsupportedStructure => "unsupported-structure",
        _ => "parse",
    };
    let help = match format {
        Format::ConventionalChangelog => {
            "Conventional Changelog uses version headings such as # 2.0.0 (2026-11-04), ## [1.2.0] (2026-11-03), or ### 1.2.1 (2026-11-04), with categories below them."
        }
        Format::GenericMarkdown => {
            "Generic Markdown accepts # or ## version headings, ## named releases, and ### categories; a body without releases is retained as unversioned notes."
        }
        Format::GitHubReleaseNotes => {
            "For GitHub Release Notes, supply a Markdown body or a JSON release object/array with tag_name and an optional string body; use a UTC ISO published_at timestamp."
        }
        Format::Changesets => {
            "Changesets accepts package histories (# package, ## version, ### Patch Changes) or a fragment with --- delimiters, flat YAML package: major/minor/patch mappings, and a Markdown summary."
        }
        Format::Towncrier => {
            "Towncrier accepts Markdown or underlined reStructuredText release headings such as Project 1.2.0 (2026-11-04), or fragments named ISSUE.TYPE[.suffix]; use --fragment-name for stdin."
        }
        Format::DebianChangelog => {
            "Debian Changelog needs 'package (version) distribution; urgency=medium', indented changes, and ' -- Name <email>  Wed, 04 Nov 2026 12:00:00 +0000' (two spaces before the date)."
        }
        Format::GnuNews => {
            "GNU NEWS uses '* Noteworthy changes in release 1.2 (2026-11-04)' followed by '**' category headings and change paragraphs."
        }
        Format::GnuChangeLog => {
            "GNU ChangeLog uses 'YYYY-MM-DD  Author <email>' records followed by indented changes such as '* file.c (function): Describe the change.'."
        }
        _ => "Check the syntax of the selected input format in docs/formats/.",
    };
    (id, help)
}

pub const fn validation_details(
    format: Format,
    code: DiagnosticCode,
) -> (&'static str, &'static str) {
    match (format, code) {
        (Format::DebianChangelog, DiagnosticCode::InvalidDate) => (
            "invalid-date",
            "Correct the calendar date in the Debian maintainer trailer; its timestamp must use an RFC 2822 date such as Wed, 04 Nov 2026 12:00:00 +0000.",
        ),
        (Format::GitHubReleaseNotes, DiagnosticCode::InvalidDate) => (
            "invalid-date",
            "Correct the date in the GitHub release's UTC published_at timestamp or version heading.",
        ),
        (Format::Towncrier, DiagnosticCode::InvalidDate) => (
            "invalid-date",
            "Use a real YYYY-MM-DD date in the Towncrier release heading: Project 1.2.0 (2026-11-04).",
        ),
        (_, DiagnosticCode::InvalidDate) => (
            "invalid-date",
            "Use a real Gregorian YYYY-MM-DD date in the selected format's release header; February 29 requires a leap year.",
        ),
        (_, DiagnosticCode::DuplicateRelease) => (
            "duplicate-release",
            "Keep each version identifier unique within a package; merge repeated release sections or correct the identifier.",
        ),
        _ => (
            "validation",
            "Check the conventions of the selected input format in docs/formats/.",
        ),
    }
}
