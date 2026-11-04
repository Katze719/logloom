# Keep a Changelog

The only supported input format is [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
including common 1.0.0 layouts. logloom parses existing documents; it does not
inspect Git history or generate changelogs.

## Minimal input

```markdown
# Changelog

## [Unreleased]

### Added

- An export API.
```

Empty releases and categories also work. See the
[complete example](../../examples/keep-a-changelog.md) for all six categories,
multiple releases, nested content, and comparison links.

## Release headings

```markdown
## [Unreleased]
## [1.2.0] - 2026-11-04
## [preview-2026.11](https://example.com/preview) - 2026-11-03
## [1.1.0] - 2026-10-01 [YANKED]
## nightly-2026.11
```

Versions are whitespace-free opaque identifiers. Dates are optional; their text
is preserved. Releases and entries keep their original order.

## Supported constructs

| Construct | Behavior |
| --- | --- |
| H1 title and introduction | Optional; introductory Markdown and metadata are preserved |
| Top-level H2 headings | Start releases; ATX and setext styles work, including closing ATX hashes |
| Top-level H3 headings | Start categories; duplicate groups remain separate |
| Added, Changed, Deprecated, Removed, Fixed, Security | Normalized without ASCII case sensitivity; Unreleased is matched the same way |
| Other category names | Preserved as custom categories |
| H4–H6 headings | Preserved as Markdown entries inside releases |
| Lists, paragraphs, quotes, code, HTML | Preserved; nested content stays inside its parent block |
| Emphasis, images, links, tables, tasks, strikethrough | Preserved in entry Markdown |
| Inline and reference links | Heading URLs and effective reference definitions are extracted; unresolved references stay in Markdown |
| LF and CRLF | Original line endings remain in `source` |

## Validation examples

| Input | Finding |
| --- | --- |
| `## [1.0] - 2025-02-29` | Error: impossible date |
| Two releases named `1.0` | Error: duplicate identifier |
| `## nightly` | Warning: published release has no date |
| `### Performance` | Warning: custom category, content preserved |
| Missing title or category | Warning; content preserved |
| Dated or misplaced Unreleased; dates out of order | Warning; nothing is reordered |

Valid dates use Gregorian `YYYY-MM-DD`, years 0001–9999. Empty groups and
non-SemVer versions are accepted. The library reports validation separately;
the [CLI](../cli.md#diagnostics) suppresses JSON whenever an error is found.

## Boundaries

- At least one top-level H2 release is required. An empty document is an error.
- Every top-level H2 starts a release, including headings in introductions or
  footers. A second H1, an H1 after releases, or an H3 before a release is unsupported.
- Dates follow a hyphen separator. Localized dates and timestamps are not
  normalized; en dash separators and arbitrary heading annotations are unsupported.
- Headings inside lists, quotes, code, or HTML blocks do not start releases.
  Raw HTML is preserved without rendering or sanitizing.
- Footnotes, front matter, math, and custom directives are not semantically
  interpreted; their text remains in `source`. CommonMark controls malformed
  inline syntax, so an unmatched delimiter may simply be ordinary text.
- Input is limited to 16 MiB and 128 nested Markdown containers, including
  inline formatting. The owned model and JSON serialization use additional memory.
- No automatic format detection is implemented; use `--format keep-a-changelog`.
