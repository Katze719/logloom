# Compatibility

These rules apply throughout 1.x. Changes are listed in the
[changelog](../CHANGELOG.md).

| Surface | Contract |
| --- | --- |
| Rust API | Existing signatures and field types stay compatible. Use constructors or `Default`; non-exhaustive types can gain fields or variants. |
| JSON | Existing field names, types, enum spellings, and attribute meanings stay stable. Additional fields and format-specific attributes may appear; readers should ignore unknown fields. New enum values may require a newer reader. |
| CLI | Existing commands, options, and [exit codes](cli.md#exit-codes) keep their meanings. Help text and human-readable output can improve. |
| Diagnostics | Match typed codes, not message wording, spacing, or colors. New checks and parser fixes can change findings, including `--strict` results. Pin the tool version when CI must stay identical. |
| Auto detection | Signatures can improve or become ambiguous as formats evolve. Use `--input-format` for repeatable automation. |
| Rust version | The declared minimum is Rust 1.99; CI uses 1.99.1. Raising the minimum requires a minor release and a changelog entry. |

Spans and retained `source` always describe the original input, including after
model edits or CLI filtering. CLI exports omit `source`. Independently decoded
GitHub bodies store reference definitions in `releases[].links`; ordinary
Markdown documents use the shared top-level `links`.
