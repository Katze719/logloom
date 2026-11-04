# Generic Markdown

For the `CHANGELOG.md` with house rules. Here are ours.

```sh
logloom parse examples/generic-markdown.md --input-format generic-markdown
```

```markdown
# Our changes
## Winter preview - 2026-11-04
### Highlights
- Faster startup.
## 0.9.0 (2026-10-01)
- Initial preview.
```

| Structure | Interpretation |
| --- | --- |
| H1 numeric/`v`-numeric release label | Release heading |
| H2 heading | Release, including custom names with spaces |
| H3 and deeper headings | Categories within the current release |
| ` - date` or `(date)` suffix | Optional original date text |
| Body without release headings | One unversioned notes container |

Rich Markdown, links, nested items, introductions, and custom categories stay in.
Invalid dates and duplicate versions still get errors. Missing dates and custom
categories don't get Keep a Changelog warnings.

Every H2 starts a release. So `## Improvements` really does become a release
named `Improvements`. The parser can't read the author's mind; check the layout
above before using this as a catch-all.

Notes without releases get `version: null` and
`attributes: {"version-status":"unknown"}`. The exact input stays in `source`.
