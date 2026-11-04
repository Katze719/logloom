# Detect and validate

## Let the file introduce itself

```sh
logloom detect examples/changesets.md
```

```text
changesets
```

`parse` and `validate` default to `--input-format auto`. `detect` uses the same
detector and prints one format name followed by a newline. All three accept `-`
for stdin and default to `CHANGELOG.md` in the current directory when no file
is given. Inspection is offline and reads at most 16 MiB.

## When auto refuses

```sh
logloom parse CHANGELOG.md --input-format keep-a-changelog
logloom validate CHANGELOG.md --input-format generic-markdown
```

Auto looks for specific signatures, rather than trying parsers until one accepts
the input. Broad Markdown readers would make that a very short guessing game.

| Clue | Candidate |
| --- | --- |
| H2 bracketed releases, Unreleased, or compact version/date headings | Keep a Changelog |
| Numeric release headings with parenthesized dates and conventional categories | Conventional Changelog |
| Named H2 releases with dash-separated dates | Generic Markdown |
| JSON release objects with `tag_name`/`tagName`, or GitHub-style Markdown headings and URLs | GitHub release notes |
| Changeset frontmatter or Major/Minor/Patch Changes categories | Changesets |
| Project-prefixed version/date headings or `123.bugfix.rst`-style filename hints | Towncrier |
| Package/version/distribution header with urgency | Debian |
| GNU NEWS outline headings or leading date/author records | GNU NEWS / ChangeLog |

Multiple signatures mean an error listing the candidates, exit 1, and no output.
Weak Markdown histories or unversioned bodies also need an explicit format.
Plain prose and unrelated JSON aren't silently treated as Generic Markdown.
Code blocks, quoted headings, and nested list headings aren't release signatures.

Normal filenames and extensions don't identify a format. Towncrier filenames
contribute a hint; conflicting content still needs an explicit choice. For stdin:

```sh
printf 'Fix a bug.\n' | logloom detect - --fragment-name 123.bugfix.rst
printf 'Fix a bug.\n' | logloom parse - --input-format towncrier \
  --fragment-name 123.bugfix.rst
```

## Check without exporting

```sh
logloom validate examples/keep-a-changelog.md
logloom validate --strict
logloom validate tests/fixtures/invalid-dates.md --color never
```

`validate` runs full parsing and the format's convention checks. stdout stays
empty; findings go to stderr with the same source locations, hints, and colors as
`parse`. Errors exit 1, warnings alone exit 0 by default. I/O failures exit 3.

Use `--strict` with `validate` or `parse` in CI: validation warnings also exit 1.
Warnings keep their severity and correction hints; a final error explains that
strict mode rejected them. `parse --strict` emits no result on warnings and
leaves existing output files untouched. Filters don't hide warnings elsewhere
in the input. Informational notes, such as no matching releases, still exit 0.

`detect` only identifies signatures. It can name a format even when a release has
an invalid date or an incomplete heading. Use `validate` to check the document.
Malformed JSON and size/nesting limits can prevent inspection itself.
