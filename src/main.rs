use std::process;

use clap::Parser;
use picsort::Args;

fn main() {
    let args = Args::parse();

    if let Err(e) = picsort::run(args) {
        println!("Application error: {e}");
        process::exit(1);
    }
}
