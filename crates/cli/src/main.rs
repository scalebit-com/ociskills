use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use core::{
    ArtifactInspector, ConsoleLogger, Installer, Publisher, RealEnvironment, RealFileSystem,
    SkillLister, SkillRemover, SkillValidator,
};
use oci::OciClientImpl;
use std::path::PathBuf;
use std::sync::Arc;
use traits::{InstallOptions, InstallScope, Logger, OciReference, PublishOptions};

#[derive(Parser)]
#[command(name = "ociskills")]
#[command(about = "Manage Agent Skills via OCI registries")]
#[command(version)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install skills from an OCI registry
    Install {
        /// OCI reference (e.g., ghcr.io/org/skills:1.0)
        reference: String,

        /// Target directory (overrides --home/--project)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Install to ~/.claude/skills (default)
        #[arg(long)]
        home: bool,

        /// Install to ./.claude/skills
        #[arg(long, conflicts_with = "home")]
        project: bool,

        /// Create target directory if it doesn't exist
        #[arg(long)]
        create_dirs: bool,

        /// Show what would be installed without installing
        #[arg(long)]
        dry_run: bool,

        /// Overwrite existing skills without prompting
        #[arg(long)]
        force: bool,

        /// Registry username (overrides Docker config and OCI_USERNAME env var)
        #[arg(long)]
        username: Option<String>,

        /// Registry password (overrides Docker config and OCI_PASSWORD env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Publish skills to an OCI registry
    Publish {
        /// OCI reference (e.g., ghcr.io/org/skills:1.0)
        reference: String,

        /// Skill directories to publish
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Show what would be published without publishing
        #[arg(long)]
        dry_run: bool,

        /// Add custom annotation (can be repeated)
        #[arg(long, value_name = "KEY=VALUE")]
        annotation: Vec<String>,

        /// Registry username (overrides Docker config and OCI_USERNAME env var)
        #[arg(long)]
        username: Option<String>,

        /// Registry password (overrides Docker config and OCI_PASSWORD env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// List installed skills
    List {
        /// List skills in ~/.claude/skills (default)
        #[arg(long)]
        home: bool,

        /// List skills in ./.claude/skills
        #[arg(long)]
        project: bool,

        /// List skills in specific directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Remove installed skills
    Remove {
        /// Skill names to remove
        #[arg(required = true)]
        skills: Vec<String>,

        /// Remove from ~/.claude/skills (default)
        #[arg(long)]
        home: bool,

        /// Remove from ./.claude/skills
        #[arg(long)]
        project: bool,

        /// Remove from specific directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Don't prompt for confirmation
        #[arg(long)]
        force: bool,
    },

    /// Validate skill directories
    Validate {
        /// Skill directories to validate
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },

    /// Inspect a remote OCI artifact
    Inspect {
        /// OCI reference to inspect
        reference: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Registry username (overrides Docker config and OCI_USERNAME env var)
        #[arg(long)]
        username: Option<String>,

        /// Registry password (overrides Docker config and OCI_PASSWORD env var)
        #[arg(long)]
        password: Option<String>,
    },

    /// Generate shell completions
    Completion {
        /// Shell type
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Configure logger based on flags
    let logger = Arc::new(ConsoleLogger::new(!cli.no_color, cli.verbose, cli.quiet));

    // Wire dependencies
    let fs = Arc::new(RealFileSystem::new());
    let env = Arc::new(RealEnvironment::new());
    let oci_result = OciClientImpl::new();

    let oci: Arc<dyn traits::OciClient> = match oci_result {
        Ok(client) => Arc::new(client),
        Err(e) => {
            logger.error(&format!("Failed to initialize OCI client: {}", e));
            std::process::exit(1);
        }
    };

    match cli.command {
        Commands::Install {
            reference,
            output,
            home,
            project,
            create_dirs,
            dry_run,
            force,
            username,
            password,
        } => {
            let scope = if project {
                InstallScope::Project
            } else {
                InstallScope::Home
            };
            let options = InstallOptions {
                scope,
                output_override: output,
                create_dirs,
                dry_run,
                force,
                username,
                password,
            };

            let reference = match OciReference::parse(&reference) {
                Ok(r) => r,
                Err(e) => {
                    logger.error(&format!("Invalid reference: {}", e));
                    std::process::exit(1);
                }
            };

            let installer = Installer::new(
                fs.clone(),
                oci.clone(),
                env.clone(),
                logger.clone(),
            );

            if let Err(e) = installer.install(&reference, &options).await {
                logger.error(&format!("Installation failed: {}", e));
                std::process::exit(1);
            }
        }

        Commands::Publish {
            reference,
            paths,
            dry_run,
            annotation,
            username,
            password,
        } => {
            let options = PublishOptions {
                dry_run,
                annotations: annotation,
                username,
                password,
            };

            let reference = match OciReference::parse(&reference) {
                Ok(r) => r,
                Err(e) => {
                    logger.error(&format!("Invalid reference: {}", e));
                    std::process::exit(1);
                }
            };

            let publisher = Publisher::new(fs.clone(), oci.clone(), logger.clone());

            match publisher.publish(&reference, &paths, &options).await {
                Ok(digest) => {
                    if !dry_run {
                        println!("Published: {}@{}", reference, digest);
                    }
                }
                Err(e) => {
                    logger.error(&format!("Publish failed: {}", e));
                    std::process::exit(1);
                }
            }
        }

        Commands::List {
            home,
            project,
            output,
            json,
        } => {
            let scope = if project {
                InstallScope::Project
            } else {
                InstallScope::Home
            };

            let lister = SkillLister::new(fs.clone(), env.clone(), logger.clone());

            if let Err(e) = lister.list(scope, output, json).await {
                logger.error(&format!("List failed: {}", e));
                std::process::exit(1);
            }
        }

        Commands::Remove {
            skills,
            home,
            project,
            output,
            force,
        } => {
            let scope = if project {
                InstallScope::Project
            } else {
                InstallScope::Home
            };

            let remover = SkillRemover::new(fs.clone(), env.clone(), logger.clone());

            if let Err(e) = remover.remove(&skills, scope, output, force).await {
                logger.error(&format!("Remove failed: {}", e));
                std::process::exit(1);
            }
        }

        Commands::Validate { paths } => {
            let validator = SkillValidator::new(fs.clone(), logger.clone());

            match validator.validate(&paths).await {
                Ok(_) => {
                    logger.info("All skills are valid");
                }
                Err(e) => {
                    logger.error(&format!("Validation failed: {}", e));
                    std::process::exit(1);
                }
            }
        }

        Commands::Inspect {
            reference,
            json,
            username,
            password,
        } => {
            let reference = match OciReference::parse(&reference) {
                Ok(r) => r,
                Err(e) => {
                    logger.error(&format!("Invalid reference: {}", e));
                    std::process::exit(1);
                }
            };

            let inspector = ArtifactInspector::new(oci.clone(), logger.clone());

            if let Err(e) = inspector.inspect(&reference, json, username, password).await {
                logger.error(&format!("Inspect failed: {}", e));
                std::process::exit(1);
            }
        }

        Commands::Completion { shell } => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "ociskills",
                &mut std::io::stdout(),
            );
        }
    }

    Ok(())
}
