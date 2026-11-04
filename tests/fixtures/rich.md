# **Lantern** changes

An introduction with *emphasis*, a comment, and custom metadata.

<!-- This comment is meaningful to maintainers. -->

## [preview-2026.11](https://example.com/preview) - 2026-11-04

A release announcement outside a category.

### Added

- **Rich** text, `code`, ~~old~~ and [a guide][guide].

  A second paragraph in the same entry.

  - A nested child.
    - A deeper child.

  ```markdown
  ## This is not a release
  ### This is not a category
  ```

- [x] A completed task with ![an icon](icon.png).

1. An ordered entry.
2. Another ordered entry.

> A quoted explanation.
>
> ## A quoted heading is not a release

| Feature | Status |
| --- | --- |
| Export | Ready |

#### Migration notes

Use the new API.

```rust
let label = "## Not a release";
```

<details>
<summary>Details</summary>
Additional context.
</details>

### Documentation

- The reference is now searchable.

[guide]: https://example.com/guide "User guide"
