# Changelog

All notable changes to logloom are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Experimental, best-effort `convert` command for converting between supported
  changelog formats and exports, with explicit loss warnings and placeholders.

## [1.0.1] - 2026-11-10

### Fixed

- Accept introductory Markdown descriptions before a component's first category
  without uncategorized-content warnings; uncategorized list items still warn.

## [1.0.0] - 2026-11-05

### Added

- `--yes-i-know-i-am-stupid` (alias `--no-size-limit`) to lift the 16 MiB input
  cap for parsing, validation, and detection, including stdin.
- `ParseOptions` for library callers that need a custom input cap or no cap.
  Existing APIs keep their defaults; nesting and metadata-expansion limits remain.
- Public model constructors and release status/package queries, with
  non-exhaustive model types for compatible future extensions.
- Optional CLI dependencies and individual YAML, TOML, and CSV export features;
  all existing CLI features remain enabled by default.
- `completions` for Bash, Zsh, Fish, PowerShell, and Elvish.
- Property tests, licensed upstream changelog fixtures, saved JSON contracts,
  parser benchmarks, and macOS/feature-matrix CI checks.

### Changed

- Define the Rust API, JSON, and CLI compatibility contract for the 1.x series.
- Model structs now use constructors or `Default` instead of external struct
  literals. Fields remain public and editable. `Severity` is non-exhaustive too.
- Independently decoded GitHub bodies store their reference definitions in
  `Release.links`, defaulting to empty when reading older JSON. Shared Markdown
  references remain in `Changelog.links`.

### Fixed

- Preserve fixture bytes across Windows Git checkouts so saved source spans stay
  valid; account for macOS filesystems rejecting non-UTF-8 filenames in CLI tests.
- Keep identically named references in separate GitHub release bodies from
  resolving to another release's URL, including filtered text/Markdown exports.

## [0.1.2] - 2026-11-05

### Added

- Automatic crates.io publishing through Trusted Publishing for version tags
  and published GitHub releases, with verified platform assets attached to the
  release, checksum checks, retry support, and a manual dry run.

## [0.1.1] - 2026-11-05

### Added

- `validate --release VERSION` to require a release to exist, with optional
  `--package NAME` for package-specific checks and exit codes for CI.
- CI artifacts for Windows x64: a standalone executable and MSI installer.
- Static musl binaries, tarballs, DEB packages, and RPM packages for Linux x64
  and ARM64, with SHA-256 checksums and package checks.

### Changed

- Add crates.io repository/documentation metadata, search keywords, and categories;
  explicitly select published files to keep CI and local agent instructions out.
- Omit the original `source` copy from CLI parse exports in every output format.
- Document crates.io installation and the published Rust library dependency.

### Fixed

- Fix Windows MSI upgrade rules so installer validation passes without
  suppressing ICE61 checks.
- Make LF/CRLF regression tests work with Windows Git checkouts without
  introducing extra carriage returns into the test inputs.
- Report directory inputs consistently on Windows and Linux, with a hint to
  choose a changelog file instead of a misleading permissions hint.

## [0.1.0] - 2026-11-04

### Added

- A Rust library for parsing existing changelogs into an ordered, serializable
  model, preserving original source text, Markdown, links, and source locations.
- Support for Keep a Changelog with component categories, Conventional
  Changelog, Generic Markdown, GitHub release notes, Changesets, Towncrier,
  Debian changelogs, GNU NEWS, and GNU ChangeLog. Changesets and Towncrier
  accept histories and fragments.
- `parse`, `validate`, and `detect` commands with conservative format detection.
  Commands read `CHANGELOG.md` by default and accept explicit paths or stdin.
- `--strict` for `parse` and `validate` to fail on validation warnings in CI.
- JSON, compact JSON, YAML, TOML, JSONL, CSV, text, and Markdown output.
- Filters for release identifiers, categories, components, packages, pending
  changes, and release counts. Published releases can be selected by newest date
  or by file order.
- Direct file output with atomic replacement. Parsing, validation, and rendering
  finish before writing; `-` selects stdout.
- Colored, format-specific diagnostics with source excerpts and correction
  hints. Repeated warnings share an explanation; filters without matches give a
  short note.
- Input, nesting, metadata expansion, and output limits for bounded processing.
- Example files for every supported input format and focused CLI and library
  docs.
