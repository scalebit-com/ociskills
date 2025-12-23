use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::{ArtifactConfig, OciReference, OciSkillsError};

/// Filesystem operations (mockable for tests)
#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, OciSkillsError>;
    async fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), OciSkillsError>;
    async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, OciSkillsError>;
    async fn create_dir_all(&self, path: &Path) -> Result<(), OciSkillsError>;
    async fn remove_dir_all(&self, path: &Path) -> Result<(), OciSkillsError>;
    async fn rename(&self, from: &Path, to: &Path) -> Result<(), OciSkillsError>;
    async fn copy_dir_all(&self, from: &Path, to: &Path) -> Result<(), OciSkillsError>;
    async fn exists(&self, path: &Path) -> bool;
    async fn is_dir(&self, path: &Path) -> bool;
    fn temp_dir(&self) -> PathBuf;
    fn home_dir(&self) -> Option<PathBuf>;
}

/// OCI registry operations (mockable for tests)
#[async_trait]
pub trait OciClient: Send + Sync {
    /// Pull artifact and extract to target directory
    /// Returns the config blob for inspection
    async fn pull(
        &self,
        reference: &OciReference,
        extract_to: &Path,
    ) -> Result<ArtifactConfig, OciSkillsError>;

    /// Pack directory contents and push as artifact
    /// Returns the digest of the pushed manifest
    async fn push(
        &self,
        reference: &OciReference,
        config: &ArtifactConfig,
        skills_dir: &Path,
    ) -> Result<String, OciSkillsError>;

    /// Inspect artifact metadata without downloading layers
    /// Returns the config blob only
    async fn inspect(
        &self,
        reference: &OciReference,
    ) -> Result<ArtifactConfig, OciSkillsError>;
}

/// Environment access (mockable for tests)
pub trait Environment: Send + Sync {
    fn get_var(&self, name: &str) -> Option<String>;
    fn current_dir(&self) -> Result<PathBuf, OciSkillsError>;
}

/// Logging (mockable for tests)
pub trait Logger: Send + Sync {
    fn debug(&self, message: &str);
    fn info(&self, message: &str);
    fn warn(&self, message: &str);
    fn error(&self, message: &str);
}
