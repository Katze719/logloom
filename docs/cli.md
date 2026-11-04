# CLI

## Parse a file

```sh
logloom parse CHANGELOG.md
logloom parse CHANGELOG.md --format keep-a-changelog --output json
```

Both give you pretty-printed JSON and a final newline. Keep a Changelog is the
default input. `--output json` is there if you enjoy extra keystrokes; JSON is
currently the only output. Pick another [input format](formats/README.md) like this:

```sh
logloom parse examples/conventional-changelog.md --format conventional-changelog
logloom parse examples/github-release-notes.json --format github-release-notes
```

## Read stdin or save the result

```sh
cat CHANGELOG.md | logloom parse -
logloom parse CHANGELOG.md > changelog.json
```

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

Errors leave stdout empty. All validation findings go to stderr before exit;
warnings alone still let the JSON through.

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
