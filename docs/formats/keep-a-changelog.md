# Keep a Changelog

The default. Follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/)
and common 1.0.0 layouts. Give it the document you've already written.

## Start here

```markdown
# Changelog

## [Unreleased]

### Added

- An export API.
```

Empty releases and categories work too. The [full example](../../examples/keep-a-changelog.md)
has all six categories, nested entries, and comparison links. For firmware/CLI/etc.
in the same release, use [component categories](extended-keep-a-changelog.md).

## Name your releases

```markdown
## [Unreleased]
## [1.2.0] - 2026-11-04
## [preview-2026.11](https://example.com/preview) - 2026-11-03
## [1.1.0] - 2026-10-01 [YANKED]
## nightly-2026.11
```

Version labels can't contain whitespace. They stay strings, so `nightly-2026.11`
is fine. Dates are optional, and we keep their text. Nothing gets sorted for you.

## What works

| Construct | Behavior |
| --- | --- |
| H1 title and introduction | Optional; introductory Markdown and metadata are preserved |
| Top-level H2 headings | Start releases; ATX and setext styles work, including closing ATX hashes |
| Top-level H3 headings | Start categories, or components in the [extended layout](extended-keep-a-changelog.md); duplicate groups remain separate |
| Added, Changed, Deprecated, Removed, Fixed, Security | Normalized without ASCII case sensitivity; Unreleased is matched the same way |
| Other category names | Preserved as custom categories |
| H4–H6 headings | Preserved as Markdown entries, except H4 categories inside components |
| Lists, paragraphs, quotes, code, HTML | Preserved; nested content stays inside its parent block |
| Emphasis, images, links, tables, tasks, strikethrough | Preserved in entry Markdown |
| Inline and reference links | Heading URLs and effective reference definitions are extracted; unresolved references stay in Markdown |
| LF and CRLF | Original line endings remain in `source` |

## What gets a comment

| Input | Finding |
| --- | --- |
| `## [1.0] - 2025-02-29` | Error: impossible date |
| Two releases named `1.0` | Error: duplicate identifier |
| `## nightly` | Warning: published release has no date |
| `### Performance` | Warning: custom category, content preserved |
| Missing title or category | Warning; content preserved |
| Dated or misplaced Unreleased; dates out of order | Warning; nothing is reordered |

Dates use Gregorian `YYYY-MM-DD`, years 0001–9999. Empty groups and non-SemVer
versions are fine. Validation is a separate library call; the
[CLI](../cli.md#diagnostics) checks automatically and holds back JSON on errors.

## The small print

- Give it at least one top-level H2 release. An empty file gets an error.
- Every top-level H2 starts a release, including headings in introductions or
  footers. A second H1, an H1 after releases, or an H3 before a release is unsupported.
- Put dates after a hyphen. Localized dates and timestamps aren't normalized;
  en dashes and arbitrary release-heading annotations aren't supported.
- Headings inside lists, quotes, code, or HTML blocks do not start releases.
  Raw HTML is preserved without rendering or sanitizing.
- Footnotes, front matter, math, and custom directives stay in `source` without
  special interpretation. CommonMark is permissive: a lonely delimiter can just be text.
- Input is limited to 16 MiB and 128 nested Markdown containers, including
  inline formatting. The owned model and JSON serialization use additional memory.
- No automatic format detection is implemented; use `--format keep-a-changelog`.
