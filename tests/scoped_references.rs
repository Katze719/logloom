//! Independent GitHub bodies retain independent Markdown reference scopes.

use logloom::{Changelog, Format, parse};

fn input() -> String {
    serde_json::json!([
        {"tag_name": "v1", "body": "- [First **change**][issue] and [issue][] and [issue].\n- [![icon][issue]][issue]\n- [Escaped][a\\]b] and `literal [issue]`.\n\n[issue]: https://example.com/one \"First title\"\n[a\\]b]: https://example.com/escaped-one\n"},
        {"tag_name": "v2", "body": "- [Second change][ISSUE] and ![image][issue].\n\n[issue]: https://example.com/two \"Second title\"\n"}
    ]).to_string()
}

#[test]
fn json_bodies_keep_local_definitions_raw_markdown_and_token_spans() {
    let source = input();
    let model = parse(&source, Format::GitHubReleaseNotes).unwrap();
    assert_eq!(model.links, Vec::new());
    assert_eq!(model.source, source);
    assert_eq!(model.releases[0].links.len(), 2);
    assert_eq!(model.releases[1].links.len(), 1);
    assert_eq!(
        model.releases[0].links[0].destination,
        "https://example.com/one"
    );
    assert_eq!(
        model.releases[1].links[0].destination,
        "https://example.com/two"
    );
    assert!(
        model.releases[0].sections[0].entries[0]
            .markdown
            .contains("[First **change**][issue]")
    );
    for release in &model.releases {
        for link in &release.links {
            let token = source.get(link.span.start..link.span.end).unwrap();
            assert!(token.starts_with('"'));
            assert!(token.ends_with('"'));
            assert_eq!(link.span, release.sections[0].entries[0].span);
        }
    }
    let encoded = serde_json::to_string(&model).unwrap();
    assert_eq!(serde_json::from_str::<Changelog>(&encoded).unwrap(), model);
}

#[cfg(feature = "cli")]
mod cli {
    use std::io::{self, Write};
    use std::process::{Command, Output, Stdio};

    use pulldown_cmark::{Event, Parser, Tag};

    use super::input;
    use logloom::Changelog;

    fn export(format: &str, release: Option<&str>) -> io::Result<Output> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_logloom"));
        command.args([
            "parse",
            "-",
            "--input-format",
            "github-release-notes",
            "--output-format",
            format,
        ]);
        if let Some(version) = release {
            command.args(["--release", version]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("missing stdin"))?
            .write_all(input().as_bytes())?;
        child.wait_with_output()
    }

    #[test]
    fn text_and_markdown_links_resolve_within_each_body_after_filtering() {
        for selected in [None, Some("v1"), Some("v2")] {
            let text = export("text", selected).unwrap();
            assert!(text.status.success(), "{:?}", text.stderr);
            let text = String::from_utf8(text.stdout).unwrap();
            if selected != Some("v2") {
                assert!(text.contains("First change <https://example.com/one>"));
                assert!(text.contains("Escaped <https://example.com/escaped-one>"));
            }
            if selected != Some("v1") {
                assert!(text.contains("Second change <https://example.com/two>"));
            }

            let markdown = export("markdown", selected).unwrap();
            assert!(markdown.status.success(), "{:?}", markdown.stderr);
            let markdown = String::from_utf8(markdown.stdout).unwrap();
            let urls: Vec<_> = Parser::new(&markdown)
                .filter_map(|event| match event {
                    Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                        Some(dest_url.to_string())
                    }
                    _ => None,
                })
                .collect();
            let mut expected = Vec::new();
            if selected != Some("v2") {
                expected.extend(std::iter::repeat_n("https://example.com/one", 5));
                expected.push("https://example.com/escaped-one");
                assert!(
                    markdown
                        .contains("[First **change**](<https://example.com/one> \"First title\")")
                );
                assert!(markdown.contains("`literal [issue]`"));
            }
            if selected != Some("v1") {
                expected.extend(std::iter::repeat_n("https://example.com/two", 2));
            }
            assert_eq!(urls, expected, "{markdown}");
        }
    }

    #[test]
    fn filtered_document_exports_keep_only_the_selected_bodys_definitions() {
        for format in [
            "json",
            "json-compact",
            #[cfg(feature = "yaml")]
            "yaml",
            #[cfg(feature = "toml")]
            "toml",
        ] {
            for (version, url) in [
                ("v1", "https://example.com/one"),
                ("v2", "https://example.com/two"),
            ] {
                let output = export(format, Some(version)).unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let model: Changelog = match format {
                    #[cfg(feature = "yaml")]
                    "yaml" => yaml_serde::from_slice(&output.stdout).unwrap(),
                    #[cfg(feature = "toml")]
                    "toml" => toml::from_str(std::str::from_utf8(&output.stdout).unwrap()).unwrap(),
                    _ => serde_json::from_slice(&output.stdout).unwrap(),
                };
                assert_eq!(model.releases.len(), 1);
                assert_eq!(model.releases[0].links[0].destination, url);
                assert_eq!(model.source, "");
            }
        }
    }
}
