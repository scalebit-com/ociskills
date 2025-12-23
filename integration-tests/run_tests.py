#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "python-dotenv>=1.0.0",
# ]
# ///

"""
OCI Skills CLI Integration Tests

This script runs comprehensive integration tests for the OCI Skills CLI tool.
It tests the publish command by creating test skills and publishing them to
an OCI registry.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Optional, TypedDict


# ============================================================================
# Type Definitions
# ============================================================================

class CommandResult(TypedDict):
    """Result from CLI command execution"""
    exit_code: int
    stdout: str
    stderr: str
    success: bool


@dataclass
class TestConfig:
    """Consolidated test configuration"""
    registry_url: str
    username: str
    password: str
    keep_test_dir: bool
    test_dir_base: Path
    cli_binary: Path
    project_root: Path


# ============================================================================
# Logging Functions
# ============================================================================

def log_section(title: str) -> None:
    """Log section header"""
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print(f"{'=' * 60}")


def log_step(step: str) -> None:
    """Log individual step"""
    print(f"\n→ {step}")


def log_info(message: str) -> None:
    """Log info message"""
    print(f"  {message}")


def log_success(message: str) -> None:
    """Log success message"""
    print(f"  ✓ {message}")


def log_error(message: str, exc: Optional[Exception] = None) -> None:
    """Log error message"""
    print(f"  ✗ {message}", file=sys.stderr)
    if exc:
        print(f"    {exc}", file=sys.stderr)


def log_command(cmd: list[str]) -> None:
    """Log command being executed"""
    print(f"  $ {' '.join(cmd)}")


def log_output(label: str, content: str, max_lines: int = 20) -> None:
    """Log command output (truncated if too long)"""
    if not content.strip():
        return

    lines = content.strip().split('\n')
    if len(lines) > max_lines:
        print(f"  {label} (first {max_lines} lines):")
        for line in lines[:max_lines]:
            print(f"    {line}")
        print(f"    ... ({len(lines) - max_lines} more lines)")
    else:
        print(f"  {label}:")
        for line in lines:
            print(f"    {line}")


def log_github_error(file: str, line: int, message: str) -> None:
    """Log error in GitHub Actions format"""
    print(f"::error file={file},line={line}::{message}", file=sys.stderr)


def log_github_warning(file: str, line: int, message: str) -> None:
    """Log warning in GitHub Actions format"""
    print(f"::warning file={file},line={line}::{message}", file=sys.stderr)


# ============================================================================
# SkillBuilder Class
# ============================================================================

class SkillBuilder:
    """Create valid test skills programmatically"""

    @staticmethod
    def create_test_skill(
        parent_dir: Path,
        skill_name: str,
        description: str,
        license: Optional[str] = "MIT"
    ) -> Path:
        """Create a complete, valid test skill"""
        skill_dir = parent_dir / skill_name
        skill_dir.mkdir(parents=True, exist_ok=True)

        # Create SKILL.md with proper frontmatter
        skill_md = SkillBuilder._generate_skill_md(
            name=skill_name,
            description=description,
            license=license
        )

        (skill_dir / "SKILL.md").write_text(skill_md)

        # Create scripts directory with example
        scripts_dir = skill_dir / "scripts"
        scripts_dir.mkdir(exist_ok=True)
        (scripts_dir / "example.sh").write_text("#!/bin/bash\necho 'Hello from test skill'")

        log_success(f"Created skill at: {skill_dir}")
        return skill_dir

    @staticmethod
    def _generate_skill_md(
        name: str,
        description: str,
        license: Optional[str]
    ) -> str:
        """Generate SKILL.md content with YAML frontmatter"""
        frontmatter = f"""---
name: {name}
description: {description}"""

        if license:
            frontmatter += f"\nlicense: {license}"

        frontmatter += "\n---\n"

        body = f"""
# {name.replace('-', ' ').title()}

{description}

## Usage

This is a test skill created for integration testing of the OCI Skills CLI.

## Capabilities

- Test publishing to OCI registry
- Test validation of skill structure
- Test artifact inspection

