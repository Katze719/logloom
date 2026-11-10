use std::error::Error;
use std::path::Path;

use annotate_snippets::Level;
use logloom::{ErrorKind, Format, ParseError};

use super::{Reporter, SourceText};

impl Reporter {
    pub fn conversion_input(
        &self,
        path: &Path,
        input: &str,
        format: &str,
        error: &(dyn Error + 'static),
    ) -> String {
        let resource = error
            .downcast_ref::<ParseError>()
            .is_some_and(|error| error.kind == ErrorKind::ResourceLimit);
        let source = SourceText::new(path, input, Format::GenericMarkdown);
        let message = format!("cannot import {}: {error}", path.display());
        let help = if resource {
            "Imported Markdown still has a 128-container nesting limit; simplify deeply nested content."
        } else {
            hint(format)
        };
        self.render(
            Level::ERROR,
            if resource {
                "resource-limit"
            } else {
                "conversion-input"
            },
            &message,
            help,
            Some((&source, error_line(error, input))),
        )
    }
}

fn error_line(error: &(dyn Error + 'static), input: &str) -> Option<usize> {
    if let Some(error) = error.downcast_ref::<serde_json::Error>() {
        return Some(error.line());
    }
    #[cfg(feature = "yaml")]
    if let Some(error) = error.downcast_ref::<yaml_serde::Error>() {
        return error.location().map(|location| location.line());
    }
    #[cfg(feature = "toml")]
    if let Some(error) = error.downcast_ref::<toml::de::Error>() {
        return error
            .span()
            .and_then(|range| input.get(..range.start))
            .map(|prefix| {
                prefix
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    .saturating_add(1)
            });
    }
    #[cfg(feature = "csv")]
    if let Some(error) = error.downcast_ref::<csv::Error>() {
        return error
            .position()
            .and_then(|position| usize::try_from(position.line()).ok());
    }
    let _ = input;
    None
}

fn hint(format: &str) -> &'static str {
    match format {
        "json" | "json-compact" => {
            "Supply a logloom document object with metadata, releases, and links. For GitHub API JSON use --input-format github-release-notes."
        }
        "yaml" => {
            "Supply logloom's YAML document schema. Expand anchors and aliases before importing; keep metadata, releases, and links."
        }
        "toml" => {
            "Supply logloom's TOML document schema, including metadata, releases, and links; missing optional values are omitted keys."
        }
        "jsonl" => {
            "Use one complete logloom Release object per nonempty line, without an outer array. Document metadata and shared links are unavailable."
        }
        "csv" => {
            "Use logloom's 14-column CSV header and entry/section/release row types. Keep metadata consistent within each release_index; attributes must contain a JSON object."
        }
        "text" => {
            "Use a logloom text export, or select a native input format if the document has structured changelog syntax."
        }
        _ => {
            "Select an unambiguous --input-format; 'logloom convert --help' lists the choices. Structured inputs must match logloom's export schema."
        }
    }
}
