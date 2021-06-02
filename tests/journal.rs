// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Journal access for integration tests.

use std::collections::HashMap;
use std::process::Command;

use rand::distributions::Alphanumeric;
use rand::Rng;
use std::ffi::OsStr;

/// Generate a random TARGET name with the given `prefix`.
///
/// This allows to identify journal entries uniquely using the
/// `target` log property and the TARGET journal field.
#[allow(dead_code)]
pub fn random_target(prefix: &str) -> String {
    format!(
        "{}-{}",
        prefix,
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect::<String>()
    )
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum Journal {
    User,
    System,
}

/// Read from journal.
///
/// `args` contains additional journalctl arguments such as filters.
pub fn read<I, S>(journal: Journal, args: I) -> Vec<HashMap<String, String>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("journalctl");
    if matches!(journal, Journal::User) {
        command.arg("--user");
    }

    let stdout = String::from_utf8(
        command
            .arg("--output=json")
            .args(args)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

// Read from the journal of the current process.
#[allow(dead_code)]
pub fn read_current_process(module: &str, target: &str) -> Vec<HashMap<String, String>> {
    // Filter by the PID of the current test process and the module path
    read(
        Journal::User,
        vec![
            format!("_PID={}", std::process::id()),
            format!("MODULE_PATH={}", module),
            format!("TARGET={}", target),
        ],
    )
}
