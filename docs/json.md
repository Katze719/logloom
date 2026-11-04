# JSON

## Select release identifiers

With jq installed, run this from the repo root:

```sh
logloom parse examples/keep-a-changelog.md | jq '.releases[].version'
```

```json
null
"2.1.0"
"2.0.0"
```

Three JSON values. Here, `null` means Unreleased. Other formats also use null for
unversioned notes or date/author records; their attributes tell you which.
Version strings stay strings. SemVer arguments can happen elsewhere.

## Select entries

```sh
logloom parse examples/keep-a-changelog.md \
  | jq '.releases[0].sections[0].entries[0] | {kind, markdown}'
```

```json
{
  "kind": "list_item",
  "markdown": "- A streaming export API."
}
```

List markers stay in. Nested lists and extra paragraphs stay with their parent
item. Other Markdown blocks get separate `markdown` entries; plain text and
reStructuredText can use `text`.

## What's in there

| Field | What you get |
| --- | --- |
| `format` | Source format, e.g. `changesets`; used by validation |
| `metadata` | Optional plain-text `title` and introductory Markdown `description` |
| `releases` | Releases, including pending changes, in document order |
| `releases[].version`, `.date` | Optional opaque identifier and original date text |
| `releases[].heading`, `.link`, `.yanked` | Heading text (decoded title for JSON), resolved heading URL, withdrawal flag |
| `releases[].attributes` | Optional string attributes such as package, bump, author, fragment type, or original timestamp |
| `releases[].sections` | Ordered groups; `category: null` means uncategorized content |
| `sections[].component` | Optional component name, e.g. `"Firmware components"`; omitted for ordinary sections |
| `sections[].category` | Tagged category, e.g. `{"kind":"added"}` or `{"kind":"custom","name":"Performance"}` |
| `sections[].heading`, `.entries` | Optional original heading and ordered entries |
| `entries[].markdown`, `.kind` | Original content and `list_item`, `markdown`, or `text` |
| `links` | Effective reference definitions (`label`, `destination`, optional `title`), in source order |
| `source` | Exact input, including comments, whitespace, and original line endings |

## Where did this come from

Releases, sections, entries, and link definitions have a `span`. `start` and `end`
count UTF-8 bytes in `source`, with the end excluded. Lines start at 1.
`date_span` points at dates outside headings, like a Debian trailer. Decoded
GitHub bodies point at their containing JSON token.

The model owns its data and supports Serde in both directions. The source copy
costs some RAM; it keeps comments, whitespace, duplicate definitions, and syntax
we don't interpret.

Edit model fields as needed. `source` and spans still describe the original file;
this isn't a Markdown editor. Run validation after deserializing if you need it.
Older JSON without `format` defaults to Keep a Changelog. Missing attributes,
component names, and date spans get their empty defaults.
