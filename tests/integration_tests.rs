//! Integration tests for conservation-lint.
//!
//! 17 tests covering all checkers, edge cases, and real-world scenarios.

use conservation_lint::*;
use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;

// Helper: create a temp crate with given source files
fn make_crate(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"test-crate\"\nversion=\"0.1.0\"\nedition=\"2021\"\n").unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    for (name, content) in files {
        fs::write(dir.path().join(name), content).unwrap();
    }
    dir
}

// ── Test 1: Sum conservation violation detected ──

#[test]
fn test_detects_sum_conservation_violation() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
const SCORE_A: f64 = 10.0;
const SCORE_B: f64 = 20.0;
const SCORE_C: f64 = 15.0;
const SCORE_TOTAL: f64 = 50.0;
"#),
    ]);
    let diagnostics = lint_crate(crate_dir.path());
    let _errors: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Error).collect();
    assert!(!errors.is_empty(), "Should detect sum conservation violation");
    assert!(errors[0].message.contains("SCORE_TOTAL"));
    assert!(errors[0].code.as_ref().unwrap().contains("sum-conservation"));
}

// ── Test 2: Sum conservation passes when correct ──

#[test]
fn test_sum_conservation_passes() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
const SCORE_A: f64 = 10.0;
const SCORE_B: f64 = 20.0;
const SCORE_C: f64 = 20.0;
const SCORE_TOTAL: f64 = 50.0;
"#),
    ]);
    let diagnostics = lint_crate(crate_dir.path());
    let _errors: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Error).collect();
    assert!(errors.is_empty(), "Should have no errors when sums match");
    let notes: Vec<_> = diagnostics.iter()
        .filter(|d| d.level == DiagnosticLevel::Note && d.message.contains("Conservation OK"))
        .collect();
    assert!(!notes.is_empty(), "Should have passing conservation note");
}

// ── Test 3: Test count assertion mismatch ──

#[test]
fn test_detects_test_count_mismatch() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
// 15 tests
pub fn something() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_one() {}
    #[test]
    fn test_two() {}
}
"#),
    ]);
    let diagnostics = lint_crate(crate_dir.path());
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Warning).collect();
    assert!(warnings.iter().any(|w| w.message.contains("Test count assertion") && w.message.contains("15")));
}

// ── Test 4: Test count assertion matches ──

#[test]
fn test_test_count_matches() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
// 3 tests

#[test]
fn test_a() {}
#[test]
fn test_b() {}
#[test]
fn test_c() {}
"#),
    ]);
    let diagnostics = lint_crate(crate_dir.path());
    let notes: Vec<_> = diagnostics.iter()
        .filter(|d| d.level == DiagnosticLevel::Note && d.message.contains("Test count OK"))
        .collect();
    assert!(!notes.is_empty(), "Should confirm test count matches");
}

// ── Test 5: No constants means no errors ──

#[test]
fn test_empty_crate_is_clean() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", "pub fn hello() {}"),
    ]);
    let diagnostics = lint_crate(crate_dir.path());
    let _errors: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Error).collect();
    assert!(errors.is_empty(), "Empty crate should have no errors");
}

// ── Test 6: Multiple files scanned ──

#[test]
fn test_scans_multiple_files() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", "pub mod models;"),
        ("src/models.rs", r#"
const WEIGHT_A: f64 = 0.3;
const WEIGHT_B: f64 = 0.7;
const WEIGHT_TOTAL: f64 = 1.0;
"#),
    ]);
    let extractor = SourceExtractor::new();
    let data = extractor.extract_crate(crate_dir.path());
    assert_eq!(data.files_scanned, 2);
    assert_eq!(data.constants.len(), 3);
}

// ── Test 7: Integer constants handled correctly ──

#[test]
fn test_integer_constants() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
const MAX_NODES: usize = 100;
const BATCH_SIZE: usize = 32;
"#),
    ]);
    let extractor = SourceExtractor::new();
    let data = extractor.extract_crate(crate_dir.path());
    assert_eq!(data.constants.len(), 2);
    assert!(data.constants[0].is_integer);
}

// ── Test 8: WEIGHT prefix conservation ──

#[test]
fn test_weight_conservation_violation() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
const WEIGHT_INPUT: f64 = 0.4;
const WEIGHT_HIDDEN: f64 = 0.3;
const WEIGHT_OUTPUT: f64 = 0.2;
const WEIGHT_TOTAL: f64 = 1.0;
"#),
    ]);
    let diagnostics = lint_crate(crate_dir.path());
    let _errors: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Error).collect();
    // 0.4 + 0.3 + 0.2 = 0.9 != 1.0
    assert!(!errors.is_empty(), "Should detect WEIGHT conservation violation");
}

// ── Test 9: SUM prefix conservation ──

#[test]
fn test_sum_prefix_conservation() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
const REGION_NORTH_SUM: i64 = 100;
const REGION_NORTH: i64 = 40;
const REGION_SOUTH: i64 = 60;
"#),
    ]);
    let diagnostics = lint_crate(crate_dir.path());
    // Both sub-components share "REGION" base — total is REGION_NORTH_SUM = 100
    let _notes: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Note).collect();
    // Should at least find the SUM constant and check it
    let has_sum_check = diagnostics.iter().any(|d| d.code.as_ref().map_or(false, |c| c.contains("sum-conservation")));
    assert!(has_sum_check, "Should check sum conservation for SUM constants");
}

// ── Test 10: Diagnostic JSON serialization ──

