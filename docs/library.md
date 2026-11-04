# The Rust bits

## Add a local dependency

There's no crates.io release yet. Point this at your checkout:

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

Feed `parse` a string; file reading is your call. Then use `validate` to check the
contents. Parsing will happily retain an impossible date so you can inspect it.

`validate` uses the format stored in the model. A Changesets file won't get
lectured about missing release dates.

## Named fragments

```rust
use logloom::{parse_named, Format};

let changelog = parse_named("Fix Unicode paths.", Format::Towncrier, "123.bugfix.rst")?;
# Ok::<(), logloom::ParseError>(())
```

Towncrier puts the issue and type in the filename. Pass that name along; nothing
gets opened. The CLI does this for files. For stdin, use `--fragment-name`.

## Results and errors

| API | Result |
| --- | --- |
| `parse(&str, Format)` | `Result<Changelog, ParseError>` |
| `parse_named(&str, Format, &str)` | Same result, using Towncrier fragment filename metadata |
| `validate(&Changelog)` | `Vec<Diagnostic>`; does not sort or mutate content |
| `validate_for_format(&Changelog, Format)` | Explicit override of the stored validation format |
| `serde_json::to_string_pretty(&changelog)` | JSON serialization |
| `logloom::detect(input)` | Detect specific signatures; returns a format or a typed detection error |
| `logloom::detect_named(input, name)` | Detection using a Towncrier fragment filename hint |
| `Category::as_str()`, `Format::as_str()` | Display names and stable format names |

`ParseError` gives you `kind`, `line`, and `message`: empty input, missing
releases, malformed headings/data, unsupported structure, or a resource limit.
`Diagnostic` gives you `severity`, `code`, `line`, and `message`.

The CLI adds the colors and source annotations. In Rust you get plain typed errors.
For every public field, run `cargo doc --no-deps --open`. Otherwise, start with
the [data model](json.md) or your [format](formats/README.md).
