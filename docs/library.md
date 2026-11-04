# Library usage

## Add a local dependency

The package is not published to crates.io. Adjust the path for your application:

```toml
[dependencies]
logloom = { path = "../logloom" }
serde_json = "1.0.154"
```

## Parse, validate, serialize

```rust
use std::io::{self, Write};
use logloom::{parse, validate, Format, Severity};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = "# Changelog\n\n## [Unreleased]\n\n### Added\n- Export API.\n";
    let changelog = parse(input, Format::KeepAChangelog)?;

    let diagnostics = validate(&changelog);
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err("changelog contains validation errors".into());
    }

    let json = serde_json::to_string_pretty(&changelog)?;
    writeln!(io::stdout().lock(), "{json}")?;
    Ok(())
}
```

`parse` accepts an in-memory string and performs no filesystem access. Validation
is separate: impossible dates remain in the parsed model until you check them.

## Results and errors

| API | Result |
| --- | --- |
| `parse(&str, Format)` | `Result<Changelog, ParseError>` |
| `validate(&Changelog)` | `Vec<Diagnostic>`; does not sort or mutate content |
| `serde_json::to_string_pretty(&changelog)` | JSON serialization |

`ParseError` provides `kind`, `line`, and `message`. Its kinds distinguish empty
input, missing releases, malformed headings, unsupported structure, and resource
limits. `Diagnostic` provides `severity`, `code`, `line`, and `message`.

The CLI adds colors and source annotations; library errors stay independent of
that presentation. For all public types and fields, run `cargo doc --no-deps --open`.
See the [data model](json.md) and [format rules](formats/keep-a-changelog.md).
