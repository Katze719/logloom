# Changelog

All notable changes to logloom are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- CI artifacts for Windows x64: a standalone executable and MSI installer.
- Static musl binaries, tarballs, DEB packages, and RPM packages for Linux x64
  and ARM64, with SHA-256 checksums and package checks.

### Changed

- Omit the original `source` copy from CLI parse exports in every output format.
- Document crates.io installation and the published Rust library dependency.

### Fixed

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
