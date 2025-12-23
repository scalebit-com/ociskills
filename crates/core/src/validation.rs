use regex::Regex;
use traits::{OciSkillsError, SkillMetadata};

/// Regex for valid skill names: lowercase alphanumeric with single hyphens
const SKILL_NAME_REGEX: &str = r"^[a-z0-9]+(-[a-z0-9]+)*$";

/// Parse and validate SKILL.md content (pure function)
pub fn parse_skill_md(content: &str) -> Result<SkillMetadata, OciSkillsError> {
    // 1. Find frontmatter delimiters (---)
    let (frontmatter, _body) = extract_frontmatter(content)?;

    // 2. Parse YAML frontmatter
    let metadata: SkillMetadata = serde_yaml::from_str(&frontmatter)
        .map_err(|e| OciSkillsError::InvalidSkillMd(format!("Invalid YAML: {}", e)))?;

    // 3. Validate required fields
    validate_skill_name(&metadata.name)?;
    validate_description(&metadata.description)?;

    // 4. Validate optional fields
    if let Some(ref compat) = metadata.compatibility {
        if compat.len() > 500 {
            return Err(OciSkillsError::Validation(
                "compatibility exceeds 500 character limit".into(),
            ));
        }
    }

    Ok(metadata)
}

/// Validate skill name format (pure function)
pub fn validate_skill_name(name: &str) -> Result<(), OciSkillsError> {
    if name.is_empty() || name.len() > 64 {
        return Err(OciSkillsError::InvalidSkillMd(
            "name must be 1-64 characters".into(),
        ));
    }

    let re = Regex::new(SKILL_NAME_REGEX).unwrap();
    if !re.is_match(name) {
        return Err(OciSkillsError::InvalidSkillMd(format!(
            "Invalid skill name '{}': must be lowercase alphanumeric with hyphens (e.g., 'my-skill')",
            name
        )));
    }

    Ok(())
}

/// Validate description (pure function)
pub fn validate_description(desc: &str) -> Result<(), OciSkillsError> {
    if desc.is_empty() || desc.len() > 1024 {
        return Err(OciSkillsError::InvalidSkillMd(
            "description must be 1-1024 characters".into(),
        ));
    }
    Ok(())
}

/// Validate that directory name matches skill name (pure function)
pub fn validate_directory_name(dir_name: &str, skill_name: &str) -> Result<(), OciSkillsError> {
    if dir_name != skill_name {
        return Err(OciSkillsError::Validation(format!(
            "Directory name '{}' does not match skill name '{}'",
            dir_name, skill_name
        )));
    }
    Ok(())
}

/// Extract frontmatter from SKILL.md content (pure function)
fn extract_frontmatter(content: &str) -> Result<(String, String), OciSkillsError> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Err(OciSkillsError::InvalidSkillMd(
            "Missing frontmatter".into(),
        ));
    }

    let rest = &content[3..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| OciSkillsError::InvalidSkillMd("Unclosed frontmatter".into()))?;

    let frontmatter = rest[..end].trim().to_string();
    let body = rest[end + 4..].trim().to_string();

    if body.is_empty() {
        return Err(OciSkillsError::InvalidSkillMd(
            "Missing content after frontmatter".into(),
        ));
    }

    Ok((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_skill_md() {
        let content = "---\nname: test-skill\ndescription: A test skill\n---\n# Test Skill\n\nInstructions here.";
        let result = parse_skill_md(content);
        assert!(result.is_ok());
        let meta = result.unwrap();
        assert_eq!(meta.name, "test-skill");
        assert_eq!(meta.description, "A test skill");
    }

    #[test]
    fn test_parse_missing_frontmatter() {
        let content = "# Just markdown\n\nNo frontmatter here.";
        let result = parse_skill_md(content);
        assert!(matches!(
            result,
            Err(OciSkillsError::InvalidSkillMd(_))
        ));
    }

    #[test]
    fn test_validate_skill_name_valid() {
        assert!(validate_skill_name("test-skill").is_ok());
        assert!(validate_skill_name("pdf").is_ok());
        assert!(validate_skill_name("data-analysis-tool").is_ok());
    }

    #[test]
    fn test_validate_skill_name_invalid() {
        assert!(validate_skill_name("Test-Skill").is_err()); // Uppercase
        assert!(validate_skill_name("-test").is_err()); // Leading hyphen
        assert!(validate_skill_name("test--skill").is_err()); // Consecutive hyphens
        assert!(validate_skill_name("test_skill").is_err()); // Underscore
    }

    #[test]
    fn test_validate_directory_name_match() {
        assert!(validate_directory_name("test-skill", "test-skill").is_ok());
    }

    #[test]
    fn test_validate_directory_name_mismatch() {
        assert!(validate_directory_name("test_skill", "test-skill").is_err());
    }
}
