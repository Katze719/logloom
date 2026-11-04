# logloom

Changelogs in, JSON out. A CLI and Rust library for Markdown histories,
release notes, and news fragments.

## Formats

- [Keep a Changelog](examples/keep-a-changelog.md), including [component categories](examples/extended-keep-a-changelog.md)
- [Conventional Changelog](examples/conventional-changelog.md)
- [Generic Markdown](examples/generic-markdown.md)
- [GitHub Release Notes](examples/github-release-notes.json)
- [Changesets](examples/changesets.md)
- [Towncrier](examples/towncrier.rst)
- [Debian Changelog](examples/debian.changelog)
- [GNU NEWS](examples/gnu-news.txt)
- [GNU ChangeLog](examples/gnu-ChangeLog)

[Syntax and limits](docs/formats/README.md) - [All examples](examples/README.md)

Yes, there are this many ways to write down "fixed a bug".

## Install

Grab a checkout and Rust 1.99.1, then:

```sh
cargo install --path . --locked
```

## Usage

```sh
logloom parse CHANGELOG.md
logloom parse CHANGELOG.md --format conventional-changelog
cat CHANGELOG.md | logloom parse -
logloom --help
```

JSON goes to stdout, complaints go to stderr. If there's an error, stdout stays empty.

Try it straight from the checkout:

```sh
cargo run -- parse examples/keep-a-changelog.md
```

More examples, the Rust API, and the small print live in the [docs](docs/README.md).

## License

[MIT](LICENSE)