## Requirements

None - this is a minimal test skill.
"""

        return frontmatter + body


# ============================================================================
# TestDirectoryManager Class
# ============================================================================

class TestDirectoryManager:
    """Manage test directory lifecycle"""

    def __init__(self, project_root: Path, keep: bool):
        self.project_root = project_root
        self.keep = keep
        self.test_dir: Optional[Path] = None
        self.test_failed = False

    def create_test_directory(self) -> Path:
        """Create integration_tests_YYYYMMDD_HHMM directory"""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M")
        test_dir = self.project_root / f"integration_tests_{timestamp}"
        test_dir.mkdir(parents=True, exist_ok=True)
        self.test_dir = test_dir
        log_info(f"Created test directory: {test_dir}")
        return test_dir

    def mark_test_failed(self) -> None:
        """Mark that test failed (affects cleanup)"""
        self.test_failed = True

    def cleanup(self) -> None:
        """Clean up test directory based on keep flag and test status"""
        if self.test_dir is None:
            return

        should_delete = not self.keep and not self.test_failed

        if should_delete:
            shutil.rmtree(self.test_dir)
            log_info(f"Cleaned up test directory: {self.test_dir}")
        else:
            reason = "keep flag" if self.keep else "test failure"
            log_info(f"Kept test directory ({reason}): {self.test_dir}")


# ============================================================================
# CliController Class
# ============================================================================

class CliController:
    """Control CLI binary lifecycle and execution"""

    def __init__(self, binary_path: Path, project_root: Path):
        self.binary_path = binary_path
        self.project_root = project_root

    def build_cli(self) -> bool:
        """Build Rust CLI using cargo"""
        log_step("Building CLI binary")
        log_command(["cargo", "build", "--release"])

        try:
            result = subprocess.run(
                ["cargo", "build", "--release"],
                cwd=self.project_root,
                capture_output=True,
                text=True,
                timeout=300
            )

            if result.returncode == 0:
                log_success("CLI built successfully")
                log_output("Build output", result.stdout, max_lines=10)
                return True
            else:
                log_error("CLI build failed")
                log_output("Build stderr", result.stderr)
                return False

        except subprocess.TimeoutExpired:
            log_error("CLI build timed out")
            return False
        except Exception as e:
            log_error("CLI build error", e)
            return False

    def run_command(
        self,
        args: list[str],
        timeout: int = 60
    ) -> CommandResult:
        """Execute CLI command and return structured result"""
        cmd = [str(self.binary_path)] + args
        log_command(cmd)

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout
            )

            return CommandResult(
                exit_code=result.returncode,
                stdout=result.stdout,
                stderr=result.stderr,
                success=result.returncode == 0
            )

        except subprocess.TimeoutExpired:
            return CommandResult(
                exit_code=-1,
                stdout="",
                stderr=f"Command timed out after {timeout}s",
                success=False
            )
        except Exception as e:
            return CommandResult(
                exit_code=-1,
                stdout="",
                stderr=str(e),
                success=False
            )

    def validate(self, skill_paths: list[Path]) -> CommandResult:
        """Validate skill directories"""
        args = ["validate"] + [str(p) for p in skill_paths]
        return self.run_command(args)

    def publish(
        self,
        reference: str,
        skill_paths: list[Path],
        username: str,
        password: str,
        dry_run: bool = False
    ) -> CommandResult:
        """Publish skills to registry"""
        args = [
            "publish",
            reference,
            "--username", username,
            "--password", password,
        ]

        if dry_run:
            args.append("--dry-run")

        args.extend([str(p) for p in skill_paths])

        return self.run_command(args, timeout=120)

    def inspect(
        self,
        reference: str,
        username: str,
        password: str,
        json_output: bool = True
    ) -> CommandResult:
        """Inspect remote artifact"""
        args = [
            "inspect",
            reference,
            "--username", username,
            "--password", password,
        ]

        if json_output:
            args.append("--json")

        return self.run_command(args)


# ============================================================================
# Configuration Loading
# ============================================================================

def load_config(cli_args: argparse.Namespace) -> TestConfig:
    """Load and validate test configuration"""
    from dotenv import load_dotenv

    # Load .env file if present (in integration-tests directory)
    script_dir = Path(__file__).parent
    env_file = script_dir / ".env"
    if env_file.exists():
        load_dotenv(env_file)
        log_info(f"Loaded .env from: {env_file}")

    # Get values with priority: CLI args > .env > system env
    registry_url = (
        cli_args.repository_url or
        os.getenv("OCI_REGISTRY_URL") or
        os.getenv("OCI_REPOSITORY_URL")
    )

    username = cli_args.username or os.getenv("OCI_USERNAME")
    password = cli_args.password or os.getenv("OCI_PASSWORD")

    # Validate required parameters
    missing = []
    if not registry_url:
        missing.append("--repository-url or OCI_REGISTRY_URL/OCI_REPOSITORY_URL")
    if not username:
        missing.append("--username or OCI_USERNAME")
    if not password:
        missing.append("--password or OCI_PASSWORD")

    if missing:
        error_msg = f"Missing required configuration: {', '.join(missing)}"
        log_github_error(__file__, 0, error_msg)
        log_error(error_msg)
        raise ValueError(error_msg)

    # Derive paths
    project_root = script_dir.parent.resolve()
    cli_binary = project_root / "target" / "release" / "ociskills"

    return TestConfig(
        registry_url=registry_url,
        username=username,
        password=password,
        keep_test_dir=cli_args.keep,
        test_dir_base=project_root,
        cli_binary=cli_binary,
        project_root=project_root
    )


# ============================================================================
# Test Functions
# ============================================================================

def test_publish_skill(config: TestConfig, test_dir: Path) -> None:
    """Test: Create and publish a skill"""
    log_section("Test: Publish Skill")

    try:
        # 1. Create test skill
        log_step("Creating test skill")
        skill_dir = SkillBuilder.create_test_skill(
            parent_dir=test_dir,
            skill_name="test-integration-skill",
            description="A test skill for integration testing of OCI Skills CLI",
            license="MIT"
        )

        # 2. Validate skill locally
        log_step("Validating skill locally")
        cli = CliController(config.cli_binary, config.project_root)
        result = cli.validate([skill_dir])

        log_output("Validation stdout", result["stdout"])
        if result["stderr"]:
            log_output("Validation stderr", result["stderr"])

        if not result["success"]:
            raise AssertionError(f"Validation failed: {result['stderr']}")

        log_success("Skill validation passed")

        # 3. Construct OCI reference with timestamp
        timestamp = datetime.now().strftime("%Y%m%d%H%M%S")
        reference = f"{config.registry_url}/test-skills:{timestamp}"
        log_info(f"Publishing to: {reference}")

        # 4. Publish skill
        log_step("Publishing skill to registry")
        result = cli.publish(
            reference=reference,
            skill_paths=[skill_dir],
            username=config.username,
            password=config.password,
            dry_run=False
        )

        log_output("Publish stdout", result["stdout"])
        if result["stderr"]:
            log_output("Publish stderr", result["stderr"])

        if not result["success"]:
            raise AssertionError(f"Publish failed (exit code {result['exit_code']}): {result['stderr']}")

        # Extract digest from output
        output_lines = result["stdout"].strip().split('\n')
        publish_line = [line for line in output_lines if line.startswith("Published:")]

        if not publish_line:
            log_error("Could not find publish confirmation in output")
            raise AssertionError("Publish output missing confirmation")

        log_success(f"Published successfully: {publish_line[0]}")

        # 5. Inspect published artifact
        log_step("Inspecting published artifact")
        result = cli.inspect(
            reference=reference,
            username=config.username,
            password=config.password,
            json_output=True
        )

        if not result["success"]:
            log_output("Inspect stderr", result["stderr"])
            raise AssertionError(f"Inspect failed: {result['stderr']}")

        # Parse and validate JSON output
        artifact_data = json.loads(result["stdout"])
        log_output("Artifact metadata", result["stdout"], max_lines=50)

        assert "skills" in artifact_data, "Missing 'skills' in artifact config"
        assert len(artifact_data["skills"]) == 1, f"Expected 1 skill, got {len(artifact_data['skills'])}"

        skill_meta = artifact_data["skills"][0]
        assert skill_meta["name"] == "test-integration-skill", f"Skill name mismatch: {skill_meta['name']}"
        assert "test skill for integration testing" in skill_meta["description"].lower(), \
            f"Description mismatch: {skill_meta['description']}"

        log_success("Artifact inspection validated")
        log_section("✓ Test Passed: Publish Skill")

    except Exception as e:
        log_github_error(__file__, 0, f"test_publish_skill failed: {e}")
        log_error(f"Test failed: {e}")
        raise


def run_all_tests(config: TestConfig, test_manager: TestDirectoryManager) -> None:
    """Run all integration tests"""
    log_section("Running Integration Tests")

    # Create test directory
    test_dir = test_manager.create_test_directory()

    # Run individual tests
    test_publish_skill(config, test_dir)

    # Future tests can be added here
    # test_install_skill(config, test_dir)
    # test_validate_invalid_skill(config, test_dir)


# ============================================================================
# Main Execution
# ============================================================================

def parse_arguments() -> argparse.Namespace:
    """Parse command-line arguments"""
    parser = argparse.ArgumentParser(
        description="OCI Skills CLI Integration Tests",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Environment Variables:
  OCI_REGISTRY_URL or OCI_REPOSITORY_URL  Registry URL base (e.g., ghcr.io/myorg)
  OCI_USERNAME                             Registry username
  OCI_PASSWORD                             Registry password/token

Configuration Priority:
  1. Command-line arguments (highest)
  2. .env file in integration-tests/
  3. System environment variables
  4. Error if required values missing

Examples:
  # Using .env file
  uv run run_tests.py --keep

  # Using CLI parameters
  uv run run_tests.py \\
    --repository-url ghcr.io/myorg \\
    --username myuser \\
    --password mytoken

  # Using python directly (after dependencies installed)
  python run_tests.py --repository-url ghcr.io/myorg --username user --password pass
        """
    )

    parser.add_argument(
        "--repository-url",
        help="OCI registry URL base (e.g., ghcr.io/myorg)"
    )

    parser.add_argument(
        "--username",
        help="Registry username"
    )

    parser.add_argument(
        "--password",
        help="Registry password/token"
    )

    parser.add_argument(
        "--keep",
        action="store_true",
        help="Keep test directory after completion"
    )

    return parser.parse_args()


