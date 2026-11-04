# logloom

Parse existing changelogs into structured JSON, from the command line or Rust.

Currently supports **Keep a Changelog 1.1.0**, including common 1.0.0 layouts.
See the [example changelog](examples/keep-a-changelog.md).

## Install

From a checkout, with Rust 1.99.1:

```sh
cargo install --path . --locked
```

## Usage

```sh
logloom parse CHANGELOG.md
cat CHANGELOG.md | logloom parse -
logloom --help
```

JSON goes to stdout; diagnostics go to stderr. Errors produce no JSON.

Try the included example without installing:

```sh
cargo run -- parse examples/keep-a-changelog.md
```

[Documentation](docs/README.md): CLI, Rust API, JSON, supported syntax, and development.

## License

[MIT](LICENSE)
