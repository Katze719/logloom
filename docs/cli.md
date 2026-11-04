# CLI examples

## Parse a file

```sh
logloom parse CHANGELOG.md
logloom parse CHANGELOG.md --format keep-a-changelog --output json
```

Both commands emit pretty-printed JSON with a final newline. Keep a Changelog
and JSON are the defaults; no other input or output formats are implemented yet.

## Read stdin or save the result

```sh
cat CHANGELOG.md | logloom parse -
logloom parse CHANGELOG.md > changelog.json
```

Input must be UTF-8. Files and stdin have a 16 MiB limit. A closed stdout pipe is
handled gracefully.

## Diagnostics

```sh
logloom parse tests/fixtures/invalid-dates.md --color never
```

Run fixture commands from the repository root. A diagnostic looks like this:

```text
error[invalid-date]: expected a real calendar date in YYYY-MM-DD format
 --> tests/fixtures/invalid-dates.md:3:1
  |
3 | ## [2.0] - 2025-02-29
  | ^^^^^^^^^^^^^^^^^^^^^ expected a real calendar date in YYYY-MM-DD format
  |
  = help: Use YYYY-MM-DD with a real Gregorian calendar date (years 0001–9999); February 29 requires a leap year.
```

Errors leave stdout empty. All validation findings appear on stderr before exit.
Warnings alone still allow JSON output. Correction hints follow the selected
input format; source excerpts appear when a location is available.

## Colors

```sh
logloom parse CHANGELOG.md --color always
logloom --color never parse CHANGELOG.md
```

`auto` is the default: colors are enabled when stderr is a terminal, unless
`NO_COLOR` is nonempty or `TERM=dumb`. Explicit `always` and `never` override
detection. Redirected diagnostics stay plain in auto mode; JSON is never colored.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success, possibly with warnings; also help/version |
| 1 | Parsing, validation, unsupported structure, or input-limit error |
| 2 | Invalid command-line arguments |
| 3 | File, UTF-8 decoding, diagnostic/output I/O, or serialization failure |

For argument details, use `logloom parse --help`. See also the
[JSON examples](json.md) and [supported syntax](formats/keep-a-changelog.md).
