//! Source code extractor: parses Rust files for constants, test counts, and distribution patterns.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A numerical constant extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    pub value: f64,
    pub file: PathBuf,
    pub line: usize,
    pub is_integer: bool,
}

/// A test count assertion found in a comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCountAssertion {
    pub expected_count: usize,
    pub file: PathBuf,
    pub line: usize,
    pub raw_comment: String,
}

/// A probability distribution pattern found in code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionPattern {
    pub file: PathBuf,
    pub line: usize,
    pub function_name: String,
    pub distribution_values: Vec<String>,
    pub appears_to_sum: Option<f64>,
}

/// All data extracted from a crate's source.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractedData {
    pub constants: Vec<Constant>,
    pub test_count_assertions: Vec<TestCountAssertion>,
    pub test_function_count: usize,
    pub distribution_patterns: Vec<DistributionPattern>,
    pub files_scanned: usize,
}

/// Extracts conservation-relevant data from Rust source files.
pub struct SourceExtractor {
    const_re: Regex,
    test_fn_re: Regex,
    test_count_comment_re: Regex,
}

impl SourceExtractor {
    pub fn new() -> Self {
        Self {
            // Match const and static declarations with numeric values
            const_re: Regex::new(
                r"(?m)^\s*(?:pub\s+)?(?:const|static)\s+(\w+)\s*:\s*\w+\s*=\s*(-?\d+\.?\d*(?:_\d+)*)\s*;"
            ).unwrap(),
            // Match #[test] function declarations
            test_fn_re: Regex::new(
                r"#\[test\]"
            ).unwrap(),
            // Match test count assertions in comments: "// N tests" or "// N test cases"
            test_count_comment_re: Regex::new(
                r"//\s*(\d+)\s+tests?"
            ).unwrap(),
        }
    }

    /// Extract all data from a crate directory.
    pub fn extract_crate(&self, crate_dir: &Path) -> ExtractedData {
        let mut data = ExtractedData::default();
        let src_dir = crate_dir.join("src");

        if !src_dir.exists() {
            return data;
        }

        for entry in WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().map(|ext| ext == "rs").unwrap_or(false)
            })
        {
            let path = entry.path().to_path_buf();
            if let Ok(content) = std::fs::read_to_string(&path) {
                data.files_scanned += 1;
                self.extract_from_file(&content, &path, &mut data);
            }
        }

        data
    }

    fn extract_from_file(&self, content: &str, path: &Path, data: &mut ExtractedData) {
        // Extract constants
        for caps in self.const_re.captures_iter(content) {
            let name = caps[1].to_string();
            let value_str = caps[2].replace('_', "");
            if let Ok(value) = value_str.parse::<f64>() {
                let line = self.find_line_number(content, &caps[0]);
                data.constants.push(Constant {
                    name,
                    value,
                    file: path.to_path_buf(),
                    line,
                    is_integer: value_str.contains('.') == false,
                });
            }
        }

        // Count test functions
        data.test_function_count += self.test_fn_re.find_iter(content).count();

        // Extract test count assertions from comments
        for caps in self.test_count_comment_re.captures_iter(content) {
            if let Ok(count) = caps[1].parse::<usize>() {
                let line = self.find_line_number(content, &caps[0]);
                data.test_count_assertions.push(TestCountAssertion {
                    expected_count: count,
                    file: path.to_path_buf(),
                    line,
                    raw_comment: caps[0].to_string(),
                });
            }
        }

        // Extract distribution patterns
        self.extract_distributions(content, path, data);
    }

    fn extract_distributions(&self, content: &str, path: &Path, data: &mut ExtractedData) {
        // Look for arrays/vecs of floats that sum near 1.0 (probability distributions)
        let array_re = Regex::new(r"(?:vec!|&)\s*\[\s*([\d.]+(?:\s*,\s*[\d.]+)+)\s*\]").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            for caps in array_re.captures_iter(line) {
                let values_str = &caps[1];
                let values: Vec<&str> = values_str.split(',').map(|s| s.trim()).collect();
                let parsed: Vec<f64> = values.iter()
                    .filter_map(|v| v.parse::<f64>().ok())
                    .collect();

                if parsed.len() >= 2 && parsed.iter().all(|&v| v >= 0.0 && v <= 1.0) {
                    let sum: f64 = parsed.iter().sum();
                    if (sum - 1.0).abs() < 0.01 || (sum > 0.0 && sum < 2.0) {
                        // Find the enclosing function name
                        let func_name = self.find_function_name(content, line_num);
                        data.distribution_patterns.push(DistributionPattern {
                            file: path.to_path_buf(),
                            line: line_num + 1,
                            function_name: func_name.unwrap_or_else(|| "unknown".to_string()),
                            distribution_values: values.iter().map(|s| s.to_string()).collect(),
                            appears_to_sum: Some(sum),
                        });
                    }
                }
            }
        }
    }

    fn find_line_number(&self, content: &str, substr: &str) -> usize {
        content
            .find(substr)
            .map(|pos| content[..pos].lines().count() + 1)
            .unwrap_or(0)
    }

    fn find_function_name(&self, content: &str, line_index: usize) -> Option<String> {
        let re = Regex::new(r"fn\s+(\w+)").ok()?;
        let lines: Vec<&str> = content.lines().take(line_index + 1).collect();
        for line in lines.iter().rev() {
            if let Some(caps) = re.captures(line) {
                return Some(caps[1].to_string());
            }
        }
        None
    }
}

/// Groups constants by their base name (stripping TOTAL/SUM/WEIGHT suffixes)
/// to identify sub-component relationships.
pub fn group_constants_by_base(constants: &[Constant]) -> HashMap<String, Vec<&Constant>> {
    let mut groups: HashMap<String, Vec<&Constant>> = HashMap::new();

    for constant in constants {
        let name = constant.name.to_uppercase();

        // Determine the base group
        let base = if name.contains("TOTAL") || name.contains("SUM") || name.contains("WEIGHT") {
            // Extract base: e.g., "SCORE_TOTAL" -> "SCORE", "TOTAL_WEIGHT" -> "WEIGHT"
            name.replace("TOTAL", "")
                .replace("SUM", "")
                .replace("WEIGHT", "")
                .replace("_", "")
                .trim()
                .to_string()
        } else {
            // Check if this looks like a sub-component (has a category prefix)
            name.split('_')
                .next()
                .unwrap_or(&name)
                .to_string()
        };

        groups.entry(base).or_default().push(constant);
    }

    groups
}
