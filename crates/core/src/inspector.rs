use std::sync::Arc;
use traits::{ArtifactConfig, Logger, OciClient, OciReference, OciSkillsError};

pub struct ArtifactInspector {
    oci: Arc<dyn OciClient>,
    logger: Arc<dyn Logger>,
}

impl ArtifactInspector {
    pub fn new(oci: Arc<dyn OciClient>, logger: Arc<dyn Logger>) -> Self {
        Self { oci, logger }
    }

    pub async fn inspect(
        &self,
        reference: &OciReference,
        json_output: bool,
    ) -> Result<ArtifactConfig, OciSkillsError> {
        let config = self.oci.inspect(reference).await?;

        if json_output {
            println!("{}", serde_json::to_string_pretty(&config)?);
        } else {
            self.logger
                .info(&format!("Artifact: {}", reference));
            self.logger
                .info(&format!("Created: {}", config.created));
            self.logger
                .info(&format!("Skills ({}):", config.skills.len()));
            for skill in &config.skills {
                self.logger
                    .info(&format!("  - {}: {}", skill.name, skill.description));
            }
        }

        Ok(config)
    }
}
