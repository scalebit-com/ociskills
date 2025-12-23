use base64::{engine::general_purpose::STANDARD, Engine};
use oci_client::secrets::RegistryAuth;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct DockerConfig {
    auths: HashMap<String, DockerAuthEntry>,
}

#[derive(Deserialize)]
struct DockerAuthEntry {
    auth: Option<String>, // base64(username:password)
}

/// Read Docker config and return appropriate RegistryAuth
pub fn get_registry_auth(registry: &str) -> RegistryAuth {
    let config_path = dirs::home_dir()
        .map(|h| h.join(".docker/config.json"))
        .filter(|p| p.exists());

    let Some(path) = config_path else {
        return RegistryAuth::Anonymous;
    };

    let Ok(content) = std::fs::read_to_string(&path) else {
        return RegistryAuth::Anonymous;
    };

    let Ok(config) = serde_json::from_str::<DockerConfig>(&content) else {
        return RegistryAuth::Anonymous;
    };

    // Try exact match, then with https://, then docker.io special case
    let auth_entry = config
        .auths
        .get(registry)
        .or_else(|| config.auths.get(&format!("https://{}", registry)))
        .or_else(|| {
            if registry == "docker.io" || registry == "index.docker.io" {
                config.auths.get("https://index.docker.io/v1/")
            } else {
                None
            }
        });

    let Some(entry) = auth_entry else {
        return RegistryAuth::Anonymous;
    };

    let Some(auth_b64) = &entry.auth else {
        return RegistryAuth::Anonymous;
    };

    let Ok(decoded) = STANDARD.decode(auth_b64) else {
        return RegistryAuth::Anonymous;
    };

    let Ok(auth_str) = String::from_utf8(decoded) else {
        return RegistryAuth::Anonymous;
    };

    let Some((username, password)) = auth_str.split_once(':') else {
        return RegistryAuth::Anonymous;
    };

    RegistryAuth::Basic(username.to_string(), password.to_string())
}
