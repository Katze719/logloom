//! Upstream excerpts are checked offline; provenance and licenses accompany them.

use logloom::{DiagnosticCode, Format, detect, parse, validate};

#[test]
fn upstream_histories_preserve_release_identifiers_content_and_original_text() {
    let cases: &[(Format, &str, &[Option<&str>], &str)] = &[
        (
            Format::KeepAChangelog,
            include_str!("fixtures/corpus/clap.md"),
            &[Some("5.0.0"), None, Some("4.5.53"), Some("4.5.52")],
            "default_values_if",
        ),
        (
            Format::ConventionalChangelog,
            include_str!("fixtures/corpus/conventional.md"),
            &[Some("8.1.3"), Some("8.1.2")],
            "conventional-commits-filter",
        ),
        (
            Format::Changesets,
            include_str!("fixtures/corpus/changesets.md"),
            &[Some("2.29.7"), Some("2.29.6"), Some("2.29.5")],
            "@changesets/apply-release-plan@7.0.13",
        ),
        (
            Format::Towncrier,
            include_str!("fixtures/corpus/towncrier.rst"),
            &[Some("24.8.0")],
            "towncrier.__version__",
        ),
    ];
    for &(format, source, versions, content) in cases {
        let model = parse(source, format).unwrap();
        assert_eq!(model.source, source);
        assert_eq!(
            model
                .releases
                .iter()
                .map(|release| release.version.as_deref())
                .collect::<Vec<_>>(),
            versions
        );
        assert!(
            model
                .releases
                .iter()
                .flat_map(|release| &release.sections)
                .flat_map(|section| &section.entries)
                .any(|entry| entry.markdown.contains(content))
        );
        if format == Format::KeepAChangelog {
            // Upstream's "TBD" and "ReleaseDate" are deliberately retained.
            assert_eq!(
                validate(&model)
                    .iter()
                    .filter(|diagnostic| diagnostic.code == DiagnosticCode::InvalidDate)
                    .count(),
                2
            );
        } else {
            assert_eq!(validate(&model), Vec::new());
        }
        if format != Format::Towncrier {
            assert_eq!(detect(source).unwrap(), format);
        }
    }
}
