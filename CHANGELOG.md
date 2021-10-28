# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Run [`cargo-release`][cr] to publish a release.

[cr]: https://github.com/sunng87/cargo-release/

## [Unreleased]

### Changed
- Update to `libsystemd` 0.4.0, and reexport `connected_to_journal()` from that crate.

### Removed
- Dependencies on `libc` and `nix`.

## [0.3.1] – 2021-10-06

### Fixed
- Compile on arm7 targets (see [GH-7]).

[GH-7]: https://github.com/lunaryorn/systemd-journal-logger.rs/pull/7

## [0.3.0] – 2021-06-07

### Added
- Add `JournalLog::with_extra_fields` and `init_with_extra_fields` to add custom fields to every log entry (see [GH-3]).
- Add new `journal_send` to send an individual log record directly to the systemd journal.
- Add new `connected_to_journal` function to check for a direct connection to the systemd log in order to automatically upgrade to journal logging in systemd services (see [GH-5]).
- Add record key values as custom journal fields (see [GH-1]).

### Changed
- Do not silently ignore journal errors; instead panic if the logger fails to send a message to the systemd journal.
- Use `CODE_MODULE` field for the Rust module path, for compatibility with the `slog-journal` and `systemd` crates.

[GH-1]: https://github.com/lunaryorn/systemd-journal-logger.rs/pull/1
[GH-3]: https://github.com/lunaryorn/systemd-journal-logger.rs/pull/3
[GH-5]: https://github.com/lunaryorn/systemd-journal-logger.rs/pull/5

## [0.2.0] – 2021-06-01

### Fixed

- Multiline messages are no longer lost (see [GH-2]), following an update of [libsystemd] (see [libsystemd GH-70] and [libsystemd GH-72]).

[GH-2]: https://github.com/lunaryorn/systemd-journal-logger.rs/pull/2
[libsystemd]: https://github.com/lucab/libsystemd-rs
[libsystemd GH-70]: https://github.com/lucab/libsystemd-rs/issues/70
[libsystemd GH-72]: https://github.com/lucab/libsystemd-rs/pull/72

## [0.1.0] – 2021-05-28 (yanked)

Initial release with `systemd_journal_logger::LOG` and `systemd_journal_logger::init`.

Do **not use** this version; it looses multiline messages and has been yanked from crates.io.

[Unreleased]: https://github.com/lunaryorn/systemd-journal-logger.rs/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/lunaryorn/systemd-journal-logger.rs/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/lunaryorn/systemd-journal-logger.rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/lunaryorn/systemd-journal-logger.rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/lunaryorn/systemd-journal-logger.rs/releases/tag/v0.1.0
