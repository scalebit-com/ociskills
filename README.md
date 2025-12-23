# OCI Skills CLI

A Rust-based CLI tool for managing [Agent Skills](https://agentskills.io) through OCI-compatible repositories.

**Website**: https://agentskills.io

## Overview

`ociskills` is a command-line tool that handles installation and publishing of Agent Skills to OCI-compatible registries. Skills are packaged as OCI artifacts and can be installed to user-level or project-level directories.

## What are Agent Skills?

Agent Skills are a **simple, open format** for giving AI agents new capabilities and expertise. Skills are folders of instructions, scripts, and resources that agents can discover and use to perform tasks more accurately and efficiently.

**Key benefits:**
- **For skill authors**: Build once, deploy across multiple agent platforms
- **For agents**: Load specialized knowledge on demand
- **For teams**: Capture organizational knowledge in portable, version-controlled packages

**Supported platforms**: Claude Code, VS Code, Cursor, GitHub Copilot, OpenAI tools, Letta, Goose, and more.

**References**:
- Specification: https://agentskills.io/specification
- GitHub: https://github.com/agentskills/agentskills
- Example skills: https://github.com/anthropics/skills

## Installation

### Prerequisites

- Rust 1.83 or later (see `rust-toolchain.toml`)
- Docker or Podman (for authentication to registries)

### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd ociskills

# Build in release mode
cargo build --release

# The binary will be at target/release/ociskills
# Optionally install it
cargo install --path crates/cli
```

## Default Skill Directories

The tool uses Claude Code-specific directories by default:

- **User-level (home)**: `~/.claude/skills` - Available across all projects
- **Project-level**: `.claude/skills` - Specific to current project

These defaults can be overridden via CLI flags or environment variables.

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OCI_SKILLS_HOME_SKILLS_DIR` | Override the user-level skills directory | `~/.claude/skills` |
| `OCI_SKILLS_PROJECT_SKILLS_DIR` | Override the project-level skills directory | `.claude/skills` |

**Resolution Order** (highest to lowest priority):
1. CLI flag (`-o`/`--output`)
2. Environment variable
3. Default Claude Code directory

## Usage

### Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--verbose` | `-v` | Enable verbose/debug output |
| `--quiet` | `-q` | Suppress non-error output |
| `--no-color` | | Disable colored output |
| `--help` | `-h` | Show help |
| `--version` | `-V` | Show version |

### Commands

#### `install` - Install skills from OCI registry

```bash
# Install to user home (~/.claude/skills) - default
ociskills install ghcr.io/myorg/skills:1.5

# Install to project directory (.claude/skills)
ociskills install ghcr.io/myorg/skills:1.5 --project

# Using immutable digest reference
ociskills install ghcr.io/myorg/skills@sha256:abc123...

# Override output directory entirely
ociskills install ghcr.io/myorg/skills:1.5 -o /custom/skills/dir

# Create directories if they don't exist
ociskills install ghcr.io/myorg/skills:1.5 --create-dirs

# Dry run - show what would be installed
ociskills install ghcr.io/myorg/skills:1.5 --dry-run

# Force overwrite without prompting
ociskills install ghcr.io/myorg/skills:1.5 --force
```

#### `publish` - Publish skills to OCI registry

```bash
# Publish single skill
ociskills publish ghcr.io/myorg/skills:1.0 ./pdf-skill

# Publish multiple skills
ociskills publish ghcr.io/myorg/skills:1.0 ./pdf-skill ./csv-skill

# Dry run - validate and show what would be published
ociskills publish ghcr.io/myorg/skills:1.0 ./pdf-skill --dry-run

# Add custom annotations
ociskills publish ghcr.io/myorg/skills:1.0 ./pdf-skill \
  --annotation "org.opencontainers.image.authors=Jane Doe"
```

#### `list` - List installed skills

```bash
# List skills in ~/.claude/skills (default)
ociskills list

# List skills in project directory
ociskills list --project

# List skills in custom directory
ociskills list -o /custom/skills/dir

# Output as JSON (for scripting)
ociskills list --json
```

#### `remove` - Remove installed skills

```bash
# Remove specific skills from home
ociskills remove pdf-skill csv-skill

# Remove from project directory
ociskills remove pdf-skill --project

# Force remove without confirmation
ociskills remove pdf-skill --force
```

#### `validate` - Validate skill directories

```bash
# Validate one or more skill directories
ociskills validate ./pdf-skill ./csv-skill
```

Validates skills locally without publishing. Checks:
- SKILL.md exists and has valid frontmatter
- Name format (lowercase, hyphens)
- Directory name matches skill name
- Description length limits

#### `inspect` - Inspect remote artifact

```bash
# Inspect a remote skill artifact
ociskills inspect ghcr.io/myorg/skills:1.0

# Output as JSON
ociskills inspect ghcr.io/myorg/skills:1.0 --json
```

Shows artifact metadata without downloading:
- Skills included (names, descriptions)
- Created timestamp
- Digest
- Size

#### `completion` - Generate shell completions

```bash
# Generate completions for your shell
ociskills completion bash > /etc/bash_completion.d/ociskills
ociskills completion zsh > ~/.zsh/completions/_ociskills
ociskills completion fish > ~/.config/fish/completions/ociskills.fish
```

Supported shells: `bash`, `zsh`, `fish`, `powershell`

## OCI Reference Format

This tool uses standard OCI reference format with an optional `oci://` prefix for clarity:

```
# Standard format (recommended)
registry.example.com/namespace/repo:tag
registry.example.com/namespace/repo@sha256:abc123...

# With optional oci:// prefix (for explicitness)
oci://registry.example.com/namespace/repo:tag
oci://registry.example.com/namespace/repo@sha256:abc123...
```

**Reference Components**:
- **Registry**: The OCI registry hostname (e.g., `ghcr.io`, `registry.example.com`)
- **Repository**: Namespace and name (e.g., `myorg/skills`)
- **Tag**: Mutable version reference (e.g., `:1.0`, `:latest`)
- **Digest**: Immutable content hash (e.g., `@sha256:abc123...`)

**Note**: Digests (`@sha256:...`) provide immutable, reproducible installations. Tags can change.

## Authentication

The tool uses Docker-compatible authentication:

```bash
# Login using docker or podman
docker login ghcr.io -u USERNAME -p TOKEN

# Then ociskills will automatically use these credentials
ociskills publish ghcr.io/myorg/skills:1.0 ./my-skill
```

Credentials are read from `~/.docker/config.json`.

## Project Structure

The project is organized as a Cargo workspace with 4 crates:

```
ociskills/
├── Cargo.toml                    # Workspace configuration
├── crates/
│   ├── traits/                   # Shared contracts & types
│   ├── oci/                      # OCI registry implementation
│   ├── core/                     # Business logic & services
│   └── cli/                      # CLI binary
└── README.md
```

### Architecture Principles

1. **IO traits only**: Traits for IO boundaries (filesystem, OCI registry, logging)
2. **Pure functions**: Business logic as pure functions
3. **IO at the edges**: All IO operations at boundaries, core logic is pure
4. **Simple DI**: Constructor injection, no complex containers
5. **Fast tests**: Mock IO traits, test business logic with unit tests

## Development

### Build

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Format Code

```bash
cargo fmt
```

### Lint

```bash
cargo clippy
```

## License

[Add license information]

## Links

- Agent Skills Website: https://agentskills.io
- Specification: https://agentskills.io/specification
- Example Skills: https://github.com/anthropics/skills
