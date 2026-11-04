# Towncrier

## The finished history

```sh
logloom parse examples/towncrier.rst --input-format towncrier
logloom parse examples/towncrier.md --input-format towncrier
```

```rst
Lantern 2.0.0 (2026-11-04)
==========================
Bugfixes
--------
- Preserve Unicode paths. (#123)
```

Markdown headings and underlined reStructuredText headings both work. For dated
headings, the last token before the date is the version; the full project heading
stays intact. Features becomes Added, Bugfixes becomes Fixed, and other headings
stay custom. Issue references and RST syntax stay as written. RST directives
aren't evaluated or rendered.

## One fragment

```sh
logloom parse examples/towncrier/123.bugfix.rst --input-format towncrier
printf 'Fix Unicode paths.\n' | logloom parse - --input-format towncrier --fragment-name 123.bugfix.rst
```

| Name | Extracted information |
| --- | --- |
| `123.bugfix.rst` | Issue 123, type bugfix, Fixed category |
| `456.feature.md` | Issue 456, type feature, Added category, Markdown body |
| `+random.removal.txt` | Orphan fragment, type removal, Removed category |
| `123.misc` | Pending miscellaneous change; empty body allowed |

The filename is doing half the paperwork. Pass it along so the attributes can
include the issue, type, and filename. Fragments have `version: null` and
`fragment: "towncrier"`. Without a name, the content stays but issue/type stay unknown.
Custom types work too. `.md` bodies use Markdown; other fragment bodies stay raw text.

`misc` can say absolutely nothing. Other empty fragment types get an error.

`ISSUE.TYPE[.suffix]` selects fragment mode, even with release-like headings in
the body. Other input without recognized release headings is treated as a fragment too.

Custom templates, full RST semantics, directory aggregation, and pyproject config
aren't handled here. In Rust, use `parse_named` to pass a filename without opening it.

Source: [Towncrier tutorial and fragment conventions](https://towncrier.readthedocs.io/en/stable/tutorial.html).
