# logloom

Changelogs in. JSON, Markdown, and friends out. A CLI and Rust library for
Markdown histories, release notes, and news fragments.

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

Grab a checkout and Rust 1.99.1, then:

```sh
cargo install --path . --locked
```

## Usage

```sh
logloom parse
logloom detect
logloom validate
logloom parse CHANGELOG.md --input-format conventional-changelog
logloom parse CHANGELOG.md --latest --category Fixed -o release.json
logloom parse CHANGELOG.md --latest --output-format markdown -o release.md
cat CHANGELOG.md | logloom parse -
logloom --help
```

No file argument? It'll read `CHANGELOG.md` in the current directory.
The result goes to stdout, complaints go to stderr. JSON is the default.
If there's an error, stdout stays empty.

Try it straight from the checkout:

```sh
cargo run -- parse examples/keep-a-changelog.md
```

More examples, the Rust API, and the small print live in the [docs](docs/README.md).
Project changes live in the [changelog](CHANGELOG.md).

## License

[MIT](LICENSE)
