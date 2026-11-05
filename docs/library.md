# The Rust bits

## Add the crate

Use the [published crate](https://crates.io/crates/logloom) with Rust 1.99 or newer:

```toml
[dependencies]
logloom = { version = "0.1.2", default-features = false }
serde_json = "1.0"
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

The library retains the exact input in `Changelog.source`, and direct Serde
serialization includes it. CLI exports omit this field; reading them back into
`Changelog` gives an empty `source`. Spans always refer to the original input.

`validate` uses the format stored in the model. A Changesets file won't get
lectured about missing release dates.

For inputs above 16 MiB, use `ParseOptions::without_input_limit().parse(input, format)`.
The same options expose `parse_named`, `detect`, and `detect_named`.
`ParseOptions::default()` keeps the cap; `max_input_bytes` accepts a custom byte
limit or `None`. Nesting and metadata-expansion limits stay in place.

Use `Release::status()`, `is_pending()`, `is_published()`, `package()`, and
`is_prerelease()` instead of interpreting format attributes yourself.
Construct editable models with `Changelog::new`, `Release::new`, `Section::new`,
`Entry::new`, and `Default` for metadata and empty spans. Fields remain mutable;
the types are non-exhaustive so future fields don't require a breaking release.

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

Default features include `cli`, `yaml`, `toml`, and `csv`. Disabling defaults
keeps the parser and Serde model; `cli` enables JSON, JSONL, text, Markdown, and
completions. Each optional export feature also enables the CLI.
See the [compatibility contract](compatibility.md).
