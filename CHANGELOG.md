# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.0](https://github.com/Syrup/diceng/compare/v1.0.2...v2.0.0) - 2026-06-20

### Added

- add #[non_exhaustive] to DieEntryKind enum
- improve verbose display and fix dice collection

### Fixed

- *(ci)* remove release_commits filter that blocks manual trigger
- *(ci)* use GH_PAT for release-plz to trigger downstream workflows

### Other

- *(ci)* manual trigger for release-plz PR creation
- *(ci)* filter release commits to feat/fix/perf only
- *(release)* add workflow_dispatch trigger for manual runs

## [1.0.2](https://github.com/Syrup/diceng/compare/v1.0.1...v1.0.2) - 2026-06-17

### Fixed

- *(ci)* correct release-plz action repo name
- *(parser)* reroll notation bugs

### Other

- *(release)* integrate release-plz for automated releases
