use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use logloom::{
    Changelog, Diagnostic, Format, MAX_INPUT_BYTES, Severity, detect, detect_named, parse,
    parse_named, validate,
};

use crate::diagnostics::{ColorMode, Reporter, SourceText};

mod help;
mod output;
mod render;
mod selection;

use render::OutputFormat;
use selection::Selection;

#[derive(Debug, Parser)]
#[command(
    version,
    propagate_version = true,
    about = "Parse and export existing changelogs",
    after_help = "Examples:\n  logloom parse --latest --output-format text\n  logloom detect\n  logloom validate"
)]
struct Cli {
    /// Diagnostic colors: auto, always, never.
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "auto",
        hide_possible_values = true
    )]
    color: ColorMode,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse a changelog file (or - for standard input).
    Parse(ParseArgs),
    /// Check a changelog without exporting it; diagnostics go to stderr.
    #[command(after_help = help::validate())]
    Validate(ValidateArgs),
    /// Print the detected input-format name without validating or exporting.
    Detect(DetectArgs),
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Input")]
struct InputArgs {
    /// UTF-8 changelog file, or - to read stdin (maximum 16 MiB).
    #[arg(default_value = "CHANGELOG.md")]
    file: PathBuf,
    /// Input format; auto requires a clear match.
    #[arg(
        long,
        value_enum,
        default_value = "auto",
        value_name = "FORMAT",
        hide_possible_values = true
    )]
    input_format: InputFormat,
    /// Towncrier filename hint, e.g. 123.bugfix.rst.
    #[arg(long, value_name = "NAME")]
    fragment_name: Option<String>,
    /// Fail on validation warnings as well as errors (useful for CI).
    #[arg(long, help_heading = "Validation")]
    strict: bool,
}

#[derive(Debug, Args)]
struct DetectArgs {
    /// UTF-8 changelog file, or - to read stdin (maximum 16 MiB).
    #[arg(default_value = "CHANGELOG.md")]
    file: PathBuf,
    /// Towncrier filename hint, e.g. 123.bugfix.rst.
    #[arg(long, value_name = "NAME")]
    fragment_name: Option<String>,
}

#[derive(Debug, Args)]
struct ValidateArgs {
    #[command(flatten)]
    input: InputArgs,
    /// Also require this exact release identifier to exist.
    #[arg(long, value_name = "VERSION", help_heading = "Validation", value_parser = clap::builder::NonEmptyStringValueParser::new())]
    release: Option<String>,
    /// Require the release to belong to this exact package.
    #[arg(long, value_name = "NAME", help_heading = "Validation", requires = "release", value_parser = clap::builder::NonEmptyStringValueParser::new())]
    package: Option<String>,
}

#[derive(Debug, Args)]
#[command(after_help = help::parse())]
struct ParseArgs {
    #[command(flatten)]
    input: InputArgs,
    /// Output format (pretty-printed JSON by default).
    #[arg(
        long,
        value_enum,
        default_value = "json",
        value_name = "FORMAT",
        help_heading = "Output",
        hide_possible_values = true
    )]
    output_format: OutputFormat,
    /// Write to a file (replace existing); '-' uses stdout.
    #[arg(short = 'o', long, value_name = "PATH", help_heading = "Output")]
    output_file: Option<PathBuf>,
    #[command(flatten)]
    selection: Selection,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InputFormat {
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
}

