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

...and many more! See `geoetl-cli drivers` for the complete list.

## Installation

### Building from Source (Current)

**Note**: GeoETL is in early development. Pre-built binaries will be available in future releases.

```bash
# Prerequisites: Rust 1.90.0 or later
git clone https://github.com/yourusername/geoetl.git
cd geoetl
cargo build --release

# The binary will be at: target/release/geoetl-cli
```

For detailed build instructions and development setup, see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Quick Start

```bash
# List available drivers
geoetl-cli drivers

# Convert between formats
geoetl-cli convert \
  -i input.geojson \
  -o output.parquet \
  --input-driver GeoJSON \
  --output-driver Parquet

# Get dataset information
geoetl-cli info data.geojson
geoetl-cli info --detailed --stats data.geojson

# Enable verbose logging
geoetl-cli -v convert -i input.geojson -o output.parquet
```

## Common Use Cases

### Check Available Formats

```bash
# See all 68+ supported driver formats
geoetl-cli drivers
```

### Convert Spatial Data

```bash
# GeoJSON to Parquet
geoetl-cli convert \
  -i cities.geojson \
  -o cities.parquet \
  --input-driver GeoJSON \
  --output-driver Parquet

# More formats coming in Phase 2
```

### Inspect Datasets

```bash
# Basic information
geoetl-cli info data.geojson

# Detailed with statistics
geoetl-cli info --detailed --stats data.geojson
```



## Documentation

### For Users
- **[User Guide](docs/USERGUIDE.md)**: Complete guide to using GeoETL with detailed examples
- **[Quick Reference](docs/QUICKREF.md)**: Fast command reference and cheat sheet

### For Developers
- **[Development Guide](docs/DEVELOPMENT.md)**: Build instructions, workflow, and contribution guidelines
- **[Vision Document](docs/VISION.md)**: Project vision, goals, and strategic roadmap
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

We welcome contributions! Whether you want to report bugs, suggest features, or contribute code, we'd love your help.

- **Report Issues**: [GitHub Issues](https://github.com/yourusername/geoetl/issues)
- **Discuss Ideas**: [GitHub Discussions](https://github.com/yourusername/geoetl/discussions)
- **Contribute Code**: See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for setup and guidelines

GeoETL is committed to leveraging and contributing back to the GeoRust ecosystem through open, transparent, community-driven development.

## Getting Help

- **Documentation**: Check the [User Guide](docs/USERGUIDE.md) for detailed usage instructions
- **Command Help**: Run `geoetl-cli --help` or `geoetl-cli <command> --help`
- **Issues**: Report bugs at [GitHub Issues](https://github.com/yourusername/geoetl/issues)
- **Questions**: Ask questions in [GitHub Discussions](https://github.com/yourusername/geoetl/discussions)

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
