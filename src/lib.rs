// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![deny(warnings, missing_docs, clippy::all)]

//! A [log] logger for the [systemd journal][1].
//!
//! [log]: https://docs.rs/log
//! [1]: https://www.freedesktop.org/software/systemd/man/journalctl.html
//!
//! # Usage
//!
//! Use [`init`] at the beginning of your `main` function to setup journal
//! logging, configure the logging level and then use the standard macros
//! from the [`log`] crate to send log messages to the systemd journal:
//!
//! ```edition2018
//! use log::{info, warn, error, LevelFilter};
//!
//! # fn main(){
//! systemd_journal_logger::init().unwrap();
//! log::set_max_level(LevelFilter::Info);
//!
//! info!("hello log");
//! warn!("warning");
//! error!("oops");
//! # }
//! ```
//!
//! In a service you can use [`connected_to_journal`] to check whether
//! the standard output or error stream of the current process is directly
//! connected to the systemd journal (the default for services started by
//! systemd) and fall back to logging to standard error if that's not the
//! case.  Take a look at the [systemd_service.rs] example for details.
//!
//! [systemd_service.rs]: https://github.com/lunaryorn/systemd-journal-logger.rs/blob/main/examples/systemd_service.rs
//!
//! # Journal fields
//!
//! The journald logger always sets the following standard [journal fields][2]:
//!
//! - `PRIORITY`: The log level mapped to a priority (see below).
//! - `MESSAGE`: The formatted log message (see [`log::Record::args()`]).
//! - `SYSLOG_IDENTIFIER`: The short name of the running process, i.e. the file name of [`std::env::current_exe()`] if successful.
//! - `SYSLOG_PID`: The PID of the running process (see [`std::process::id()`]).
//! - `CODE_FILE`: The filename the log message originates from (see [`log::Record::file()`], only if present).
//! - `CODE_LINE`: The line number the log message originates from (see [`log::Record::line()`], only if present).
//!
//! Additionally it also adds the following non-standard fields:
//!
//! - `TARGET`: The target of the log record (see [`log::Record::target()`]).
//! - `CODE_MODULE`: The module path of the log record (see [`log::Record::module_path()`], only if present).
//!
//! In addition to these fields the logger also adds all structures key-values (see [`log::Record::key_values`])
//! from each log record as journal fields, after converting the keys to uppercase letters and replacing invalid
//! characters with underscores.
//!
//! You can also add custom fields to every log entry with [`JournalLog::with_extra_fields`] and [`init_with_extra_fields`]:
//!
//! ```edition2018
//! use log::{info, warn, error, LevelFilter};
//!
//! # fn main(){
//! systemd_journal_logger::init_with_extra_fields(vec![("VERSION", env!("CARGO_PKG_VERSION"))]).unwrap();
//! log::set_max_level(LevelFilter::Info);
//!
//! info!("this message has an extra VERSION field in the journal");
//! # }
//! ```
//!
//! You can display these extra fields with `journalctl --output=verbose` and extract them with any of the structured
//! output formats of `journalctl`, e.g. `journalctl --output=json`.
//!
//! [2]: https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html
//!
//! # Log levels and priorities
//!
//! The journal logger maps [`log::Level`] to journal priorities as follows:
//!
//! - [`Level::Error`] → [`libsystemd::logging::Priority::Error`]
//! - [`Level::Warn`] → [`libsystemd::logging::Priority::Warning`]
//! - [`Level::Info`] → [`libsystemd::logging::Priority::Notice`]
//! - [`Level::Debug`] → [`libsystemd::logging::Priority::Info`]
//! - [`Level::Trace`] → [`libsystemd::logging::Priority::Debug`]
//!
//! # Errors
//!
//! This crate currently does not implement any kind of error handling for journal messages.
//! In particular it will **panic** when sending a record to journald fails, e.g. if journald is
//! not running.  There are no plans to change this behaviour, give that journald is reliable in
//! general.
//!
//! To implement an alternative error handling behaviour define a custom log implementation around
//! [`journal_send`] which sends a single log record to the journal.

use std::borrow::Cow;
use std::os::unix::io::{AsRawFd, RawFd};
use std::str::FromStr;

use libc::{dev_t, ino_t};
use libsystemd::errors::SdError;
use libsystemd::logging::Priority;
use log::kv::{Error, Key, Value, Visitor};
use log::{Level, Log, Metadata, Record, SetLoggerError};

