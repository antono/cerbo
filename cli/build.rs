use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let cli_dir = PathBuf::from(&manifest_dir);
    let md_path = cli_dir.join("man").join("cerbo.md");

    // Try to convert Markdown to troff using mandown CLI
    // If mandown is not available, skip man page generation (non-fatal)
    match Command::new("mandown").arg(&md_path).output() {
        Ok(output) if output.status.success() => {
            let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
            let output_path = PathBuf::from(&out_dir).join("cerbo.1");

            // Get troff output, add .nh (disable hyphenation)
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let mut troff = String::from(".nh\n");
            troff.push_str(&stdout_str);

            // Fix literal dots at start of lines in no-fill mode
            // In troff, \& before . prevents macro interpretation
            // We need to escape: .vault-path/, .cerbo/, and similar literal dots
            let result = troff
                .replace("\n.vault-path/", "\n\\&.vault-path/")
                .replace("\n  .cerbo/", "\n  \\&.cerbo/");

            fs::write(&output_path, result)
                .unwrap_or_else(|e| panic!("Failed to write {}: {}", output_path.display(), e));
        }
        Ok(output) => {
            println!(
                "cargo:warning=mandown failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            println!("cargo:warning=Skipping man page generation");
        }
        Err(_) => {
            println!("cargo:warning=mandown not found. Install with: cargo install mandown");
            println!("cargo:warning=Skipping man page generation");
        }
    }

    println!("cargo:rerun-if-changed={}", md_path.display());
}
