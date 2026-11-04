# Stuff to feed it

From the repo root:

```sh
cargo run -- parse examples/conventional-changelog.md --format conventional-changelog
```

| `--format` | Example files |
| --- | --- |
| `keep-a-changelog` (default) | [History](keep-a-changelog.md), [component categories](extended-keep-a-changelog.md) |
| `conventional-changelog` | [Generated Markdown](conventional-changelog.md) |
| `generic-markdown` | [Custom Markdown](generic-markdown.md) |
| `github-release-notes` | [Markdown body](github-release-notes.md), [JSON export](github-release-notes.json) |
| `changesets` | [Package history](changesets.md), [multi-package fragment](changesets-fragment.md) |
| `towncrier` | [reStructuredText history](towncrier.rst), [Markdown history](towncrier.md), [named fragment](towncrier/123.bugfix.rst) |
| `debian-changelog` | [Package changelog](debian.changelog) |
| `gnu-news` | [Release notes](gnu-news.txt) |
| `gnu-changelog` | [Date/author records](gnu-ChangeLog) |

These are made-up project histories, and the integration tests actually parse
them. Copy one, tweak it, or use it to see what the JSON looks like.
