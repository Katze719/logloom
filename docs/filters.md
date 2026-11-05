# Filters

## One release, please

```sh
logloom parse CHANGELOG.md --latest
logloom parse CHANGELOG.md --release 2.1.0
logloom parse CHANGELOG.md --release 2.1.0 --release 2.0.0
logloom parse CHANGELOG.md --first-published
logloom parse CHANGELOG.md --unreleased
logloom parse CHANGELOG.md --limit 3
```

| Option | Keeps |
| --- | --- |
| `--release VERSION` | Exact release identifier, case-sensitive; repeatable |
| `--latest` | Published match with the newest calendar date |
| `--first-published` | First matching published release in file order |
| `--unreleased` | Pending changes, fragments, and GitHub drafts |
| `--limit N` | First N matches, including pending changes; N must be positive |

Versions are opaque strings. `v2.1.0` and `2.1.0` are different, and nobody here
is sorting them. `--latest` and `--first-published` skip pending changes and drafts;
prereleases and yanked releases still count. GNU ChangeLog's dated records count
as published.
Unversioned notes with unknown release status match neither published selector
nor `--unreleased`; omit those flags to keep them.

`--latest` uses `YYYY-MM-DD` dates, with day precision. If any published match
has no date, or several share the newest date, it exits 1 without output.
Narrow the matches with a filter, use `--release`, or choose `--first-published`.
Version strings and timestamps don't break ties. No matches is still a success.

The two published selectors conflict with each other and with `--release`,
`--unreleased`, and `--limit`. `--unreleased` also conflicts with `--release`.

## Less changelog, more signal

```sh
logloom parse CHANGELOG.md --category Fixed --category Security
logloom parse examples/extended-keep-a-changelog.md \
  --component "Firmware components" --category Added
logloom parse examples/changesets-fragment.md --input-format changesets \
  --package @lantern/cli --unreleased
```

| Option | Matches |
| --- | --- |
| `--category NAME` | Normalized category or custom category name |
| `--component NAME` | Component-scoped sections; global sections don't match |
| `--package NAME` | Exact `attributes.package`, case-sensitive |

All three are repeatable. Categories and components ignore ASCII case.
Standard categories are Added, Changed, Deprecated, Removed, Fixed, and Security.
For Conventional Changelog, `Features` maps to `Added` and `Bug Fixes` to `Fixed`.
Custom names like `Performance` work too.

Different filters must all match; repeated values for one filter are alternatives.
Section filters remove unmatched sections and releases with no matching sections.
An empty category can still match. Published selection and `--limit` run after these filters.

No matches? Still a success, with a short note on stderr. JSON, YAML, and TOML have an empty `releases` array;
JSONL is empty, CSV keeps its header, and text/Markdown keep the introduction
and reference links. The entire input is validated first, so filters won't hide
broken dates or malformed headings.

JSON, YAML, and TOML retain [original source spans](json.md#where-did-this-come-from),
but omit the `source` copy. Spans refer to the original input.
See [output formats](output-formats.md) for what each export includes.
