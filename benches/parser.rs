//! Dependency-free, reproducible end-to-end parser timings.

use std::error::Error;
use std::hint::black_box;
use std::io::{self, Write};
use std::num::NonZeroU32;
use std::time::Instant;

use logloom::{Format, detect, parse, validate};

fn main() -> Result<(), Box<dyn Error>> {
    let iterations: NonZeroU32 = std::env::var("LOGLOOM_BENCH_ITERATIONS")
        .unwrap_or_else(|_| "5".to_owned())
        .parse()?;
    let mut output = io::stdout().lock();
    writeln!(
        output,
        "case,bytes,iterations,parse_ms,detect_parse_validate_ms"
    )?;
    for (name, input, format) in samples() {
        let start = Instant::now();
        for _ in 0..iterations.get() {
            black_box(parse(black_box(&input), format)?);
        }
        let parse_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(iterations.get());
        let start = Instant::now();
        for _ in 0..iterations.get() {
            let detected = detect(black_box(&input))?;
            let model = parse(&input, detected)?;
            black_box(validate(&model));
        }
        let pipeline_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(iterations.get());
        writeln!(
            output,
            "{name},{},{iterations},{parse_ms:.3},{pipeline_ms:.3}",
            input.len()
        )?;
    }
    Ok(())
}

fn samples() -> Vec<(&'static str, String, Format)> {
    let markdown = |count| {
        format!(
            "# Changelog\n\n## [Unreleased]\n\n### Added\n{}",
            "- Preserve **Unicode** 🦀 and [references][guide].\n".repeat(count)
        ) + "\n[guide]: https://example.com/guide\n"
    };
    let body = "- A change with a [reference][issue].\n".repeat(1000)
        + "\n[issue]: https://example.com/issue\n";
    let github = (0..100)
        .map(|index| {
            serde_json::json!({
                "tag_name": format!("v{index}"), "body": body,
            })
        })
        .collect();
    vec![
        ("markdown-small", markdown(200), Format::KeepAChangelog),
        (
            "markdown-many-entries",
            markdown(20_000),
            Format::KeepAChangelog,
        ),
        (
            "markdown-near-limit",
            markdown(300_000),
            Format::KeepAChangelog,
        ),
        (
            "markdown-tiny-entries",
            format!(
                "# Changelog\n\n## [Unreleased]\n\n### Added\n{}",
                "- x\n".repeat(250_000)
            ),
            Format::KeepAChangelog,
        ),
        (
            "markdown-long-line",
            format!(
                "# Changelog\n\n## [Unreleased]\n\n### Added\n- {}\n",
                "🦀".repeat(1_000_000)
            ),
            Format::KeepAChangelog,
        ),
        (
            "github-many-bodies",
            serde_json::Value::Array(github).to_string(),
            Format::GitHubReleaseNotes,
        ),
        (
            "markdown-nested",
            format!(
                "# Changelog\n\n## [Unreleased]\n\n{}deep\n",
                "> ".repeat(100)
            ),
            Format::KeepAChangelog,
        ),
    ]
}
