use daily_logger::init_logger;
use log::info;
use log::debug;
use log::error;
use log::warn;
use log::trace;

fn main() {
    init_logger(log::LevelFilter::Trace, log::LevelFilter::Info, "/home/kx/Dev/log_test1");

    let uuid = "222-444-555-666";
    info!(target: "vending", uuid = uuid; "order specific log 1");
    info!(target: "vending", uuid = uuid; "order specific log 2");
    info!(target: "ui", "generic log 1");
    info!(target: "ui", "generic log 2");
    debug!("random debug msg");
    warn!(target: "ui", "warning ui");
    warn!(target: "vending", "warning Vending");
    error!(target: "vending", uuid = uuid; "order specific error 1");
    trace!(target: "random", "trace msgs 1");
    trace!(target: "random", uuid = uuid; "trace msgs 2");
}