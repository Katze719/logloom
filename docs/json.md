# JSON and data model

## Select release identifiers

With the optional external tool jq, from the repository root:

```sh
logloom parse examples/keep-a-changelog.md | jq '.releases[].version'
```

```json
null
"2.1.0"
"2.0.0"
```

This is a stream of three JSON values. `null` identifies pending changes
(Unreleased); other identifiers remain strings and need not be SemVer.

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

List markers stay in the Markdown. Nested lists and multi-paragraph items stay
inside their parent entry; other blocks become separate `markdown` entries.

## Field guide

| Field | Contents |
| --- | --- |
| `metadata` | Optional plain-text `title` and introductory Markdown `description` |
| `releases` | Releases, including pending changes, in document order |
| `releases[].version`, `.date` | Optional opaque identifier and original date text |
| `releases[].heading`, `.link`, `.yanked` | Original heading Markdown, resolved heading URL, withdrawal flag |
| `releases[].sections` | Ordered groups; `category: null` means uncategorized content |
| `sections[].category` | Tagged category, e.g. `{"kind":"added"}` or `{"kind":"custom","name":"Performance"}` |
| `sections[].heading`, `.entries` | Optional original heading and ordered entries |
| `entries[].markdown`, `.kind` | Source Markdown and `list_item` or `markdown` |
| `links` | Effective reference definitions (`label`, `destination`, optional `title`), in source order |
| `source` | Exact input, including comments, whitespace, and original line endings |

## Source locations and ownership

Releases, sections, entries, and link definitions carry a `span` with half-open
UTF-8 byte offsets (`start`, `end`) and a one-based `line` in `source`.

The owned model implements Serde serialization and deserialization. Keeping the
original source costs memory but preserves unused or duplicate definitions and
unfamiliar syntax. Editing fields does not rewrite `source` or recalculate spans;
deserialization does not run validation automatically. This is an extraction
model, not a Markdown renderer or round-trip editor.
