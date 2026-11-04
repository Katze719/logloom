use std::io::{self, IsTerminal};
use std::path::Path;

use annotate_snippets::{AnnotationKind, Group, Level, Origin, Renderer, Snippet};
use clap::ValueEnum;
use logloom::{Diagnostic, ErrorKind, Format, ParseError, Severity};

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

    pub fn parse_error(&self, error: &ParseError, source: &SourceText<'_>) -> String {
        let (id, help) = match error.kind {
            ErrorKind::ResourceLimit => (
                "resource-limit",
                "Keep input and expanded repeated metadata at or below 16 MiB, and Markdown nesting at or below 128 containers.",
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
            Severity::Warning => Level::WARNING,
        };
        self.render(
            level,
            id,
            &diagnostic.message,
            help,
            Some((source, diagnostic.line)),
        )
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
            let line = line.unwrap_or(1);
            if let Some(text) = line
                .checked_sub(1)
                .and_then(|index| source.lines.get(index))
                .filter(|text| !text.is_empty())
            {
                group = group.element(
                    Snippet::source(*text)
                        .path(source.path.as_str())
                        .line_start(line)
                        .annotation(AnnotationKind::Primary.span(0..text.len()).label(message)),
                );
            } else {
                group = group.element(Origin::path(source.path.as_str()));
            }
        }
        group = group.element(Level::HELP.message(help));
        self.renderer.render(&[group])
    }
}
