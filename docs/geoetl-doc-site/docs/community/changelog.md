---
sidebar_position: 3
title: Changelog
description: GeoETL version history and changes
---

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2025-12-07

### Added

- **ST_Area Spatial UDF** ([#spatial-udfs](https://github.com/geoyogesh/geoetl/commit/HEAD))
  - Calculate area of polygon geometries using GEOS
  - Returns 0 for Points and LineStrings (non-areal geometries)
  - Supports WKB binary, GeoArrow native types, and mixed geometry arrays
  - Proper null value propagation

- **ST_Length Spatial UDF** ([#spatial-udfs](https://github.com/geoyogesh/geoetl/commit/HEAD))
  - Calculate length of LineStrings and perimeter of Polygons using GEOS
  - Returns 0 for Point geometries
  - Supports WKB binary, GeoArrow native types, and mixed geometry arrays
  - Proper null value propagation

### Changed

- **Spatial UDF Documentation**
  - Updated module documentation with new functions
  - Added `ST_Area` and `ST_Length` to registered UDF list

---

## [0.5.0] - 2025-11-28

### Added

- **Spatial UDFs for SQL Queries** ([#5abd25d](https://github.com/geoyogesh/geoetl/commit/5abd25d), [#1545ef1](https://github.com/geoyogesh/geoetl/commit/1545ef1))
  - **ST_Distance**: Calculate minimum distance between geometries using GEOS
    - Supports WKT strings, WKB binary, and GeoArrow native types
    - Optimized point-to-point distance using direct Euclidean calculation
    - Works with mixed geometry types from GeoJSON (Union arrays)
  - **ST_Point / ST_MakePoint**: Create point geometry from X, Y coordinates
  - **ST_GeomFromText**: Parse WKT strings to GeoArrow WKB format
  - **ST_GeomFromWKB**: Validate and tag WKB binary as GeoArrow geometry

- **GeoArrow Native Type Support** ([#5abd25d](https://github.com/geoyogesh/geoetl/commit/5abd25d))
  - New `geoarrow_types.rs` module with extension type utilities
  - Support for GeoArrow Struct (Point with x/y fields)
  - Support for GeoArrow FixedSizeList (interleaved coordinates)
  - Proper null value propagation across all geometry operations

- **Static GEOS Linking** ([#855abb5](https://github.com/geoyogesh/geoetl/commit/855abb5))
  - Added `geos-static` feature flag for portable release binaries
  - Release binaries no longer require GEOS to be installed on target systems
  - CI uses dynamic linking for speed; releases use static linking for portability

### Changed

- **Architecture Refactoring**
  - Separated construction functions from spatial operations
  - Added geozero dependency for geometry format conversion
  - `geoetl-core` now depends on `geoetl-operations` for spatial UDF registration

---

## [0.4.2] - 2025-11-15

### Fixed

- **GeoParquet CRS Handling** ([#0e287dc](https://github.com/geoyogesh/geoetl/commit/0e287dc))
  - Extract CRS from GeoParquet schema-level metadata
  - Properly read coordinate reference system information from all GeoParquet files

- **Table Name Sanitization** ([#343355a](https://github.com/geoyogesh/geoetl/commit/343355a))
  - Sanitize table names to handle special characters in filenames
  - Prevents SQL parsing errors when filenames contain spaces or special characters

---

## [0.4.1] - 2025-11-10

### Changed

- **CI Optimization** ([#5b481f0](https://github.com/geoyogesh/geoetl/commit/5b481f0))
  - Optimized CI pipeline by combining build, test, and coverage jobs
  - Added e2e test coverage flag

- **Code Refactoring** ([#5ba905c](https://github.com/geoyogesh/geoetl/commit/5ba905c))
  - Moved table name inference to format-specific implementations
  - Improved modularity and separation of concerns

- **Documentation Improvements** ([#6dbd918](https://github.com/geoyogesh/geoetl/commit/6dbd918))
  - Improved website with custom icons and accurate content

### Dependencies

- Enhanced Dependabot configuration with comprehensive monitoring

---

## [0.4.0] - 2025-11-08

### Added

- **SQL Query Support** ([#54e1ab5](https://github.com/geoyogesh/geoetl/commit/54e1ab5))
  - Added `--sql` flag to execute SQL queries on input datasets during conversion
  - Added `--table-name` flag to override auto-inferred table names
  - Automatic table name inference from input filenames (e.g., `cities.csv` → table `"cities"`)
  - Full DataFusion SQL capabilities: WHERE, SELECT, JOIN, GROUP BY, ORDER BY, LIMIT
  - Support for filtering, column selection, aggregations, sorting, and limiting results
  - Enables data transformation workflows without intermediate files

- **Comprehensive SQL Testing**
  - Added 7 integration tests covering SQL query functionality:
    - SQL filtering with WHERE clauses
    - Column selection with SELECT
    - Aggregations with GROUP BY
    - Sorting with ORDER BY
    - Custom table name overrides
    - Invalid SQL query error handling
    - Multi-step filter and transform workflows

- **SQL Documentation**
  - Added 5 SQL query examples to convert.md (Examples 7-11)
  - Added 3 common SQL workflow examples
  - Documented table name inference behavior
  - Added data processing options section

- **Release Blog Posts** ([#6aa66f6](https://github.com/geoyogesh/geoetl/commit/6aa66f6))
  - Added missing release blog posts for v0.1.2, v0.2.0, and v0.3.1
  - Created release blog post guidelines in docs/README_blog_release_post.md

### Changed

- **Documentation Restructure** ([#8526f07](https://github.com/geoyogesh/geoetl/commit/8526f07))
  - Established single source of truth for documentation in website
  - Removed promotional content and duplicated documentation
  - Added comprehensive community section (changelog, contributing, roadmap)
  - Reorganized drivers documentation with dedicated vector format pages
  - Added detailed getting started guides and tutorials
  - Created programs reference section with command documentation
  - Added FAQ and glossary for better discoverability
  - Improved troubleshooting guide
  - Total: 4,442 additions, 1,628 deletions across 38 files

- `convert` operation now accepts optional `sql_query` and `table_name_override` parameters
- `initialize_context` function now returns both `SessionContext` and inferred/custom table name
- All existing tests updated to include new optional SQL parameters

### Breaking Changes

None - all new parameters are optional and backward compatible

---

## [0.3.1] - 2025-11-06

### Added

- **Shell Completions Support** ([#218975b](https://github.com/geoyogesh/geoetl/commit/218975b))
  - Added `completions` subcommand to generate shell completion scripts
  - Support for 5 shells: bash, zsh, fish, powershell, and elvish
  - Enables tab completion for commands, subcommands, and options
  - Updated documentation with installation instructions and examples

- **New Geospatial Format Scaffolding** ([#c0f4932](https://github.com/geoyogesh/geoetl/commit/c0f4932))
  - Arrow IPC format module for zero-copy data exchange
  - GeoPackage format module for SQLite-based vector data
  - OpenStreetMap (OSM) format module for OSM PBF/XML data
  - Shapefile format module for ESRI Shapefile support

- **GeoParquet Streaming I/O Enhancements** ([#9631d93](https://github.com/geoyogesh/geoetl/commit/9631d93))
  - Implemented statistics inference for improved performance
  - Enhanced streaming I/O capabilities
  - Reduced memory usage for large file processing

### Changed

- **Documentation Updates**
  - Refactored GeoParquet ADR to follow Michael Nygard template ([#40a0a7c](https://github.com/geoyogesh/geoetl/commit/40a0a7c))
  - Added shell completions documentation to README.md, QUICKREF.md, and doc site
  - Removed version-specific annotations from documentation

### Dependencies

- Upgraded `geoarrow` from 0.5.0 to 0.6.2
- Added `clap_complete` 4.5.50 for shell completion generation

### Removed

- Removed performance tests from GeoParquet module in end to end tests

---

## [0.3.0] - 2025-11-04

### Added

- **GeoParquet Format Support** ([ADR 004](https://github.com/geoyogesh/geoetl/blob/main/docs/adr/004-streaming-geoparquet-architecture.md))
  - Implemented production-ready GeoParquet format with Apache Arrow and GeoArrow integration
  - Full read/write support with WKB (Well-Known Binary) geometry encoding
  - Streaming architecture with O(1) memory complexity
  - Native GeoArrow types: Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon
  - Schema preservation and GeoParquet metadata handling (CRS, bbox)
  - DataFusion FileFormat and DataSink integration

- **InsertOp::Overwrite Support**
  - Added Overwrite mode for GeoJSON writer
  - Added Overwrite mode for CSV writer
  - Enables file replacement without manual deletion

- **E2E Test Infrastructure for GeoParquet**
  - Comprehensive E2E tests with natural-earth dataset
  - GeoParquet roundtrip conversion tests
  - Cross-format conversion tests (GeoJSON ↔ GeoParquet, CSV ↔ GeoParquet)
  - Test data: natural-earth_cities.parquet (15KB, 243 features)

- **Comprehensive Documentation**
  - New tutorials for working with GeoParquet and GeoJSON
  - New reference pages for supported drivers
  - Blog post announcing GeoParquet support
  - Updated documentation with GeoParquet features

- **GeoParquet Benchmarks** ([bench/README.md](https://github.com/geoyogesh/geoetl/blob/main/bench/README.md#geoparquet-performance))
  - Added comprehensive GeoParquet benchmark suite
  - Performance comparison tables for all three formats
  - Conversion benchmark commands and results
  - Analysis of compression, throughput, and memory efficiency

### Changed

- **Documentation Structure**
  - Removed common-operations.md - consolidated into format-specific tutorials
  - Reorganized tutorial flow: Installation → First Conversion → Drivers → GeoJSON → CSV → GeoParquet → Troubleshooting
  - Added Reference section with Supported Drivers page
  - Updated all cross-references and navigation links

- **Driver Count Accuracy**
  - Updated documentation from "68+ drivers" to accurate "3 drivers (GeoJSON, CSV, GeoParquet)"
  - Added note about 68+ planned drivers via GDAL integration
  - Updated all driver count references throughout documentation

### Dependencies

- Added `parquet` v53.3.0
- Added `geoarrow` v0.5.0

### Breaking Changes

None

---

## [0.2.0] - 2025-11-03

### Added

- **Streaming CSV Architecture** ([ADR 002](https://github.com/geoyogesh/geoetl/blob/main/docs/adr/002-streaming-csv-architecture.md))
  - Implemented production-ready CSV format support with inline WKT geometry conversion
  - O(1) memory complexity: processes large files in constant memory
  - Production-ready throughput
  - Automatic single-partition write enforcement for proper CSV formatting
  - Inline geometry ↔ WKT conversion during streaming (no buffering)

- **Configurable Batch Size** ([#156](https://github.com/geoyogesh/geoetl/issues/156))
  - Added `--batch-size` CLI parameter for performance tuning
  - Default: 8,192 features (conservative, memory-efficient)
  - Users can tune memory/speed tradeoff for their workload

- **Configurable Partitioning Parameters**
  - Added `--read-partitions` CLI parameter to control parallel reading
  - Added `--write-partitions` CLI parameter to control parallel writing
  - CSV and GeoJSON formats automatically override write partitions to 1 with warning
  - Enables future parallel processing optimizations for other formats

- **GeoJSON Incremental Decoder** ([ADR 001](https://github.com/geoyogesh/geoetl/blob/main/docs/adr/001-streaming-geojson-architecture.md))
  - Implemented state machine-based incremental JSON parsing
  - Handles incomplete JSON chunks across byte stream boundaries
  - Eliminates OOM errors on large files
  - Supports FeatureCollection and newline-delimited GeoJSON formats

- **Comprehensive Benchmarking Infrastructure** ([bench/](https://github.com/geoyogesh/geoetl/tree/main/bench))
  - Real-time monitoring script with CPU, memory, disk I/O tracking
  - Automated benchmark suite with JSON result output
  - Data download scripts for Microsoft Buildings dataset
  - Systematic testing across multiple dataset sizes
  - Performance regression testing framework

- **Architecture Decision Records (ADRs)**
  - [ADR 001: Streaming GeoJSON Architecture](https://github.com/geoyogesh/geoetl/blob/main/docs/adr/001-streaming-geojson-architecture.md)
  - [ADR 002: Streaming CSV Architecture](https://github.com/geoyogesh/geoetl/blob/main/docs/adr/002-streaming-csv-architecture.md)
  - [ADR 003: GeoJSON Performance Optimization Strategy](https://github.com/geoyogesh/geoetl/blob/main/docs/adr/003-geojson-performance-optimization.md)

- **Factory Pattern for Writers**
  - Implemented `WriterFactory` trait for consistent writer creation
  - Added factory methods to CSV and GeoJSON formats
  - Increased test coverage for writer initialization

### Changed

- **GeoJSON Performance Optimization**
  - Updated default batch_size for optimal performance
  - Applied to both SessionConfig and physical execution plan

- **CSV Format Production-Ready**
  - Production-ready throughput
  - Efficient memory usage
  - Recommended for performance-critical workloads

- **Documentation Restructuring**
  - Removed outdated implementation docs (superseded by ADRs)
  - Enhanced DataFusion integration guide
  - Clear separation: ADRs (architecture), bench/README (procedures), blog (public)

### Fixed

- **GeoJSON Schema Inference** - Reduced memory usage from scanning entire file to sampling first 10 MB
- **GeoJSON Reader OOM** - Fixed out-of-memory errors on large files by implementing streaming decoder
- **CSV Write Partitioning** - Fixed invalid CSV output when write_partitions > 1 (now enforced to 1 with warning)

### Removed

- **Outdated Documentation** - Superseded by ADRs

---

## [0.1.2] - 2025-11-01

### Added

- **Custom Error Types**: Implemented comprehensive error handling system with `GeoEtlError` enum
  - Added specialized error types for IO, driver, format, conversion, validation, configuration, data processing, and geometry operations
  - Integrated error types across CLI, core, and operations crates
  - All error handling tests passing
- **Automated Documentation Deployment**: Integrated Cloudflare Pages deployment into release workflow
  - Documentation automatically deploys to production after GitHub release creation
  - Uses CircleCI with Wrangler CLI for deployment

### Changed

- **Documentation Reorganization**:
  - Removed redundant user guide (content on website)
  - Updated all references to point to website
  - Moved format-specific documentation to package directories

### Removed

- User guide markdown file - Superseded by documentation website
- Format documentation directory - Documentation moved to respective package directories

---

## [0.1.0] - 2025-10-31

### Added

- **Initial Project Structure**: Created Rust workspace with initial crates
- **CLI Framework**: Implemented robust CLI using `clap`
- **Core Commands**:
  - **`drivers`**: Lists all available vector format drivers and capabilities
  - **`convert`**: Initial implementation of format conversion
  - **`info`**: Placeholder for dataset metadata display
- **Driver and Operations Logic**: Driver registry and convert function
- **Unit Tests**: Initial unit tests for convert command handler
- **Documentation**: README, VISION, DEVELOPMENT, ADR documents
- **CI/CD**: CircleCI configuration and Makefile

---

## See Also

- [GitHub Releases](https://github.com/geoyogesh/geoetl/releases)
- [Roadmap](./roadmap.md)
- [Contributing Guide](./contributing.md)
