//! Saved documents protect legacy decoding and the exported field contract.

use logloom::{Changelog, Format, parse};

const CASES: [(Format, &str, &str); 2] = [
    (
        Format::KeepAChangelog,
        include_str!("fixtures/contracts/legacy-input.md"),
        include_str!("fixtures/contracts/legacy-export.json"),
    ),
    (
        Format::GitHubReleaseNotes,
        include_str!("fixtures/contracts/scoped-input.json"),
        include_str!("fixtures/contracts/scoped-export.json"),
    ),
];

#[test]
fn saved_exports_decode_and_match_the_current_model_without_source() {
    for (format, source, encoded) in CASES {
        let mut model = parse(source, format).unwrap();
        model.source.clear();
        let decoded: Changelog = serde_json::from_str(encoded).unwrap();
        assert_eq!(decoded, model);
        let expected: serde_json::Value = serde_json::from_str(encoded).unwrap();
        let mut actual = serde_json::to_value(model).unwrap();
        actual.as_object_mut().unwrap().remove("source");
        assert_eq!(actual, expected);
    }
}

#[test]
fn older_fields_default_and_unknown_additions_are_ignored() {
    let mut encoded: serde_json::Value = serde_json::from_str(CASES[0].2).unwrap();
    let object = encoded.as_object_mut().unwrap();
    object.remove("format");
    object.insert("future_field".into(), serde_json::json!({"value": 42}));
    let release = encoded["releases"][0].as_object_mut().unwrap();
    release.insert("future_release_field".into(), serde_json::json!([1, 2, 3]));
    release.remove("attributes");
    release.remove("date_span");
    release.remove("links");
    let model: Changelog = serde_json::from_value(encoded).unwrap();
    assert_eq!(model.format, Format::KeepAChangelog);
    assert_eq!(model.source, "");
    assert_eq!(model.releases[0].links, Vec::new());
    assert_eq!(
        model.releases[0].attributes,
        std::collections::BTreeMap::new()
    );
}

#[cfg(feature = "cli")]
#[test]
fn cli_json_exports_match_saved_documents() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    for (format, source, expected) in CASES {
        let mut child = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args(["parse", "-", "--input-format", format.as_str()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
            serde_json::from_str::<serde_json::Value>(expected).unwrap()
        );
    }
}
