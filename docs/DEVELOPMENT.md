# Development Guide

This guide is for developers who want to contribute to GeoETL or build it from source. If you're looking to **use** GeoETL, see the [README](../README.md) and [User Guide](USERGUIDE.md) instead.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Project Structure](#project-structure)
- [Setup](#setup)
  - [Pre-commit Hooks](#pre-commit-hooks-optional)
- [Development Workflow](#development-workflow)
- [Docker Development Environment](#docker-based-workflow)
- [License and Security Checks](#license-and-security-checks)
- [Pre-Commit Checklist](#pre-commit-checklist)
- [Documentation](#documentation)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)

## Related Documentation

- [User Guide](USERGUIDE.md) - Complete guide to using GeoETL CLI
- [Vision](VISION.md) - Project vision and roadmap
- [Architecture Decision Records](adr/) - Technical design decisions

## Prerequisites

- Rust 1.90.0 or later
- [mise](https://mise.jdx.dev/) for tool version management (optional but recommended)
- [prek](https://github.com/j178/prek) for pre-commit hooks (optional but recommended)

## Project Structure

```
geoetl/
├── crates/
│   ├── geoetl-cli/     # Command-line interface
│   └── geoetl-core/    # Core library functionality
├── docs/               # Documentation
└── Cargo.toml          # Workspace configuration
```

## Setup

Install dependencies using mise (if available):
```bash
mise install
```

Or manually ensure you have Rust 1.90.0 installed.

### Pre-commit Hooks (Optional)

The project uses pre-commit hooks to automatically run formatting, linting, tests, and security checks before each commit.

#### Install prek

prek is a fast, Rust-based pre-commit tool (compatible with standard pre-commit):

```bash
# Using standalone installer (recommended)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/j178/prek/releases/latest/download/prek-installer.sh | sh

# Or using Homebrew
brew install prek

# Or using pip
pip install prek

# Or build from source
cargo install --locked --git https://github.com/j178/prek
```

Alternatively, you can use the standard pre-commit tool:
```bash
pip install pre-commit
```

#### Setup Hooks

Install the git hooks:
```bash
prek install
# or: pre-commit install
```

The hooks will now run automatically on `git commit`. They will:
- Auto-format code with `cargo fmt`
- Check for lint warnings with `cargo clippy`
- Run tests with `cargo test`
- Verify licenses and dependencies with `cargo deny`
- Check for security vulnerabilities with `cargo audit`

#### Run Manually

Run all hooks on all files:
```bash
prek run --all-files
# or: pre-commit run --all-files
```

Skip hooks for a single commit:
```bash
git commit --no-verify
```

### Docker-Based Workflow

The repository includes a containerized development environment for consistent tooling:

```bash
# Start the watcher-driven dev container (fmt + clippy + tests on change)
docker compose up geoetl-dev

# Run a CLI command inside the dev container
docker compose run --rm --entrypoint /opt/geoetl/bin/geoetl-cli-dev.sh geoetl-dev drivers

# Run the full check suite once, without the watcher
docker compose --profile test run --rm geoetl-test
```

Both services share cached cargo volumes (`cargo-target`, `cargo-registry`, `cargo-git`) for faster incremental builds. Stop containers with `docker compose down`.

## Development Workflow

### Quick Development Cycle

Format, lint, and run the CLI in one command:
```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings -D clippy::pedantic && cargo run -p geoetl-cli
```
Or use the consolidated helper:
```bash
mise run check
```

### Code Formatting

Format all code:
```bash
cargo fmt --all
```

Check formatting without making changes:
```bash
cargo fmt --all --check
```

### Linting

Run Clippy on all targets:
```bash
cargo clippy --workspace --all-targets -- -D clippy::pedantic
```

Treat warnings as errors (CI mode):
```bash
cargo clippy --workspace --all-targets -- -D warnings -D clippy::pedantic
```

### Testing

Run all tests:
```bash
cargo test --workspace --all-targets
```

Test specific crate:
```bash
cargo test -p geoetl-core
cargo test -p geoetl-cli
```

Run tests with output:
```bash
cargo test -- --nocapture
```

### Running

Run the CLI:
```bash
cargo run -p geoetl-cli
```

Run with arguments:
```bash
cargo run -p geoetl-cli -- [args]
```

### Building

Build all crates:
```bash
cargo build
```

Build release version:
```bash
cargo build --release
```

Build specific crate:
```bash
cargo build -p geoetl-cli
```

### Checking

Quick compile check without building:
```bash
cargo check
```

### License and Security Checks

The project uses `cargo-deny` to ensure license compliance, check for security vulnerabilities, and validate dependencies.

#### Install cargo-deny

```bash
cargo install cargo-deny
```

#### Run All Checks

```bash
cargo deny check
```

Or use the mise task:
```bash
mise security
```

This runs both `cargo audit` (security vulnerabilities) and `cargo deny check` (licenses, bans, sources).

#### Individual Checks

Check licenses only:
```bash
cargo deny check licenses
```

Check security advisories:
```bash
cargo deny check advisories
```

Check for banned or duplicate dependencies:
```bash
cargo deny check bans
```

Check dependency sources:
```bash
cargo deny check sources
```

#### Configuration

License and dependency policies are configured in `deny.toml` at the workspace root. Allowed licenses include:
- MIT, Apache-2.0
- BSD-2-Clause, BSD-3-Clause
- ISC, Zlib, CC0-1.0
- Unicode-3.0

## Pre-Commit Checklist

Before committing code, ensure:

1. Code is formatted: `cargo fmt --all`
2. No Clippy warnings: `cargo clippy --workspace --all-targets -- -D warnings -D clippy::pedantic`
3. All tests pass: `cargo test --workspace --all-targets`
4. Documentation builds: `cargo doc --no-deps`
5. License and security checks pass: `cargo deny check`

All in one command:
```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings -D clippy::pedantic && cargo test --workspace --all-targets && cargo deny check
```

### mise Tasks

Common workflows are available through mise:
```bash
mise run fmt      # rustfmt across the workspace
mise run lint     # clippy with pedantic warnings denied
mise run test     # workspace tests
mise run check    # fmt + lint + test
mise run security # cargo audit + cargo deny check
```

## Documentation

Generate and open documentation for all crates:
```bash
cargo doc --open
```

The generated documentation can be found in `target/doc/`.

Build with private items:
```bash
cargo doc --document-private-items --open
```

## Troubleshooting

### Clean build artifacts
```bash
cargo clean
```

### Update dependencies
```bash
cargo update
```

### Check for outdated dependencies
```bash
cargo outdated
```

## Contributing

We welcome contributions to GeoETL! Here's how to get started:

### Getting Started

1. **Fork and Clone**
   ```bash
   git clone https://github.com/YOUR_USERNAME/geoetl.git
   cd geoetl
   ```

2. **Create a Branch**
   ```bash
   git checkout -b feature/my-new-feature
   ```

3. **Make Changes**
   - Follow the Rust coding standards
   - Write tests for new functionality
   - Ensure all checks pass (`mise run check`)

4. **Commit Your Changes**
   ```bash
   git add .
   git commit -m "feat: add new feature"
   ```

5. **Push and Create PR**
   ```bash
   git push origin feature/my-new-feature
   ```
   Then open a Pull Request on GitHub

### Contribution Guidelines

- **Code Quality**: All code must pass `cargo fmt`, `cargo clippy`, and `cargo test`
- **Tests**: Add tests for new features and bug fixes
- **Documentation**: Update documentation for user-facing changes
- **Commit Messages**: Use conventional commit format (e.g., `feat:`, `fix:`, `docs:`)
- **Small PRs**: Keep pull requests focused and reasonably sized

### Areas for Contribution

- **Format Support**: Implement readers/writers for additional formats
- **Spatial Operations**: Add new spatial algorithms
- **Performance**: Optimize existing operations
- **Documentation**: Improve docs and examples
- **Testing**: Add test coverage
- **Bug Fixes**: Address open issues

### Questions?

- Open an issue for bugs or feature requests
- Start a discussion for questions or ideas
- Check existing issues before creating new ones
