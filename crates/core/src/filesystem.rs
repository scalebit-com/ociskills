use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use traits::{FileSystem, OciSkillsError};

pub struct RealFileSystem;

impl RealFileSystem {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl FileSystem for RealFileSystem {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, OciSkillsError> {
        fs::read(path).await.map_err(OciSkillsError::Io)
    }

    async fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), OciSkillsError> {
        fs::write(path, content).await.map_err(OciSkillsError::Io)
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, OciSkillsError> {
        let mut entries = fs::read_dir(path).await.map_err(OciSkillsError::Io)?;
        let mut results = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(OciSkillsError::Io)? {
            results.push(entry.path());
        }

        Ok(results)
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), OciSkillsError> {
        fs::create_dir_all(path)
            .await
            .map_err(OciSkillsError::Io)
    }

    async fn remove_dir_all(&self, path: &Path) -> Result<(), OciSkillsError> {
        fs::remove_dir_all(path)
            .await
            .map_err(OciSkillsError::Io)
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), OciSkillsError> {
        fs::rename(from, to).await.map_err(OciSkillsError::Io)
    }

    async fn copy_dir_all(&self, from: &Path, to: &Path) -> Result<(), OciSkillsError> {
        fs::create_dir_all(to).await.map_err(OciSkillsError::Io)?;

        let mut entries = fs::read_dir(from).await.map_err(OciSkillsError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(OciSkillsError::Io)? {
            let file_type = entry.file_type().await.map_err(OciSkillsError::Io)?;
            let target = to.join(entry.file_name());

            if file_type.is_dir() {
                Box::pin(self.copy_dir_all(&entry.path(), &target)).await?;
            } else {
                fs::copy(entry.path(), target)
                    .await
                    .map_err(OciSkillsError::Io)?;
            }
        }

        Ok(())
    }

    async fn exists(&self, path: &Path) -> bool {
        fs::metadata(path).await.is_ok()
    }

    async fn is_dir(&self, path: &Path) -> bool {
        fs::metadata(path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false)
    }

    fn temp_dir(&self) -> PathBuf {
        std::env::temp_dir()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }
}
