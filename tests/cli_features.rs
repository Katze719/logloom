//! Optional exports and completion scripts reflect the compiled CLI.

#![cfg(feature = "cli")]

use std::process::Command;

#[test]
fn completion_scripts_include_current_commands_and_filters() {
    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args(["completions", shell])
            .output()
            .unwrap();
        assert!(output.status.success(), "{shell}: {:?}", output.stderr);
        assert_eq!(output.stderr, Vec::<u8>::new());
        let script = String::from_utf8(output.stdout).unwrap();
        for fragment in [
            "logloom",
            "parse",
            "convert",
            "validate",
            "detect",
            "input-format",
            "release",
            "yes-i-know-i-am-stupid",
        ] {
            assert!(script.contains(fragment), "{shell}: {fragment}");
        }
        assert!(!script.contains('\u{1b}'));
    }
    for arguments in [vec!["completions"], vec!["completions", "unknown"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
}

#[test]
fn help_presents_the_size_override_by_its_primary_name_and_visible_alias() {
    for args in [
        vec!["--help"],
        vec!["parse", "--help"],
        vec!["validate", "--help"],
        vec!["detect", "--help"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stderr, Vec::<u8>::new());
        let help = String::from_utf8(output.stdout).unwrap();
        let line = help
            .lines()
            .find(|line| line.trim_start().starts_with("--yes-i-know-i-am-stupid"))
            .unwrap();
        assert!(line.contains("Read inputs larger than 16 MiB"));
        assert!(help.contains("[alias: --no-size-limit]"));
        assert!(
            !help
                .lines()
                .any(|line| line.trim_start().starts_with("--no-size-limit"))
        );
    }
}

#[test]
fn optional_exports_are_available_exactly_when_their_feature_is_enabled() {
    for (name, enabled) in [
        ("yaml", cfg!(feature = "yaml")),
        ("toml", cfg!(feature = "toml")),
        ("csv", cfg!(feature = "csv")),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args([
                "parse",
                "examples/keep-a-changelog.md",
                "--output-format",
                name,
            ])
            .output()
            .unwrap();
        if enabled {
            assert!(output.status.success(), "{name}: {:?}", output.stderr);
            assert_ne!(output.stdout, Vec::<u8>::new());
        } else {
            assert_eq!(output.status.code(), Some(2));
            assert_eq!(output.stdout, Vec::<u8>::new());
        }
    }
}
