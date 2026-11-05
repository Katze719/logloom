use std::io::{self, IsTerminal};
use std::path::Path;

use annotate_snippets::{AnnotationKind, Group, Level, Origin, Renderer, Snippet};
use clap::ValueEnum;
use logloom::{DetectionError, Diagnostic, ErrorKind, Format, ParseError, Severity};

mod formats;
mod keep_a_changelog;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl ColorMode {
    // Clap errors occur before Cli exists. Recover only this presentation option,
    // leaving validation of the command line entirely to clap.
    pub fn argument_override() -> Self {
        let mut arguments = std::env::args_os().skip(1);
        let mut color = Self::Auto;
        while let Some(argument) = arguments.next() {
            if argument == "--" {
                break;
            }
            let value = if argument == "--color" {
                arguments.next().and_then(|value| value.into_string().ok())
            } else {
                argument
                    .to_str()
                    .and_then(|value| value.strip_prefix("--color="))
                    .map(str::to_owned)
            };
            if let Some(value) = value.and_then(|value| Self::from_str(&value, false).ok()) {
                color = value;
            }
        }
        color
    }

    pub const fn clap_choice(self) -> clap::ColorChoice {
        match self {
            Self::Auto => clap::ColorChoice::Auto,
            Self::Always => clap::ColorChoice::Always,
            Self::Never => clap::ColorChoice::Never,
        }
    }
}

pub struct SourceText<'a> {
    path: String,
    lines: Vec<&'a str>,
    format: Format,
}

impl<'a> SourceText<'a> {
    pub fn new(path: &Path, input: &'a str, format: Format) -> Self {
        Self {
            path: if path == Path::new("-") {
                "<stdin>".to_owned()
            } else {
                path.display().to_string()
            },
            lines: input.lines().collect(),
            format,
        }
    }
}

pub struct Reporter {
    renderer: Renderer,
}

