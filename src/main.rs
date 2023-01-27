#[macro_use]
extern crate log;

use std::process;

use clap::Parser;
use env_logger::Env;
use picsort::Args;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    info!("Application started with args: {:?}", args);

    if let Err(e) = picsort::run(args) {
        error!("Application error: {e}");
        process::exit(1);
    }
}
