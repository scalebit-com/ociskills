use crate::validation;
use std::path::PathBuf;
use std::sync::Arc;
use traits::{
    ArtifactConfig, FileSystem, Logger, OciClient, OciReference, OciSkillsError, PublishOptions,
};
use uuid::Uuid;

pub struct Publisher {
    fs: Arc<dyn FileSystem>,
    oci: Arc<dyn OciClient>,
    logger: Arc<dyn Logger>,
}

impl Publisher {
    pub fn new(fs: Arc<dyn FileSystem>, oci: Arc<dyn OciClient>, logger: Arc<dyn Logger>) -> Self {
        Self { fs, oci, logger }
    }

    pub async fn publish(
        &self,
        reference: &OciReference,
        skill_paths: &[PathBuf],
        options: &PublishOptions,
    ) -> Result<String, OciSkillsError> {
        // 1. Validate all skill directories
        let mut skills = Vec::new();
        for path in skill_paths {
            let skill_md = path.join("SKILL.md");
            if !self.fs.exists(&skill_md).await {
                return Err(OciSkillsError::InvalidSkillMd(format!(
                    "SKILL.md not found in {}",
                    path.display()
                )));
            }

            let content = self.fs.read_file(&skill_md).await?;
            let metadata = validation::parse_skill_md(&String::from_utf8_lossy(&content))?;

            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| OciSkillsError::Validation("Invalid path".into()))?;

            validation::validate_directory_name(dir_name, &metadata.name)?;
            skills.push(metadata);
            self.logger.info(&format!("Validated: {}", dir_name));
        }

        // 2. If dry_run, just show what would be published
        if options.dry_run {
            self.logger.info("Dry run - would publish:");
            for skill in &skills {
                self.logger
                    .info(&format!("  - {} ({})", skill.name, skill.description));
            }
            return Ok("dry-run".into());
        }

        // 3. Build config
        let config = ArtifactConfig {
            version: "1".into(),
            created: chrono::Utc::now().to_rfc3339(),
            skills,
        };

        // 4. Copy to temp staging and push
        let temp_dir = self
            .fs
            .temp_dir()
            .join(format!("ociskills-publish-{}", Uuid::new_v4()));
        self.fs.create_dir_all(&temp_dir).await?;

        for path in skill_paths {
            let name = path.file_name().unwrap();
            self.fs.copy_dir_all(path, &temp_dir.join(name)).await?;
        }

        self.logger
            .info(&format!("Publishing to: {}", reference));
        let digest = self.oci.push(reference, &config, &temp_dir).await?;

        // Clean up temp directory
        if self.fs.exists(&temp_dir).await {
            let _ = self.fs.remove_dir_all(&temp_dir).await;
        }

        self.logger
            .info(&format!("Published: {}@{}", reference, digest));

        Ok(digest)
    }
}
