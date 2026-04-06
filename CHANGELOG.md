# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-05

### Changed
- Dependencies have been updated.
- The command-line parsing code has been modernized.

## [0.0.8] - 2026-03-29

### Changed
- The prompt symbol is now colored.

## [0.0.7] - 2026-03-29

### Changed
- Shell Agent is now aware of its own errors (e.g., context window exceeded, network issues, etc.).

## [0.0.6] - 2026-03-29

### Changed
- Error handling has been improved.
- Shell Agent no longer relies on OpenAI storing the conversation state.

## [0.0.5] - 2026-03-23

### Changed
- A more helpful error is given when the `OPENAI_API_KEY` environment variable is not set.

## [0.0.4] - 2026-03-23

### Changed
- Shell Agent now allows shell invocations to be interrupted with CTRL-C.
- CTRL-C no longer exits the main loop (but CTRL-D still does).
- Shell Agent now hides the `OPENAI_API_KEY` env var from child processes.
- Child processes no longer inherit STDIN.

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
