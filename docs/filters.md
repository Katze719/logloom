# Filters

## One release, please

```sh
logloom parse CHANGELOG.md --latest
logloom parse CHANGELOG.md --version 2.1.0
logloom parse CHANGELOG.md --version 2.1.0 --version 2.0.0
logloom parse CHANGELOG.md --unreleased
logloom parse CHANGELOG.md --limit 3
```

| Option | Keeps |
| --- | --- |
| `--version VERSION` | Exact release identifier, case-sensitive; repeatable |
| `--latest` | First matching published release in file order |
| `--unreleased` | Pending changes, fragments, and GitHub drafts |
| `--limit N` | First N matches, including pending changes; N must be positive |

Versions are opaque strings. `v2.1.0` and `2.1.0` are different, and nobody here
is sorting them. `--latest` skips pending changes and drafts; prereleases and
yanked releases still count. GNU ChangeLog's dated records count as published.
Unversioned notes with unknown release status match neither `--latest` nor
`--unreleased`; omit those flags to keep them.

`--latest` can't be combined with `--version`, `--unreleased`, or `--limit`.
`--unreleased` also conflicts with `--version`.

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
An empty category can still match. `--latest` and `--limit` run after these filters.

No matches? Successful JSON with an empty `releases` array. The entire input is
still validated first, so filters won't hide broken dates or malformed headings.
The [original source and spans](json.md#where-did-this-come-from) stay in the JSON.
