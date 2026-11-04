# Development

## Build and check

Use stable Rust 1.99.1 (edition 2024), pinned in `rust-toolchain.toml` and CI.
rustup users get the pinned toolchain; a distribution-provided installation of
the same version also works. Nightly is unnecessary.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
cargo test --all-features
cargo test --doc
cargo doc --no-deps
```

Optional tools: `bacon clippy` and `cargo nextest run --all-features`. CI runs the
required checks and additionally uses nextest when installed. Fixtures cover
standard, unconventional, malformed, rich, and nested Markdown inputs.

## Code map

| File | Responsibility |
| --- | --- |
| [`src/lib.rs`](../src/lib.rs) | Public in-memory parsing API and format dispatch |
| [`src/model.rs`](../src/model.rs) | Owned, format-independent, Serde-compatible types |
| [`src/error.rs`](../src/error.rs) | Typed parsing failures |
| [`src/parsers/`](../src/parsers/) | Format implementations |
| [`src/validation.rs`](../src/validation.rs) | Convention checks, separate from parsing |
| [`src/cli.rs`](../src/cli.rs) | Argument parsing, I/O, and exit codes; uses the public API |
| [`src/diagnostics.rs`](../src/diagnostics.rs) | Source annotations and colors |
| [`src/diagnostics/`](../src/diagnostics/) | Format-specific correction hints |

## Design choices

| Choice | Reason |
| --- | --- |
| One Cargo package, library and binary | Shared parser without workspace overhead |
| pulldown-cmark events | CommonMark, source offsets, and reference resolution without recursive AST traversal; comrak's richer AST was unnecessary |
| clap derive, Serde, serde_json | CLI arguments and JSON |
| annotate-snippets | Source diagnostics directly; color-eyre was unnecessary |
| Small calendar check | No chrono/jiff dependency needed for date-only validation |

## Adding a format

1. Extend `Format` and add a parser using the existing model; no parser trait is required.
2. Add format-specific diagnostic hints and a document in `examples/`.
3. Cover supported constructs and failures with fixtures and public API/CLI tests.
4. Document syntax and limits, link the example from the README, and run the checks above.

Follow [AGENTS.md](../AGENTS.md) and the mandatory
[Namtao Rust guidelines](https://www.namtao.com/rust/). Their strict Clippy rules
and test exceptions are enforced in Cargo.toml and clippy.toml. Production code
must avoid panics and unsafe code; do not silence lints to bypass these rules.
