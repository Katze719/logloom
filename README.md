# logloom

Changelogs in. JSON, Markdown, and friends out.
A CLI and Rust library for the changelog you've already written.

## Input formats

- [Keep a Changelog](examples/keep-a-changelog.md), including [component categories](examples/extended-keep-a-changelog.md)
- [Conventional Changelog](examples/conventional-changelog.md)
- [Generic Markdown](examples/generic-markdown.md)
- [GitHub Release Notes](examples/github-release-notes.json)
- [Changesets](examples/changesets.md)
- [Towncrier](examples/towncrier.rst)
- [Debian Changelog](examples/debian.changelog)
- [GNU NEWS](examples/gnu-news.txt)
- [GNU ChangeLog](examples/gnu-ChangeLog)

[Syntax and limits](docs/formats/README.md) - [All examples](examples/README.md) - [Output formats](docs/output-formats.md)

Yes, there are this many ways to write down "fixed a bug".

## Install

Install from [crates.io](https://crates.io/crates/logloom) with Rust 1.99 or newer:

```sh
cargo install logloom --locked
```

Prefer a binary? Check [Releases](https://github.com/Katze719/logloom/releases)
or [CI builds](docs/installation.md) for Windows x64 and Linux x64/ARM64 installers and binaries.

## Usage

```sh
logloom parse
logloom detect
logloom validate
logloom validate --release 1.0.1
logloom parse CHANGELOG.md --input-format conventional-changelog
logloom parse CHANGELOG.md --latest --category Fixed -o release.json
logloom parse CHANGELOG.md --latest --output-format markdown -o release.md
logloom parse enormous-changelog.md --yes-i-know-i-am-stupid
cat CHANGELOG.md | logloom parse -
logloom --help
```

No file argument? It'll read `CHANGELOG.md` in the current directory.
The result goes to stdout, complaints go to stderr. JSON is the default.
If there's an error, stdout stays empty.

Release names stay as written. If your next version is called `spring-cleaning`,
that's fine. `--latest` goes by release date.
Inputs stop at 16 MiB by default; the long flag above lifts that cap.

Using it from Rust? See the [library guide](docs/library.md) and
[API reference](https://docs.rs/logloom).
More examples and the small print live in the [docs](docs/README.md).
Project changes live in the [changelog](CHANGELOG.md).

## License

[MIT](LICENSE)
