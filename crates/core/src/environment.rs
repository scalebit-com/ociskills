use std::path::PathBuf;
use traits::{Environment, OciSkillsError};

pub struct RealEnvironment;

impl RealEnvironment {
    pub fn new() -> Self {
        Self
    }
}

impl Environment for RealEnvironment {
    fn get_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn current_dir(&self) -> Result<PathBuf, OciSkillsError> {
        std::env::current_dir().map_err(OciSkillsError::Io)
    }
}
