use clap::ValueEnum;

use super::{InputFormat, render::OutputFormat};

pub fn parse() -> String {
    format!(
        "Input formats:\n{}\n\nOutput formats:\n{}\n\nExamples:\n  logloom parse --latest --output-format text\n  logloom parse --release 1.2.0 -o release.json\n  logloom parse - --input-format keep-a-changelog",
        formats::<InputFormat>(),
        formats::<OutputFormat>(),
    )
}

pub fn validate() -> String {
    format!(
        "Input formats:\n{}\n\nExamples:\n  logloom validate\n  logloom validate --strict --release 1.2.0\n  logloom validate --release 2.0.0 --package @lantern/core\n  logloom validate other.md --input-format keep-a-changelog",
        formats::<InputFormat>()
    )
}

fn formats<T: ValueEnum>() -> String {
    let names: Vec<_> = T::value_variants()
        .iter()
        .filter_map(T::to_possible_value)
        .map(|value| value.get_name().to_owned())
        .collect();
    names
        .chunks(3)
        .map(|row| format!("  {}", row.join(", ")))
        .collect::<Vec<_>>()
        .join("\n")
}
