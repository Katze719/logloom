# Changelog

All notable changes to Lantern are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- A streaming export API.
- A command-line preview with **Markdown** support.

## [2.1.0] - 2026-10-14

### Changed

- Preserve the order of imported records.

### Deprecated

- The `legacy_mode` configuration key.

### Removed

- The obsolete compatibility mode.

### Fixed

- Handle Unicode paths when importing files.
  - Preserve accented characters.
  - Support filenames containing emoji.

### Security

- Reject oversized network frames.

## [2.0.0] - 2026-09-01 [YANKED]

### Added

- Initial public API. See the [documentation][docs] for usage examples.

## [1.0.0] - 2026-08-01

### Firmware components

#### Added

- Initial firmware components for Lantern.

#### Changed

- Initial changes to firmware components for Lantern.

[Unreleased]: https://example.com/compare/v2.1.0...HEAD
[2.1.0]: https://example.com/compare/v2.0.0...v2.1.0
[2.0.0]: https://example.com/releases/v2.0.0
[docs]: https://example.com/docs
