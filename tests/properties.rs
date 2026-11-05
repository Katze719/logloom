//! Generated inputs exercise source fidelity, offsets, ordering, and fallible parsing.

use logloom::{Changelog, Format, Span, detect, parse, parse_named, validate};
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;

const FORMATS: [Format; 9] = [
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

fn check_span(source: &str, span: Span) -> TestCaseResult {
    prop_assert!(source.get(span.start..span.end).is_some(), "{span:?}");
    let prefix = source
        .get(..span.start)
        .ok_or_else(|| TestCaseError::fail("invalid span start"))?;
    let line = prefix
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        .checked_add(1)
        .ok_or_else(|| TestCaseError::fail("line number overflow"))?;
    prop_assert_eq!(span.line, line);
    Ok(())
}

fn check_model(model: &Changelog, source: &str) -> TestCaseResult {
    prop_assert_eq!(model.source.as_str(), source);
    prop_assert!(
        model
            .releases
            .windows(2)
            .all(|pair| matches!(pair, [first, second] if first.span.start <= second.span.start))
    );
    for release in &model.releases {
        check_span(source, release.span)?;
        if let Some(span) = release.date_span {
            check_span(source, span)?;
        }
        for link in &release.links {
            check_span(source, link.span)?;
        }
        for section in &release.sections {
            check_span(source, section.span)?;
            for entry in &section.entries {
                check_span(source, entry.span)?;
            }
        }
    }
    for link in &model.links {
        check_span(source, link.span)?;
    }
    let encoded =
        serde_json::to_string(model).map_err(|error| TestCaseError::fail(error.to_string()))?;
    prop_assert_eq!(
        &serde_json::from_str::<Changelog>(&encoded)
            .map_err(|error| TestCaseError::fail(error.to_string()))?,
        model
    );
    let before = model.clone();
    let _ = validate(model);
    prop_assert_eq!(model, &before);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { max_shrink_iters: 2048, failure_persistence: Some(Box::new(proptest::test_runner::FileFailurePersistence::Direct("tests/properties.proptest-regressions"))), ..ProptestConfig::default() })]

    #[test]
    fn arbitrary_utf8_is_rejected_or_preserved_without_panics(chars in prop::collection::vec(any::<char>(), 0..2048)) {
        let source: String = chars.into_iter().collect();
        let _ = detect(&source);
        for format in FORMATS {
            if let Ok(model) = parse(&source, format) {
                prop_assert_eq!(model.format, format);
                check_model(&model, &source).unwrap();
            }
        }
        for name in ["123.bugfix.rst", "+orphan.feature.md", "123.misc", "invalid"] {
            if let Ok(model) = parse_named(&source, Format::Towncrier, name) {
                check_model(&model, &source).unwrap();
            }
        }
    }

    #[test]
    fn generated_markdown_preserves_entry_order_and_crlf(
        items in prop::collection::vec("[a-zA-Z0-9 🦀é]{1,60}", 1..50),
        crlf in any::<bool>(),
    ) {
        let body = items.iter().fold(String::new(), |mut body, item| {
            body.push_str("- ");
            body.push_str(item);
            body.push('\n');
            body
        });
        let mut source = format!("# Changelog\n\n## [Unreleased]\n\n### Added\n{body}");
        if crlf {
            source = source.replace('\n', "\r\n");
        }
        let model = parse(&source, Format::KeepAChangelog).unwrap();
        check_model(&model, &source).unwrap();
        let entries = &model.releases[0].sections[0].entries;
        prop_assert_eq!(entries.len(), items.len());
        for (entry, item) in entries.iter().zip(items) {
            let expected = format!("- {item}");
            prop_assert_eq!(entry.markdown.as_str(), expected.trim());
        }
    }

    #[test]
    fn structured_bodies_keep_local_links_and_json_token_offsets(
        bodies in prop::collection::vec(prop::collection::vec(any::<char>(), 0..256), 1..12),
    ) {
        let items: Vec<_> = bodies.into_iter().enumerate().map(|(index, chars)| {
            let text: String = chars.into_iter().collect();
            serde_json::json!({"tag_name": format!("v{index}"), "body": text})
        }).collect();
        let source = serde_json::to_string_pretty(&items).unwrap();
        if let Ok(model) = parse(&source, Format::GitHubReleaseNotes) {
            check_model(&model, &source).unwrap();
            prop_assert_eq!(model.releases.len(), items.len());
        }
    }
}
