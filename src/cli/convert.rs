use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{Args, ValueEnum};
use logloom::{Changelog, Format, ParseOptions, Severity, validate};

use super::selection::Selection;
use super::{CliError, diagnostic_error, finish_write, input_name, output, read_input, render};
use crate::diagnostics::{Reporter, SourceText};

mod import;
mod keep_a_changelog;
mod native;
#[cfg(feature = "csv")]
mod table;
mod text;

type Failure = Box<dyn Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ConversionFormat {
    Auto,
    KeepAChangelog,
    ConventionalChangelog,
    GenericMarkdown,
    #[value(name = "github-release-notes")]
    GitHubReleaseNotes,
    Changesets,
    Towncrier,
    DebianChangelog,
    GnuNews,
    #[value(name = "gnu-changelog")]
    GnuChangeLog,
    Json,
    JsonCompact,
    #[cfg(feature = "yaml")]
    Yaml,
    #[cfg(feature = "toml")]
    Toml,
    Jsonl,
    #[cfg(feature = "csv")]
    Csv,
    Text,
    Markdown,
}

impl ConversionFormat {
    const fn native(self) -> Option<Format> {
        match self {
            Self::KeepAChangelog => Some(Format::KeepAChangelog),
            Self::ConventionalChangelog => Some(Format::ConventionalChangelog),
            Self::GenericMarkdown | Self::Markdown => Some(Format::GenericMarkdown),
            Self::GitHubReleaseNotes => Some(Format::GitHubReleaseNotes),
            Self::Changesets => Some(Format::Changesets),
            Self::Towncrier => Some(Format::Towncrier),
            Self::DebianChangelog => Some(Format::DebianChangelog),
            Self::GnuNews => Some(Format::GnuNews),
            Self::GnuChangeLog => Some(Format::GnuChangeLog),
            _ => None,
        }
    }

    const fn export(self) -> Option<render::OutputFormat> {
        Some(match self {
            Self::Json => render::OutputFormat::Json,
            Self::JsonCompact => render::OutputFormat::JsonCompact,
            #[cfg(feature = "yaml")]
            Self::Yaml => render::OutputFormat::Yaml,
            #[cfg(feature = "toml")]
            Self::Toml => render::OutputFormat::Toml,
            Self::Jsonl => render::OutputFormat::Jsonl,
            #[cfg(feature = "csv")]
            Self::Csv => render::OutputFormat::Csv,
            Self::Text => render::OutputFormat::Text,
            Self::Markdown => render::OutputFormat::Markdown,
            _ => return None,
        })
    }
}

#[derive(Debug, Args)]
#[command(after_help = help())]
pub struct ConvertArgs {
    /// UTF-8 changelog or export file, or - for stdin (16 MiB by default).
    #[arg(default_value = "CHANGELOG.md", help_heading = "Input")]
    file: PathBuf,
    /// Source format; auto requires an unambiguous signature.
    #[arg(
        long,
        value_enum,
        default_value = "auto",
        hide_possible_values = true,
        help_heading = "Input"
    )]
    input_format: ConversionFormat,
    /// Towncrier source fragment filename hint, e.g. 123.bugfix.rst.
    #[arg(long, value_name = "NAME", help_heading = "Input")]
    fragment_name: Option<String>,
    /// Required destination format; filenames do not select it.
    #[arg(long, value_parser = target_formats(), value_name = "FORMAT", help_heading = "Output")]
    output_format: ConversionFormat,
    /// Write to a file (replace existing); '-' uses stdout.
    #[arg(short = 'o', long, value_name = "PATH", help_heading = "Output")]
    output_file: Option<PathBuf>,
    /// Fail on validation or conversion warnings; the experimental note is exempt.
    #[arg(long, help_heading = "Validation")]
    strict: bool,
    #[command(flatten)]
    selection: Selection,
}

fn target_formats() -> impl TypedValueParser<Value = ConversionFormat> {
    PossibleValuesParser::new(
        ConversionFormat::value_variants()
            .iter()
            .filter(|format| **format != ConversionFormat::Auto)
            .filter_map(ValueEnum::to_possible_value),
    )
    .try_map(|name| ConversionFormat::from_str(&name, false))
}

fn help() -> String {
    let formats = ConversionFormat::value_variants()
        .iter()
        .filter_map(ValueEnum::to_possible_value)
        .filter(|value| value.get_name() != "auto")
        .map(|value| value.get_name().to_owned())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Experimental — best effort. Structure, metadata, and formatting may be lost.\nMissing required values use documented placeholders. Review the result.\n\nSource and destination formats (optional formats depend on build features):\n  {formats}\n\nInput also accepts auto. Targets are canonical histories, not fragments.\n\nExamples:\n  logloom convert CHANGELOG.md --output-format debian-changelog\n  logloom convert changelog.json --input-format json --output-format keep-a-changelog\n  logloom convert exported.json --input-format json --output-format towncrier -o converted.md"
    )
}

#[derive(Default)]
struct Warnings {
    messages: Vec<String>,
    omitted: usize,
}

impl Warnings {
    fn add(&mut self, message: impl Into<String>) {
        let message: String = message.into().chars().take(4096).collect();
        if self.messages.contains(&message) {
            return;
        }
        // Bound diagnostics independently of the input-size override.
        if self.messages.len() < 1024 {
            self.messages.push(message);
        } else {
            self.omitted = self.omitted.saturating_add(1);
        }
    }

