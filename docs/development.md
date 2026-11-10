# Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
cargo test --all-features
cargo test --doc
cargo test --no-default-features
cargo test --no-default-features --features cli
cargo nextest run --all-features # if installed
```

Property tests run with the normal suite; failures save replayable seeds.
Use `PROPTEST_CASES=4096 cargo test --test properties` for a longer run.
The small [upstream corpus](../tests/fixtures/corpus/README.md) runs offline.

```sh
cargo bench --no-default-features --bench parser
```

The benchmark prints CSV timings for small, large, near-limit, nested, and
multi-body inputs. Set `LOGLOOM_BENCH_ITERATIONS` to a positive count (default 5).
Compare release builds on the same machine; on Linux, prefix the command with
`/usr/bin/time -v` to record peak memory too. CI compiles benchmarks without
enforcing machine-dependent timing thresholds.

## Publish a release

Update the package version in `Cargo.toml` and `Cargo.lock`, and add its matching
release entry to `CHANGELOG.md`,
then publish a GitHub release with the matching `vX.Y.Z` tag. GitHub can create
the tag while publishing the release. The Release workflow runs on
`release.published`, checks the tagged commit, builds installers, publishes the
crate, and attaches the release assets.

Pushing a tag alone doesn't start publishing. This avoids running the release
pipeline twice when creating a tag and a release together. Branch pushes and
pull requests still run the Rust workflow normally.

For verification or a deliberate retry, run the Release workflow manually with
an existing version tag. `dry-run` defaults to `true`; turn it off explicitly to
publish. The concurrency group serializes runs for the same tag; it doesn't
skip later runs.
