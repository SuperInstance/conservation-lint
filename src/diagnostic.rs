//! Diagnostic output in cargo-diagnostic JSON format.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Severity level for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
}

/// Source span for a diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Span {
    pub file: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
}

/// Source location for a diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}

/// A single diagnostic finding, compatible with cargo's JSON message format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub code: Option<String>,
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered: Option<String>,
}

impl Diagnostic {
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: msg.into(),
            code: None,
            location: None,
            rendered: None,
        }
    }

    pub fn warning(msg: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: msg.into(),
            code: None,
            location: None,
            rendered: None,
        }
    }

    pub fn note(msg: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Note,
            message: msg.into(),
            code: None,
            location: None,
            rendered: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn at(mut self, file: PathBuf, line: usize, column: usize) -> Self {
        self.location = Some(Location {
            file,
            line,
            column,
        });
        self
    }

    pub fn with_rendered(mut self, rendered: impl Into<String>) -> Self {
        self.rendered = Some(rendered.into());
        self
    }
}
