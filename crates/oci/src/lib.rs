mod auth;
mod extractor;
mod packer;

use async_trait::async_trait;
use oci_client::client::ClientConfig;
use oci_client::manifest::{OciDescriptor, OciImageManifest};
use oci_client::{Client, Reference};
use sha2::{Digest, Sha256};
use std::path::Path;
use traits::{ArtifactConfig, OciClient, OciReference, OciSkillsError};

pub struct OciClientImpl {
    client: Client,
}

impl OciClientImpl {
    pub fn new() -> Result<Self, OciSkillsError> {
        let config = ClientConfig::default();
        let client = Client::new(config);
        Ok(Self { client })
    }

    fn to_oci_reference(&self, reference: &OciReference) -> Result<Reference, OciSkillsError> {
        let ref_str = reference.to_string_without_prefix();
        ref_str
            .parse()
            .map_err(|e| OciSkillsError::InvalidReference(format!("{}", e)))
    }

    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl OciClient for OciClientImpl {
    async fn pull(
        &self,
        reference: &OciReference,
        extract_to: &Path,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<ArtifactConfig, OciSkillsError> {
        let oci_ref = self.to_oci_reference(reference)?;
        let auth = auth::get_registry_auth_with_override(
            oci_ref.registry(),
            username,
            password,
        );

        // 1. Pull image data
        let image_data = self
            .client
            .pull(&oci_ref, &auth, vec![])
            .await
            .map_err(|e| OciSkillsError::Oci(e.to_string()))?;

        // 2. Get manifest (already an OciImageManifest)
        let image_manifest = image_data.manifest.ok_or_else(|| {
            OciSkillsError::Oci("No manifest in image data".into())
        })?;

        // 3. Pull and parse config blob
        let config_desc = &image_manifest.config;
        let mut config_bytes = Vec::new();
        self.client
            .pull_blob(&oci_ref, config_desc, &mut config_bytes)
            .await
            .map_err(|e| OciSkillsError::Oci(e.to_string()))?;

        let config: ArtifactConfig = serde_json::from_slice(&config_bytes)
            .map_err(|e| OciSkillsError::Oci(format!("Invalid config blob: {}", e)))?;

        // 4. Pull layer blob (tar.gz)
        if image_manifest.layers.is_empty() {
            return Err(OciSkillsError::Oci("No layers found in manifest".into()));
        }

        let layer = &image_manifest.layers[0];
        let mut tar_gz_data = Vec::new();
        self.client
            .pull_blob(&oci_ref, layer, &mut tar_gz_data)
            .await
            .map_err(|e| OciSkillsError::Oci(e.to_string()))?;

        // 5. Extract tar.gz to extract_to directory
        extractor::extract_tar_gz(&tar_gz_data, extract_to)?;

        Ok(config)
    }

    async fn push(
        &self,
        reference: &OciReference,
        config: &ArtifactConfig,
        skills_dir: &Path,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<String, OciSkillsError> {
        let oci_ref = self.to_oci_reference(reference)?;
        let _auth = auth::get_registry_auth_with_override(
            oci_ref.registry(),
            username,
            password,
        );

        // 1. Create tar.gz of skills_dir
        let tar_gz_data = packer::create_tar_gz(skills_dir)?;
        let layer_digest = format!("sha256:{}", Self::sha256_hex(&tar_gz_data));

        // 2. Push layer blob
        self.client
            .push_blob(&oci_ref, &tar_gz_data, &layer_digest)
            .await
            .map_err(|e| OciSkillsError::Oci(format!("Failed to push layer: {}", e)))?;

        let layer_desc = OciDescriptor {
            media_type: "application/vnd.agentskills.skill.v1.tar+gzip".to_string(),
            digest: layer_digest.clone(),
            size: tar_gz_data.len() as i64,
            ..Default::default()
        };

        // 3. Push config blob
        let config_json = serde_json::to_vec(config)?;
        let config_digest = format!("sha256:{}", Self::sha256_hex(&config_json));

        self.client
            .push_blob(&oci_ref, &config_json, &config_digest)
            .await
            .map_err(|e| OciSkillsError::Oci(format!("Failed to push config: {}", e)))?;

        let config_desc = OciDescriptor {
            media_type: "application/vnd.agentskills.config.v1+json".to_string(),
            digest: config_digest.clone(),
            size: config_json.len() as i64,
            ..Default::default()
        };

        // 4. Create and push manifest
        let manifest = OciImageManifest {
            schema_version: 2,
            media_type: Some("application/vnd.oci.image.manifest.v1+json".into()),
            config: config_desc,
            layers: vec![layer_desc],
            ..Default::default()
        };

        let manifest_digest = self
            .client
            .push_manifest(&oci_ref, &manifest.into())
            .await
            .map_err(|e| OciSkillsError::Oci(format!("Failed to push manifest: {}", e)))?;

        Ok(manifest_digest)
    }

    async fn inspect(
        &self,
        reference: &OciReference,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<ArtifactConfig, OciSkillsError> {
        let oci_ref = self.to_oci_reference(reference)?;
        let auth = auth::get_registry_auth_with_override(
            oci_ref.registry(),
            username,
            password,
        );

        // 1. Pull manifest (lightweight)
        let (manifest, _digest) = self
            .client
            .pull_manifest(&oci_ref, &auth)
            .await
            .map_err(|e| OciSkillsError::Oci(e.to_string()))?;

        // Extract image manifest
        let image_manifest = match manifest {
            oci_client::manifest::OciManifest::Image(img) => img,
            _ => return Err(OciSkillsError::Oci("Expected image manifest".into())),
        };

        // 2. Pull and parse config blob only (skip layer)
        let config_desc = &image_manifest.config;
        let mut config_bytes = Vec::new();
        self.client
            .pull_blob(&oci_ref, config_desc, &mut config_bytes)
            .await
            .map_err(|e| OciSkillsError::Oci(e.to_string()))?;

        let config: ArtifactConfig = serde_json::from_slice(&config_bytes)
            .map_err(|e| OciSkillsError::Oci(format!("Invalid config blob: {}", e)))?;

        Ok(config)
    }
}
