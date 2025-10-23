# Development Guide

This document provides guidelines and commands for developing geoetl.

## Documentation

- [User Guide](USERGUIDE.md) - Complete guide to using GeoETL CLI
- [Vision](VISION.md) - Project vision and roadmap
- [Development Guide](DEVELOPMENT.md) - This document

## Prerequisites

- Rust 1.90.0 or later
- [mise](https://mise.jdx.dev/) for tool version management (optional but recommended)

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

## Development Workflow

### Quick Development Cycle

Format, lint, and run the CLI in one command:
```bash
cargo fmt --all && cargo clippy --workspace --all-targets --all-features && cargo run -p geoetl-cli
```

### Code Formatting

Format all code:
```bash
cargo fmt --all
```

Check formatting without making changes:
```bash
cargo fmt --check
```

### Linting

Run Clippy on all targets:
```bash
cargo clippy --all-targets --all-features
```

Treat warnings as errors (CI mode):
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Testing

Run all tests:
```bash
cargo test
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

## Pre-Commit Checklist

Before committing code, ensure:

1. Code is formatted: `cargo fmt`
2. No Clippy warnings: `cargo clippy --all-targets --all-features -- -D warnings`
3. All tests pass: `cargo test`
4. Documentation builds: `cargo doc --no-deps`

All in one command:
```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

## Documentation

Build documentation:
```bash
cargo doc --no-deps --open
```

Build with private items:
```bash
cargo doc --no-deps --document-private-items --open
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
