# CLI

## Parse a file

```sh
logloom parse
logloom parse CHANGELOG.md
logloom parse CHANGELOG.md --input-format keep-a-changelog --output-format json
```

Without a file argument, `parse`, `validate`, and `detect` read `CHANGELOG.md`
from the current directory. Pass another path or `-` for stdin.

Both give you pretty-printed JSON and a final newline. Input defaults to
`auto`, which refuses ambiguous formats. `--output-format json` is there if you enjoy extra keystrokes;
`json-compact` gives you single-line JSON. Pick another
[input format](formats/README.md) like this:

```sh
logloom parse examples/conventional-changelog.md --input-format conventional-changelog
logloom parse examples/github-release-notes.json --input-format github-release-notes
```

`--input-format` selects the reader, `--output-format` selects the result format,
and `--output-file` chooses where it goes.

```sh
logloom parse CHANGELOG.md --output-format yaml
logloom parse CHANGELOG.md --latest --output-format markdown -o release.md
```

Output choices: `json`, `json-compact`, `text`, `yaml`, `markdown`, `toml`,
`jsonl`, and `csv`. See [output formats](output-formats.md) for their shapes.

## Pick the useful bits

```sh
logloom --version
logloom parse CHANGELOG.md --release 2.1.0
logloom parse CHANGELOG.md --latest --category Fixed
logloom parse CHANGELOG.md --first-published
logloom parse CHANGELOG.md --unreleased --component "Firmware components"
```

`--version` is always the tool version, including after a subcommand.
`--release` selects a release identifier. `--latest` compares calendar dates;
`--first-published` uses file order. [Filters](filters.md) covers the details.

## Inspect or check

```sh
logloom detect CHANGELOG.md
logloom validate CHANGELOG.md --input-format keep-a-changelog
logloom validate --release 1.2.0
```

`detect` prints one format name. `validate` exports nothing: findings go to
stderr, errors exit 1, and warnings alone exit 0 by default. Both accept `-` for stdin and
`--fragment-name` for Towncrier. See [detection and validation](detection.md).

`validate --release VERSION` also requires that exact release to exist. A missing
release exits 1 with a hint on stderr. Add `--package NAME` to check a specific
package; both values must belong to the same release. stdout stays empty.

For CI, let warnings fail the job too:

```sh
logloom validate --strict
logloom parse --strict --output-format markdown -o release.md
```

`--strict` applies to `parse` and `validate`. Any validation warning makes the
command exit 1. Parsing with warnings leaves stdout empty and existing output
files untouched. The whole input is checked before filters run.

## Read stdin or save the result

```sh
cat CHANGELOG.md | logloom parse -
logloom parse CHANGELOG.md > changelog.json
logloom parse CHANGELOG.md --latest --output-file release.json
logloom parse CHANGELOG.md --output-format json-compact -o changelog.json
```

`--output-file` (`-o`) writes to a file and keeps stdout empty. Existing files
are replaced only after parsing, full validation, and serialization succeed;
a failed write leaves the old file intact. Unlike shell redirection, it won't
truncate your file before logloom has even had a look.

The parent directory must already exist. Using the input path as the output
(including a symlink to it) is a usage error. An output symlink to another file
is replaced with a regular file; its target stays untouched. `--output-file -`
uses stdout, just like omitting the option. Use `--output-file ./-` for a file
literally named `-`.

JSON is pretty-printed by default (`--output-format json`). Use
`--output-format json-compact` for one line, still with a final newline.
All output formats work with stdout and file output. The filename extension
doesn't select the format.

Use UTF-8 and keep the input at or below 16 MiB. That includes stdin.
Closing a pipe early is fine; the CLI handles it.

## Diagnostics

```sh
logloom parse tests/fixtures/invalid-dates.md --color never
```

Run that from the repo root. Here's what you'll get:

```text
error[invalid-date]: expected a real calendar date in YYYY-MM-DD format
 --> tests/fixtures/invalid-dates.md:3:1
  |
3 | ## [2.0] - 2025-02-29
  | ^^^^^^^^^^^^^^^^^^^^^
  |
  = help: Use YYYY-MM-DD with a real Gregorian calendar date (years 0001–9999); February 29 requires a leap year.
```

February 29, 2025 didn't happen. The error comes with a location and a hint for
the selected format. When there's source text to point at, it'll underline it.

Input errors leave stdout empty and output files untouched. All validation
findings go to stderr before exit; warnings alone still let the result through
unless `--strict` is enabled.
Repeated warnings share one explanation and list the other affected lines.
Errors keep their own source excerpts. A filter with no matches gets a short
note on stderr and still exits 0.

## Colors

```sh
logloom parse CHANGELOG.md --color always
logloom --color never parse CHANGELOG.md
```

The default is `auto`: colors on a stderr terminal, plain text when redirected.
A nonempty `NO_COLOR` or `TERM=dumb` also disables auto colors. `always` and
`never` override that. Exports stay free of diagnostic escape-code confetti.

## Exit codes

| Code | What happened |
| --- | --- |
| 0 | Success, possibly with warnings unless strict; also help/version |
| 1 | Parsing/validation failure, missing required release, unsupported structure, input-limit error, or warnings with `--strict` |
| 2 | Invalid command-line arguments |
| 3 | File, UTF-8 decoding, diagnostic/output I/O, or serialization failure |

`logloom parse --help` groups input, output, and filter options, lists the formats,
and gives you commands to copy. More examples:
[JSON examples](json.md) and [formats](formats/README.md).

## Shell completions

```sh
logloom completions bash > logloom.bash
source logloom.bash
```

Also accepts `zsh`, `fish`, `powershell`, and `elvish`. Generated scripts reflect
the commands and optional exports in your build.
