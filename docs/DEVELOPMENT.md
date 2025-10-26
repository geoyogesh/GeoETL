# Development Guide

This guide is for developers who want to contribute to GeoETL or build it from source. If you're looking to **use** GeoETL, see the [README](../README.md) and [User Guide](USERGUIDE.md) instead.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Project Structure](#project-structure)
- [Setup](#setup)
  - [Pre-commit Hooks](#pre-commit-hooks-optional)
- [Docker Development Environment](#docker-based-workflow)
- [Testing CI Pipeline Locally](#testing-ci-pipeline-locally)
- [Development Workflow](#development-workflow)
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
This installs Rust 1.90.0 plus the command-line tools used throughout this guide
(`taplo`, `cargo-audit`, `cargo-deny`, `cargo-llvm-cov`, and `cargo-outdated`).

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
- Auto-format Rust code with `cargo fmt`
- Auto-format TOML files with `taplo format`
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

The repository includes multiple containerized environments for different use cases:

#### Development Containers

Development environment with full Rust toolchain for consistent tooling:

```bash
# Start the watcher-driven dev container (fmt + clippy + tests on change)
docker compose up geoetl-dev

# Run a CLI command inside the dev container
docker compose run --rm --entrypoint /opt/geoetl/bin/geoetl-cli-dev.sh geoetl-dev drivers

# Run the full check suite once, without the watcher
docker compose --profile test run --rm geoetl-test
```

Both services share cached cargo volumes (`cargo-target`, `cargo-registry`, `cargo-git`) for faster incremental builds. Stop containers with `docker compose down`.

#### Production Build (Multi-Stage)

The production Docker setup uses a multi-stage build to create a minimal runtime container that replicates real-world deployment scenarios where the CLI runs completely standalone without any build dependencies.

**Architecture:**

1. **Builder Stage**: Full Rust toolchain (~2.5GB)
   - Compiles CLI with all dependencies
   - Optimizes and strips debug symbols
   - Verifies binary execution

2. **Runtime Stage**: Minimal container (~140MB)
   - Contains only the compiled CLI binary
   - Only core system libraries (glibc, libgcc) - all other deps statically linked
   - No additional packages installed - fully self-contained binary
   - No build tools, no source code, no Rust compiler
   - Runs as non-root user for security

**Usage:**

```bash
# Build the production container
docker compose -f docker/docker-compose.yml --profile prod build geoetl-cli

# Run with default command (--help)
docker compose -f docker/docker-compose.yml --profile prod up geoetl-cli

# Run with custom command
docker compose -f docker/docker-compose.yml --profile prod run --rm geoetl-cli --version

# Test that binary runs without build dependencies (check linked libraries)
docker compose -f docker/docker-compose.yml --profile prod run --rm --entrypoint=/bin/bash geoetl-cli -c "ldd /usr/local/bin/geoetl"
# Expected output shows only core system libraries (all other deps statically linked):
#   linux-vdso.so.1 (kernel virtual library)
#   libgcc_s.so.1 (GCC runtime)
#   libc.so.6 (GNU C library)
#   /lib/ld-linux-aarch64.so.1 (dynamic linker)
```

**Container Types:**

| Container | Size | Contents | Use Case |
|-----------|------|----------|----------|
| `geoetl-dev` | ~2.5GB | Full Rust toolchain + dev tools | Development & testing |
| `geoetl-builder` | ~2.5GB | Build stage artifacts | CI/CD builds |
| `geoetl-cli` | ~140MB | Self-contained CLI binary only | Production deployment |

**Multi-Architecture Builds:**

```bash
# Build for ARM64 (Apple Silicon, ARM servers)
docker buildx build --platform linux/arm64 -f docker/prod/Dockerfile -t geoetl/cli:arm64 .

# Build for AMD64 (x86_64)
docker buildx build --platform linux/amd64 -f docker/prod/Dockerfile -t geoetl/cli:amd64 .

# Build multi-arch
docker buildx build --platform linux/amd64,linux/arm64 -f docker/prod/Dockerfile -t geoetl/cli:latest .
```

**Security Features:**
- Non-root user execution (UID 1000)
- Minimal attack surface
- No build tools in runtime container
- Stripped binary (no debug symbols)
- Latest security patches

**Testing the Production Build:**

Verify the production container works as expected:

```bash
# Build the production container
docker compose -f docker/docker-compose.yml --profile prod build geoetl-cli

# Test CLI version
docker run --rm geoetl/cli:latest --version
# Expected: geoetl 0.1.0

# Test CLI help
docker run --rm geoetl/cli:latest --help
# Expected: Full help output with commands (convert, info, drivers)

# Check binary dependencies (should only show core system libs)
docker run --rm --entrypoint /bin/bash geoetl/cli:latest -c 'ldd /usr/local/bin/geoetl'
# Expected output:
#   linux-vdso.so.1
#   libgcc_s.so.1 => /lib/aarch64-linux-gnu/libgcc_s.so.1
#   libc.so.6 => /lib/aarch64-linux-gnu/libc.so.6
#   /lib/ld-linux-aarch64.so.1

# Verify non-root user (security check)
docker run --rm --entrypoint /bin/bash geoetl/cli:latest -c 'whoami && id'
# Expected: geoetl, uid=1000(geoetl) gid=1000(geoetl)

# Check binary size
docker run --rm --entrypoint /bin/bash geoetl/cli:latest -c 'ls -lh /usr/local/bin/geoetl'
# Expected: ~1.3MB (stripped and optimized)

# Check container size
docker images geoetl/cli:latest --format 'Size: {{.Size}}'
# Expected: ~140MB
```

**What the tests verify:**
- ✅ CLI binary is fully functional
- ✅ Only core system libraries required (SSL, SQLite statically linked)
- ✅ Runs as non-root user for security
- ✅ Binary is optimized and stripped (~1.3MB)
- ✅ Container is minimal (~140MB vs ~2.5GB dev container)

## Testing CI Pipeline Locally

The project uses CircleCI for continuous integration. You can test the CI pipeline locally before pushing changes using the CircleCI CLI.

### Install CircleCI CLI

**macOS:**
```bash
brew install circleci
```

**Linux:**
```bash
curl -fLSs https://raw.githubusercontent.com/CircleCI-public/circleci-cli/master/install.sh | bash
```

**Other platforms:** See [CircleCI CLI documentation](https://circleci.com/docs/local-cli/)

### Validate Configuration

Check if your `.circleci/config.yml` is valid:
```bash
circleci config validate
```

### Run Individual Jobs

The CircleCI pipeline includes the following jobs:

**Format Check:**
```bash
circleci local execute format
```
Runs `cargo fmt --all -- --check` to verify code formatting.

**Lint Check:**
```bash
circleci local execute lint
```
Runs `cargo clippy` with strict linting rules.

**Build:**
```bash
circleci local execute build
```
Builds the workspace in release mode.

**Tests:**
```bash
circleci local execute test
```
Runs all workspace tests.

**Coverage:**
```bash
circleci local execute coverage
```
Generates code coverage reports and enforces 80% minimum coverage threshold.

**Security:**
```bash
circleci local execute security
```
Runs `cargo audit` and `cargo deny check` for security vulnerabilities and license compliance.

### Test Results Summary

All jobs run in a `rust:1.90.0` Docker container. Expected results:
- **format**: Code formatting validation
- **lint**: No clippy warnings with pedantic rules
- **build**: Successful workspace compilation in ~7-8 seconds
- **test**: All unit tests passing (currently 5 tests in geoetl-core)
- **coverage**: Minimum 80% line coverage (currently ~80-95%)
- **security**: No vulnerabilities or license violations

### Limitations of Local Execution

When running CircleCI jobs locally, note:
- **Cache operations**: Cache save/restore shows "not supported" errors (expected)
- **Workspace persistence**: Cross-job artifacts aren't shared locally
- **Platform warnings**: Architecture mismatches (arm64 vs amd64) are expected on Apple Silicon
- **Workflows**: Local execution runs individual jobs, not full workflows with dependencies

These limitations don't affect the actual CI behavior in CircleCI cloud.

### CI/CD Pipeline

The full pipeline runs automatically on push and includes:
1. **format** - Code formatting check
2. **lint** - Linting with clippy
3. **build** - Compilation (requires format + lint)
4. **test** - Test suite (requires build)
5. **coverage** - Code coverage analysis (requires test)
6. **security** - Security audit (requires test)

Jobs run in parallel where possible to optimize CI time.

## Development Workflow

### Quick Development Cycle

Format, lint, and run the CLI in one command:
```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings -D clippy::pedantic && cargo run -p geoetl-cli
```
Or use the consolidated helper:
```bash
make check
```

### Code Formatting

Format all Rust code and TOML files:
```bash
make fmt
# or manually:
cargo fmt --all
taplo format
```

Check formatting without making changes:
```bash
cargo fmt --all --check
taplo format --check
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

### Code Coverage

The project uses `cargo-llvm-cov` for code coverage analysis.

#### Install cargo-llvm-cov

If you used `mise install`, this tool is already available. Otherwise install it manually:
```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

#### Generate Coverage Reports

Show coverage summary table in terminal (used by `make check`):
```bash
make coverage
```
This displays a summary table with pass/fail status, without verbose line-by-line details.

Generate and open detailed coverage report in browser:
```bash
make coverage-open
```
This generates a full HTML report with line-by-line coverage highlighting.

Or use cargo commands directly:

```bash
# HTML report (opens automatically)
cargo llvm-cov --workspace --all-targets --open --fail-under-lines 80

# HTML report (manual open at target/llvm-cov/html/index.html)
cargo llvm-cov --workspace --all-targets --html --fail-under-lines 80

# Detailed text output with line-by-line coverage
cargo llvm-cov --workspace --all-targets --text --fail-under-lines 80

# LCOV format (for CI/CD integration)
cargo llvm-cov --workspace --all-targets --lcov --output-path lcov.info --fail-under-lines 80

# Clean coverage artifacts
cargo llvm-cov clean
```

#### Coverage Reports Location

- HTML reports: `target/llvm-cov/html/index.html`
- LCOV reports: `lcov.info` (root directory)
- Coverage artifacts are automatically excluded from git via `.gitignore`

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

If you used `mise install`, this tool is already available. Otherwise install it manually:
```bash
cargo install cargo-deny
```

#### Run All Checks

```bash
cargo deny check
```

Run both checks together via the Makefile target:
```bash
make security
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
5. License and security checks pass: `cargo audit && cargo deny check`
6. Code coverage is adequate: `cargo llvm-cov --workspace --all-targets --text --fail-under-lines 80`

All in one command using make:
```bash
make check
```

Or using cargo commands directly:
```bash
cargo fmt --all && \
cargo clippy --workspace --all-targets -- -D warnings -D clippy::pedantic && \
cargo test --workspace --all-targets && \
cargo audit && \
cargo deny check && \
cargo llvm-cov --workspace --all-targets --open --fail-under-lines 80
```

### Makefile Tasks

Common workflows are available through the Makefile:
```bash
make fmt            # Format Rust code (cargo fmt) and TOML files (taplo)
make lint           # clippy with pedantic warnings denied
make test           # workspace tests
make security       # cargo audit + cargo deny check
make coverage       # generate coverage summary (text)
make coverage-open  # generate and open coverage report in browser
make check          # fmt + lint + test + security + coverage (complete CI check)
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

`mise install` provides the `cargo-outdated` command. If you skipped mise, install it with `cargo install cargo-outdated`.
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
   - Ensure all checks pass (`make check`)

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
