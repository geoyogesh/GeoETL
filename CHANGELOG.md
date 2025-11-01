# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2025-11-01

### Added

- **Custom Error Types**: Implemented comprehensive error handling system with `GeoEtlError` enum
  - Added specialized error types for IO, driver, format, conversion, validation, configuration, data processing, and geometry operations
  - Integrated error types across CLI, core, and operations crates
  - All error handling tests passing
- **Automated Documentation Deployment**: Integrated Cloudflare Pages deployment into release workflow
  - Documentation automatically deploys to production after GitHub release creation
  - Deployed to https://geoetl-web-circleci.pages.dev on every release tag
  - Uses CircleCI with Wrangler CLI for deployment

### Changed

- **Documentation Reorganization**:
  - Removed redundant `docs/USERGUIDE.md` (content already on website at https://geoetl.com)
  - Updated all references in README.md, QUICKREF.md, DEVELOPMENT.md to point to website
  - Moved format-specific documentation to package directories:
    - `docs/formats/csv-*.md` → `crates/formats/datafusion-csv/docs/`
    - `docs/formats/geojson-*.md` → `crates/formats/datafusion-geojson/docs/`
  - Updated `DATAFUSION_GEOSPATIAL_FORMAT_INTEGRATION_GUIDE.md` with new documentation paths

### Removed

- `docs/USERGUIDE.md` - Superseded by documentation website
- `docs/formats/` directory - Documentation moved to respective package directories

## [0.1.0] - 2025-10-31

### Added

- **Initial Project Structure**: Created a new Rust workspace with the initial crates:
  - `geoetl-cli`: The command-line interface for user interaction.
  - `geoetl-core`: The core library containing business logic and driver management.
  - `geoetl-operations`: Crate for handling specific operations like `convert`.
  - Placeholders for `formats` crates (`datafusion-csv`, `datafusion-geojson`, etc.).
- **CLI Framework**:
  - Implemented a robust CLI using `clap` for argument parsing.
  - Added global flags for logging control: `--verbose` for INFO and `--debug` for DEBUG levels.
  - Set up `tracing` for structured logging and `tracing_log` to bridge standard `log` messages.
- **Core Commands**:
  - **`drivers`**: A fully functional command that lists all 68+ available vector format drivers and their capabilities (read, write, info) in a formatted table.
  - **`convert`**: Initial implementation of the format conversion command. It includes argument parsing for input/output paths, driver names, geometry column, and geometry type. It validates that the specified drivers exist and have the required read/write support.
  - **`info`**: A placeholder for a future command to display dataset metadata, with `--detailed` and `--stats` flags.
- **Driver and Operations Logic**:
  - `geoetl-core` now includes a driver registry for managing available format drivers.
  - `geoetl-operations` contains the initial `convert` function, which is called by the CLI.
- **Unit Tests**:
  - Added initial unit tests for the `convert` command handler to ensure correct validation of input/output drivers and their capabilities.
- **Documentation**:
  - Created extensive initial documentation including:
    - `README.md` with project overview, features, and quick start.
    - `VISION.md` outlining the project's long-term goals.
    - `DEVELOPMENT.md` for contributor guidelines.
    - `adr/0001-high-level-architecture.md` detailing the technical architecture.
- **CI/CD**:
  - Set up a basic CircleCI configuration (`config-ci.yml`) for continuous integration.
  - Included a `Makefile` for common development tasks.
