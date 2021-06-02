# systemd-journal-logger

A [log] logger for the [systemd journal][1].

[log]: https://docs.rs/log
[1]: https://www.freedesktop.org/software/systemd/man/systemd-journald.service.html

## Usage

```toml
[dependencies]
log = "^0.4"
systemd-journal-logger = "^0.2"
```

Then initialize the logger at the start of `main`:

```rust
use log::{info, LevelFilter};

fn main() {
    systemd_journal_logger::init();
    log::set_max_level(LevelFilter::Info);

    info!("Hello systemd journal");
}
```

You can also add additional fields to every log message, such as the version of your executable:

```rust
use log::{info, LevelFilter};

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    systemd_journal_logger::init_with_extra_fields(vec![("VERSION", version)]).unwrap();
    log::set_max_level(LevelFilter::Info);

    info!("Hello systemd journal");
}
```

These extra fields appear in the output of `journalctl --output=verbose` or in any of the JSON output formats of `journalctl`.

See [systemd_service.rs](./examples/systemd_service.rs) for a simple example of logging in a systemd service which automatically falls back to a different logger if not started through systemd.

## Related projects

- [rust-systemd](https://github.com/jmesmon/rust-systemd) provides a [logger implementation][1] based on the `libsystemd` C library.
- [slog-journald](https://github.com/slog-rs/journald) provides an [slog] logger for the systemd journal, also based on the `libsystemd` C library.

Both loggers use mostly the same fields and priorities as this implementation.

[1]: https://docs.rs/systemd/0.8.2/systemd/journal/struct.JournalLog.html
[slog]: https://github.com/slog-rs/slog

## License

Either [MIT](./LICENSE-MIT) or [Apache 2.0](./LICENSE-APACHE-2.0), at your option.