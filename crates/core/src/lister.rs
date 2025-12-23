use crate::validation;
use std::path::PathBuf;
use std::sync::Arc;
use traits::{
    Environment, FileSystem, InstallScope, Logger, OciSkillsError, SkillMetadata,
    DEFAULT_HOME_SKILLS_SUBDIR, DEFAULT_PROJECT_SKILLS_SUBDIR, ENV_HOME_SKILLS_DIR,
    ENV_PROJECT_SKILLS_DIR,
};

pub struct SkillLister {
    fs: Arc<dyn FileSystem>,
    env: Arc<dyn Environment>,
    logger: Arc<dyn Logger>,
}

impl SkillLister {
    pub fn new(fs: Arc<dyn FileSystem>, env: Arc<dyn Environment>, logger: Arc<dyn Logger>) -> Self {
        Self { fs, env, logger }
    }

    pub async fn list(
        &self,
        scope: InstallScope,
        output_override: Option<PathBuf>,
        json_output: bool,
    ) -> Result<Vec<SkillMetadata>, OciSkillsError> {
        let target_dir = self.resolve_dir(scope, output_override)?;

        if !self.fs.exists(&target_dir).await {
            if json_output {
                println!("[]");
            } else {
                self.logger
                    .info(&format!("No skills directory found at {}", target_dir.display()));
            }
            return Ok(vec![]);
        }

        let entries = self.fs.read_dir(&target_dir).await?;
        let mut skills = Vec::new();

        for entry in entries {
            if self.fs.is_dir(&entry).await {
                let skill_md = entry.join("SKILL.md");
                if self.fs.exists(&skill_md).await {
                    let content = self.fs.read_file(&skill_md).await?;
                    if let Ok(meta) =
                        validation::parse_skill_md(&String::from_utf8_lossy(&content))
                    {
                        skills.push(meta);
                    }
                }
            }
        }

        if json_output {
            println!("{}", serde_json::to_string_pretty(&skills)?);
        } else {
            if skills.is_empty() {
                self.logger.info("No skills installed");
            } else {
                self.logger
                    .info(&format!("Found {} skill(s):", skills.len()));
                for skill in &skills {
                    self.logger
                        .info(&format!("  - {}: {}", skill.name, skill.description));
                }
            }
        }

        Ok(skills)
    }

    fn resolve_dir(
        &self,
        scope: InstallScope,
        output_override: Option<PathBuf>,
    ) -> Result<PathBuf, OciSkillsError> {
        if let Some(dir) = output_override {
            return Ok(dir);
        }

        match scope {
            InstallScope::Home => {
                if let Some(env_dir) = self.env.get_var(ENV_HOME_SKILLS_DIR) {
                    Ok(PathBuf::from(env_dir))
                } else {
                    Ok(self
                        .fs
                        .home_dir()
                        .ok_or_else(|| {
                            OciSkillsError::Io(std::io::Error::new(
                                std::io::ErrorKind::NotFound,
                                "Could not determine home directory",
                            ))
                        })?
                        .join(DEFAULT_HOME_SKILLS_SUBDIR))
                }
            }
            InstallScope::Project => {
                if let Some(env_dir) = self.env.get_var(ENV_PROJECT_SKILLS_DIR) {
                    Ok(PathBuf::from(env_dir))
                } else {
                    Ok(self.env.current_dir()?.join(DEFAULT_PROJECT_SKILLS_SUBDIR))
                }
            }
        }
    }
}
