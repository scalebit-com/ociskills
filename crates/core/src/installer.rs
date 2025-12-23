use crate::validation;
use std::sync::Arc;
use traits::{
    Environment, FileSystem, InstallOptions, InstallScope, Logger, OciClient, OciReference,
    OciSkillsError, Skill, DEFAULT_HOME_SKILLS_SUBDIR, DEFAULT_PROJECT_SKILLS_SUBDIR,
    ENV_HOME_SKILLS_DIR, ENV_PROJECT_SKILLS_DIR,
};
use uuid::Uuid;

pub struct Installer {
    fs: Arc<dyn FileSystem>,
    oci: Arc<dyn OciClient>,
    env: Arc<dyn Environment>,
    logger: Arc<dyn Logger>,
}

impl Installer {
    pub fn new(
        fs: Arc<dyn FileSystem>,
        oci: Arc<dyn OciClient>,
        env: Arc<dyn Environment>,
        logger: Arc<dyn Logger>,
    ) -> Self {
        Self {
            fs,
            oci,
            env,
            logger,
        }
    }

    pub async fn install(
        &self,
        reference: &OciReference,
        options: &InstallOptions,
    ) -> Result<Vec<Skill>, OciSkillsError> {
        // 1. Resolve target directory (options.output_override > env var > scope default)
        let target_dir = self.resolve_target_dir(options)?;
        self.logger
            .info(&format!("Target directory: {}", target_dir.display()));

        // 2. Check target exists (or create if options.create_dirs)
        if !self.fs.exists(&target_dir).await {
            if options.create_dirs {
                self.logger.info("Creating target directory");
                self.fs.create_dir_all(&target_dir).await?;
            } else {
                return Err(OciSkillsError::DirectoryNotFound {
                    path: target_dir,
                    hint: "use --create-dirs to create".into(),
                });
            }
        }

        // 3. If dry_run, fetch config only and display
        if options.dry_run {
            self.logger
                .info(&format!("Pulling artifact metadata: {}", reference));
            let config = self.oci.inspect(reference).await?;
            self.logger.info("Dry run - would install:");
            for skill in &config.skills {
                self.logger
                    .info(&format!("  - {} ({})", skill.name, skill.description));
            }
            return Ok(vec![]);
        }

        // 4. Pull to temp directory
        self.logger
            .info(&format!("Pulling artifact: {}", reference));
        let temp_dir = self
            .fs
            .temp_dir()
            .join(format!("ociskills-{}", Uuid::new_v4()));
        self.fs.create_dir_all(&temp_dir).await?;

        let config = self.oci.pull(reference, &temp_dir).await?;

        // 5. Validate and install each skill
        let mut installed = Vec::new();
        for skill_meta in &config.skills {
            let skill_src = temp_dir.join(&skill_meta.name);
            let skill_dest = target_dir.join(&skill_meta.name);

            self.logger
                .info(&format!("Validating skill: {}", skill_meta.name));

            // Validate
            let skill_md_path = skill_src.join("SKILL.md");
            if !self.fs.exists(&skill_md_path).await {
                self.logger.error(&format!(
                    "Skill missing SKILL.md: {}",
                    skill_meta.name
                ));
                continue;
            }

            let content = self.fs.read_file(&skill_md_path).await?;
            let metadata = validation::parse_skill_md(&String::from_utf8_lossy(&content))?;
            validation::validate_directory_name(&skill_meta.name, &metadata.name)?;

            // Handle existing
            if self.fs.exists(&skill_dest).await {
                if !options.force {
                    self.logger.warn(&format!(
                        "Skill already exists, replacing: {}",
                        skill_meta.name
                    ));
                }
                self.fs.remove_dir_all(&skill_dest).await?;
            }

            // Move skill
            self.logger
                .info(&format!("Installing skill: {}", skill_meta.name));
            self.fs.rename(&skill_src, &skill_dest).await?;
            installed.push(Skill {
                metadata,
                directory_name: skill_meta.name.clone(),
            });
        }

        // Clean up temp directory
        if self.fs.exists(&temp_dir).await {
            let _ = self.fs.remove_dir_all(&temp_dir).await;
        }

        self.logger.info(&format!(
            "Successfully installed {} skill(s) to {}",
            installed.len(),
            target_dir.display()
        ));

        Ok(installed)
    }

    fn resolve_target_dir(
        &self,
        options: &InstallOptions,
    ) -> Result<std::path::PathBuf, OciSkillsError> {
        // Priority: output_override > env var > scope default
        if let Some(ref dir) = options.output_override {
            return Ok(dir.clone());
        }

        match options.scope {
            InstallScope::Home => {
                if let Some(env_dir) = self.env.get_var(ENV_HOME_SKILLS_DIR) {
                    Ok(std::path::PathBuf::from(env_dir))
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
                    Ok(std::path::PathBuf::from(env_dir))
                } else {
                    Ok(self.env.current_dir()?.join(DEFAULT_PROJECT_SKILLS_SUBDIR))
                }
            }
        }
    }
}
