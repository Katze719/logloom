# Publishing

## Before the upload

1. Pick an unpublished version in `Cargo.toml` and refresh `Cargo.lock` with
   `cargo check --all-features`.
2. Move the relevant `Unreleased` entries into a dated release in `CHANGELOG.md`.
3. Run the [development checks](development.md#build-and-check) and the checks
   below. Replace `0.1.1` with the version you're releasing.

```sh
cargo run --locked -- validate --strict --release 0.1.1
cargo package --locked --list
cargo publish --dry-run --locked
```

The dry run builds the packaged source without uploading it. Commit the release
changes first; the normal publish flow expects a clean package tree. No need to
teach `--allow-dirty` bad habits.

## Package contents

`Cargo.toml` declares the repository, API docs, README, license, keywords, and
categories. Its `include` list keeps source, tests, fixtures, examples, docs,
the changelog, and build/lint configuration in the crate. CI scripts, local
agent instructions, and build artifacts stay out. Cargo also adds its normalized
manifest, lockfile, and VCS metadata.

Review the file list whenever you add a directory needed by the build or tests.
The package must build and test without files from your checkout.

## Upload when ready

```sh
cargo publish --locked
```

This uploads to crates.io using your Cargo credentials. A published version can't
be overwritten, so use a fresh version for the next upload. The package metadata
goes with that upload; editing the local manifest doesn't update crates.io.
