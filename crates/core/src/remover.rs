use std::path::PathBuf;
use std::sync::Arc;
use traits::{
    Environment, FileSystem, InstallScope, Logger, OciSkillsError, DEFAULT_HOME_SKILLS_SUBDIR,
    DEFAULT_PROJECT_SKILLS_SUBDIR, ENV_HOME_SKILLS_DIR, ENV_PROJECT_SKILLS_DIR,
};

pub struct SkillRemover {
    fs: Arc<dyn FileSystem>,
    env: Arc<dyn Environment>,
    logger: Arc<dyn Logger>,
}

impl SkillRemover {
    pub fn new(fs: Arc<dyn FileSystem>, env: Arc<dyn Environment>, logger: Arc<dyn Logger>) -> Self {
        Self { fs, env, logger }
    }

    pub async fn remove(
        &self,
        skill_names: &[String],
        scope: InstallScope,
        output_override: Option<PathBuf>,
        force: bool,
    ) -> Result<Vec<String>, OciSkillsError> {
        let target_dir = self.resolve_dir(scope, output_override)?;
        let mut removed = Vec::new();

        for name in skill_names {
            let skill_path = target_dir.join(name);
            if !self.fs.exists(&skill_path).await {
                self.logger.warn(&format!("Skill not found: {}", name));
                continue;
            }

            if !force {
                self.logger.info(&format!("Removing: {}", name));
            }

            self.fs.remove_dir_all(&skill_path).await?;
            removed.push(name.clone());
            self.logger.info(&format!("Removed: {}", name));
        }

        Ok(removed)
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
