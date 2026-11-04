# Changesets

## The whole changelog

```sh
logloom parse examples/changesets.md --format changesets
```

```markdown
# @lantern/core
## 2.0.0
### Minor Changes
- Add exports.
### Patch Changes
- Updated dependencies:
  - @lantern/shared@1.2.0
```

H2 versions become releases. The H1 package name lands in `attributes.package`.
Bump headings stay custom categories. Missing dates are fine. Commit links,
thank-yous, and nested dependency lists stay in the entries.

## One little YAML sandwich

```sh
logloom parse examples/changesets-fragment.md --format changesets
```

```markdown
---
"@lantern/core": minor
"@lantern/cli": patch
---
Add streaming exports and expose them in the CLI.
```

Each package gets a pending release (`version: null`) with `package`, `bump`, and
`fragment: "changesets"` attributes. Each gets the summary, in frontmatter order.
An empty mapping works for documentation-only changes. Summary spans point into
the original file; reference definitions are shared.

Keep the YAML flat: plain or quoted package names, major/minor/patch values,
and comments. Nested mappings, anchors, aliases, multiline scalars, and flow
mappings get an error.

Summaries copied across packages can add up to 16 MiB. Pass one file at a time;
dependency graphs, version plans, and directory aggregation aren't implemented.

Sources: [Changesets fragment format](https://changesets.dev/faq#what-is-a-changeset),
[published package history](https://github.com/changesets/changesets/blob/main/packages/cli/CHANGELOG.md).
