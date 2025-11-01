# GeoETL Quick Reference

Fast reference for GeoETL CLI commands and common operations.

## Installation & Help

```bash
# Build from source
cargo build --release

# Run with cargo
cargo run -p geoetl-cli -- [COMMAND]

# Get help
geoetl-cli --help
geoetl-cli [COMMAND] --help

# Version info
geoetl-cli --version
```

## Global Options

```bash
-v, --verbose    # INFO level logging
-d, --debug      # DEBUG level logging
-h, --help       # Show help
-V, --version    # Show version
```

## Commands Overview

| Command | Purpose | Status |
|---------|---------|--------|
| `convert` | Convert between formats | 🚧 Phase 2 |
| `info` | Display dataset info | 🚧 Phase 2 |
| `drivers` | List available drivers | ✅ Ready |

## Convert

```bash
# Convert with explicit drivers
geoetl-cli convert \
  -i input.geojson \
  -o output.parquet \
  --input-driver GeoJSON \
  --output-driver Parquet
```

## Info

```bash
# Basic info (I/O implementation in Phase 2)
geoetl-cli info data.geojson

# Detailed info with statistics (I/O implementation in Phase 2)
geoetl-cli info --detailed --stats data.shp
```

## Drivers

```bash
# List all drivers with their capabilities
geoetl-cli drivers
```

## Popular Drivers

### Core Formats
- `GeoJSON` - GeoJSON
- `ESRI Shapefile` - Shapefile / DBF
- `GPKG` - GeoPackage vector
- `FlatGeobuf` - FlatGeobuf
- `Parquet` - (Geo)Parquet
- `Arrow` - (Geo)Arrow IPC

### Databases
- `PostgreSQL` - PostgreSQL/PostGIS
- `MySQL` - MySQL
- `SQLite` - SQLite / Spatialite
- `MongoDBv3` - MongoDB

### Web/Cloud
- `WFS` - OGC WFS (read only)
- `OAPIF` - OGC API - Features (read only)
- `CARTO` - Carto
- `Elasticsearch` - Elasticsearch

### Other
- `CSV` - Comma Separated Value
- `DXF` - AutoCAD DXF
- `KML` - Keyhole Markup Language
- `GPX` - GPS Exchange Format
- `MVT` - Mapbox Vector Tiles
- `OSM` - OpenStreetMap (read only)

## Common Workflows

### Simple Conversion (Phase 2)
```bash
geoetl-cli convert \
  -i input.geojson \
  -o output.shp \
  --input-driver GeoJSON \
  --output-driver "ESRI Shapefile"
```

### Data Quality Check
```bash
geoetl-cli info --detailed --stats data.geojson
geoetl-cli validate --geometry --attributes data.geojson
```

## Logging Examples

### Default (WARN)
```bash
geoetl-cli convert -i input.geojson -o output.shp
```

### Verbose (INFO)
```bash
geoetl-cli -v convert -i input.geojson -o output.shp
```

### Debug (DEBUG)
```bash
geoetl-cli -d convert -i input.geojson -o output.shp
```

## Tips

1. **Driver specification**: Currently requires explicit `--input-driver` and `--output-driver` (auto-detection coming in Phase 2)
2. **Verbose output**: Add `-v` to see progress and detailed information
3. **Check capabilities**: Use `geoetl-cli drivers` to see what each driver supports
4. **Command help**: Every command has detailed help with `--help`
5. **Phase 1 Status**: CLI framework and driver registry are complete; file I/O implementation is Phase 2

## Error Troubleshooting

### Driver not found
```bash
# List available drivers
geoetl-cli drivers

# Check if driver supports read/write
geoetl-cli drivers --detailed | grep -i "driver_name"
```

### Command not working
```bash
# Check command help
geoetl-cli [COMMAND] --help

# Enable verbose logging
geoetl-cli -v [COMMAND] [OPTIONS]

# Enable debug logging
geoetl-cli -d [COMMAND] [OPTIONS]
```

## Development Status

- ✅ **Phase 1**: CLI framework, driver registry, logging
- 🚧 **Phase 2**: Vector I/O, DataFusion, command implementations
- 📅 **Phase 3**: Advanced operations, optimization, benchmarking
- 🔮 **Phase 4**: Distributed processing, cloud support, plugins

## More Information

- **Full Documentation**: See [Documentation Website](https://geoetl.com)
- **Development**: See [DEVELOPMENT.md](DEVELOPMENT.md)
- **Vision**: See [VISION.md](VISION.md)
