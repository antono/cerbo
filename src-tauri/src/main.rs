// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;

const VERSION: &str = "0.1.0";

#[derive(Parser)]
#[command(name = "cerbo-desktop")]
#[command(version = VERSION)]
#[command(about = "Local-first markdown wiki desktop app")]
struct Args {
    #[arg(short, long, help = "Show verbose debug info")]
    debug: bool,
    #[arg(short, long, help = "Show configuration and vault info")]
    info: bool,
}

fn main() {
    let args = Args::parse();

    if args.info {
        use cerbo_core::context::CoreContext;
        use cerbo_core::vault;
        use cerbo_core::CerboContext;

        fn display_path(p: &std::path::PathBuf) -> String {
            if let Ok(home) = std::env::var("HOME") {
                if let Some(rest) = p.to_str().and_then(|s| s.strip_prefix(&home)) {
                    return format!("~{}", rest);
                }
            }
            p.to_string_lossy().to_string()
        }

        match CoreContext::new() {
            Ok(core) => {
                let ctx = CerboContext {
                    config_dir: core.config_dir,
                    cache_dir: core.cache_dir,
                };
                println!("Config:  {}", display_path(&ctx.config_dir));
                println!("Cache:   {}", display_path(&ctx.cache_dir));
                println!();

                match vault::vault_list(&ctx) {
                    Ok(reg) => {
                        if reg.vaults.is_empty() {
                            println!("No vaults registered");
                        } else {
                            println!("Vaults: {} registered", reg.vaults.len());
                            for v in &reg.vaults {
                                match vault::vault_page_count(&ctx, &v.id) {
                                    Ok(count) => println!(
                                        "├── {} ({}) - {} pages",
                                        v.name,
                                        display_path(&v.path),
                                        count
                                    ),
                                    Err(_) => {
                                        println!("├── {} ({})", v.name, display_path(&v.path))
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("Error loading vaults: {}", e),
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if args.debug {
        println!("Debug mode enabled");
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    cerbo_lib::run()
}
