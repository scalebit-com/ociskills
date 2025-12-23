use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OciSkillsError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Directory not found: {path} (hint: {hint})")]
    DirectoryNotFound { path: PathBuf, hint: String },

    #[error("OCI error: {0}")]
    Oci(String),

    #[error("Invalid OCI reference: {0}")]
    InvalidReference(String),

    #[error("Invalid SKILL.md: {0}")]
    InvalidSkillMd(String),

    #[error("Authentication required for {registry}")]
    AuthRequired { registry: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
