//! Construction and semantic queries are part of the public library contract.

use logloom::{
    Category, Changelog, Diagnostic, DiagnosticCode, Entry, EntryKind, ErrorKind, Format,
    LinkDefinition, Metadata, ParseError, Release, ReleaseStatus, Section, Severity, Span, parse,
    parse_named,
};

#[test]
fn models_can_be_constructed_and_edited_without_struct_literals() {
    let mut document = Changelog::new(Format::GenericMarkdown);
    let mut metadata = Metadata::default();
    metadata.title = Some("News".into());
    document.metadata = metadata;
    let mut release = Release::new(Some("preview".into()));
    let mut section = Section::new(Some(Category::Added));
    let mut entry = Entry::new("- A library.".into(), EntryKind::ListItem);
    entry.span = Span::new(0, 12, 1);
    section.entries.push(entry);
    release.sections.push(section);
    release.links.push(LinkDefinition::new(
        "guide".into(),
        "https://example.com".into(),
    ));
    document.releases.push(release);
    assert!(document.releases[0].is_published());
    assert_eq!(document.releases[0].span, Span::default());
    let encoded = serde_json::to_string(&document).unwrap();
    assert_eq!(
        serde_json::from_str::<Changelog>(&encoded).unwrap(),
        document
    );
    let error = ParseError::new(ErrorKind::MalformedInput, Some(2), "invalid record");
    assert_eq!(error.to_string(), "line 2: invalid record");
    let mut diagnostic = Diagnostic::new(
        Severity::Warning,
        DiagnosticCode::MissingDate,
        "no date".into(),
    );
    diagnostic.line = Some(3);
    assert_eq!(diagnostic.line, Some(3));
}

#[test]
fn semantic_queries_cover_all_release_states_and_follow_edits() {
    let cases = [
        (
            "## [Unreleased]\n",
            Format::KeepAChangelog,
            ReleaseStatus::Pending,
        ),
        (
            "## [preview] - 2026-11-04\n",
            Format::KeepAChangelog,
            ReleaseStatus::Published,
        ),
        (
            "Release notes.",
            Format::GenericMarkdown,
            ReleaseStatus::Unversioned,
        ),
        (
            "2026-11-04  Alice <alice@example.com>\n\n\t* file: Fix it.\n",
            Format::GnuChangeLog,
            ReleaseStatus::DatedRecord,
        ),
        (
            r#"{"tag_name":"v2","draft":true,"prerelease":true,"published_at":null}"#,
            Format::GitHubReleaseNotes,
            ReleaseStatus::Draft,
        ),
    ];
    for (source, format, status) in cases {
        let model = parse(source, format).unwrap();
        let release = &model.releases[0];
        assert_eq!(release.status(), status);
        assert_eq!(
            release.is_pending(),
            matches!(status, ReleaseStatus::Pending | ReleaseStatus::Draft)
        );
        assert_eq!(
            release.is_published(),
            matches!(
                status,
                ReleaseStatus::Published | ReleaseStatus::DatedRecord
            )
        );
    }
    let fragment = parse_named("Fix it.", Format::Towncrier, "123.bugfix.rst").unwrap();
    assert!(fragment.releases[0].is_pending());
    let mut draft = Release::new(Some("v1".into()));
    draft.attributes.insert("draft".into(), "true".into());
    draft.attributes.insert("prerelease".into(), "true".into());
    draft
        .attributes
        .insert("package".into(), "@example/core".into());
    assert_eq!(draft.status(), ReleaseStatus::Draft);
    assert!(draft.is_prerelease());
    assert_eq!(draft.package(), Some("@example/core"));
    draft.attributes.remove("draft");
    draft.yanked = true;
    assert!(draft.is_published());
    let plain = Release::new(None);
    assert_eq!(plain.package(), None);
    assert!(!plain.is_prerelease());
}
