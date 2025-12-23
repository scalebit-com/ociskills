use crate::validation;
use std::path::PathBuf;
use std::sync::Arc;
use traits::{FileSystem, Logger, OciSkillsError, SkillMetadata};

pub struct SkillValidator {
    fs: Arc<dyn FileSystem>,
    logger: Arc<dyn Logger>,
}

impl SkillValidator {
    pub fn new(fs: Arc<dyn FileSystem>, logger: Arc<dyn Logger>) -> Self {
        Self { fs, logger }
    }

    pub async fn validate(&self, paths: &[PathBuf]) -> Result<Vec<SkillMetadata>, OciSkillsError> {
        let mut results = Vec::new();

        for path in paths {
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| OciSkillsError::Validation("Invalid path".into()))?;

            let skill_md = path.join("SKILL.md");
            if !self.fs.exists(&skill_md).await {
                return Err(OciSkillsError::InvalidSkillMd(format!(
                    "SKILL.md not found in {}",
                    path.display()
                )));
            }

            let content = self.fs.read_file(&skill_md).await?;
            let metadata = validation::parse_skill_md(&String::from_utf8_lossy(&content))?;
            validation::validate_directory_name(dir_name, &metadata.name)?;

            self.logger.info(&format!("✓ {} - valid", dir_name));
            results.push(metadata);
        }

        Ok(results)
    }
}
