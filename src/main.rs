// Bitcoin Educational Implementation - CLI

use bit_coin::{Cli, CliHandler};
use clap::Parser;

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    // Use data directory in current folder
    let data_dir = "./data";

    let mut handler = match CliHandler::new(data_dir) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error initializing: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = handler.handle(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
