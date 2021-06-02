// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![deny(warnings, clippy::all)]

//! Setup logging in a systemd service.
//!
//! Build this example with cargo build --example systemd_service, then run it directly:
//!
//!     ./target/debug/examples/systemd_service
//!
//! This will print logs to standard error.
//!
//! Then run the example as a systemd user service:
//!
//!     systemd-run --user --wait ./target/debug/examples/systemd_service
//!
//! Now it logs to the systemd journal; you can use journalctl --user -e --output=verbose
//! to inspect it.

use log::{info, set_max_level, LevelFilter};
use simple_logger::SimpleLogger;
use systemd_journal_logger::{connected_to_journal, init_with_extra_fields};

fn main() {
    if connected_to_journal() {
        // If the output streams of this process are directly connected to the
        // systemd journal log directly to the journal to preserve structured
        // log entries (e.g. proper multiline messages, metadata fields, etc.)
        init_with_extra_fields(vec![("VERSION", env!("CARGO_PKG_VERSION"))]).unwrap();
    } else {
        // Otherwise fall back to logging to standard error.
        SimpleLogger::new().init().unwrap();
    }

    set_max_level(LevelFilter::Info);

    info!("Hello\nworld!");
}
