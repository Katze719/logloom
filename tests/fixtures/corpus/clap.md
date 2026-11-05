# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/)
and this project adheres to [Semantic Versioning](https://semver.org/).

## 5.0.0 - TBD

*available through `unstable-v5` feature flag*

### Breaking Changes

- Made `ArgPredicate` `non_exhaustive`
- *(help)* Change default `Command::term_width` to "source format"
- *(help)* Change default `Command::max_term_width` to 100
- *(derive)* `Vec<Vec<T>>` types are now assuming to capture occurrences
- *(derive)* `ValueEnum` variants now use the full doc comment, not summary, for `PossibleValue::help`

### Features

- *(derive)* Group values by their occurrence with `Vec<Vec<T>>`

<!-- next-header -->
## [Unreleased] - ReleaseDate

## [4.5.53] - 2025-11-19

### Features

- Add `default_values_if`, `default_values_ifs`

## [4.5.52] - 2025-11-17

### Fixes

- Don't panic when `args_conflicts_with_subcommands` conflicts with an `ArgGroup`