/// A systemd journal logger.
pub struct JournalLog<K, V> {
    /// Extra fields to add to every logged message.
    extra_fields: Vec<(K, V)>,
}

/// Convert a a log level to a journal priority.
fn level_to_priority(level: Level) -> Priority {
    match level {
        Level::Error => Priority::Error,
        Level::Warn => Priority::Warning,
        Level::Info => Priority::Notice,
        Level::Debug => Priority::Info,
        Level::Trace => Priority::Debug,
    }
}

/// Collect the standard journal fields from `record`.
///
/// Sets
///
/// - the standard `SYSLOG_IDENTIFIER` and `SYSLOG_PID` fields to the short name (if it exists)
///   and the PID of the current process.
/// - the standard `CODE_FILE` and `CODE_LINE` fields to the file and line of `record`,
/// - the custom `TARGET` field to `record.target`, and
/// - the custom `CODE_MODULE` to the module path of `record`.
fn standard_fields<'a>(record: &'a Record) -> Vec<(Cow<'a, str>, Cow<'a, str>)> {
    let mut fields = Vec::with_capacity(6);

    if let Some(short_name) = std::env::current_exe()
        .ok()
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
    {
        fields.push(("SYSLOG_IDENTIFIER".into(), short_name.into()));
    }
    fields.push(("SYSLOG_PID".into(), std::process::id().to_string().into()));

    if let Some(file) = record.file() {
        fields.push(("CODE_FILE".into(), file.into()));
    }
    if let Some(line) = record.line().map(|l| l.to_string()) {
        fields.push(("CODE_LINE".into(), line.into()));
    }
    // Non-standard fields
    fields.push(("TARGET".into(), record.target().into()));
    if let Some(module) = record.module_path() {
        fields.push(("CODE_MODULE".into(), module.into()))
    }
    fields
}

/// Whether `c` is a valid character in the key of a journal field.
///
/// Journal field keys may only contain ASCII uppercase letters A to Z,
/// numbers 0 to 9 and the underscore.
pub fn is_valid_key_char(c: char) -> bool {
    matches!(c, 'A'..='Z' | '0'..='9' | '_')
}

/// Escape a `key` for use in a systemd journal field.
///
/// Journal keys must only contain ASCII uppercase letters, numbers and
/// the underscore, and the must not start with an underscore.
///
/// If all characters in `key` are valid (see [`is_valid_key_char`])
/// return `key`, otherwise transform `key` to a ASCII uppercase, and
/// replace all invalid characters with an underscore.
///
/// If the resulting value starts with an underscore prepend the
/// prefix "UNDERSCORE" to the result.
pub fn escape_journal_key(key: &str) -> Cow<str> {
    if key.chars().all(is_valid_key_char) {
        Cow::Borrowed(key)
    } else {
        let escaped = key
            .to_ascii_uppercase()
            .replace(|c| !is_valid_key_char(c), "_");
        if escaped.starts_with('_') {
            format!("UNDERSCORE{}", escaped).into()
        } else {
            escaped.into()
        }
    }
}

struct CollectKeyValues<'a, 'kvs>(&'a mut Vec<(Key<'kvs>, Value<'kvs>)>);

impl<'a, 'kvs> Visitor<'kvs> for CollectKeyValues<'a, 'kvs> {
    fn visit_pair(&mut self, key: Key<'kvs>, value: Value<'kvs>) -> Result<(), Error> {
        self.0.push((key, value));
        Ok(())
    }
}

/// Send a single log record to the journal.
///
/// Extract the standard fields from `record` (see [`crate`] documentation),
/// and append all `extra_fields`; the send the resulting fields to the
/// systemd journal with [`libsystemd::logging::journal_send`] and return
/// the result of that function.
pub fn journal_send<'a, K, V>(
    record: &Record,
    extra_fields: impl Iterator<Item = &'a (K, V)>,
) -> Result<(), SdError>
where
    K: AsRef<str> + 'a,
    V: AsRef<str> + 'a,
{
    let mut record_fields = Vec::with_capacity(record.key_values().count());
    // Our visitor nevers fails so we can safely unwrap here
    record
        .key_values()
        .visit(&mut CollectKeyValues(&mut record_fields))
        .unwrap();
    libsystemd::logging::journal_send(
        level_to_priority(record.level()),
        &format!("{}", record.args()),
        standard_fields(record)
            .into_iter()
            .chain(extra_fields.map(|(k, v)| (escape_journal_key(k.as_ref()), v.as_ref().into())))
            .chain(
                record_fields
                    .iter()
                    .map(|(k, v)| (escape_journal_key(k.as_ref()), format!("{}", v).into())),
            ),
    )
}

