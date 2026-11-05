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
