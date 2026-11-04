# Output formats

```sh
logloom parse CHANGELOG.md --output-format yaml
logloom parse CHANGELOG.md --latest --output-format text
logloom parse CHANGELOG.md --category Fixed --output-format csv -o fixes.csv
```

All formats use the same [filters](filters.md). The input is validated before
anything goes to stdout or a file. The filename doesn't pick the format for you.

| `--output-format` | What comes out |
| --- | --- |
| `json` | Full model, pretty-printed; the default |
| `json-compact` | Same model on one line |
| `yaml` | Full model as YAML |
| `toml` | Full model as TOML; absent optional values are omitted |
| `jsonl` | One compact release object per line |
| `csv` | A header, then one row per entry; empty groups also get rows |
| `text` | Release headings, attributes, component/category labels, and readable entries |
| `markdown` | A Markdown document with release and category headings |

Exports keep document order and use UTF-8. Nonempty output ends with a newline.
The buffered export has a 256 MiB limit; exceeding it is an error (exit 3), with
stdout empty and an existing output file untouched. Usually a sign to use a filter.

## Full model: JSON, YAML, TOML

Same [fields](json.md#whats-in-there), different punctuation. These formats retain
the entire original `source`, metadata, links, and source spans, even after
filtering. TOML has no null, so missing optional values are absent keys.
All three deserialize back into `Changelog` with Serde.

## JSONL

```sh
logloom parse examples/keep-a-changelog.md --output-format jsonl \
  | jq -r '.version // "pending"'
```

Each line is a `Release`, including its sections, entries, attributes, and spans.
There's no outer object or array, and no document metadata, source copy, or link
definitions. Spans still refer to the original input. No matches means no bytes.

## CSV

```sh
logloom parse examples/extended-keep-a-changelog.md --release 1.0.0 \
  --output-format csv -o firmware.csv
```

| Columns | Contents |
| --- | --- |
| `format`, `release_index` | Input format and zero-based index in the filtered releases |
| `row_type` | `entry`, `section` for an empty group, or `release` for an empty release |
| `version`, `date`, `yanked` | Release values; absent values are empty cells |
| `release_heading`, `release_link` | Original heading and optional URL |
| `attributes` | Compact JSON object, including package/bump or author metadata |
| `component`, `category`, `section_heading` | Component, normalized category, and original section heading |
| `kind`, `content` | Entry kind and original Markdown/text; blank for empty groups |

Commas, quotes, and embedded newlines are quoted by the CSV writer. A multiline
entry stays one CSV record. No matches gives you just the header. Document
metadata, link definitions, and source spans aren't included.

## Text and Markdown

```sh
logloom parse examples/extended-keep-a-changelog.md --release 1.0.0 \
  --output-format markdown
```

```markdown
## 1.0.0 - 2026-08-01

### Firmware components

#### Added

- Initial firmware components for Lantern.
```

An excerpt; the title, introduction, other selected sections, and reference links
are included too. Package, bump, author, and other release metadata appear below
the release heading. Unnamed
sections use `Uncategorized`; unversioned notes use `Unversioned`, dated GNU
records use `Changes`, and pending changes use `Unreleased`.

Markdown keeps entry Markdown and fences plain text/reStructuredText as `text`
code blocks. Text removes Markdown styling while retaining code, nested lists,
and link URLs; raw HTML and reStructuredText stay literal.

These are reading views of the selected releases. They omit the source copy and
spans and normalize headings rather than reproducing the original file byte for
byte. No matches keeps the title, introduction, and reference links.
