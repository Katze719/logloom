# CLI

## Parse a file

```sh
logloom parse CHANGELOG.md
logloom parse CHANGELOG.md --input-format keep-a-changelog --output-format json
```

Both give you pretty-printed JSON and a final newline. Keep a Changelog is the
default input. `--output-format json` is there if you enjoy extra keystrokes;
`json-compact` gives you single-line JSON. Pick another
[input format](formats/README.md) like this:

```sh
logloom parse examples/conventional-changelog.md --input-format conventional-changelog
logloom parse examples/github-release-notes.json --input-format github-release-notes
```

`--input-format` selects the reader, `--output-format` selects the result format,
and `--output-file` chooses where it goes.

## Pick the useful bits

```sh
logloom --version
logloom parse CHANGELOG.md --version 2.1.0
logloom parse CHANGELOG.md --latest --category Fixed
logloom parse CHANGELOG.md --unreleased --component "Firmware components"
```

`logloom --version` is the tool version. Under `parse`, `--version` selects a
release. [Filters](filters.md) covers combinations, packages, and limits.

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
creates a file literally named `-`; omit the option for stdout.

JSON is pretty-printed by default (`--output-format json`). Use
`--output-format json-compact` for one line, still with a final newline.
Both work with stdout and file output.

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
  | ^^^^^^^^^^^^^^^^^^^^^ expected a real calendar date in YYYY-MM-DD format
  |
  = help: Use YYYY-MM-DD with a real Gregorian calendar date (years 0001–9999); February 29 requires a leap year.
```

February 29, 2025 didn't happen. The error comes with a location and a hint for
the selected format. When there's source text to point at, it'll underline it.

Input errors leave stdout empty and output files untouched. All validation
findings go to stderr before exit; warnings alone still let the JSON through.

## Colors

```sh
logloom parse CHANGELOG.md --color always
logloom --color never parse CHANGELOG.md
```

The default is `auto`: colors on a stderr terminal, plain text when redirected.
A nonempty `NO_COLOR` or `TERM=dumb` also disables auto colors. `always` and
`never` override that. Your JSON stays free of escape-code confetti.

## Exit codes

| Code | What happened |
| --- | --- |
| 0 | Success, possibly with warnings; also help/version |
| 1 | Parsing, validation, unsupported structure, or input-limit error |
| 2 | Invalid command-line arguments |
| 3 | File, UTF-8 decoding, diagnostic/output I/O, or serialization failure |

More flags: `logloom parse --help`. More things to copy:
[JSON examples](json.md) and [formats](formats/README.md).
