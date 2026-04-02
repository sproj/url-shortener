use std::fmt::Display;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub field: String,
    pub code: &'static str,
    pub message: String,
}

impl Display for ValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]: {}", self.field, self.code, self.message)
    }
}

pub type ValidationRule<T> = fn(&T, &str, &mut Vec<ValidationIssue>);
