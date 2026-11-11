//! Component-scoped Keep a Changelog categories and compatibility regressions.

use logloom::{Category, Changelog, DiagnosticCode, EntryKind, ErrorKind, Format, parse, validate};

const EXTENDED: &str = include_str!("../examples/extended-keep-a-changelog.md");
const COMPONENTS: &str = include_str!("fixtures/components.md");

#[test]
fn extended_example_exposes_categories_for_their_component() {
    let changelog = parse(EXTENDED, Format::KeepAChangelog).unwrap();
    assert_eq!(changelog.releases.len(), 4);
    let release = &changelog.releases[3];
    assert_eq!(release.version.as_deref(), Some("1.0.0"));
    assert_eq!(release.sections.len(), 3);
    assert_eq!(
        release.sections[0].component.as_deref(),
        Some("Firmware components")
    );
    assert_eq!(release.sections[0].category, None);
    assert_eq!(release.sections[0].entries[0].kind, EntryKind::Markdown);
    assert_eq!(
        release.sections[1].component.as_deref(),
        Some("Firmware components")
    );
    assert_eq!(release.sections[1].category, Some(Category::Added));
    assert_eq!(release.sections[1].heading.as_deref(), Some("#### Added"));
    assert_eq!(
        release.sections[1].entries[0].markdown,
        "- Initial firmware components for Lantern."
    );
    assert_eq!(
        release.sections[2].component.as_deref(),
        Some("Firmware components")
    );
    assert_eq!(release.sections[2].category, Some(Category::Changed));
    assert!(
        changelog.releases[0]
            .sections
            .iter()
            .all(|section| section.component.is_none())
    );
    assert_eq!(validate(&changelog), Vec::new());
    assert_eq!(changelog.source, EXTENDED);
    let encoded = serde_json::to_string(&changelog).unwrap();
    assert_eq!(
        serde_json::from_str::<Changelog>(&encoded).unwrap(),
        changelog
    );
}

#[test]
fn mixed_components_and_release_wide_categories_preserve_order_and_reset_scope() {
    let changelog = parse(COMPONENTS, Format::KeepAChangelog).unwrap();
    let sections = &changelog.releases[0].sections;
    let groups: Vec<_> = sections
        .iter()
        .map(|section| (section.component.as_deref(), section.category.clone()))
        .collect();
    assert_eq!(
        groups,
        [
            (None, Some(Category::Added)),
            (Some("Firmware components"), Some(Category::Added)),
            (Some("Firmware components"), Some(Category::Fixed)),
            (Some("CLI 🦀"), Some(Category::Changed)),
            (Some("CLI 🦀"), Some(Category::Changed)),
            (
                Some("CLI 🦀"),
                Some(Category::Custom("Performance".to_owned()))
            ),
            (None, Some(Category::Fixed)),
            (Some("Firmware components"), Some(Category::Removed)),
        ]
    );
    assert_eq!(sections[2].entries, Vec::new());
    assert_eq!(sections[1].entries.len(), 1);
    assert!(sections[1].entries[0].markdown.contains("#### Security"));
    assert!(sections[1].entries[0].markdown.contains("nested details"));
    assert_eq!(changelog.releases[1].sections[0].component, None);
    assert_eq!(changelog.releases[1].sections[1].component, None);
    assert_eq!(
        changelog.releases[1].sections[1].category,
        Some(Category::Custom("Documentation".to_owned()))
    );
}

#[test]
fn component_names_do_not_warn_but_custom_subcategories_still_do() {
    let changelog = parse(COMPONENTS, Format::KeepAChangelog).unwrap();
    let diagnostics = validate(&changelog);
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == DiagnosticCode::CustomCategory)
    );
    for diagnostic in diagnostics {
        let line = diagnostic.line.unwrap();
        let source = COMPONENTS
            .lines()
            .nth(line.checked_sub(1).unwrap())
            .unwrap();
        assert!(matches!(source, "#### Performance" | "### Documentation"));
    }
}

#[test]
fn component_prose_before_its_first_category_is_preserved() {
    let input = "# Changelog\n## Unreleased\n### Firmware\nComponent introduction.\n\n#### Added\n- A driver.\n";
    let changelog = parse(input, Format::KeepAChangelog).unwrap();
    let sections = &changelog.releases[0].sections;
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].component.as_deref(), Some("Firmware"));
    assert_eq!(sections[0].category, None);
    assert_eq!(sections[0].entries[0].markdown, "Component introduction.");
    assert_eq!(sections[1].category, Some(Category::Added));
    assert_eq!(validate(&changelog), Vec::new());
}

#[test]
fn component_descriptions_accept_multiple_markdown_blocks_and_category_descriptions() {
    let input = "# Changelog\n## Unreleased\n### Firmware\nFirst paragraph with **emphasis**.\n\nSecond paragraph.\n\n> Applies to production images.\n\n#### Added\nCategory description.\n\n- A driver.\n### Build tools\nBuild-time changes only.\n\n#### Fixed\n- A build fix.\n### Firmware\nAnother component introduction.\n\n#### Changed\n- An API update.\n";
    let changelog = parse(input, Format::KeepAChangelog).unwrap();
    let sections = &changelog.releases[0].sections;
    assert_eq!(sections.len(), 6);
    assert_eq!(sections[0].entries.len(), 3);
    assert_eq!(sections[1].entries[0].markdown, "Category description.");
    assert_eq!(sections[2].component.as_deref(), Some("Build tools"));
    assert_eq!(sections[4].component.as_deref(), Some("Firmware"));
    assert_eq!(validate(&changelog), Vec::new());
}

