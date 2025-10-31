# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
