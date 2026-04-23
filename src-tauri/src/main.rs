// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::{Parser, ValueEnum};

const VERSION: &str = "0.1.0";

#[derive(Parser)]
#[command(name = "cerbo-desktop")]
#[command(version = VERSION)]
#[command(about = "Local-first markdown wiki desktop app")]
struct Args {
    #[arg(short, long, help = "Show verbose debug info")]
    debug: bool,
}

fn main() {
    let args = Args::parse();

    if args.debug {
        println!("Debug mode enabled");
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    cerbo_lib::run()
}
