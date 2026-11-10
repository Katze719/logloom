# Convert a changelog (experimental)

**Experimental — best effort.** Conversion can lose structure, metadata, and
formatting. Check the result before using it. Every run prints this reminder on
stderr; the converted document contains only the requested output.

```sh
logloom convert CHANGELOG.md --output-format debian-changelog
logloom convert changelog.json --input-format json --output-format keep-a-changelog
logloom convert export.csv --input-format csv --output-format yaml -o changelog.yaml
cat CHANGELOG.md | logloom convert - --input-format keep-a-changelog --output-format gnu-news
```

`convert` reads one file, `CHANGELOG.md` by default. The destination format is
required. All [input format names](formats/README.md) and
[export format names](output-formats.md) work as sources and destinations,
including conversion to the same format. YAML, TOML, and CSV require their
respective build features. `json` and `json-compact` read the same JSON model;
GitHub API JSON uses `github-release-notes`.

`--input-format auto` recognizes clear native signatures and structured exports.
Ambiguous input needs an explicit format; arbitrary text isn't an auto-detection
fallback. File extensions don't select formats.

## What gets written

| Destination | Output |
| --- | --- |
| Keep a Changelog | Markdown history, with component categories where representable |
| Conventional Changelog, Generic Markdown, GitHub release notes | Markdown history |
| Changesets | Package Markdown history |
| Towncrier | Markdown history |
| Debian Changelog | Package records with maintainer trailers |
| GNU NEWS | Outline release history |
| GNU ChangeLog | Date-and-author records |
| JSON, YAML, TOML, JSONL, CSV, text, Markdown | Existing export representation |

Inputs can include supported histories, saved GitHub JSON, and Changesets or
Towncrier fragments. Use `--fragment-name 123.bugfix.rst` for a Towncrier fragment
on stdin. Fragment, RST, and GitHub JSON *destinations* aren't offered yet.
There is no directory aggregation, Git history processing, or network access.

Native output can normalize layout, even when source and destination match. Entry order
is retained. Unsupported metadata is kept as labeled prose where possible;
components may become combined category labels. Raw text/RST and introductions
with structural headings may become fenced text blocks. Body-local references
are resolved before combining Markdown so equal labels keep their own URLs.

Changesets histories have one package heading. Other package names remain in
release prose when combining packages. GNU ChangeLog has no version field;
versions become prose. Unversioned notes can become `Unreleased` in Markdown
histories. Dates that would prevent recognizing that heading become prose too.
These adaptations produce conversion warnings.

## Returning to Keep a Changelog

```sh
logloom convert CHANGELOG.md --input-format keep-a-changelog --output-format json -o changelog.json
logloom convert changelog.json --input-format json --output-format keep-a-changelog -o restored.md
```

The JSON document retains the source format and structured content. Returning a
Keep a Changelog model to its own format preserves release links, ordered
reference definitions, raw entry Markdown, unnamed sections, component
introductions, empty categories, and ordinary H5/H6 content. Existing release
and category headings are reused when they still agree with the model. Edited
JSON fields take precedence over old headings or a retained source copy.

This path verifies the reconstructed model before accepting it and adds no
conversion-loss warning when the content and structure match. `--strict` still
rejects actual validation warnings. Nonrepresentable model edits or foreign
metadata use the general best-effort writer and its warnings.

The CLI JSON export omits the original source, so byte-for-byte layout isn't
guaranteed. Blank lines, title styling, and reference-definition syntax can
change. Spans in the model continue to describe the original input; parsing the
restored file creates new spans for that file. The same behavior applies to full
YAML and TOML models that retain `format: keep-a-changelog`.

## Missing values

Missing or unrepresentable required values use deterministic placeholders,
with warnings identifying the affected release. Existing values and native
pending representations are preferred. The source model isn't changed.

| Field | Placeholder |
| --- | --- |
| Package | `logloom-placeholder` |
| Maintainer / author | `Unknown <unknown@example.invalid>` |
| Date | `1970-01-01` |
| Debian distribution | `UNRELEASED` |
| Debian urgency | `low` |
| Required version | `0.0.0-logloom-placeholder-N`, with one-based release position `N` |
| Missing Debian clock / timezone | Release date at `00:00:00 +0000` |

Placeholders make a document writable. They aren't facts about your release.

## Import limitations

JSON, YAML, and TOML read the existing document model, including its backward
compatible defaults. YAML anchors and aliases must be expanded before importing; explicit YAML
mapping keys (`? key`) are unsupported.
JSONL contains releases but no document metadata, source format, or shared link
definitions. CSV also omits reference definitions and spans. It groups releases
by `release_index` in first-seen order; adjacent entries with identical section
labels cannot reliably reveal separate sections. Conflicting release metadata
and malformed records are errors.

Markdown reading views reconstruct component scopes and special release labels
heuristically. Attributes rendered as prose and earlier source spans aren't
restored. This import uses generic validation rather than assuming the original
format was Keep a Changelog.

Text import recognizes clear reading-view release headings and standard category
labels heuristically. Uncertain content stays text; without a recognizable
release structure, the complete input becomes unversioned text. Formatting,
custom-category boundaries, and other missing information cannot be recovered.
These imports warn about their limitations.

Exported spans still describe the original source. They aren't rewritten or used
to underline unrelated lines in an imported JSON/YAML/TOML file.

## Validation, filters, and files

The complete input is validated before [filters](filters.md) run. Parsing and
validation errors leave stdout empty and existing destination files untouched.
Warnings normally allow output. `--strict` also rejects validation and conversion
warnings; the experimental reminder alone doesn't fail the command.

```sh
logloom convert CHANGELOG.md --input-format keep-a-changelog \
  --release 1.2.0 --output-format towncrier -o release.md
```

`-o`, stdin/stdout, atomic replacement, color options, and
[exit codes](cli.md#exit-codes) follow the existing CLI rules. Input and output
cannot name the same file. Native destinations with no selected releases produce
no bytes and a warning. Empty JSONL and header-only CSV imports are supported.

The 16 MiB input limit, 128-container Markdown nesting limit, parser expansion
limits, and 256 MiB buffered output limit remain in effect. `--no-size-limit`
only removes the input-byte cap. Diagnostics retain at most 1,024 conversion
warnings and summarize additional occurrences.

The experimental command's format mappings and reconstruction heuristics may
change as bugs are fixed. Existing parsing APIs and export schemas keep their
[compatibility contract](compatibility.md).