impl<K, V> JournalLog<K, V>
where
    K: AsRef<str> + Sync + Send,
    V: AsRef<str> + Sync + Send,
{
    /// Create a logger which adds extra fields to every log entry.
    ///
    /// # Journal fields
    ///
    /// `extra_fields` is a sequence of key value pairs to add as extra fields
    /// to every log entry logged through the new logger.  The extra fields will
    /// be *appended* to the standard journal fields written by the logger.
    ///
    /// ## Restrictions on field names
    ///
    /// Each key in the sequence must be a valid journal field name, i.e.
    /// contain only uppercase alphanumeric characters and the underscore, and
    /// it must not start with an underscore.
    ///
    /// Invalid keys in `extra_fields` are escaped with [`escape_journal_key`],
    /// which transforms them to ASCII uppercase and replaces all invalid
    /// characters with underscores.
    ///
    /// `extra_fields` should **not** contain any of the journal fields already
    /// added by this logger; while journald supports multiple values for a field
    /// journald clients may not handle unexpected multi-value fields properly and
    /// likely show only the first value.  Specifically even `journalctl` will only
    /// shouw the first `MESSAGE` value of journal entries.
    ///
    /// See the [`crate`] documentation at [`crate`] for details about the standard
    /// journal fields this logger uses.
    ///
    /// ## Restrictions on values
    ///
    /// There are no restrictions on the value.
    pub fn with_extra_fields(extra_fields: Vec<(K, V)>) -> Self {
        Self { extra_fields }
    }
}

/// The [`Log`] interface for [`JournalLog`].
impl<K, V> Log for JournalLog<K, V>
where
    K: AsRef<str> + Sync + Send,
    V: AsRef<str> + Sync + Send,
{
    /// Whether this logger is enabled.
    ///
    /// Always returns `true`.
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    /// Send the given `record` to the systemd journal.
    ///
    /// # Errors
    ///
    /// **Panic** if sending the `record` to journald fails,
    /// i.e. if journald is not running.
    fn log(&self, record: &Record) {
        journal_send(record, self.extra_fields.iter()).unwrap();
    }

    /// Flush log records.
    ///
    /// A no-op for journal logging.
    fn flush(&self) {}
}

/// The static instance of systemd journal logger.
pub static LOG: JournalLog<&'static str, &'static str> = JournalLog {
    extra_fields: vec![],
};

fn fd_has_device_and_inode(fd: RawFd, device: dev_t, inode: ino_t) -> bool {
    nix::sys::stat::fstat(fd).map_or(false, |stat| stat.st_dev == device && stat.st_ino == inode)
}

/// Whether this process is directly connected to the journal.
///
/// Inspects the `$JOURNAL_STREAM` environment variable and compares the device and inode
/// numbers in this variable against the stdout and stderr file descriptors.
///
/// Return `true` if either stream matches the device and inode numbers in `$JOURNAL_STREAM`,
/// and `false` otherwise (or in case of any IO error).
///
/// Systemd sets `$JOURNAL_STREAM` to the device and inode numbers of the standard output
/// or standard error streams of the current process if either of these streams is connected
/// to the systemd journal.
///
/// Systemd explicitly recommends that services check this variable to upgrade their logging
/// to the native systemd journal protocol.
///
/// See section “Environment Variables Set or Propagated by the Service Manager” in
/// [systemd.exec(5)][1] for more information.
///
/// [1]: https://www.freedesktop.org/software/systemd/man/systemd.exec.html#Environment%20Variables%20Set%20or%20Propagated%20by%20the%20Service%20Manager
pub fn connected_to_journal() -> bool {
    std::env::var_os("JOURNAL_STREAM")
        .as_ref()
        .and_then(|value| value.to_str())
        .and_then(|value| value.split_once(':'))
        .and_then(|(device, inode)| u64::from_str(device).ok().zip(u64::from_str(inode).ok()))
        .map_or(false, |(device, inode)| {
            fd_has_device_and_inode(std::io::stderr().as_raw_fd(), device, inode)
                || fd_has_device_and_inode(std::io::stdout().as_raw_fd(), device, inode)
        })
}

