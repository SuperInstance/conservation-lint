//! Conservation checker: verifies extracted data against conservation laws.

use crate::diagnostic::{Diagnostic, DiagnosticLevel};
use crate::extractor::{ExtractedData, Constant};

/// The kind of conservation check performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckKind {
    /// Sub-components should sum to their declared total.
    SumConservation,
    /// Test count in comments should match actual #[test] count.
    TestCountAssertion,
    /// Probability distributions should sum to ~1.0.
    EntropyConservation,
}

/// Result of a single conservation check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub kind: CheckKind,
    pub passed: bool,
    pub message: String,
    pub diagnostic: Diagnostic,
}

/// Runs conservation checks on extracted data.
pub struct ConservationChecker {
    /// Tolerance for floating-point comparisons.
    pub tolerance: f64,
}

impl ConservationChecker {
    pub fn new() -> Self {
        Self { tolerance: 0.001 }
    }

    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Run all checks and return diagnostics.
    pub fn check(&self, data: &ExtractedData) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        diagnostics.extend(self.check_sum_conservation(&data.constants));
        diagnostics.extend(self.check_test_count_assertions(data));
        diagnostics.extend(self.check_entropy_conservation(&data.distribution_patterns));

        diagnostics
    }

    /// Check that sub-component constants sum to their declared totals.
    pub fn check_sum_conservation(&self, constants: &[Constant]) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Group constants: find totals and their sub-components
        // A "total" constant must end with TOTAL/SUM/WEIGHT or have it as a suffix after _
        let totals: Vec<&Constant> = constants.iter().filter(|c| {
            let name = c.name.to_uppercase();
            name.ends_with("_TOTAL") || name.ends_with("_SUM") || name.ends_with("_WEIGHT")
                || name == "TOTAL" || name == "SUM" || name == "WEIGHT"
        }).collect();

        let non_totals: Vec<&Constant> = constants.iter().filter(|c| {
            let name = c.name.to_uppercase();
            !(name.ends_with("_TOTAL") || name.ends_with("_SUM") || name.ends_with("_WEIGHT")
                || name == "TOTAL" || name == "SUM" || name == "WEIGHT")
        }).collect();

        for total_const in &totals {
            // Find sub-components that share a naming prefix
            let total_name = total_const.name.to_uppercase();
            // Strip only the suffix that made this a "total" constant
            let base = if total_name.ends_with("_TOTAL") {
                total_name.strip_suffix("_TOTAL").unwrap()
            } else if total_name.ends_with("_SUM") {
                total_name.strip_suffix("_SUM").unwrap()
            } else if total_name.ends_with("_WEIGHT") {
                total_name.strip_suffix("_WEIGHT").unwrap()
            } else if total_name == "TOTAL" || total_name == "SUM" || total_name == "WEIGHT" {
                ""
            } else {
                total_name.trim_end_matches("TOTAL")
                    .trim_end_matches("SUM")
                    .trim_end_matches("WEIGHT")
                    .trim_matches('_')
            }.trim_matches('_').to_string();

            if base.is_empty() {
                continue;
            }

            // Find sub-components whose names start with the base prefix
            let sub_components: Vec<&&Constant> = non_totals.iter().filter(|c| {
                let name = c.name.to_uppercase();
                // Match if the constant name starts with the base prefix
                name.starts_with(&base) || name.contains(&base)
            }).collect();

            if sub_components.is_empty() {
                // No sub-components found — skip this total
                continue;
            }

            let actual_sum: f64 = sub_components.iter().map(|c| c.value).sum();
            let expected = total_const.value;

            if (actual_sum - expected).abs() > self.tolerance {
                let sub_names: Vec<String> = sub_components.iter().map(|c| {
                    format!("{}={}", c.name, c.value)
                }).collect();

                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Error,
                    message: format!(
                        "Conservation violation: {} = {} but sub-components sum to {} (diff: {})",
                        total_const.name,
                        expected,
                        actual_sum,
                        (actual_sum - expected).abs()
                    ),
                    code: Some("conservation-lint::sum-conservation".to_string()),
                    location: Some(crate::diagnostic::Location {
                        file: total_const.file.clone(),
                        line: total_const.line,
                        column: 1,
                    }),
                    rendered: Some(format!(
                        "Components: [{}] should sum to {} ({})",
                        sub_names.join(", "),
                        expected,
                        total_const.name
                    )),
                });
            } else {
                // Log passing check as a note
                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Note,
                    message: format!(
                        "✓ Conservation OK: {} = {} matches sub-component sum {}",
                        total_const.name, expected, actual_sum
                    ),
                    code: Some("conservation-lint::sum-conservation".to_string()),
                    location: Some(crate::diagnostic::Location {
                        file: total_const.file.clone(),
                        line: total_const.line,
                        column: 1,
                    }),
                    rendered: None,
                });
            }
        }

        diagnostics
    }

    /// Check test count assertions against actual test function count.
    pub fn check_test_count_assertions(&self, data: &ExtractedData) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for assertion in &data.test_count_assertions {
            if assertion.expected_count != data.test_function_count {
                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Warning,
                    message: format!(
                        "Test count assertion says {} tests, but found {} #[test] functions",
                        assertion.expected_count, data.test_function_count
                    ),
                    code: Some("conservation-lint::test-count".to_string()),
                    location: Some(crate::diagnostic::Location {
                        file: assertion.file.clone(),
                        line: assertion.line,
                        column: 1,
                    }),
                    rendered: Some(format!(
                        "Comment: \"{}\" — expected {} tests, found {}",
                        assertion.raw_comment,
                        assertion.expected_count,
                        data.test_function_count
                    )),
                });
            } else {
                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Note,
                    message: format!(
                        "✓ Test count OK: assertion of {} matches actual count",
                        assertion.expected_count
                    ),
                    code: Some("conservation-lint::test-count".to_string()),
                    location: Some(crate::diagnostic::Location {
                        file: assertion.file.clone(),
                        line: assertion.line,
                        column: 1,
                    }),
                    rendered: None,
                });
            }
        }

        diagnostics
    }

    /// Check that probability distributions sum to approximately 1.0.
    pub fn check_entropy_conservation(
        &self,
        distributions: &[crate::extractor::DistributionPattern],
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for dist in distributions {
            if let Some(sum) = dist.appears_to_sum {
                if (sum - 1.0).abs() > self.tolerance {
                    diagnostics.push(Diagnostic {
                        level: DiagnosticLevel::Warning,
                        message: format!(
                            "Entropy violation: distribution in {} sums to {} (expected ~1.0)",
                            dist.function_name, sum
                        ),
                        code: Some("conservation-lint::entropy-conservation".to_string()),
                        location: Some(crate::diagnostic::Location {
                            file: dist.file.clone(),
                            line: dist.line,
                            column: 1,
                        }),
                        rendered: Some(format!(
                            "Distribution values: {:?} — sum = {} (entropy conservation requires sum ≈ 1.0)",
                            dist.distribution_values, sum
                        )),
                    });
                } else {
                    diagnostics.push(Diagnostic {
                        level: DiagnosticLevel::Note,
                        message: format!(
                            "✓ Entropy OK: distribution in {} sums to {}",
                            dist.function_name, sum
                        ),
                        code: Some("conservation-lint::entropy-conservation".to_string()),
                        location: Some(crate::diagnostic::Location {
                            file: dist.file.clone(),
                            line: dist.line,
                            column: 1,
                        }),
                        rendered: None,
                    });
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_tolerance_default() {
        let checker = ConservationChecker::new();
        assert!((checker.tolerance - 0.001).abs() < f64::EPSILON);
    }
}
