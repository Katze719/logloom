# Component categories

Put a component above its categories. The extra `#` has a job now.
This uses the same `keep-a-changelog` format:

```sh
cargo run -- parse examples/extended-keep-a-changelog.md
```

```markdown
## [1.0.0] - 2026-08-01
### Firmware components
#### Added
- A new driver.
#### Changed
- Update the firmware API.
```

Each category gets its component name. Pick those two fields from the result:

```json
[
  {"component":"Firmware components","category":{"kind":"added"}},
  {"component":"Firmware components","category":{"kind":"changed"}}
]
```

## How the headings work

- A nonstandard H3 heading with a top-level H4 before the next H1/H2/H3 names a
  component. Its H4 headings name categories; custom categories stay supported.
- Standard H3 categories (`Added`, `Changed`, etc.) always remain release-wide.
  A custom H3 without H4 remains a custom category. Standard names ignore ASCII case.
- Sections stay in source order, including repeated components and categories.
  New H3 or release headings reset the component context.
- Prose before a component's first category is retained as an uncategorized
  section with the same component. Empty categories are retained.
- H5/H6 subheadings remain Markdown entries. This extension has one component
  level; nested component trees are not inferred. Headings inside lists, quotes,
  and code do not affect the grouping.

Ordinary sections leave `component` out of the JSON. Older JSON still loads.
Repeated component names can add up to 16 MiB across sections; after that,
you'll get a resource-limit error.
