use std::error::Error;
use std::io::{self, Write};

use clap::ValueEnum;
use logloom::{Changelog, Release};

mod readable;
mod table;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Pretty-printed JSON of the full model.
    Json,
    /// Single-line JSON of the full model.
    JsonCompact,
    /// Readable text with Markdown styling removed.
    Text,
    /// YAML of the full model.
    Yaml,
    /// Markdown of the selected releases.
    Markdown,
    /// TOML of the full model, omitting absent optional fields.
    Toml,
    /// One compact release object per line.
    Jsonl,
    /// A header and one row per entry, including empty groups.
    Csv,
}

impl OutputFormat {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::JsonCompact => "json-compact",
            Self::Text => "text",
            Self::Yaml => "yaml",
            Self::Markdown => "markdown",
            Self::Toml => "toml",
            Self::Jsonl => "jsonl",
            Self::Csv => "csv",
        }
    }
}

pub fn render(changelog: &Changelog, format: OutputFormat) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buffer = Buffer::default();
    match format {
        OutputFormat::Json => serde_json::to_writer_pretty(&mut buffer, changelog)?,
        OutputFormat::JsonCompact => serde_json::to_writer(&mut buffer, changelog)?,
        OutputFormat::Yaml => yaml_serde::to_writer(&mut buffer, changelog)?,
        OutputFormat::Toml => buffer.write_all(toml::to_string_pretty(changelog)?.as_bytes())?,
        OutputFormat::Jsonl => {
            for release in &changelog.releases {
                serde_json::to_writer(&mut buffer, release)?;
                buffer.write_all(b"\n")?;
            }
        }
        OutputFormat::Csv => table::render(changelog, &mut buffer)?,
        OutputFormat::Text => readable::text(changelog, &mut buffer)?,
        OutputFormat::Markdown => readable::markdown(changelog, &mut buffer)?,
    }
    if !buffer.bytes.is_empty() && !buffer.bytes.ends_with(b"\n") {
        buffer.write_all(b"\n")?;
    }
    Ok(buffer.bytes)
}

// Repeated CSV metadata and resolved text can amplify a small input. Bound the
// buffered export before opening the destination, just as we bound parser input.
const MAX_OUTPUT_BYTES: usize = 256 * 1024 * 1024;

struct Buffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            limit: MAX_OUTPUT_BYTES,
        }
    }
}

impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > self.limit)
        {
            return Err(io::Error::other(
                "rendered output exceeds the 256 MiB limit; select fewer releases or sections",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn release_label(release: &Release) -> &str {
    release.version.as_deref().unwrap_or_else(|| {
        if release
            .attributes
            .get("version-status")
            .is_some_and(|status| status == "unknown")
        {
            "Unversioned"
        } else if release
            .attributes
            .get("record-kind")
            .is_some_and(|kind| kind == "dated-change")
        {
            "Changes"
        } else {
            "Unreleased"
        }
    })
}

#[cfg(test)]
mod tests {
    use logloom::{Format, parse};

    use super::{Buffer, table};

    #[test]
    fn csv_metadata_amplification_is_bounded_before_external_output() {
        let input = format!(
            "# Changelog\n\n## [{}] - 2026-10-01\n\n### Added\n\n{}",
            "v".repeat(500),
            "- A change.\n".repeat(30)
        );
        let model = parse(&input, Format::KeepAChangelog).unwrap();
        let mut buffer = Buffer {
            limit: 4096,
            ..Buffer::default()
        };
        assert!(input.len() < buffer.limit);
        let error = table::render(&model, &mut buffer).unwrap_err();
        assert!(error.to_string().contains("rendered output exceeds"));
        assert!(buffer.bytes.len() <= buffer.limit);
    }
}