#[test]
fn component_introductions_accept_lists() {
    for body in [
        "- An uncategorized change.\n",
        "1. An uncategorized change.\n",
    ] {
        let input = format!(
            "# Changelog\n## Unreleased\n### Firmware\nComponent description.\n\n{body}\n#### Added\n- A driver.\n"
        );
        let changelog = parse(&input, Format::KeepAChangelog).unwrap();
        assert_eq!(validate(&changelog), Vec::new());
        assert_eq!(changelog.releases[0].sections[0].entries.len(), 2);
        assert_eq!(
            changelog.releases[0].sections[0].entries[1].kind,
            EntryKind::ListItem
        );
    }
}

#[test]
fn standard_h3_categories_keep_h4_subheadings_as_markdown() {
    let input = "# Changelog\n## Unreleased\n### Added\n#### Changed\n- A detailed note.\n";
    let changelog = parse(input, Format::KeepAChangelog).unwrap();
    let sections = &changelog.releases[0].sections;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].component, None);
    assert_eq!(sections[0].category, Some(Category::Added));
    assert_eq!(sections[0].entries[0].markdown, "#### Changed");
    assert_eq!(sections[0].entries[0].kind, EntryKind::Markdown);
}

#[test]
fn quoted_listed_and_fenced_headings_do_not_create_components() {
    for body in [
        "> #### Added\n",
        "```md\n#### Added\n```\n",
        "- A list item\n  #### Added\n",
    ] {
        let input = format!("# Changelog\n## Unreleased\n### Documentation\n{body}");
        let changelog = parse(&input, Format::KeepAChangelog).unwrap();
        assert_eq!(changelog.releases[0].sections.len(), 1);
        assert_eq!(changelog.releases[0].sections[0].component, None);
        assert_eq!(
            changelog.releases[0].sections[0].category,
            Some(Category::Custom("Documentation".to_owned()))
        );
    }
}

#[test]
fn empty_component_categories_are_retained_and_empty_names_are_errors() {
    let changelog = parse(
        "# Changelog\n## Unreleased\n### Firmware\n#### Added\n#### Fixed\n",
        Format::KeepAChangelog,
    )
    .unwrap();
    assert_eq!(changelog.releases[0].sections.len(), 2);
    assert!(
        changelog.releases[0]
            .sections
            .iter()
            .all(|section| section.entries.is_empty())
    );
    assert_eq!(validate(&changelog), Vec::new());
    let malformed = parse(
        "## Unreleased\n### Firmware\n####\n",
        Format::KeepAChangelog,
    )
    .unwrap_err();
    assert_eq!(malformed.kind, ErrorKind::MalformedHeading);
    assert_eq!(malformed.line, Some(3));
    let unsupported = parse(
        "### Firmware\n#### Added\n## Unreleased\n",
        Format::KeepAChangelog,
    )
    .unwrap_err();
    assert_eq!(unsupported.kind, ErrorKind::UnsupportedStructure);
    assert_eq!(unsupported.line, Some(1));
}

#[test]
fn component_spans_and_names_survive_crlf_and_unicode() {
    let input = COMPONENTS.replace("\r\n", "\n").replace('\n', "\r\n");
    let changelog = parse(&input, Format::KeepAChangelog).unwrap();
    assert_eq!(changelog.source, input);
    assert_eq!(
        changelog.releases[0].sections[3].component.as_deref(),
        Some("CLI 🦀")
    );
    for section in &changelog.releases[0].sections {
        assert_eq!(
            section.heading.as_deref().unwrap(),
            input
                .get(section.span.start..section.span.end)
                .unwrap()
                .trim()
        );
        for entry in &section.entries {
            assert_eq!(
                entry.markdown,
                input.get(entry.span.start..entry.span.end).unwrap().trim()
            );
        }
    }
}

#[test]
fn standard_changelog_json_does_not_gain_component_fields() {
    let changelog = parse(include_str!("fixtures/standard.md"), Format::KeepAChangelog).unwrap();
    let encoded = serde_json::to_string(&changelog).unwrap();
    assert!(!encoded.contains("\"component\""));
    let decoded: Changelog = serde_json::from_str(&encoded).unwrap();
    assert!(
        decoded
            .releases
            .iter()
            .flat_map(|release| &release.sections)
            .all(|section| section.component.is_none())
    );
}

#[test]
fn repeated_large_component_names_are_bounded() {
    let input = format!(
        "# Changelog\n## Unreleased\n### {}\n{}",
        "x".repeat(1_000_000),
        "#### Added\n".repeat(18)
    );
    let result = std::panic::catch_unwind(|| parse(&input, Format::KeepAChangelog)).unwrap();
    assert_eq!(result.unwrap_err().kind, ErrorKind::ResourceLimit);
}
