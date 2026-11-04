//! Conservative signatures, ambiguity, bounded inspection, and filename hints.

use logloom::{DetectionError, ErrorKind, Format, MAX_INPUT_BYTES, detect, detect_named, parse};

#[test]
fn examples_are_detected_from_signatures_across_all_formats() {
    for (input, format) in [
        (
            include_str!("../examples/keep-a-changelog.md"),
            Format::KeepAChangelog,
        ),
        (
            include_str!("../examples/extended-keep-a-changelog.md"),
            Format::KeepAChangelog,
        ),
        (
            include_str!("../examples/conventional-changelog.md"),
            Format::ConventionalChangelog,
        ),
        (
            include_str!("../examples/generic-markdown.md"),
            Format::GenericMarkdown,
        ),
        (
            include_str!("../examples/github-release-notes.json"),
            Format::GitHubReleaseNotes,
        ),
        (
            include_str!("../examples/github-release-notes.md"),
            Format::GitHubReleaseNotes,
        ),
        (
            include_str!("../examples/changesets.md"),
            Format::Changesets,
        ),
        (
            include_str!("../examples/changesets-fragment.md"),
            Format::Changesets,
        ),
        (include_str!("../examples/towncrier.md"), Format::Towncrier),
        (include_str!("../examples/towncrier.rst"), Format::Towncrier),
        (
            include_str!("../examples/debian.changelog"),
            Format::DebianChangelog,
        ),
        (include_str!("../examples/gnu-news.txt"), Format::GnuNews),
        (
            include_str!("../examples/gnu-ChangeLog"),
            Format::GnuChangeLog,
        ),
    ] {
        assert_eq!(detect(input).unwrap(), format);
        assert_eq!(detect(&input.replace('\n', "\r\n")).unwrap(), format);
    }
}

#[test]
fn overlapping_or_weak_signatures_are_never_resolved_by_parser_order() {
    for input in [
        "# Changelog\n\n## 1.0.0 (2026-11-04)\n\n- A change.\n",
        "# Release notes\n\n- A change.\n",
        "## What's Changed\n\n- A change.\n",
    ] {
        let error = detect(input).unwrap_err();
        assert!(matches!(error, DetectionError::Ambiguous { .. }));
        assert!(error.to_string().contains("generic-markdown"));
    }
    let error =
        detect("# Changelog\n\n## [Unreleased]\n\n### Patch Changes\n\n- A change.\n").unwrap_err();
    assert_eq!(
        error,
        DetectionError::Ambiguous {
            candidates: vec![Format::KeepAChangelog, Format::Changesets]
        }
    );
}

#[test]
fn ordinary_names_and_extensions_never_supply_a_format() {
    for name in [
        "CHANGELOG.md",
        "NEWS.rst",
        "notes.json",
        "1.0.md",
        "release.custom.md",
    ] {
        assert_eq!(
            detect_named("Just some prose.", name),
            Err(DetectionError::Unrecognized)
        );
    }
    assert_eq!(
        detect("{\"payload\":{\"tag_name\":\"v1\"}}"),
        Err(DetectionError::Unrecognized)
    );
    assert_eq!(
        detect("[[{\"tag_name\":\"v1\"}]]"),
        Err(DetectionError::Unrecognized)
    );
}

#[test]
fn named_towncrier_fragments_are_detected_but_conflicting_content_is_ambiguous() {
    for name in [
        "123.bugfix.rst",
        "123.custom.md",
        "+new.feature.md",
        "42.misc.rst",
    ] {
        assert_eq!(
            detect_named("A small fix.", name).unwrap(),
            Format::Towncrier
        );
    }
    assert_eq!(detect_named("", "42.misc.rst").unwrap(), Format::Towncrier);
    let error = detect_named("## [Unreleased]\n", "42.feature.md").unwrap_err();
    assert_eq!(
        error,
        DetectionError::Ambiguous {
            candidates: vec![Format::KeepAChangelog, Format::Towncrier]
        }
    );
}

#[test]
fn fenced_quoted_and_listed_headings_do_not_identify_a_format() {
    for input in [
        "```markdown\n## [Unreleased]\n### Patch Changes\n```\n",
        "> ## [Unreleased]\n",
    ] {
        assert_eq!(detect(input), Err(DetectionError::Unrecognized));
    }
    let error = detect("- ## [Unreleased]\n").unwrap_err();
    assert!(
        matches!(error, DetectionError::Ambiguous { candidates } if !candidates.contains(&Format::KeepAChangelog))
    );
    let error = detect("GNU Project notes\n\n```text\nVersion 1.0\n```\n").unwrap_err();
    assert_eq!(error, DetectionError::Unrecognized);
    let error =
        detect("## What's Changed\n\n```text\nhttps://github.com/example/repo\n```\n").unwrap_err();
    assert!(matches!(error, DetectionError::Ambiguous { .. }));
}

#[test]
fn inspection_is_distinct_from_parsing_and_validation() {
    let malformed = "# Changelog\n\n## [1.0 - 2026-11-04\n";
    assert_eq!(detect(malformed).unwrap(), Format::KeepAChangelog);
    assert!(parse(malformed, Format::KeepAChangelog).is_err());
    let input = "{\"tag_name\":42,\"body\":\"notes\"}";
    assert_eq!(detect(input).unwrap(), Format::GitHubReleaseNotes);
    assert!(parse(input, Format::GitHubReleaseNotes).is_err());
}

#[test]
fn empty_oversized_deep_and_invalid_json_inputs_have_typed_failures() {
    assert!(
        matches!(detect("\n "), Err(DetectionError::InvalidInput(error)) if error.kind == ErrorKind::EmptyInput)
    );
    let oversized = "a".repeat(MAX_INPUT_BYTES.checked_add(1).unwrap());
    assert!(
        matches!(detect(&oversized), Err(DetectionError::InvalidInput(error)) if error.kind == ErrorKind::ResourceLimit)
    );
    let nested = format!("{}content\n", "> ".repeat(130));
    assert!(
        matches!(detect(&nested), Err(DetectionError::InvalidInput(error)) if error.kind == ErrorKind::ResourceLimit)
    );
    let error = detect("{\n\"tag_name\":\n}").unwrap_err();
    assert!(
        matches!(error, DetectionError::InvalidInput(ref error) if error.kind == ErrorKind::MalformedInput && error.line == Some(3))
    );
    assert!(std::error::Error::source(&error).is_some());
    let nested_json = format!("{}0{}", "[".repeat(130), "]".repeat(130));
    assert!(detect(&nested_json).is_err());
}
