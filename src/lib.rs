//! # conservation-lint
//!
//! A cargo-compatible linter that verifies conservation laws across Rust crates at build time.
//!
//! ## What it checks
//!
//! 1. **Sum conservation**: constants with "TOTAL", "SUM", or "WEIGHT" in the name must equal
//!    the sum of their sub-component constants.
//! 2. **Test count assertions**: comments like `// 15 tests` are verified against actual `#[test]` count.
//! 3. **Entropy conservation**: functions that transform probability distributions should preserve
//!    total probability (sum ≈ 1.0).
//!
//! ## Output
//!
//! Outputs findings as JSON in cargo-diagnostic format, compatible with `cargo`'s message pipeline.

pub mod diagnostic;
pub mod extractor;
pub mod checker;

pub use diagnostic::{Diagnostic, DiagnosticLevel, Location, Span};
pub use extractor::{Constant, ExtractedData, SourceExtractor};
pub use checker::{CheckResult, CheckKind, ConservationChecker};

use std::path::Path;

/// Run all conservation checks on a crate directory and return diagnostics.
pub fn lint_crate(crate_dir: &Path) -> Vec<Diagnostic> {
    let extractor = SourceExtractor::new();
    let data = extractor.extract_crate(crate_dir);
    let checker = ConservationChecker::new();
    checker.check(&data)
}

/// Run checks and return JSON string of diagnostics.
pub fn lint_crate_json(crate_dir: &Path) -> String {
    let diagnostics = lint_crate(crate_dir);
    serde_json::to_string_pretty(&diagnostics).unwrap_or_else(|e| {
        serde_json::to_string_pretty(&vec![Diagnostic {
            level: DiagnosticLevel::Error,
            message: format!("Failed to serialize diagnostics: {}", e),
            code: Some("conservation-lint::internal".to_string()),
            location: None,
            rendered: None,
        }]).unwrap()
    })
}
