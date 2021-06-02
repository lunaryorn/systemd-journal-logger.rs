// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use log::info;

mod journal;

use pretty_assertions::assert_eq;

#[test]
fn init() {
    systemd_journal_logger::init().unwrap();
    log::set_max_level(log::LevelFilter::Info);

    let target = journal::random_target("init");

    info!(target: &target, "Hello World");

    let entries = journal::read_current_process(module_path!(), &target);
    assert_eq!(entries.len(), 1);

    assert_eq!(entries[0]["TARGET"], target);
    assert_eq!(entries[0]["MESSAGE"], "Hello World");
}