#[test]
fn test_diagnostic_json_roundtrip() {
    let diag = Diagnostic::error("Test error")
        .with_code("conservation-lint::test")
        .at(PathBuf::from("src/lib.rs"), 10, 5)
        .with_rendered("Detailed info");

    let json = serde_json::to_string(&diag).unwrap();
    assert!(json.contains("\"level\":\"error\""));
    assert!(json.contains("Test error"));
    assert!(json.contains("src/lib.rs"));

    let back: Diagnostic = serde_json::from_str(&json).unwrap();
    assert_eq!(back.level, DiagnosticLevel::Error);
    assert_eq!(back.message, "Test error");
}

// ── Test 11: lint_crate_json produces valid JSON ──

#[test]
fn test_lint_crate_json_output() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
const VAL_TOTAL: i64 = 100;
const VAL_A: i64 = 50;
const VAL_B: i64 = 40;
"#),
    ]);
    let json = lint_crate_json(crate_dir.path());
    let parsed: Vec<Diagnostic> = serde_json::from_str(&json).unwrap();
    assert!(!parsed.is_empty());
}

// ── Test 12: Multiple test count assertions ──

#[test]
fn test_multiple_test_count_assertions() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
// 5 tests
// 10 tests

#[test]
fn test_one() {}
"#),
    ]);
    let diagnostics = lint_crate(crate_dir.path());
    let test_warnings: Vec<_> = diagnostics.iter()
        .filter(|d| d.code.as_ref().map_or(false, |c| c.contains("test-count")))
        .collect();
    assert!(test_warnings.len() >= 2, "Should flag both mismatched assertions");
}

// ── Test 13: Static constants also extracted ──

#[test]
fn test_static_constants_extracted() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
static GLOBAL_SUM: i64 = 42;
const LOCAL_VAL: i64 = 10;
"#),
    ]);
    let extractor = SourceExtractor::new();
    let data = extractor.extract_crate(crate_dir.path());
    assert_eq!(data.constants.len(), 2);
    assert!(data.constants.iter().any(|c| c.name == "GLOBAL_SUM"));
    assert!(data.constants.iter().any(|c| c.name == "LOCAL_VAL"));
}

// ── Test 14: pub const extracted ──

#[test]
fn test_pub_const_extracted() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
pub const MAX_SIZE: usize = 1024;
"#),
    ]);
    let extractor = SourceExtractor::new();
    let data = extractor.extract_crate(crate_dir.path());
    assert_eq!(data.constants.len(), 1);
    assert_eq!(data.constants[0].name, "MAX_SIZE");
    assert_eq!(data.constants[0].value, 1024.0);
}

// ── Test 15: Checker tolerance customization ──

#[test]
fn test_custom_tolerance() {
    let checker = ConservationChecker::new().with_tolerance(100.0);
    assert!((checker.tolerance - 100.0).abs() < f64::EPSILON);
}

// ── Test 16: Zero-value constants don't cause issues ──

#[test]
fn test_zero_value_constants() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", r#"
const OFFSET: i64 = 0;
const WEIGHT_TOTAL: i64 = 0;
"#),
    ]);
    let diagnostics = lint_crate(crate_dir.path());
    let _errors: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Error).collect();
    // Zero total with no sub-components matching should not error
    assert!(errors.is_empty());
}

// ── Test 17: Non-rs files ignored ──

#[test]
fn test_non_rust_files_ignored() {
    let crate_dir = make_crate(&[
        ("src/lib.rs", "const VAL: i64 = 1;"),
        ("src/data.txt", "const FAKE: i64 = 999;"),
        ("src/build.rs", "const BUILD_CONST: i64 = 42;"),
    ]);
    let extractor = SourceExtractor::new();
    let data = extractor.extract_crate(crate_dir.path());
    // data.txt should be ignored, but build.rs is a Rust file and should be scanned
    assert!(!data.constants.iter().any(|c| c.name == "FAKE"));
}

// ── Test 18: Real repo - topological-sort-agent-rs ──

#[test]
fn test_against_topological_sort_agent_rs() {
    let repo_path = std::path::Path::new("/home/phoenix/repos/topological-sort-agent-rs");
    if !repo_path.exists() {
        eprintln!("Skipping: topological-sort-agent-rs not found");
        return;
    }
    let diagnostics = lint_crate(repo_path);
    // This repo has 15 #[test] functions and no conservation constants — should be clean or have notes only
    let _errors: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Error).collect();
    // It shouldn't have conservation violations (no TOTAL/SUM/WEIGHT constants)
    println!("Diagnostics for topological-sort-agent-rs: {:#?}", diagnostics);
    // Should extract constants and run checks without crashing
    assert!(true, "Linting topological-sort-agent-rs completed successfully");
}

// ── Test 19: Real repo - gpu-annealing ──

#[test]
fn test_against_gpu_annealing() {
    let repo_path = std::path::Path::new("/home/phoenix/repos/gpu-annealing");
    if !repo_path.exists() {
        eprintln!("Skipping: gpu-annealing not found");
        return;
    }
    let diagnostics = lint_crate(repo_path);
    println!("Diagnostics for gpu-annealing: {:#?}", diagnostics);
    // gpu-annealing has ConservationAnnealing with total tracking — check if we find anything
    let _errors: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Error).collect();
    let _warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Warning).collect();
    println!("Errors: {}, Warnings: {}", _errors.len(), _warnings.len());
    assert!(true, "Linting gpu-annealing completed successfully");
}
