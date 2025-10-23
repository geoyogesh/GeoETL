# GeoETL

**A modern, high-performance CLI tool for spatial data conversion and processing**

[![Rust](https://img.shields.io/badge/rust-1.90%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

GeoETL is a next-generation geospatial ETL (Extract, Transform, Load) tool built with Rust, designed to become a modern replacement for GDAL. It leverages cutting-edge technologies like Apache DataFusion and DataFusion Ballista to deliver blazing-fast performance on both single-node and distributed systems.

## Vision

To become the modern standard for vector spatial data processing, empowering users with **5-10x faster performance** than GDAL, seamless scalability from laptop to cluster, and an intuitive developer experience.

**Read the full vision**: [docs/VISION.md](docs/VISION.md)

## Key Features

- **High Performance**: Vectorized execution using Apache DataFusion, multi-threaded by default
- **Scalable**: Seamlessly scale from single-node to distributed processing with Ballista
- **Memory Safe**: Built with Rust for zero-cost memory safety guarantees
- **Cloud Native**: First-class support for cloud storage (S3, Azure Blob, GCS)
- **Modern Architecture**: Leverages the GeoRust ecosystem for spatial operations
- **Streaming I/O**: Process datasets larger than available RAM
- **No GDAL Dependency**: Clean, modern implementation without legacy dependencies

## Technology Stack

### Core Technologies
- **[Rust](https://www.rust-lang.org/)**: Memory-safe systems programming language
- **[Apache DataFusion](https://arrow.apache.org/datafusion/)**: SQL query engine for fast analytics
- **[DataFusion Ballista](https://github.com/apache/arrow-datafusion/tree/main/ballista)**: Distributed compute platform
- **[Apache Arrow](https://arrow.apache.org/)**: Columnar in-memory data format

### GeoRust Ecosystem
- **[geo](https://docs.rs/geo/)**: Geospatial algorithms and operations
- **[geozero](https://docs.rs/geozero/)**: Zero-copy geospatial data streaming
- **[proj](https://docs.rs/proj/)**: Coordinate reference system transformations
- **[rstar](https://docs.rs/rstar/)**: R-tree spatial indexing
- **[geojson](https://docs.rs/geojson/)**, **[flatgeobuf](https://docs.rs/flatgeobuf/)**: Format support

## Project Status

**Phase 1: Foundation** (Q1 2026 - Current)

GeoETL is in active early development. We are currently establishing the core architecture and foundational components.

### Supported Drivers

**68+ vector format drivers** including:

**Core Formats**:
- GeoJSON, GeoJSONSeq
- ESRI Shapefile
- GeoPackage (GPKG)
- FlatGeobuf
- (Geo)Parquet
- (Geo)Arrow IPC

**Databases**:
- PostgreSQL/PostGIS
- MySQL, SQLite/Spatialite
- Oracle Spatial, MongoDB
- Microsoft SQL Server

**CAD & Engineering**:
- AutoCAD DXF, DWG
- Microstation DGN
- ESRI File Geodatabase

**Web Services**:
- OGC WFS, OGC API - Features
- Carto, Elasticsearch
- Google Earth Engine

...and many more! See `geoetl-cli drivers --detailed` for the complete list.

## Quick Start

### Prerequisites

- Rust 1.90.0 or later
- [mise](https://mise.jdx.dev/) (optional, for tool version management)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/geoetl.git
cd geoetl

# Install Rust toolchain (if using mise)
mise install

# Build the project
cargo build --release

# Run the CLI
cargo run -p geoetl-cli
```

### Basic Usage

```bash
# List available drivers (2 currently supported: GeoJSON, Parquet)
geoetl-cli drivers

# Convert between formats (command structure implemented, I/O in Phase 2)
geoetl-cli convert \
  -i input.geojson \
  -o output.parquet \
  --input-driver GeoJSON \
  --output-driver Parquet

# Get dataset information
geoetl-cli info data.geojson
geoetl-cli info --detailed --stats data.shp

# Enable verbose logging
geoetl-cli -v convert -i input.geojson -o output.parquet
```

**Note**: Core I/O functionality (convert, info) is currently in Phase 2 development. The CLI framework, driver registry, and logging are fully implemented.

## Documentation

- **[User Guide](docs/USERGUIDE.md)**: Complete guide to using GeoETL CLI with examples
- **[Quick Reference](docs/QUICKREF.md)**: Fast command reference and cheat sheet
- **[Vision Document](docs/VISION.md)**: Project vision, goals, and strategic roadmap
- **[Development Guide](docs/DEVELOPMENT.md)**: Setup, workflow, and contribution guidelines
- **[Architecture Decision Records](docs/adr/)**: Detailed technical design decisions
  - [ADR 0001: High-Level Architecture](docs/adr/0001-high-level-architecture.md)

## Architecture

GeoETL is organized as a Rust workspace with the following crates:

```
geoetl/
├── crates/
│   ├── geoetl-cli/      # Command-line interface
│   ├── geoetl-core/     # Core library with spatial operations
│   ├── geoetl-formats/  # Format readers and writers (planned)
│   ├── geoetl-exec/     # Query execution engine (planned)
│   └── geoetl-ballista/ # Distributed execution (planned)
└── docs/                # Documentation
```

**High-level data flow**:

```
CLI → Core Library → DataFusion Engine → Format I/O → Data Sources
                          ↓
                  Single-Node / Ballista
```

See [ADR 0001](docs/adr/0001-high-level-architecture.md) for detailed architecture documentation.

## Development

### Development Workflow

```bash
# Format, lint, and run
cargo fmt && cargo clippy --all-targets --all-features && cargo run -p geoetl-cli

# Run tests
cargo test

# Build documentation
cargo doc --no-deps --open
```

See the [Development Guide](docs/DEVELOPMENT.md) for comprehensive development instructions.

## Roadmap

### Phase 1: Foundation (Q1 2026 - Current)
- ✅ Workspace structure
- ✅ Vision and architecture documentation
- ✅ CLI framework with clap (argument parsing, logging)
- ✅ Driver registry (68+ GDAL-compatible drivers)
- ✅ CLI command structure (convert, info, drivers)
- ✅ Tabled-based output formatting
- ⏳ DataFusion integration
- ⏳ Basic vector I/O implementation (GeoJSON, Parquet)

### Phase 2: Core Functionality (Q2 2026)
- Vector I/O implementation (read/write operations)
- Driver auto-detection from file extensions
- Core spatial operations
- CRS transformations
- Performance benchmarking

### Phase 3: Advanced Features (Q3 2026)
- Advanced spatial algorithms
- Query optimization
- Performance parity with GDAL

### Phase 4: Distribution (Q4 2026)
- Ballista integration
- Cloud storage support
- Horizontal scaling

## Performance Goals

| Operation | Target vs GDAL | Method |
|-----------|---------------|---------|
| Format conversion | 5-10x faster | Vectorized processing |
| Spatial filtering | 5x faster | R-tree indexing, SIMD |
| Buffer operations | 3-5x faster | Parallel execution |
| Spatial joins | 5x faster | Partition-based parallelism |
| Distributed (1TB) | Linear scaling | Ballista partitioning |

## Contributing

We welcome contributions! GeoETL is committed to:
- Leveraging and contributing back to the GeoRust ecosystem
- Open, transparent development
- Community-driven feature development

### How to Contribute

1. Check existing issues or create a new one
2. Fork the repository
3. Create a feature branch
4. Make your changes following our coding standards
5. Submit a pull request

See [DEVELOPMENT.md](docs/DEVELOPMENT.md) for detailed contribution guidelines.

## Community

- **Issues**: [GitHub Issues](https://github.com/yourusername/geoetl/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/geoetl/discussions)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

GeoETL builds on the shoulders of giants:

- The [GeoRust](https://github.com/georust) community for excellent geospatial libraries
- The [Apache Arrow](https://arrow.apache.org/) project for DataFusion and Arrow
- The [GDAL](https://gdal.org/) project for decades of geospatial innovation
- The [Rust](https://www.rust-lang.org/) community for an amazing language and ecosystem

---

**Status**: Early Development | **Rust Version**: 1.90+ | **License**: MIT/Apache-2.0