    fn release(&mut self, index: usize, message: impl std::fmt::Display) {
        self.add(format!("release {}: {message}", index.saturating_add(1)));
    }

    const fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

pub fn run(
    args: &ConvertArgs,
    options: ParseOptions,
    reporter: &Reporter,
) -> Result<ExitCode, CliError> {
    writeln!(io::stderr().lock(), "{}", reporter.conversion_note())
        .map_err(|error| diagnostic_error(&error, reporter))?;
    let input = read_input(&args.file, options, reporter)?;
    let mut warnings = Warnings::default();
    let (format, mut changelog) = load_document(args, &input, options, &mut warnings, reporter)?;
    let source = SourceText::new(&args.file, &input, changelog.format);
    if !check(
        &changelog,
        &source,
        format.native().is_some(),
        args.strict,
        reporter,
    )? {
        return Ok(ExitCode::from(1));
    }
    args.selection.apply(&mut changelog).map_err(|error| {
        CliError::new(
            1,
            reporter.selection_error(
                &error.message,
                error.help,
                if format.native().is_some() {
                    error.line
                } else {
                    None
                },
                &source,
            ),
        )
    })?;
    if args.selection.is_active() && changelog.releases.is_empty() {
        writeln!(io::stderr().lock(), "{}", reporter.no_matches())
            .map_err(|error| diagnostic_error(&error, reporter))?;
    }
    let bytes = export(&changelog, args.output_format, &mut warnings).map_err(|error| {
        CliError::new(
            3,
            reporter.error(
                "serialization",
                &format!("cannot convert changelog: {error}"),
                "Choose another --output-format or select fewer releases.",
            ),
        )
    })?;
    for message in &warnings.messages {
        writeln!(
            io::stderr().lock(),
            "{}",
            reporter.conversion_warning(message)
        )
        .map_err(|error| diagnostic_error(&error, reporter))?;
    }
    if warnings.omitted != 0 {
        writeln!(
            io::stderr().lock(),
            "{}",
            reporter.conversion_warning(&format!(
                "{} further conversion warnings omitted",
                warnings.omitted
            ))
        )
        .map_err(|error| diagnostic_error(&error, reporter))?;
    }
    if args.strict && !warnings.is_empty() {
        return Ok(ExitCode::from(1));
    }
    write_result(args, &bytes, reporter)
}

fn write_result(
    args: &ConvertArgs,
    bytes: &[u8],
    reporter: &Reporter,
) -> Result<ExitCode, CliError> {
    if let Some(path) = &args.output_file
        && path != Path::new("-")
    {
        output::write_file(path, &args.file, bytes, reporter)?;
    } else {
        finish_write(io::stdout().lock().write_all(bytes), reporter)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn load_document(
    args: &ConvertArgs,
    input: &str,
    options: ParseOptions,
    warnings: &mut Warnings,
    reporter: &Reporter,
) -> Result<(ConversionFormat, Changelog), CliError> {
    let name = input_name(&args.file, args.fragment_name.as_deref());
    let format = if args.input_format == ConversionFormat::Auto {
        import::detect(input, name, options)
            .map_err(|error| input_error(args, input, args.input_format, &error, reporter))?
    } else {
        args.input_format
    };
    if args.fragment_name.is_some() && format != ConversionFormat::Towncrier {
        return Err(CliError::new(
            2,
            reporter.error(
                "fragment-name",
                "--fragment-name applies only to Towncrier",
                "Select --input-format towncrier, or omit --fragment-name.",
            ),
        ));
    }
    let changelog = import::read(input, format, name, options, warnings).map_err(|error| {
        if let Some(native) = format.native()
            && let Some(error) = error.downcast_ref::<logloom::ParseError>()
        {
            return CliError::new(
                1,
                reporter.parse_error(error, &SourceText::new(&args.file, input, native)),
            );
        }
        input_error(args, input, format, &error, reporter)
    })?;
    Ok((format, changelog))
}

fn input_error(
    args: &ConvertArgs,
    input: &str,
    format: ConversionFormat,
    error: &Failure,
    reporter: &Reporter,
) -> CliError {
    let name = format
        .to_possible_value()
        .map(|value| value.get_name().to_owned())
        .unwrap_or_default();
    CliError::new(
        1,
        reporter.conversion_input(&args.file, input, &name, error.as_ref()),
    )
}

fn export(
    document: &Changelog,
    format: ConversionFormat,
    warnings: &mut Warnings,
) -> Result<Vec<u8>, Failure> {
    if let Some(export) = format.export() {
        import::export_warnings(document, format, warnings);
        return render::render(document, export);
    }
    let native = format
        .native()
        .ok_or("choose an explicit destination format")?;
    native::render(document, native, warnings)
}

fn check(
    document: &Changelog,
    source: &SourceText<'_>,
    original_spans: bool,
    strict: bool,
    reporter: &Reporter,
) -> Result<bool, CliError> {
    let mut diagnostics = validate(document);
    if !original_spans {
        for diagnostic in &mut diagnostics {
            diagnostic.line = None;
        }
    }
    super::write_diagnostics(&diagnostics, source, reporter)?;
    Ok(!diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Error
            || (strict && diagnostic.severity == Severity::Warning)
    }))
}
