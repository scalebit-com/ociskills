# Code Review

This document provides a review of the OCI Skills CLI codebase, highlighting areas for improvement and suggesting concrete solutions.

## Table of Contents

1.  [Inconsistent Error Handling in `oci/auth.rs`](#inconsistent-error-handling-in-ociauthrs)
2.  [Magic Strings for OCI Media Types in `oci/lib.rs`](#magic-strings-for-oci-media-types-in-ocilibrs)
3.  [Unclear `push` Method Return Value in `traits/io.rs`](#unclear-push-method-return-value-in-traitsiors)
4.  [Redundant `to_string_without_prefix` Method in `traits/models.rs`](#redundant-to_string_without_prefix-method-in-traitsmodelsrs)
5.  [Potential for Panic in `cli/main.rs`](#potential-for-panic-in-climainrs)

---

## 1. Inconsistent Error Handling in `oci/auth.rs`

**Issue:** The `get_registry_auth` function in `oci/auth.rs` returns `RegistryAuth::Anonymous` in multiple failure scenarios. This approach makes it difficult to debug authentication issues because the caller has no information about *why* the authentication failed (e.g., missing config file, invalid JSON, decoding error).

**Code Example:**

```rust
// crates/oci/src/auth.rs

pub fn get_registry_auth(registry: &str) -> RegistryAuth {
    let config_path = dirs::home_dir()
        .map(|h| h.join(".docker/config.json"))
        .filter(|p| p.exists());

    let Some(path) = config_path else {
        return RegistryAuth::Anonymous; // Reason for failure is lost
    };

    let Ok(content) = std::fs::read_to_string(&path) else {
        return RegistryAuth::Anonymous; // Reason for failure is lost
    };

    let Ok(config) = serde_json::from_str::<DockerConfig>(&content) else {
        return RegistryAuth::Anonymous; // Reason for failure is lost
    };

    // ... more silent failures
}
```

**Proposed Solution:**

Refactor the function to return a `Result<RegistryAuth, OciSkillsError>` to provide more specific error information. This will improve debuggability and make the function's behavior more explicit.

**Revised Code:**

```rust
// crates/oci/src/auth.rs (proposed)
use traits::OciSkillsError;

pub fn get_registry_auth(registry: &str) -> Result<RegistryAuth, OciSkillsError> {
    let config_path = dirs::home_dir()
        .map(|h| h.join(".docker/config.json"))
        .ok_or_else(|| OciSkillsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Home directory not found",
        )))?;

    if !config_path.exists() {
        return Ok(RegistryAuth::Anonymous); // Explicitly anonymous if no config
    }

    let content = std::fs::read_to_string(&config_path)?;
    let config: DockerConfig = serde_json::from_str(&content)?;

    // ... logic to find auth entry ...

    let Some(entry) = auth_entry else {
        return Ok(RegistryAuth::Anonymous);
    };

    let Some(auth_b64) = &entry.auth else {
        return Ok(RegistryAuth::Anonymous);
    };

    let decoded = STANDARD.decode(auth_b64)
        .map_err(|e| OciSkillsError::Oci(format!("Invalid base64 in auth config: {}", e)))?;

    let auth_str = String::from_utf8(decoded)
        .map_err(|e| OciSkillsError::Oci(format!("Invalid UTF-8 in auth config: {}", e)))?;

    let Some((username, password)) = auth_str.split_once(':') else {
        return Err(OciSkillsError::Oci("Invalid username:password format in auth config".into()));
    };

    Ok(RegistryAuth::Basic(username.to_string(), password.to_string()))
}
```

---

## 2. Magic Strings for OCI Media Types in `oci/lib.rs`

**Issue:** The OCI media types in `oci/lib.rs` are hardcoded as strings. This increases the risk of typos and makes it harder to maintain or update these values if they change.

**Code Example:**

```rust
// crates/oci/src/lib.rs

let layer_desc = OciDescriptor {
    media_type: "application/vnd.agentskills.skill.v1.tar+gzip".to_string(),
    // ...
};

// ...

let config_desc = OciDescriptor {
    media_type: "application/vnd.agentskills.config.v1+json".to_string(),
    // ...
};

// ...

let manifest = OciImageManifest {
    media_type: Some("application/vnd.oci.image.manifest.v1+json".into()),
    // ...
};
```

**Proposed Solution:**

Define these media types as constants in a dedicated module (e.g., `traits::models`). This centralizes the values, reduces the chance of typos, and makes the code more readable and maintainable.

**Revised Code:**

```rust
// crates/traits/src/models.rs (proposed)
pub mod media_types {
    pub const OCI_IMAGE_MANIFEST: &str = "application/vnd.oci.image.manifest.v1+json";
    pub const SKILL_LAYER: &str = "application/vnd.agentskills.skill.v1.tar+gzip";
    pub const SKILL_CONFIG: &str = "application/vnd.agentskills.config.v1+json";
}

// crates/oci/src/lib.rs (proposed)
use traits::models::media_types;

// ...

let layer_desc = OciDescriptor {
    media_type: media_types::SKILL_LAYER.to_string(),
    // ...
};

let config_desc = OciDescriptor {
    media_type: media_types::SKILL_CONFIG.to_string(),
    // ...
};

let manifest = OciImageManifest {
    media_type: Some(media_types::OCI_IMAGE_MANIFEST.into()),
    // ...
};
```

---

## 3. Unclear `push` Method Return Value in `traits/io.rs`

**Issue:** The `push` method in the `OciClient` trait returns a `String`, which represents the manifest digest. The method name `push` does not clearly communicate what value is being returned.

**Code Example:**

```rust
// crates/traits/src/io.rs

#[async_trait]
pub trait OciClient: Send + Sync {
    // ...
    async fn push(
        &self,
        reference: &OciReference,
        config: &ArtifactConfig,
        skills_dir: &Path,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<String, OciSkillsError>; // Returns a digest string
    // ...
}
```

**Proposed Solution:**

Create a `PushResponse` struct to make the return type more explicit. This improves code clarity and makes it easier to extend the return value in the future without breaking changes.

**Revised Code:**

```rust
// crates/traits/src/models.rs (proposed)
#[derive(Debug, Clone)]
pub struct PushResponse {
    pub manifest_digest: String,
}

// crates/traits/src/io.rs (proposed)
use crate::models::PushResponse;

#[async_trait]
pub trait OciClient: Send + Sync {
    // ...
    async fn push(
        &self,
        reference: &OciReference,
        config: &ArtifactConfig,
        skills_dir: &Path,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<PushResponse, OciSkillsError>;
    // ...
}

// crates/oci/src/lib.rs (proposed)
async fn push(...) -> Result<PushResponse, OciSkillsError> {
    // ...
    let manifest_digest = self.client.push_manifest(&oci_ref, &manifest.into()).await?;
    Ok(PushResponse { manifest_digest })
}
```

---

## 4. Redundant `to_string_without_prefix` Method in `traits/models.rs`

**Issue:** The `OciReference` struct has a `to_string_without_prefix` method, which is then used by the `Display` implementation. This is an unnecessary layer of abstraction. The `Display` implementation can be implemented directly for a more idiomatic and concise solution.

**Code Example:**

```rust
// crates/traits/src/models.rs

impl OciReference {
    // ...
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
```

**Proposed Solution:**

Implement the `Display` trait directly and remove the `to_string_without_prefix` method.

**Revised Code:**

```rust
// crates/traits/src/models.rs (proposed)

impl std::fmt::Display for OciReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.reference {
            TagOrDigest::Tag(tag) => write!(f, "{}/{}:{}", self.registry, self.repository, tag),
            TagOrDigest::Digest(digest) => write!(f, "{}/{}@{}", self.registry, self.repository, digest),
        }
    }
}
```

---

## 5. Potential for Panic in `cli/main.rs`

**Issue:** The `main` function in `cli/main.rs` uses `std::process::exit(1)` to handle errors. This is not idiomatic Rust and can prevent proper resource cleanup (e.g., running destructors).

**Code Example:**

```rust
// crates/cli/src/main.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...
    let oci: Arc<dyn traits::OciClient> = match oci_result {
        Ok(client) => Arc::new(client),
        Err(e) => {
            logger.error(&format!("Failed to initialize OCI client: {}", e));
            std::process::exit(1); // Abrupt exit
        }
    };

    // ... similar exits for other errors
}
```

**Proposed Solution:**

Refactor the `main` function to return a `Result` and use the `?` operator for error propagation. This allows for a single, clean exit point and ensures that all resources are properly deallocated.

**Revised Code:**

```rust
// crates/cli/src/main.rs (proposed)

// Define a dedicated error type for the CLI
#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("OCI client initialization failed: {0}")]
    OciInit(#[from] traits::OciSkillsError),
    #[error("Installation failed: {0}")]
    Install(traits::OciSkillsError),
    // ... other error variants
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        // Create a logger just for the final error message
        let logger = ConsoleLogger::new(true, true, false);
        logger.error(&format!("{}", e));
        std::process::exit(1);
    }
}

async fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let logger = Arc::new(ConsoleLogger::new(!cli.no_color, cli.verbose, cli.quiet));

    let oci = Arc::new(OciClientImpl::new().map_err(CliError::OciInit)?);

    match cli.command {
        Commands::Install { .. } => {
            // ...
            let installer = Installer::new(...);
            installer.install(&reference, &options).await.map_err(CliError::Install)?;
        }
        // ... other commands
    }

    Ok(())
}
```
