// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

const VERSION: &str = "0.1.0";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version") {
        println!("cerbo-desktop {}", VERSION);
        std::process::exit(0);
    }

    cerbo_lib::run()
}
