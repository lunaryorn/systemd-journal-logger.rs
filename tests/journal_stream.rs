// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Test whether connected_to_journal successfully detects a journal
//! connection under a systemd environment.
//!
//! We do this with a small custom executable without the standard
//! test harness which restarts itself in a transient user service
//! with systemd-run, and then calls connected_to_journal and logs
//! the return value to the systemd journal.
//!
//! The main process waits until the transient service finished
//! and then looks for the journal entry and asserts its contents.

#![deny(warnings, clippy::all)]

use std::env::VarError;
use std::process::Command;

use log::{info, LevelFilter};
use pretty_assertions::assert_eq;

mod journal;

fn main() {
    let env_name = "_TEST_LOG_TARGET";

    // On Github Actions use the system instance for this test, because there's
    // no user instance running apparently.
    let use_system_instance = std::env::var_os("GITHUB_ACTIONS").is_some();

    match std::env::var(env_name) {
        Ok(target) => {
            use systemd_journal_logger::*;

            systemd_journal_logger::init().unwrap();
            log::set_max_level(LevelFilter::Debug);

            info!(
                target: &target,
                "connected_to_journal() -> {}",
                connected_to_journal()
            );
        }
        Err(VarError::NotUnicode(value)) => {
            panic!("Value of ${} not unicode: {:?}", env_name, value);
        }
        Err(VarError::NotPresent) => {
            // Restart this binary under systemd-run and then check the journal for the test result
            let exe = std::env::current_exe().unwrap();
            let target = journal::random_target("journal_stream");
            let status = if use_system_instance {
                let mut cmd = Command::new("sudo");
                cmd.arg("systemd-run");
                cmd
            } else {
                let mut cmd = Command::new("systemd-run");
                cmd.arg("--user");
                cmd
            }
            .arg("--description=systemd-journal-logger integration test: journal_stream")
            .arg(format!("--setenv={}={}", env_name, target))
            // Wait until the process exited and unload the entire unit afterwards to
            // leave no state behind
            .arg("--wait")
            .arg("--collect")
            .arg(exe)
            .status()
            .unwrap();

            assert!(status.success());

            let journal = if use_system_instance {
                journal::Journal::System
            } else {
                journal::Journal::User
            };
            let entries = journal::read(
                journal,
                vec![
                    format!("MODULE_PATH={}", module_path!()),
                    format!("TARGET={}", &target),
                ],
            );
            assert_eq!(entries.len(), 1);

            assert_eq!(entries[0]["TARGET"], target);
            assert_eq!(entries[0]["MESSAGE"], "connected_to_journal() -> true");
        }
    }
}
