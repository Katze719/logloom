# Conventional Changelog

Feed it the generated file. The generator already did its thing; logloom reads the result.

```sh
logloom parse examples/conventional-changelog.md --input-format conventional-changelog
```

## Looks like this

```markdown
# [2.0.0](https://example.com/compare/v1...v2) (2026-11-04)
### Features
* **export:** add streaming support ([abc1234](https://example.com/commit/abc1234))
### 1.0.1 (2026-10-01)
### Bug Fixes
* Preserve Unicode paths, closes #42.
```

| Input | Result |
| --- | --- |
| `#`, `##`, or `###` version heading | A release, including patch histories using H3 |
| `(YYYY-MM-DD)` or ` - YYYY-MM-DD` | Optional date; calendar validity checked separately |
| `Features`, `Bug Fixes`, `Performance Improvements` | Added, Fixed, Changed categories |
| Breaking-change and other headings | Custom categories, original text retained |
| Commit/issue links and scoped entries | Original Markdown preserved |

Release labels start with a digit, `v` plus a digit, `Unreleased`, or `Next`.
Labels stay strings. Headings in lists, code, and quotes stay inside that content.

Common Angular/conventionalcommits output works. Custom generator templates may
need [Generic Markdown](generic-markdown.md). Without a recognizable release
heading, this parser returns an error. All those commit and issue links stay intact.

Sources: [Conventional Changelog](https://github.com/conventional-changelog/conventional-changelog),
[upstream history](https://github.com/conventional-changelog/conventional-changelog/blob/master/packages/conventional-changelog/CHANGELOG.md).
Use the rendered history; this parser doesn't read commits.
