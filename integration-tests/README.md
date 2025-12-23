# OCI Skills CLI - Integration Tests

Comprehensive integration tests for the OCI Skills CLI tool. These tests verify end-to-end functionality by creating test skills, publishing them to an OCI registry, and validating the published artifacts.

## Overview

The integration test suite:

- **Builds** the Rust CLI from source
- **Creates** test skills programmatically with proper structure
- **Validates** skills locally before publishing
- **Publishes** skills to an OCI registry
- **Inspects** published artifacts and validates metadata
- **Logs** extensively with GitHub Actions error formatting
- **Manages** test directories with automatic cleanup

## Prerequisites

1. **UV package manager** - Install with `brew install uv`
2. **Rust toolchain** - For building the CLI
3. **OCI registry access** - GitHub Container Registry (ghcr.io) or similar
4. **Registry credentials** - Username and password/token

## Setup

### 1. Configure Credentials

Copy the example environment file and fill in your values:

```bash
cd integration-tests
cp .env.example .env
```

Edit `.env` with your registry details:

```bash
# Registry URL (base only)
OCI_REGISTRY_URL=ghcr.io/your-org

# Credentials
OCI_USERNAME=your-username
OCI_PASSWORD=your-github-token
```

**For GitHub Container Registry (ghcr.io):**
- Username: Your GitHub username
- Password: Personal Access Token with `packages:write` scope
- Create token at: https://github.com/settings/tokens/new

### 2. Verify Prerequisites

```bash
# Check UV is installed
uv --version

# Check Rust is installed
cargo --version
```

## Running Tests

### Using UV (Recommended)

UV automatically manages Python dependencies:

```bash
# Run all tests
uv run run_tests.py

# Keep test directory after run
uv run run_tests.py --keep
```

### Using CLI Parameters

Override environment variables with command-line flags:

```bash
uv run run_tests.py \
  --repository-url ghcr.io/myorg \
  --username myuser \
  --password mytoken
```

### Using Python Directly

After installing dependencies:

```bash
pip install python-dotenv
python run_tests.py
```

## Configuration

### Priority Order

The test script loads configuration in this order (highest priority first):

1. **Command-line arguments** (`--repository-url`, `--username`, `--password`)
2. **Environment variables from .env file** (in `integration-tests/`)
3. **System environment variables** (`OCI_REGISTRY_URL`, `OCI_USERNAME`, `OCI_PASSWORD`)

### Required Parameters

- **Registry URL**: Base URL like `ghcr.io/myorg` (script appends `/test-skills:timestamp`)
- **Username**: Registry username
- **Password**: Registry password or token

### Optional Parameters

- `--keep`: Keep test directory after completion (default: cleanup on success)

## What Gets Published

The test creates and publishes skills to your registry with timestamped tags:

```
ghcr.io/your-org/test-skills:20251223143700
```

Each test run creates a unique artifact with the current timestamp.

## Test Directory Management

### Automatic Cleanup

- **On success**: Test directory is automatically deleted
- **On failure**: Test directory is kept for debugging
- **With `--keep` flag**: Directory is always kept

### Test Directory Format

```
integration_tests_YYYYMMDD_HHMM/
└── test-integration-skill/
    ├── SKILL.md          # Valid skill with frontmatter
    └── scripts/
        └── example.sh    # Sample script
```

These directories are gitignored via `integration_tests_*/` pattern.

## GitHub Actions Integration

The test script outputs errors in GitHub Actions format:

```
::error file=run_tests.py,line=0::Test suite failed: ...
```

This ensures test failures appear prominently in GitHub Actions logs and annotations.

### Example GitHub Actions Workflow

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install UV
        run: curl -LsSf https://astral.sh/uv/install.sh | sh

      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Run Integration Tests
        env:
          OCI_REGISTRY_URL: ghcr.io/${{ github.repository_owner }}
          OCI_USERNAME: ${{ github.actor }}
          OCI_PASSWORD: ${{ secrets.GITHUB_TOKEN }}
        run: |
          cd integration-tests
          uv run run_tests.py
```

## Test Output

### Successful Run

```
============================================================
  OCI Skills CLI Integration Tests
============================================================

→ Loading configuration
  ✓ Configuration loaded
  Registry: ghcr.io/myorg
  Username: myuser
  Binary: /path/to/target/release/ociskills

→ Building CLI binary
  $ cargo build --release
  ✓ CLI built successfully

============================================================
  Running Integration Tests
============================================================
  Created test directory: /path/to/integration_tests_20251223_1430

============================================================
  Test: Publish Skill
============================================================

→ Creating test skill
  ✓ Created skill at: /path/to/test-integration-skill

→ Validating skill locally
  ✓ Skill validation passed

→ Publishing skill to registry
  Publishing to: ghcr.io/myorg/test-skills:20251223143000
  ✓ Published successfully

→ Inspecting published artifact
  ✓ Artifact inspection validated

============================================================
  ✓ Test Passed: Publish Skill
============================================================

============================================================
  ✓ All Tests Passed
============================================================
  Cleaned up test directory
```

### Failed Run

Errors are logged in GitHub Actions format and the test directory is kept:

```
::error file=run_tests.py,line=0::test_publish_skill failed: ...
  ✗ Test failed: Publish failed...
  Kept test directory (test failure): /path/to/integration_tests_20251223_1430
```

## Troubleshooting

### Missing Credentials

```
Error: Missing required configuration: --repository-url or OCI_REGISTRY_URL
```

**Solution**: Create `.env` file or pass CLI parameters

### Build Failures

```
Error: CLI build failed
```

**Solution**: Check Rust toolchain and run `cargo build --release` manually

### Authentication Failures

```
Error: Publish failed: authentication required
```

**Solutions**:
- Verify credentials are correct
- For ghcr.io, ensure token has `packages:write` scope
- Check registry URL format (should be `ghcr.io/org`, not `https://ghcr.io`)

### Permission Denied

```
Error: Permission denied when pushing to registry
```

**Solution**: Ensure your user has push access to the repository/organization

## Extending Tests

Add new test functions to `run_tests.py`:

```python
def test_install_skill(config: TestConfig, test_dir: Path) -> None:
    """Test: Install a published skill"""
    log_section("Test: Install Skill")

    # 1. Publish a skill
    # 2. Install it to custom directory
    # 3. Verify files exist
    # 4. Verify SKILL.md content
```

Then call them from `run_all_tests()`:

```python
def run_all_tests(config: TestConfig, test_manager: TestDirectoryManager) -> None:
    test_dir = test_manager.create_test_directory()

    test_publish_skill(config, test_dir)
    test_install_skill(config, test_dir)  # Add new test
```

## Architecture

### Core Components

- **CliController**: Builds CLI and executes commands with typed results
- **SkillBuilder**: Programmatically creates valid test skills
- **TestDirectoryManager**: Manages test directory lifecycle and cleanup
- **TestConfig**: Type-safe configuration management

### Type Safety

All functions use proper Python typing:

```python
def publish(
    self,
    reference: str,
    skill_paths: list[Path],
    username: str,
    password: str,
    dry_run: bool = False
) -> CommandResult:
```

### Logging Strategy

- **INFO**: Normal progress messages
- **SUCCESS**: Completed steps (with ✓)
- **ERROR**: Test failures (with ✗) in GitHub Actions format
- **Section headers**: Visual separators for test phases

## Files

- `run_tests.py` - Main test script (UV script format)
- `.env.example` - Configuration template
- `.env` - Your credentials (gitignored)
- `README.md` - This file

## License

Same as the parent project.
