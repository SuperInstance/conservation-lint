//! Binary entry point for conservation-lint.

use clap::Parser;
use conservation_lint::{lint_crate, lint_crate_json, DiagnosticLevel};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "conservation-lint")]
#[command(about = "Verify conservation laws across Rust crates at build time")]
struct Cli {
    /// Path to the crate directory to lint
    crate_dir: PathBuf,

    /// Output as JSON (cargo-diagnostic format)
    #[arg(long, short = 'j')]
    json: bool,

    /// Only show errors and warnings (not notes)
    #[arg(long, short = 'q')]
    quiet: bool,
}

fn main() {
    let cli = Cli::parse();

    if !cli.crate_dir.exists() {
        eprintln!("Error: directory {:?} does not exist", cli.crate_dir);
        std::process::exit(1);
    }

    if !cli.crate_dir.join("Cargo.toml").exists() {
        eprintln!("Warning: {:?} does not appear to be a Rust crate (no Cargo.toml)", cli.crate_dir);
    }

    if cli.json {
        println!("{}", lint_crate_json(&cli.crate_dir));
    } else {
        let diagnostics = lint_crate(&cli.crate_dir);

        if diagnostics.is_empty() {
            println!("conservation-lint: No issues found. Crate appears to conserve.");
            return;
        }

        let mut errors = 0;
        let mut warnings = 0;

        for diag in &diagnostics {
            if cli.quiet && diag.level == DiagnosticLevel::Note {
                continue;
            }

            let level_str = match diag.level {
                DiagnosticLevel::Error => "ERROR",
                DiagnosticLevel::Warning => "WARN",
                DiagnosticLevel::Note => "NOTE",
            };

            let code = diag.code.as_deref().unwrap_or("conservation-lint");
            let loc = diag.location.as_ref().map(|l| {
                format!("{}:{}:{}", l.file.display(), l.line, l.column)
            }).unwrap_or_else(|| "<no location>".to_string());

            println!("[{}] [{}] {}: {}", level_str, code, loc, diag.message);

            if let Some(ref rendered) = diag.rendered {
                println!("  → {}", rendered);
            }

            match diag.level {
                DiagnosticLevel::Error => errors += 1,
                DiagnosticLevel::Warning => warnings += 1,
                DiagnosticLevel::Note => {}
            }
        }

        println!("\n--- Summary ---");
        println!("Errors: {} | Warnings: {} | Total: {}", errors, warnings, diagnostics.len());

        if errors > 0 {
            std::process::exit(1);
        }
    }
}