/// Initialize journal logging.
///
/// Set [`LOG`] as logger with [`log::set_logger`].
///
/// This function can only be called once during the lifetime of a program.
///
/// # Default level
///
/// This function does not configure any default level for
/// [`log`]; you need to configure the maximum level explicitly with
/// [`log::set_max_level`], as [`log`] normally defaults to
/// [`log::LevelFilter::Off`] which disables all logging.
///
/// # Errors
///
/// This function returns an error if any logger was previously registered.
pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOG)
}

/// Initialize journal logging with extra journal fields.
///
/// Create a new [`JournalLog`] with the given fields and set it as logger
/// with [`log::set_boxed_logger`].
///
/// This function can only be called once during the lifetime of a program.
///
/// See [`init`] for more information about log levels and error behaviour,
/// and [`JournalLog::with_extra_fields`] for details about `extra_fields`.
pub fn init_with_extra_fields<K, V>(extra_fields: Vec<(K, V)>) -> Result<(), SetLoggerError>
where
    K: AsRef<str> + Sync + Send + 'static,
    V: AsRef<str> + Sync + Send + 'static,
{
    log::set_boxed_logger(Box::new(JournalLog::with_extra_fields(extra_fields)))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use libsystemd::logging::Priority;
    use log::{Level, Record};
    use pretty_assertions::assert_eq;

    #[test]
    fn level_to_priority() {
        assert!(matches!(
            crate::level_to_priority(Level::Error),
            Priority::Error
        ));
        assert!(matches!(
            crate::level_to_priority(Level::Warn),
            Priority::Warning
        ));
        assert!(matches!(
            crate::level_to_priority(Level::Info),
            Priority::Notice
        ));
        assert!(matches!(
            crate::level_to_priority(Level::Debug),
            Priority::Info
        ));
        assert!(matches!(
            crate::level_to_priority(Level::Trace),
            Priority::Debug
        ));
    }

    #[test]
    fn is_valid_key_char() {
        for c in 'A'..='Z' {
            assert!(super::is_valid_key_char(c));
        }
        for c in '0'..='9' {
            assert!(super::is_valid_key_char(c));
        }
        assert!(super::is_valid_key_char('_'));

        for c in 'a'..='z' {
            assert!(!super::is_valid_key_char(c));
        }
        assert!(!super::is_valid_key_char('ö'));
        assert!(!super::is_valid_key_char('ß'));
    }

    #[test]
    fn escape_journal_key() {
        for case in &["FOO", "123_FOO_BAR", "123"] {
            assert_eq!(super::escape_journal_key(case), *case);
            assert!(matches!(super::escape_journal_key(case), Cow::Borrowed(_)));
        }

        let cases = vec![
            ("foo", "FOO"),
            ("_foo", "UNDERSCORE_FOO"),
            ("Hallöchen", "HALL_CHEN"),
            ("öß", "UNDERSCORE__"),
        ];
        for (key, expected) in cases {
            assert_eq!(super::escape_journal_key(key), expected);
        }
    }

    #[test]
    fn standard_fields() {
        let record = Record::builder()
            .level(Level::Warn)
            .line(Some(10))
            .file(Some("foo.rs"))
            .target("testlog")
            .module_path(Some(module_path!()))
            .build();
        let fields = super::standard_fields(&record);

        assert_eq!(fields[0].0, "SYSLOG_IDENTIFIER");
        assert!(
            (&fields[0].1).contains("systemd_journal_logger"),
            "SYSLOG_IDENTIFIER={}",
            fields[0].1
        );

        assert_eq!(fields[1].0, "SYSLOG_PID");
        assert_eq!((&fields[1].1).as_ref(), std::process::id().to_string());

        assert_eq!(
            fields[2..],
            vec![
                (Cow::Borrowed("CODE_FILE"), Cow::Borrowed("foo.rs")),
                (Cow::Borrowed("CODE_LINE"), Cow::Borrowed("10")),
                (Cow::Borrowed("TARGET"), Cow::Borrowed("testlog")),
                (
                    Cow::Borrowed("CODE_MODULE"),
                    Cow::Borrowed("systemd_journal_logger::tests")
                )
            ]
        )
    }
}
