# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GeoETL is a modern, high-performance CLI tool for spatial data conversion and processing built in Rust. It leverages Apache DataFusion for vectorized query execution and aims to be faster than GDAL for vector geospatial operations.

## Build & Development Commands

```bash
# Build
cargo build                          # Debug build
cargo build --release                # Release build

# Run CLI
cargo run -p geoetl-cli -- [args]    # Run CLI with arguments
cargo run -p geoetl-cli -- drivers   # List available drivers
cargo run -p geoetl-cli -- --help    # Show help

# Testing
cargo test --workspace --all-targets           # Run all tests
cargo test -p geoetl-core                      # Test specific crate
cargo test -- --nocapture                      # Tests with output

# Linting & Formatting
cargo fmt --all                                # Format code
cargo clippy --workspace --all-targets -- -D warnings -D clippy::pedantic

# Coverage (requires cargo-llvm-cov)
cargo llvm-cov --workspace --all-targets --fail-under-lines 80

# All checks (format + lint + test + security + coverage)
make check
```

## Architecture

### Workspace Crates

```
crates/
├── geoetl-cli/           # CLI binary - argument parsing, command dispatch
├── geoetl-core/          # Core library - driver registry, operations, error handling
├── geoetl-core-common/   # Shared types and factory interfaces
├── geoetl-operations/    # Spatial UDFs (ST_Distance, etc.) for DataFusion
└── formats/              # DataFusion format implementations
    ├── datafusion-csv/
    ├── datafusion-geojson/
    ├── datafusion-geoparquet/
    ├── datafusion-flatgeobuf/
    ├── datafusion-shapefile/
    ├── datafusion-geopackage/
    ├── datafusion-arrow/
    ├── datafusion-geojsonseq/
    ├── datafusion-osm/
    └── datafusion-shared/    # Common utilities for format crates
```

### Data Flow

```
CLI Command → geoetl-core operations → DataFusion SessionContext
    → Format Reader (TableProvider) → Arrow RecordBatch
    → SQL/Transform → Format Writer (DataSink) → Output File
```

### Key Patterns

1. **Factory Pattern**: Each format crate registers a factory via `geoetl_core_common::driver_registry()` that creates readers/writers
2. **DataFusion Integration**: Formats implement `FileFormat`, `FileSource`, and `DataSink` traits
3. **GeoArrow**: Geometry columns use GeoArrow encoding (Arrow extension types with `geoarrow.*` metadata)
4. **Streaming Execution**: Conversions use DataFusion's streaming `DataSink` for memory efficiency

### Adding a New Format Driver

1. Create crate in `crates/formats/datafusion-{format}/`
2. Implement:
   - `FileFormat` trait for schema inference and physical planning
   - `FileSource`/`TableProvider` for reading
   - `DataSink` for writing
3. Create factory implementing `FormatFactory` trait
4. Register in `geoetl_core::init::initialize()` via `register_{format}_format()`

See `docs/DATAFUSION_GEOSPATIAL_FORMAT_INTEGRATION_GUIDE.md` for detailed implementation guide.

## Key Dependencies

- **DataFusion 50.x**: Query engine and execution framework
- **Arrow 56**: Columnar data format
- **GeoArrow 0.6.x**: Geospatial Arrow encoding
- **GeoZero**: Zero-copy geometry streaming
- **GEOS 10.0**: Geometry operations (statically linked)

## Error Handling

Errors use `thiserror` with structured error types in `geoetl_core::error`:
- `GeoEtlError` - top-level enum
- `DriverError` - driver not found, operation not supported
- `IoError` - file read/write failures
- `FormatError` - parsing/encoding issues

All errors provide `user_message()` and `recovery_suggestion()` methods for CLI output.

## Testing

### Running Tests

```bash
cargo test --workspace --all-targets           # All tests
cargo test -p geoetl-cli                       # Specific crate
cargo test e2e_convert                         # Single test by name
cargo test -- --nocapture                      # With output
```

### E2E Tests

End-to-end tests verify complete CLI workflows using `assert_cmd`:

```
crates/geoetl-cli/tests/
├── e2e_convert.rs          # CSV/GeoJSON conversion tests
└── e2e_geoparquet.rs       # GeoParquet format tests

crates/geoetl-core/tests/
├── integration_convert.rs  # Core conversion operations
└── integration_st_distance.rs  # Spatial UDF tests

crates/formats/datafusion-*/tests/
├── e2e_*.rs                # Format-specific e2e tests
└── test_writer_integration.rs  # Writer integration tests
```

Test data lives in `crates/geoetl-cli/tests/e2e_data/` (CSV, GeoJSON, GeoParquet samples).

### Conventions

- Unit tests in `#[cfg(test)]` modules within source files
- E2E/integration tests in `tests/` directories
- Use `tempfile::TempDir` for isolated file operations
- Initialize drivers with `geoetl_core::init::initialize()` before tests

## CI/CD

CircleCI pipeline runs: format → lint → build → test → coverage → security

Coverage minimum: 80% line coverage
