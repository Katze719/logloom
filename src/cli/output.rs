use std::fs;
use std::io::{self, Write};
use std::path::Path;

use tempfile::NamedTempFile;

use super::CliError;
use crate::diagnostics::Reporter;

pub fn write_file(
    path: &Path,
    input: &Path,
    bytes: &[u8],
    reporter: &Reporter,
) -> Result<(), CliError> {
    if input != Path::new("-")
        && same_path(input, path).map_err(|error| output_error(path, &error, reporter))?
    {
        return Err(CliError::new(
            2,
            reporter.error(
                "output-path",
                "input and output refer to the same file",
                "Choose a different --output-file path to keep your changelog intact.",
            ),
        ));
    }
    persist(path, bytes).map_err(|error| output_error(path, &error, reporter))
}

fn same_path(input: &Path, output: &Path) -> io::Result<bool> {
    let output = match fs::canonicalize(output) {
        Ok(path) => path,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    Ok(fs::canonicalize(input)? == output)
}

fn persist(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.flush()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn output_error(path: &Path, error: &io::Error, reporter: &Reporter) -> CliError {
    CliError::new(
        3,
        reporter.error(
            "output-io",
            &format!("cannot write {}: {error}", path.display()),
            "Check that the parent directory exists and is writable, and that disk space is available.",
        ),
    )
}
