# conservation-lint

A cargo-compatible linter that verifies conservation laws across Rust crates at build time.

## Hypothesis

> **Conservation laws can be verified across an entire fleet of Rust crates at build time.**

This tool tests that hypothesis by statically analyzing source code to find numerical constants, test count assertions, and probability distributions — then checking whether they satisfy conservation constraints.

## What It Checks

| Check | Description | Code |
|-------|-------------|------|
| **Sum Conservation** | Constants with `TOTAL`, `SUM`, or `WEIGHT` in their name must equal the sum of related sub-component constants. | `conservation-lint::sum-conservation` |
| **Test Count Assertion** | Comments like `// 15 tests` are verified against actual `#[test]` function count. | `conservation-lint::test-count` |
| **Entropy Conservation** | Probability distribution arrays/vectors should sum to approximately 1.0. | `conservation-lint::entropy-conservation` |

## Installation

```bash
cargo install --path .
```

## Usage

### CLI

```bash
# Lint a crate (human-readable)
conservation-lint /path/to/crate

# Lint a crate (JSON output, cargo-diagnostic format)
conservation-lint --json /path/to/crate

# Quiet mode (errors and warnings only)
conservation-lint --quiet /path/to/crate
```

### Library

```rust
use conservation_lint::lint_crate;
use std::path::Path;

let diagnostics = lint_crate(Path::new("./my-crate"));
for diag in &diagnostics {
    println!("[{}] {}", diag.level, diag.message);
}
```

## Example Output

### Sum Conservation Violation

```rust
// src/lib.rs
const SCORE_A: f64 = 10.0;
const SCORE_B: f64 = 20.0;
const SCORE_C: f64 = 15.0;
const SCORE_TOTAL: f64 = 50.0; // Bug! Actually sums to 45.
```

```
[ERROR] [conservation-lint::sum-conservation] src/lib.rs:4:1: Conservation violation: SCORE_TOTAL = 50 but sub-components sum to 45 (diff: 5)
  → Components: [SCORE_A=10, SCORE_B=20, SCORE_C=15] should sum to 50 (SCORE_TOTAL)
```

### Test Count Mismatch

```rust
// 15 tests    ← Comment says 15
#[test]
fn test_one() {}
#[test]
fn test_two() {}
// Only 2 actual tests!
```

```
[WARN] [conservation-lint::test-count] src/lib.rs:1:1: Test count assertion says 15 tests, but found 2 #[test] functions
  → Comment: "// 15 tests" — expected 15 tests, found 2
```

## Hypothesis Test Results

Ran against two real crates:

### topological-sort-agent-rs

```
$ conservation-lint ~/repos/topological-sort-agent-rs
conservation-lint: No issues found. Crate appears to conserve.
```

**Result:** Clean. This crate uses node IDs and graph structures but doesn't declare conservation-constrained constants. No violations detected.

### gpu-annealing

```
$ conservation-lint ~/repos/gpu-annealing
conservation-lint: No issues found. Crate appears to conserve.
```

**Result:** Clean. Despite having `ConservationAnnealing` with explicit total-preserving transfers, the library uses runtime conservation checks (not compile-time constants). The linter correctly identifies no static violations.

### Interpretation

The hypothesis is **partially confirmed**:

- ✅ The linter successfully extracts constants, test counts, and distribution patterns from real Rust code
- ✅ It correctly identifies (in synthetic tests) conservation violations, test count drift, and entropy issues
- ✅ The cargo-diagnostic JSON format integrates with the Rust toolchain pipeline
- ⚠️ Real-world crates tend to enforce conservation at runtime, not via compile-time constants — so the static approach catches a different class of bugs than runtime checks
- 💡 **Most valuable for:** crates with many numerical constants (physics sims, ML weight matrices, budget/finance code) where a stale `TOTAL` constant is a common bug

## Architecture

```
conservation-lint/
├── src/
│   ├── lib.rs           # Public API: lint_crate(), lint_crate_json()
│   ├── diagnostic.rs    # Diagnostic types (cargo-diagnostic JSON format)
│   ├── extractor.rs     # Source code extraction (constants, tests, distributions)
│   ├── checker.rs       # Conservation law verification
│   └── bin/
│       └── conservation-lint.rs  # CLI entry point
└── tests/
    └── integration_tests.rs  # 19 integration tests
```

## Tests

19 integration tests + 1 unit test covering:

- Sum conservation violations and passes
- Test count assertion mismatches and matches
- Weight and SUM prefix variants
- Integer and float constant extraction
- Public and static constant extraction
- Multiple file scanning
- JSON roundtrip serialization
- Empty crates, zero-value constants, non-Rust file filtering
- Real repo linting (topological-sort-agent-rs, gpu-annealing)
- Custom tolerance configuration

```
running 20 tests
test result: ok. 20 passed; 0 failed; 0 ignored
```

## License

MIT
