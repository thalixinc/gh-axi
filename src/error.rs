//! Structured error type + exit-code mapping (AXI family convention).
//!
//! Exit codes: 0 = success, 1 = operational failure, 2 = usage / validation.

use std::fmt;

/// A structured CLI error carrying a message, `code`, and optional `help` suggestions.
#[derive(Debug, Clone)]
pub struct AxiError {
    pub message: String,
    pub code: &'static str,
    pub suggestions: Vec<String>,
}

impl AxiError {
    /// Operational failure (exit 1).
    pub fn operational(message: impl Into<String>, code: &'static str) -> Self {
        Self {
            message: message.into(),
            code,
            suggestions: Vec::new(),
        }
    }

    /// Usage / validation failure (exit 2).
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: "VALIDATION_ERROR",
            suggestions: Vec::new(),
        }
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }

    /// 2 for validation/usage, 1 otherwise.
    pub fn exit_code(&self) -> i32 {
        if self.code == "VALIDATION_ERROR" {
            2
        } else {
            1
        }
    }
}

impl fmt::Display for AxiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AxiError {}

pub type Result<T, E = AxiError> = std::result::Result<T, E>;
