# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.3] - 2026-03-22

### Changed
- Shell Agent now indicates when compaction has occurred.
- Shell Agent now flushes output more often to make streaming more useful.
- The default compaction threshold has been increased to 200,000 tokens (a reasonable threshold for the default model, `gpt-5.2`).

## [0.0.2] - 2026-03-22

### Added
- Added a configurable context compaction threshold.

## [0.0.1] - 2026-03-03

### Added
- Added the only tool: `run_shell_command`.

## [0.0.0] - 2026-03-02

### Added
- Initial release.
