# Architectural Review

This document provides a high-level architectural review of the OCI Skills CLI, focusing on its design principles, structure, and areas for improvement.

## Table of Contents

1.  [High-Level Architecture](#high-level-architecture)
2.  [Strengths](#strengths)
3.  [Areas for Improvement](#areas-for-improvement)
    *   [3.1. Tight Coupling Between `core` and `oci` Crates](#31-tight-coupling-between-core-and-oci-crates)
    *   [3.2. Lack of a Clear Service Layer](#32-lack-of-a-clear-service-layer)
    *   [3.3. Monolithic `cli` Crate](#33-monolithic-cli-crate)
    *   [3.4. Limited Configuration Options](#34-limited-configuration-options)

---

## 1. High-Level Architecture

The project is structured as a Cargo workspace with four main crates:

*   **`traits`**: Defines the core abstractions and data models for the application, such as `OciClient`, `FileSystem`, and `Skill`.
*   **`oci`**: Provides the concrete implementation for interacting with OCI registries.
*   **`core`**: Contains the application's business logic, such as installing, publishing, and validating skills.
*   **`cli`**: The command-line interface that parses user input and orchestrates the application's functionality.

The architecture follows the principle of **Dependency Inversion**, with the `core` crate depending on the abstractions defined in `traits`, and the `oci` and `cli` crates providing concrete implementations.

## 2. Strengths

*   **Clear Separation of Concerns**: The workspace structure effectively separates the application's concerns into distinct crates, making the codebase easier to navigate and maintain.
*   **Testability**: The use of traits for I/O operations (e.g., `OciClient`, `FileSystem`) allows for easy mocking and testing of the `core` business logic.
*   **Modularity**: The crate-based architecture promotes modularity and allows for the potential reuse of individual components in other applications.

## 3. Areas for Improvement

### 3.1. Tight Coupling Between `core` and `oci` Crates

**Issue:** The `core` crate has a direct dependency on the `oci` crate, which violates the Dependency Inversion principle. The `core` crate should only depend on the abstractions defined in `traits`.

**Impact:** This tight coupling makes it difficult to swap out the `oci` implementation with a different one (e.g., a mock implementation for testing) without modifying the `core` crate.

**Proposed Solution:**

Remove the direct dependency on `oci` from the `core` crate's `Cargo.toml`. The `OciClient` implementation should be instantiated in the `cli` crate and injected into the `core` components that require it.

---

### 3.2. Lack of a Clear Service Layer

**Issue:** The `core` crate contains a collection of structs (e.g., `Installer`, `Publisher`) that encapsulate business logic, but there is no clear service layer that orchestrates these components.

**Impact:** This can lead to code duplication and makes it more difficult to manage the application's overall workflow.

**Proposed Solution:**

Introduce a "service" layer in the `core` crate that coordinates the various business logic components. For example, an `OciSkillsService` could take dependencies on the `Installer`, `Publisher`, and `Validator` and provide a unified API for the `cli` crate to interact with.

---

### 3.3. Monolithic `cli` Crate

**Issue:** The `cli` crate's `main.rs` file contains a large amount of code for parsing command-line arguments and dispatching to the appropriate `core` components.

**Impact:** This makes the `main.rs` file difficult to read and maintain.

**Proposed Solution:**

Break down the `cli` crate into smaller modules, each responsible for a specific command (e.g., `install.rs`, `publish.rs`). This will improve the organization of the `cli` crate and make it easier to add new commands in the future.

---

### 3.4. Limited Configuration Options

**Issue:** The application's configuration is limited to environment variables and command-line arguments.

**Impact:** This can be inconvenient for users who prefer to manage configuration in a dedicated file.

**Proposed Solution:**

Introduce support for a configuration file (e.g., `~/.ociskills/config.toml`) that allows users to specify default values for options such as the output directory, registry credentials, and more. The application should be able to merge configuration from the file, environment variables, and command-line arguments, with the latter taking precedence.