def main() -> int:
    """Main test execution"""
    exit_code = 0
    test_manager: Optional[TestDirectoryManager] = None

    try:
        log_section("OCI Skills CLI Integration Tests")

        # 1. Parse arguments
        args = parse_arguments()

        # 2. Load configuration
        log_step("Loading configuration")
        config = load_config(args)
        log_success("Configuration loaded")
        log_info(f"Registry: {config.registry_url}")
        log_info(f"Username: {config.username}")
        log_info(f"Binary: {config.cli_binary}")

        # 3. Setup test directory manager
        test_manager = TestDirectoryManager(
            project_root=config.project_root,
            keep=config.keep_test_dir
        )

        # 4. Build CLI
        cli = CliController(config.cli_binary, config.project_root)
        if not cli.build_cli():
            raise RuntimeError("Failed to build CLI")

        # Verify binary exists
        if not config.cli_binary.exists():
            raise RuntimeError(f"Binary not found after build: {config.cli_binary}")

        # 5. Run tests
        run_all_tests(config, test_manager)

        log_section("✓ All Tests Passed")

    except Exception as e:
        log_github_error(__file__, 0, f"Test suite failed: {e}")
        log_error(f"Test suite failed: {e}")

        if test_manager:
            test_manager.mark_test_failed()

        exit_code = 1

    finally:
        # Cleanup
        if test_manager:
            test_manager.cleanup()

    return exit_code


if __name__ == "__main__":
    sys.exit(main())
