# Poking the code

## Build and check

Use Rust 1.99.1, edition 2024. `rust-toolchain.toml` pins it, CI uses it, and
rustup will pick it up. A distro install of the same version works too.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
cargo test --all-features
cargo test --doc
cargo doc --no-deps
```

`bacon clippy` is handy while editing. `cargo nextest run --all-features` also
works. CI runs the required checks and uses nextest when it's installed.
The fixtures include normal changelogs and things a changelog probably shouldn't do.

## Where things live

| File | Responsibility |
| --- | --- |
| [`src/lib.rs`](../src/lib.rs) | Public in-memory parsing API and format dispatch |
| [`src/model.rs`](../src/model.rs) | Owned, format-independent, Serde-compatible types |
| [`src/error.rs`](../src/error.rs) | Typed parsing failures |
| [`src/detection.rs`](../src/detection.rs) | Conservative format signatures and typed ambiguity errors |
| [`src/parsers/`](../src/parsers/) | Format implementations |
| [`src/parsers/markdown.rs`](../src/parsers/markdown.rs) | Shared Markdown blocks, links, and nesting limits |
| [`src/validation.rs`](../src/validation.rs) | Convention checks, separate from parsing |
| [`src/cli.rs`](../src/cli.rs) | Argument parsing, I/O, and exit codes; uses the public API |
| [`src/cli/render.rs`](../src/cli/render.rs) | Output formats and the buffered export limit |
| [`src/cli/render/`](../src/cli/render/) | Readable text/Markdown and CSV views |
| [`src/diagnostics.rs`](../src/diagnostics.rs) | Source annotations and colors |
| [`src/diagnostics/`](../src/diagnostics/) | Format-specific correction hints |

## Why these bits

| Choice | Reason |
| --- | --- |
| One Cargo package, library and binary | Shared parser without workspace overhead |
| pulldown-cmark events | CommonMark, source offsets, and reference resolution without recursive AST traversal; comrak's richer AST was unnecessary |
| clap derive, Serde, serde_json | CLI arguments, JSON, and JSONL |
| yaml_serde, toml, csv | Format-specific serialization and quoting; YAML uses the [YAML organization's maintained fork](https://github.com/yaml/yaml-serde) |
| annotate-snippets | Source diagnostics directly; color-eyre was unnecessary |
| Small calendar check | No chrono/jiff dependency needed for date-only validation |
| Flat Changesets mapping reader | Supports the fragment mapping subset; rejects unsupported YAML instead of evaluating aliases or templates |

## Add a format

1. Add it to `Format` and the CLI. Parse into the existing model and store the format for validation. Add conservative detection signatures and ambiguity tests.
2. Give it useful error hints and a file in `examples/`.
3. Test the normal input, the weird input, and the broken input through the API and CLI.
4. Write down the syntax and limits, link the example from the README, and run the checks above.

The rules live in [AGENTS.md](../AGENTS.md) and the mandatory
[Namtao Rust guidelines](https://www.namtao.com/rust/). Cargo.toml and clippy.toml
enforce them. Clippy is picky on purpose: fix the code when it complains.
Keep production code free of panics and unsafe code, and keep the lint settings strict.
