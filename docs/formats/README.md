# Pick a format

Use `--input-format` to pick one. The default is Keep a Changelog.
Format guessing and network fetching aren't implemented, so bring the file.

```sh
logloom parse CHANGELOG.md --input-format conventional-changelog
```

| `--input-format` | What to feed it |
| --- | --- |
| `keep-a-changelog` | [Keep a Changelog Markdown](keep-a-changelog.md), including [component categories](extended-keep-a-changelog.md) |
| `conventional-changelog` | [Generated Markdown histories](conventional-changelog.md) |
| `generic-markdown` | [Custom Markdown release headings or notes](generic-markdown.md) |
| `github-release-notes` | [Saved Markdown bodies and release JSON](github-release-notes.md) |
| `changesets` | [Package histories and individual changeset fragments](changesets.md) |
| `towncrier` | [Markdown/reStructuredText histories and named fragments](towncrier.md) |
| `debian-changelog` | [Debian package records](debian-changelog.md) |
| `gnu-news`, `gnu-changelog` | [GNU release notes and date/author records](gnu.md) |

The original text and order stay intact. Files can be up to 16 MiB, with at most
128 nested Markdown containers. That's already quite a few blockquotes.

Changeset summaries copied across packages and component names repeated across
sections each have a 16 MiB expansion limit. Pass individual documents or
fragments. Rendering, templates, directory aggregation, and history generation
aren't part of this tool.

Start with an [example file](../../examples/README.md), then see what ends up
in the [JSON](../json.md).
