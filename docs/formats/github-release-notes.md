# GitHub Release Notes

Bring the saved Markdown or JSON export. logloom stays offline.

## Markdown body

```sh
logloom parse examples/github-release-notes.md --format github-release-notes
```

```markdown
## What's Changed
* Add exports by @alice in #42.
## New Contributors
* Welcome @bob.
**Full Changelog**: https://github.com/example/lantern/compare/v1...v2
```

Sections, contributors, links, and prose stay in order. A bare body doesn't tell
us its release tag, so it gets `version: null` and
`attributes.version-status: "unknown"`. Version-headed Markdown histories work too.

## JSON export

```sh
logloom parse examples/github-release-notes.json --format github-release-notes
```

Use one release object or an array. Array order stays as written:

```json
{
  "tag_name": "v1.0.0",
  "name": "First release",
  "published_at": "2026-11-04T12:00:00Z",
  "body": "## Changes\n- New API."
}
```

| JSON field | Model field |
| --- | --- |
| Required `tag_name` | `version` |
| `name`, `html_url` | `heading`, `link` |
| UTC ISO `published_at` | ISO `date`, original `attributes.published-at` |
| `body` (string, null, or absent) | Ordered Markdown sections and entries |
| `draft`, `prerelease` | String attributes; missing flags default to false |

The usual `gh` names work too: `tagName`, `publishedAt`, `url`, `isDraft`, and
`isPrerelease`. Unknown fields stay in the original JSON `source`.

Body spans point at the containing JSON string, since decoding changes the byte
offsets. `date_span` points at the original timestamp. Release assets stay on
GitHub; this is just reading the notes.

Sources: [release-note structure](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes),
[REST release schema](https://docs.github.com/en/rest/releases/releases#list-releases).