#[derive(Debug)]
pub struct CliError {
    pub code: ExitCode,
    message: String,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl CliError {
    fn new(code: u8, message: String) -> Self {
        Self {
            code: ExitCode::from(code),
            message,
        }
    }
}

pub fn run() -> Result<ExitCode, CliError> {
    // try_parse avoids clap's process::exit path. Help and version also use fallible I/O.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let color = ColorMode::argument_override();
            let error = error.with_cmd(&Cli::command().color(color.clap_choice()));
            let is_error = error.use_stderr();
            finish_write(error.print(), &Reporter::new(color))?;
            return Ok(if is_error {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            });
        }
    };
    let reporter = Reporter::new(cli.color);
    match cli.command {
        Command::Parse(args) => parse_file(&args, &reporter),
        Command::Validate(args) => validate_file(&args, &reporter),
        Command::Detect(args) => {
            let input = read_input(&args.file, &reporter)?;
            let name = input_name(&args.file, args.fragment_name.as_deref());
            let format = detect_input(&input, name, &args.file, &reporter)?;
            finish_write(
                writeln!(io::stdout().lock(), "{}", format.as_str()),
                &reporter,
            )?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

impl InputFormat {
    const fn explicit(self) -> Option<Format> {
        Some(match self {
            Self::Auto => return None,
            Self::KeepAChangelog => Format::KeepAChangelog,
            Self::ConventionalChangelog => Format::ConventionalChangelog,
            Self::GenericMarkdown => Format::GenericMarkdown,
            Self::GitHubReleaseNotes => Format::GitHubReleaseNotes,
            Self::Changesets => Format::Changesets,
            Self::Towncrier => Format::Towncrier,
            Self::DebianChangelog => Format::DebianChangelog,
            Self::GnuNews => Format::GnuNews,
            Self::GnuChangeLog => Format::GnuChangeLog,
        })
    }
}

fn input_name<'a>(file: &'a Path, fragment_name: Option<&'a str>) -> Option<&'a str> {
    fragment_name.or_else(|| {
        if file == Path::new("-") {
            None
        } else {
            file.file_name().and_then(|name| name.to_str())
        }
    })
}

fn detect_input(
    input: &str,
    name: Option<&str>,
    path: &Path,
    reporter: &Reporter,
) -> Result<Format, CliError> {
    name.map_or_else(|| detect(input), |name| detect_named(input, name))
        .map_err(|error| CliError::new(1, reporter.detection_error(&error, path, input)))
}

fn read_document(args: &InputArgs, reporter: &Reporter) -> Result<Changelog, CliError> {
    let explicit = args.input_format.explicit();
    if args.fragment_name.is_some() && explicit.is_some_and(|format| format != Format::Towncrier) {
        return Err(CliError::new(
            2,
            reporter.error(
                "fragment-name",
                "--fragment-name applies only to Towncrier",
                "Use --input-format towncrier when providing a news-fragment filename.",
            ),
        ));
    }
    let input = read_input(&args.file, reporter)?;
    let name = input_name(&args.file, args.fragment_name.as_deref());
    let format = explicit.map_or_else(|| detect_input(&input, name, &args.file, reporter), Ok)?;
    if args.fragment_name.is_some() && format != Format::Towncrier {
        return Err(CliError::new(
            2,
            reporter.error(
                "fragment-name",
                "--fragment-name applies only to Towncrier",
                "Select --input-format towncrier, or omit --fragment-name for a history file.",
            ),
        ));
    }
    let source = SourceText::new(&args.file, &input, format);
    let parsed = name.map_or_else(
        || parse(&input, format),
        |name| parse_named(&input, format, name),
    );
    parsed.map_err(|error| CliError::new(1, reporter.parse_error(&error, &source)))
}

fn check_document(
    path: &Path,
    changelog: &Changelog,
    strict: bool,
    reporter: &Reporter,
) -> Result<ExitCode, CliError> {
    let source = SourceText::new(path, &changelog.source, changelog.format);
    let diagnostics = validate(changelog);
    write_diagnostics(&diagnostics, &source, reporter)?;
    let has_errors = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count();
    let rejected_warnings = strict && warnings != 0;
    if rejected_warnings && !has_errors {
        writeln!(
            io::stderr().lock(),
            "{}",
            reporter.strict_warnings(warnings)
        )
        .map_err(|error| diagnostic_error(&error, reporter))?;
    }
    if has_errors || rejected_warnings {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn validate_file(args: &ValidateArgs, reporter: &Reporter) -> Result<ExitCode, CliError> {
    let changelog = read_document(&args.input, reporter)?;
    let status = check_document(&args.input.file, &changelog, args.input.strict, reporter)?;
    if status != ExitCode::SUCCESS {
        return Ok(status);
    }
    if let Some(version) = &args.release
        && !changelog.releases.iter().any(|release| {
            release.version.as_ref() == Some(version)
                && args
                    .package
                    .as_ref()
                    .is_none_or(|package| release.attributes.get("package") == Some(package))
        })
    {
        let scope = args
            .package
            .as_ref()
            .map_or_else(String::new, |package| format!(" for package {package:?}"));
        return Err(CliError::new(
            1,
            reporter.error(
                "release-not-found",
                &format!(
                    "release {version:?}{scope} was not found in {}",
                    args.input.file.display()
                ),
                if args.package.is_some() {
                    "Check the exact release identifier and package name, including case and any 'v' prefix."
                } else {
                    "Use the exact release identifier from the changelog, including case and any 'v' prefix."
                },
            ),
        ));
    }
    Ok(ExitCode::SUCCESS)
}

fn parse_file(args: &ParseArgs, reporter: &Reporter) -> Result<ExitCode, CliError> {
    let mut changelog = read_document(&args.input, reporter)?;
    let status = check_document(&args.input.file, &changelog, args.input.strict, reporter)?;
    if status != ExitCode::SUCCESS {
        return Ok(status);
    }
    args.selection.apply(&mut changelog).map_err(|error| {
        let source = SourceText::new(&args.input.file, &changelog.source, changelog.format);
        CliError::new(
            1,
            reporter.selection_error(&error.message, error.help, error.line, &source),
        )
    })?;
    if args.selection.is_active() && changelog.releases.is_empty() {
        writeln!(io::stderr().lock(), "{}", reporter.no_matches())
            .map_err(|error| diagnostic_error(&error, reporter))?;
    }
    // Render the whole result before touching stdout or the destination file.
    let bytes = render::render(&changelog, args.output_format).map_err(|error| {
        CliError::new(
            3,
            reporter.error(
                "serialization",
                &format!(
                    "cannot render changelog as {}: {error}",
                    args.output_format.name()
                ),
                "Try fewer releases or sections, or choose another --output-format.",
            ),
        )
    })?;
    if let Some(path) = &args.output_file
        && path != Path::new("-")
    {
        output::write_file(path, &args.input.file, &bytes, reporter)?;
    } else {
        finish_write(io::stdout().lock().write_all(&bytes), reporter)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn read_input(path: &Path, reporter: &Reporter) -> Result<String, CliError> {
    let limit = u64::try_from(MAX_INPUT_BYTES)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| {
            CliError::new(
                3,
                reporter.error(
                    "input-limit",
                    "input limit is not representable",
                    "Use a platform supporting the documented 16 MiB input limit.",
                ),
            )
        })?;
    let mut bytes = Vec::new();
    let result = if path == Path::new("-") {
        io::stdin().lock().take(limit).read_to_end(&mut bytes)
    } else if path.is_dir() {
        // Windows refuses to open directories with PermissionDenied, while
        // Unix generally reports IsADirectory when reading. Keep the hint uniform.
        Err(io::Error::new(
            io::ErrorKind::IsADirectory,
            "input path is a directory",
        ))
    } else {
        File::open(path).and_then(|file| file.take(limit).read_to_end(&mut bytes))
    };
    result.map_err(|error| {
        CliError::new(
            3,
            reporter.error(
                "read-input",
                &format!("cannot read {}: {error}", path.display()),
                match error.kind() {
                    io::ErrorKind::NotFound => {
                        "Check the file path, or use '-' to read changelog text from stdin."
                    }
                    io::ErrorKind::IsADirectory => "Pass a changelog file instead of a directory.",
                    _ => "Check the file path and read permissions, or use '-' to read from stdin.",
                },
            ),
        )
    })?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(CliError::new(
            1,
            reporter.error(
                "resource-limit",
                &format!("{} exceeds the 16 MiB input limit", path.display()),
                "Split the changelog into documents of at most 16 MiB each.",
            ),
        ));
    }
    String::from_utf8(bytes).map_err(|error| {
        CliError::new(
            3,
            reporter.error(
                "invalid-utf8",
                &format!("{} is not UTF-8: {error}", path.display()),
                "Save or convert the changelog to UTF-8 before parsing.",
            ),
        )
    })
}

fn write_diagnostics(
    diagnostics: &[Diagnostic],
    source: &SourceText<'_>,
    reporter: &Reporter,
) -> Result<(), CliError> {
    let mut stderr = io::stderr().lock();
    let mut seen_warnings: Vec<&Diagnostic> = Vec::new();
    for diagnostic in diagnostics {
        let rendered = if diagnostic.severity == Severity::Warning {
            if seen_warnings.iter().any(|previous| {
                previous.code == diagnostic.code && previous.message == diagnostic.message
            }) {
                continue;
            }
            seen_warnings.push(diagnostic);
            let group: Vec<_> = diagnostics
                .iter()
                .filter(|other| {
                    other.severity == Severity::Warning
                        && other.code == diagnostic.code
                        && other.message == diagnostic.message
                })
                .collect();
            reporter.warning_group(&group, source)
        } else {
            reporter.validation(diagnostic, source)
        };
        writeln!(stderr, "{rendered}").map_err(|error| diagnostic_error(&error, reporter))?;
    }
    Ok(())
}

fn diagnostic_error(error: &io::Error, reporter: &Reporter) -> CliError {
    CliError::new(
        3,
        reporter.error(
            "diagnostic-io",
            &format!("cannot write diagnostics: {error}"),
            "Check that stderr is writable.",
        ),
    )
}

fn finish_write(result: io::Result<()>, reporter: &Reporter) -> Result<(), CliError> {
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(CliError::new(
            3,
            reporter.error(
                "output-io",
                &format!("cannot write output: {error}"),
                "Check the output destination's permissions and available disk space.",
            ),
        )),
    }
}
