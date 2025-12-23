use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Parsed SKILL.md frontmatter (per agentskills.io/specification)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Required: 1-64 chars, lowercase alphanumeric and hyphens, must match directory name
    pub name: String,
    /// Required: 1-1024 chars, explains what skill does AND when to use it
    pub description: String,
    /// Optional: license name or reference to bundled license file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Optional: max 500 chars, environment requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,
    /// Optional: arbitrary key-value metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Optional: space-delimited list of pre-approved tools (experimental)
    #[serde(rename = "allowed-tools", skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<String>,
}

/// A validated skill ready for installation
#[derive(Debug, Clone)]
pub struct Skill {
    pub metadata: SkillMetadata,
    pub directory_name: String,
}

/// OCI reference (registry/repo:tag or registry/repo@digest)
#[derive(Debug, Clone)]
pub struct OciReference {
    pub registry: String,
    pub repository: String,
    pub reference: TagOrDigest,
}

impl OciReference {
    /// Parse an OCI reference string (supports optional oci:// prefix)
    pub fn parse(s: &str) -> Result<Self, crate::OciSkillsError> {
        // Strip optional "oci://" prefix
        let s = s.strip_prefix("oci://").unwrap_or(s);

        // Find registry (everything before first /)
        let (registry, rest) = s
            .split_once('/')
            .ok_or_else(|| crate::OciSkillsError::InvalidReference(
                "Missing repository in reference".into()
            ))?;

        // Check for digest (@sha256:...)
        if let Some((repo, digest)) = rest.split_once('@') {
            return Ok(Self {
                registry: registry.to_string(),
                repository: repo.to_string(),
                reference: TagOrDigest::Digest(digest.to_string()),
            });
        }

        // Check for tag (:version)
        if let Some((repo, tag)) = rest.split_once(':') {
            return Ok(Self {
                registry: registry.to_string(),
                repository: repo.to_string(),
                reference: TagOrDigest::Tag(tag.to_string()),
            });
        }

        // No tag or digest, default to :latest
        Ok(Self {
            registry: registry.to_string(),
            repository: rest.to_string(),
            reference: TagOrDigest::Tag("latest".to_string()),
        })
    }

    /// Convert to string format (without oci:// prefix)
    pub fn to_string_without_prefix(&self) -> String {
        match &self.reference {
            TagOrDigest::Tag(tag) => format!("{}/{}:{}", self.registry, self.repository, tag),
            TagOrDigest::Digest(digest) => format!("{}/{}@{}", self.registry, self.repository, digest),
        }
    }
}

impl std::fmt::Display for OciReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_without_prefix())
    }
}

#[derive(Debug, Clone)]
pub enum TagOrDigest {
    Tag(String),
    Digest(String), // sha256:abc123...
}

/// Config blob stored in OCI artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactConfig {
    pub version: String,
    pub created: String,
    pub skills: Vec<SkillMetadata>,
}

/// Installation scope
#[derive(Debug, Clone, Copy)]
pub enum InstallScope {
    Home,    // ~/.claude/skills
    Project, // .claude/skills
}

/// Options for install command
#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub scope: InstallScope,
    pub output_override: Option<PathBuf>,
    pub create_dirs: bool,
    pub dry_run: bool,
    pub force: bool,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Options for publish command
#[derive(Debug, Clone)]
pub struct PublishOptions {
    pub dry_run: bool,
    pub annotations: Vec<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Environment variable names and defaults
pub const ENV_HOME_SKILLS_DIR: &str = "OCI_SKILLS_HOME_SKILLS_DIR";
pub const ENV_PROJECT_SKILLS_DIR: &str = "OCI_SKILLS_PROJECT_SKILLS_DIR";
pub const DEFAULT_HOME_SKILLS_SUBDIR: &str = ".claude/skills";
pub const DEFAULT_PROJECT_SKILLS_SUBDIR: &str = ".claude/skills";
