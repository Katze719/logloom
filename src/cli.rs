use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use logloom::{
    Diagnostic, Format, MAX_INPUT_BYTES, Severity, parse, parse_named, validate_for_format,
};

use crate::diagnostics::{ColorMode, Reporter, SourceText};

mod output;
mod selection;

use selection::Selection;

#[derive(Debug, Parser)]
#[command(version, about = "Parse existing changelogs into structured JSON")]
struct Cli {
    /// Color diagnostics: auto detects stderr terminals and respects `NO_COLOR`.
    #[arg(long, global = true, value_enum, default_value = "auto")]
    color: ColorMode,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse a changelog file (or - for standard input).
    #[command(disable_version_flag = true)]
    Parse(ParseArgs),
}

#[derive(Debug, Args)]
struct ParseArgs {
    /// UTF-8 changelog file, or - to read stdin (maximum 16 MiB).
    file: PathBuf,
    /// Input changelog format.
    #[arg(long, value_enum, default_value = "keep-a-changelog")]
    input_format: InputFormat,
    /// Towncrier fragment filename for stdin, e.g. 123.bugfix.rst.
    #[arg(long)]
    fragment_name: Option<String>,
    /// Output format. JSON ends with a newline.
    #[arg(long, value_enum, default_value = "json")]
    output_format: OutputFormat,
    /// Write JSON to this file instead of stdout; replaces an existing file.
    #[arg(short = 'o', long, value_name = "PATH")]
    output_file: Option<PathBuf>,
    #[command(flatten)]
    selection: Selection,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InputFormat {
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

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Pretty-printed JSON.
    Json,
    /// Single-line JSON.
    JsonCompact,
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
    let Command::Parse(args) = cli.command;
    parse_file(&args, &Reporter::new(cli.color))
}

fn parse_file(args: &ParseArgs, reporter: &Reporter) -> Result<ExitCode, CliError> {
    let format = match args.input_format {
        InputFormat::KeepAChangelog => Format::KeepAChangelog,
        InputFormat::ConventionalChangelog => Format::ConventionalChangelog,
        InputFormat::GenericMarkdown => Format::GenericMarkdown,
        InputFormat::GitHubReleaseNotes => Format::GitHubReleaseNotes,
        InputFormat::Changesets => Format::Changesets,
        InputFormat::Towncrier => Format::Towncrier,
        InputFormat::DebianChangelog => Format::DebianChangelog,
        InputFormat::GnuNews => Format::GnuNews,
        InputFormat::GnuChangeLog => Format::GnuChangeLog,
    };
    if args.fragment_name.is_some() && format != Format::Towncrier {
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
    let source = SourceText::new(&args.file, &input, format);
    let name = args.fragment_name.as_deref().or_else(|| {
        if args.file == Path::new("-") {
            None
        } else {
            args.file.file_name().and_then(|name| name.to_str())
        }
    });
    let parsed = name.map_or_else(
        || parse(&input, format),
        |name| parse_named(&input, format, name),
    );
    let mut changelog =
        parsed.map_err(|error| CliError::new(1, reporter.parse_error(&error, &source)))?;
    let diagnostics = validate_for_format(&changelog, format);
    write_diagnostics(&diagnostics, &source, reporter)?;
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Ok(ExitCode::from(1));
    }
    args.selection.apply(&mut changelog);
    // Serialize before writing, so a serialization failure cannot produce partial JSON.
    let result = match args.output_format {
        OutputFormat::Json => serde_json::to_vec_pretty(&changelog),
        OutputFormat::JsonCompact => serde_json::to_vec(&changelog),
    };
    let mut json = result.map_err(|error| {
        CliError::new(
            3,
            reporter.error(
                "serialization",
                &format!("cannot serialize changelog: {error}"),
                "Check that the changelog can be represented as JSON.",
            ),
        )
    })?;
    json.push(b'\n');
    if let Some(path) = &args.output_file {
        output::write_file(path, &args.file, &json, reporter)?;
    } else {
        finish_write(io::stdout().lock().write_all(&json), reporter)?;
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
    } else {
        File::open(path).and_then(|file| file.take(limit).read_to_end(&mut bytes))
    };
    result.map_err(|error| {
        CliError::new(
            3,
            reporter.error(
                "read-input",
                &format!("cannot read {}: {error}", path.display()),
                "Check the file path and read permissions, or use '-' to read from stdin.",
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
    for diagnostic in diagnostics {
        writeln!(stderr, "{}", reporter.validation(diagnostic, source)).map_err(|error| {
            CliError::new(
                3,
                reporter.error(
                    "diagnostic-io",
                    &format!("cannot write diagnostics: {error}"),
                    "Check that stderr is writable.",
                ),
            )
        })?;
    }
    Ok(())
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