impl Reporter {
    pub fn new(color: ColorMode) -> Self {
        let colored = match color {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                io::stderr().is_terminal()
                    && std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
                    && std::env::var_os("TERM").is_none_or(|value| value != "dumb")
            }
        };
        Self {
            renderer: if colored {
                Renderer::styled()
            } else {
                Renderer::plain()
            },
        }
    }

    pub fn error(&self, id: &str, message: &str, help: &str) -> String {
        self.render(Level::ERROR, id, message, help, None)
    }

    pub fn detection_error(&self, error: &DetectionError, path: &Path, input: &str) -> String {
        let (id, line, help) = match error {
            DetectionError::Ambiguous { .. } => (
                "ambiguous-format",
                None,
                "Select --input-format FORMAT with 'logloom parse' or 'logloom validate' to choose one of these formats.",
            ),
            DetectionError::InvalidInput(error) if error.kind == ErrorKind::ResourceLimit => (
                "resource-limit",
                error.line,
                "Use --yes-i-know-i-am-stupid for larger input. Markdown nesting still has a limit of 128 containers.",
            ),
            DetectionError::InvalidInput(error) if error.kind == ErrorKind::EmptyInput => (
                "empty-input",
                error.line,
                "Provide a changelog file, or pipe changelog text into 'logloom parse -'.",
            ),
            DetectionError::InvalidInput(error) => (
                "detect-input",
                error.line,
                "Fix the input syntax at the marked location. If this isn't JSON, select --input-format explicitly with parse or validate.",
            ),
            _ => (
                "unknown-format",
                None,
                "Select --input-format with parse or validate if you know the format; 'logloom parse --help' lists the choices.",
            ),
        };
        // Only the path and lines are used here; detection has no selected format.
        let source = SourceText::new(path, input, Format::GenericMarkdown);
        let message = match error {
            DetectionError::InvalidInput(error) => error.message.clone(),
            _ => error.to_string(),
        };
        self.render(Level::ERROR, id, &message, help, Some((&source, line)))
    }

    pub fn selection_error(
        &self,
        message: &str,
        help: &str,
        line: Option<usize>,
        source: &SourceText<'_>,
    ) -> String {
        self.render(
            Level::ERROR,
            "latest-release",
            message,
            help,
            Some((source, line)),
        )
    }

    pub fn no_matches(&self) -> String {
        self.render(
            Level::NOTE,
            "no-matches",
            "no releases match the selected filters",
            "Check the filter values; run without filters to see the available changes.",
            None,
        )
    }

    pub fn strict_warnings(&self, count: usize) -> String {
        let message = if count == 1 {
            "1 validation warning is not allowed with --strict".to_owned()
        } else {
            format!("{count} validation warnings are not allowed with --strict")
        };
        self.error(
            "strict-validation",
            &message,
            "Fix the validation warnings, or omit --strict to allow warnings.",
        )
    }

    pub fn parse_error(&self, error: &ParseError, source: &SourceText<'_>) -> String {
        let (id, help) = match error.kind {
            ErrorKind::ResourceLimit => (
                "resource-limit",
                "Use --yes-i-know-i-am-stupid for input size. Expanded repeated metadata stays capped at 16 MiB; Markdown nesting stays capped at 128 containers.",
            ),
            ErrorKind::EmptyInput if source.format == Format::Towncrier => (
                "empty-input",
                "Provide changelog or fragment text; only named misc fragments may be empty.",
            ),
            ErrorKind::EmptyInput if source.format != Format::KeepAChangelog => (
                "empty-input",
                "Provide a nonempty changelog file, or pass '-' to read changelog text from stdin.",
            ),
            _ => match source.format {
                Format::KeepAChangelog => keep_a_changelog::parse_details(error.kind),
                format => formats::parse_details(format, error.kind),
            },
        };
        self.render(
            Level::ERROR,
            id,
            &error.message,
            help,
            Some((source, error.line)),
        )
    }

    pub fn validation(&self, diagnostic: &Diagnostic, source: &SourceText<'_>) -> String {
        let (id, help) = match source.format {
            Format::KeepAChangelog => keep_a_changelog::validation_details(diagnostic.code),
            format => formats::validation_details(format, diagnostic.code),
        };
        let level = match diagnostic.severity {
            Severity::Error => Level::ERROR,
            _ => Level::WARNING,
        };
        self.render(
            level,
            id,
            &diagnostic.message,
            help,
            Some((source, diagnostic.line)),
        )
    }

    pub fn warning_group(&self, diagnostics: &[&Diagnostic], source: &SourceText<'_>) -> String {
        let Some((first, rest)) = diagnostics.split_first() else {
            return String::new();
        };
        let mut output = self.validation(first, source);
        if rest.is_empty() {
            return output;
        }
        let lines: Vec<_> = rest
            .iter()
            .filter_map(|diagnostic| diagnostic.line)
            .map(|line| line.to_string())
            .collect();
        let summary = format!("{} more occurrences in the same file", rest.len());
        let mut group = Group::with_title(Level::NOTE.primary_title(&summary));
        let locations: Vec<_> = lines
            .chunks(12)
            .map(|chunk| format!("also at lines {}", chunk.join(", ")))
            .collect();
        for location in &locations {
            group = group.element(Level::NOTE.message(location));
        }
        output.push('\n');
        output.push_str(&self.renderer.render(&[group]));
        output
    }

    fn render(
        &self,
        level: Level<'_>,
        id: &str,
        message: &str,
        help: &str,
        location: Option<(&SourceText<'_>, Option<usize>)>,
    ) -> String {
        let mut group = Group::with_title(level.primary_title(message).id(id));
        if let Some((source, line)) = location {
            if let Some(line) = line
                && let Some(text) = line
                    .checked_sub(1)
                    .and_then(|index| source.lines.get(index))
                    .filter(|text| !text.is_empty())
            {
                group = group.element(
                    Snippet::source(*text)
                        .path(source.path.as_str())
                        .line_start(line)
                        .annotation(AnnotationKind::Primary.span(0..text.len())),
                );
            } else {
                group = group.element(Origin::path(source.path.as_str()));
            }
        }
        group = group.element(Level::HELP.message(help));
        self.renderer.render(&[group])
    }
}
