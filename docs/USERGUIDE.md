# GeoETL User Guide

## Table of Contents

- [Introduction](#introduction)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Commands](#commands)
  - [convert](#convert---convert-between-vector-formats)
  - [info](#info---display-dataset-information)
  - [drivers](#drivers---list-available-drivers)
- [Driver System](#driver-system)
- [Common Workflows](#common-workflows)
- [Logging and Debugging](#logging-and-debugging)

---

## Introduction

GeoETL is a modern, high-performance CLI tool for spatial data conversion and processing, built in Rust. It aims to be 5-10x faster than GDAL with support for distributed processing.

### Key Features

- **Fast**: Built with Rust for maximum performance
- **Compatible**: Support for 68+ vector formats (2 currently implemented: GeoJSON, Parquet)
- **Modern**: Designed to leverage Apache DataFusion for analytics
- **Intuitive**: Clean, user-friendly command-line interface
- **Extensible**: Designed for single-node to distributed processing

---

## Installation

For installation instructions, see the main [README](../README.md#installation).

### Quick Reference

```bash
# Run directly with cargo (during development)
cargo run -p geoetl-cli -- [COMMAND] [OPTIONS]

# Or use the compiled binary
./target/release/geoetl-cli [COMMAND] [OPTIONS]
```

---

## Getting Started

### Quick Start

```bash
# Get help
geoetl-cli --help

# List all available drivers
geoetl-cli drivers

# Convert a GeoJSON to Shapefile (when implemented)
geoetl-cli convert -i data.geojson -o data.shp

# Get information about a dataset (when implemented)
geoetl-cli info data.geojson
```

### Global Options

All commands support these global options:

- `-v, --verbose` - Enable verbose (INFO level) logging
- `-d, --debug` - Enable debug (DEBUG level) logging
- `-h, --help` - Show help information
- `-V, --version` - Show version information

---

## Commands

### `convert` - Convert Between Vector Formats

Convert spatial datasets from one format to another.

#### Usage

```bash
geoetl-cli convert [OPTIONS] --input <DATASET> --output <DATASET>
```

#### Options

| Option | Description |
|--------|-------------|
| `-i, --input <DATASET>` | Input dataset path (required) |
| `-o, --output <DATASET>` | Output dataset path (required) |
| `--input-driver <DRIVER>` | Input driver (required) |
| `--output-driver <DRIVER>` | Output driver (required) |

#### Examples

```bash
# Convert with explicit drivers
geoetl-cli convert \
  -i input.geojson \
  -o output.shp \
  --input-driver GeoJSON \
  --output-driver "ESRI Shapefile"

# Convert between formats
geoetl-cli convert \
  -i input.json \
  -o output.gpkg \
  --input-driver GeoJSON \
  --output-driver GPKG

# Convert with verbose logging
geoetl-cli -v convert \
  -i data.geojson \
  -o data.shp \
  --input-driver GeoJSON \
  --output-driver "ESRI Shapefile"

# Convert Shapefile to GeoPackage
geoetl-cli convert \
  -i cities.shp \
  -o cities.gpkg \
  --input-driver "ESRI Shapefile" \
  --output-driver GPKG

# Convert CSV to GeoJSON
geoetl-cli convert \
  -i locations.csv \
  -o locations.geojson \
  --input-driver CSV \
  --output-driver GeoJSON
```



#### Status

⚠️ **Phase 1 Development** - Command structure implemented, conversion logic pending.

---

### `info` - Display Dataset Information

Display information about a vector dataset including layer details, geometry types, coordinate reference system, and statistics.

#### Usage

```bash
geoetl-cli info [OPTIONS] <DATASET>
```

#### Options

| Option | Description |
|--------|-------------|
| `<DATASET>` | Input dataset path (required) |
| `--detailed` | Show detailed layer information |
| `-s, --stats` | Show statistics for each field |

#### Examples

```bash
# Basic information
geoetl-cli info data.geojson

# Detailed information
geoetl-cli info --detailed data.shp

# Include field statistics
geoetl-cli info --stats data.gpkg

# Both detailed and stats
geoetl-cli info --detailed --stats cities.geojson

# With debug logging
geoetl-cli -d info --detailed --stats data.shp
```

#### Expected Output

```
Dataset: data.geojson
Driver: GeoJSON
Layer: features
Geometry Type: Point
Feature Count: 1,234
Extent: [-180.0, -90.0, 180.0, 90.0]
Coordinate System: EPSG:4326 (WGS 84)

Fields:
  - id (Integer)
  - name (String)
  - population (Integer)
  - area (Real)
```

#### Status

⚠️ **Phase 1 Development** - Command structure implemented, info logic pending.

---

### `drivers` - List Available Drivers

List all available vector format drivers and their capabilities.

#### Usage

```bash
geoetl-cli drivers
```

#### Examples

```bash
# List all drivers with their capabilities
geoetl-cli drivers
```

#### Output Example

```
Available Drivers (2 total):

+------------+--------------+-----------+-----------+-----------+
| Short Name | Long Name    | Info      | Read      | Write     |
+------------+--------------+-----------+-----------+-----------+
| GeoJSON    | GeoJSON      | Supported | Supported | Supported |
+------------+--------------+-----------+-----------+-----------+
| Parquet    | (Geo)Parquet | Supported | Supported | Supported |
+------------+--------------+-----------+-----------+-----------+
```

**Support Status:**
- **Supported**: Feature is fully implemented
- **Planned**: Feature is planned for future implementation
- **Not Supported**: Feature is not available

#### Status

✅ **Implemented** - Fully functional.

---

## Driver System

GeoETL has a registry of 68+ vector format drivers compatible with GDAL. Currently, 2 drivers are fully implemented.

### Driver Capabilities

Each driver has three capability flags:
- **Info**: Can read metadata about the dataset
- **Read**: Can read data from this format
- **Write**: Can write data to this format

### Currently Supported Formats

| Driver | Long Name | Info | Read | Write |
|--------|-----------|------|------|-------|
| GeoJSON | GeoJSON | Supported | Supported | Supported |
| Parquet | (Geo)Parquet | Supported | Supported | Supported |

### Planned Formats (Phase 2)

This is a partial list of formats planned for implementation in Phase 2. For a complete list of all drivers, run `geoetl-cli drivers`.

| Driver | Long Name | Status |
|--------|-----------|--------|
| GeoJSONSeq | GeoJSONSeq: sequence of GeoJSON features | Planned |
| ESRI Shapefile | ESRI Shapefile / DBF | Planned |
| GPKG | GeoPackage vector | Planned |
| FlatGeobuf | FlatGeobuf | Planned |
| Arrow | (Geo)Arrow IPC File Format / Stream | Planned |
| CSV | Comma Separated Value (.csv) | Planned |

For a complete list of all drivers and their current status, run:
```bash
geoetl-cli drivers
```

---

## Common Workflows

### Checking Available Drivers

```bash
# List all available drivers with their capabilities
geoetl-cli drivers
```

### Format Conversion Workflow

```bash
# 1. Check supported formats
geoetl-cli drivers

# 2. Convert your data
geoetl-cli convert \
  -i input.geojson \
  -o output.parquet \
  --input-driver GeoJSON \
  --output-driver Parquet

# 3. View information about the result
geoetl-cli info --detailed --stats output.parquet
```



### Dataset Information Workflow

```bash
# Get basic dataset information
geoetl-cli info data.geojson

# Get detailed information with statistics
geoetl-cli info --detailed --stats data.geojson
```

---

## Logging and Debugging

### Logging Levels

GeoETL uses structured logging with three levels:

1. **WARN (default)**: Only warnings and errors
2. **INFO (--verbose, -v)**: Informational messages about operations
3. **DEBUG (--debug, -d)**: Detailed debugging information

### Examples

```bash
# Default: Only show warnings
geoetl-cli convert -i input.geojson -o output.shp

# Verbose: Show progress and information
geoetl-cli -v convert -i input.geojson -o output.shp

# Debug: Show detailed debugging information
geoetl-cli -d convert -i input.geojson -o output.shp
```

### Typical Output

**Default (WARN):**
```bash
$ geoetl-cli convert -i data.geojson -o data.shp
# (Only errors/warnings shown)
```

**Verbose (INFO):**
```bash
$ geoetl-cli -v convert -i data.geojson -o data.parquet --input-driver GeoJSON --output-driver Parquet
2025-10-23T00:00:00.000000Z  INFO Converting data.geojson to data.parquet
2025-10-23T00:00:00.100000Z  INFO Convert command:
2025-10-23T00:00:00.100000Z  INFO Input: data.geojson
2025-10-23T00:00:00.100000Z  INFO Output: data.parquet
2025-10-23T00:00:00.100000Z  INFO Input driver: GeoJSON
2025-10-23T00:00:00.100000Z  INFO Output driver: Parquet
2025-10-23T00:00:00.200000Z  WARN Not yet implemented - Phase 1 development
```

**Debug (DEBUG):**
```bash
$ geoetl-cli -d info data.geojson --detailed --stats
2025-10-23T00:00:00.000000Z  INFO Displaying info for data.geojson
2025-10-23T00:00:00.100000Z  INFO Info command:
2025-10-23T00:00:00.100000Z  INFO Input: data.geojson
2025-10-23T00:00:00.100000Z DEBUG Detailed: true
2025-10-23T00:00:00.100000Z DEBUG Stats: true
2025-10-23T00:00:00.200000Z  WARN Not yet implemented - Phase 1 development
```

---

## Troubleshooting

### Command Not Found

If you get "command not found", ensure the binary is in your PATH or use the full path:

```bash
# Use full path
/path/to/geoetl/target/release/geoetl-cli --help

# Or use cargo run
cargo run -p geoetl-cli -- --help
```

### Driver Not Found

If a driver is not recognized:

```bash
# List all available drivers with their capabilities
geoetl-cli drivers

# Check if a specific driver is supported
geoetl-cli drivers | grep -i "driver_name"
```

### Getting Help

```bash
# General help
geoetl-cli --help

# Command-specific help
geoetl-cli convert --help
geoetl-cli info --help
geoetl-cli drivers --help
```

---

## Development Status

### ✅ Phase 1 (Current)
- [x] CLI framework and argument parsing
- [x] Driver registry (68+ drivers, 2 currently supported)
- [x] Logging infrastructure
- [x] Command structure (convert, info, drivers)
- [x] Tabled-based output formatting

### 🚧 Phase 2 (In Progress)
- [ ] Basic vector I/O (GeoJSON, Parquet)
- [ ] DataFusion integration
- [ ] Info command implementation
- [ ] Convert command implementation
- [ ] Driver auto-detection from file extensions

### 📅 Phase 3 (Planned)
- [ ] Advanced spatial operations
- [ ] Performance optimization
- [ ] Additional format support (Shapefile, GeoPackage, FlatGeobuf, etc.)
- [ ] Benchmarking against GDAL

### 🔮 Phase 4 (Future)
- [ ] Distributed processing (Ballista)
- [ ] Cloud storage support
- [ ] CRS transformations
- [ ] Spatial indexing

---

## Contributing

GeoETL is under active development and we welcome contributions!

For development setup, contribution guidelines, and how to get started, see the [Development Guide](DEVELOPMENT.md).

---

## License

(Add your license information here)

---

## Links

- [GitHub Repository](https://github.com/yourusername/geoetl)
- [Development Guide](DEVELOPMENT.md)
- [Vision Document](VISION.md)
- [Issue Tracker](https://github.com/yourusername/geoetl/issues)
