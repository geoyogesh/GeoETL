---
sidebar_position: 3
---

# Understanding Drivers

Learn about GeoETL's driver system and the 68+ supported geospatial formats.

## What are Drivers?

**Drivers** are modules that enable GeoETL to read from and write to different geospatial file formats. Think of them as translators that understand specific file formats.

Each driver has three capabilities:
- **Info**: Read metadata about a dataset
- **Read**: Load data from a file
- **Write**: Save data to a file

## Listing Available Drivers

View all supported drivers:

```bash
geoetl-cli drivers
```

This shows a table with:
- **Short Name**: The driver identifier you use in commands
- **Long Name**: Full descriptive name
- **Info**: Metadata support status
- **Read**: Read capability status
- **Write**: Write capability status

### Support Status

Each capability has one of three statuses:

| Status | Meaning | Available? |
|--------|---------|------------|
| **Supported** | Fully implemented and working | ✅ Yes |
| **Planned** | Will be implemented in future | 🚧 Soon |
| **Not Supported** | Not planned for implementation | ❌ No |

## Currently Working Drivers

### CSV - Comma Separated Value

**Status**: ✅ Fully Supported

**Use cases**:
- Simple tabular data with geometries
- Excel-compatible format
- Data analysis and visualization

**Geometry format**: WKT (Well-Known Text)

**Example**:
```bash
geoetl-cli convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column wkt
```

**Sample CSV with WKT**:
```csv
id,name,population,wkt
1,San Francisco,873965,"POINT(-122.4194 37.7749)"
2,New York,8336817,"POINT(-74.006 40.7128)"
```

### GeoJSON - Geographic JSON

**Status**: ✅ Fully Supported

**Use cases**:
- Web mapping applications
- JavaScript/web development
- Human-readable format
- Version control friendly

**Format**: JSON with geometry objects

**Example**:
```bash
geoetl-cli convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV
```

**Sample GeoJSON**:
```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {"name": "San Francisco"},
      "geometry": {
        "type": "Point",
        "coordinates": [-122.4194, 37.7749]
      }
    }
  ]
}
```

## Planned Drivers (Coming Soon)

### High Priority (Q1-Q2 2026)

#### GeoPackage (GPKG)
**Status**: 🚧 Planned

**Use cases**:
- SQLite-based geospatial database
- OGC standard format
- Mobile and offline applications
- Large datasets

**Will support**:
- Multiple layers per file
- Spatial indexing
- Attributes and metadata

#### ESRI Shapefile
**Status**: 🚧 Planned

**Use cases**:
- Industry standard format
- GIS software compatibility
- Legacy data conversion
- Wide tool support

**Will support**:
- .shp, .shx, .dbf, .prj files
- Attribute data
- Coordinate systems

#### Parquet / GeoParquet
**Status**: 🚧 Planned

**Use cases**:
- Big data analytics
- Cloud storage optimization
- Columnar format benefits
- Fast querying

**Will support**:
- Efficient compression
- Column pruning
- Predicate pushdown
- Metadata preservation

#### FlatGeobuf (FGB)
**Status**: 🚧 Planned

**Use cases**:
- Streaming data
- Web mapping
- Large dataset handling
- HTTP range requests

**Will support**:
- Spatial indexing
- Cloud-optimized
- Partial reads

## Driver Catalog by Category

### Vector Formats

**Core Formats** (2 working, more planned):
- ✅ GeoJSON, GeoJSONSeq
- ✅ CSV (with WKT)
- 🚧 ESRI Shapefile
- 🚧 GeoPackage (GPKG)
- 🚧 FlatGeobuf
- 🚧 (Geo)Parquet
- 🚧 (Geo)Arrow IPC

### Database Formats

**Planned**:
- 🚧 PostgreSQL/PostGIS
- 🚧 MySQL
- 🚧 SQLite/Spatialite
- 🚧 Oracle Spatial
- 🚧 Microsoft SQL Server
- 🚧 MongoDB

### CAD & Engineering

**Planned**:
- 🚧 AutoCAD DXF
- 🚧 AutoCAD DWG
- 🚧 Microstation DGN
- 🚧 ESRI File Geodatabase

### Web Services

**Planned**:
- 🚧 OGC WFS (Web Feature Service)
- 🚧 OGC API - Features
- 🚧 Carto
- 🚧 Elasticsearch
- 🚧 Google Earth Engine

## Choosing the Right Driver

Use this guide to select the best format:

### For Web Applications
→ **GeoJSON**
- JavaScript-friendly
- Human-readable
- Wide browser support

### For Data Analysis
→ **CSV** (now) or **Parquet** (coming soon)
- Excel/spreadsheet compatible
- Easy to inspect
- Good for small-medium datasets

### For Large Datasets
→ **GeoPackage** or **Parquet** (coming soon)
- Efficient storage
- Spatial indexing
- Query performance

### For GIS Software Compatibility
→ **Shapefile** or **GeoPackage** (coming soon)
- Industry standard
- Universal support
- Metadata preservation

### For Cloud/Big Data
→ **Parquet** or **FlatGeobuf** (coming soon)
- Columnar storage
- Cloud-optimized
- Compression

## Using Drivers in Commands

### Basic Syntax

```bash
geoetl-cli convert \
  --input <file> \
  --output <file> \
  --input-driver <DRIVER> \
  --output-driver <DRIVER>
```

### Driver Name Rules

1. **Case-sensitive**: Use exact capitalization
   ```bash
   # ✅ Correct
   --input-driver GeoJSON

   # ❌ Wrong
   --input-driver geojson
   ```

2. **Use Short Name**: From the drivers table
   ```bash
   # ✅ Correct
   --input-driver CSV

   # ❌ Wrong
   --input-driver "Comma Separated Value (.csv)"
   ```

3. **Check availability**: Use `geoetl-cli drivers` to verify

## Driver Capabilities

### Checking Read Support

Before using a driver as input, verify it supports reading:

```bash
geoetl-cli drivers | grep -i "shapefile"
```

Look for "Supported" in the "Read" column.

### Checking Write Support

Before using a driver as output, verify it supports writing:

```bash
geoetl-cli drivers | grep -i "geojson"
```

Look for "Supported" in the "Write" column.

### Error Handling

If you try to use an unsupported operation:

```bash
# This will fail because GML doesn't support read yet
geoetl-cli convert -i data.gml -o data.geojson \
  --input-driver GML --output-driver GeoJSON
```

Error message:
```
Error: Input driver 'GML' does not support reading.
```

## Geometry Format Support

Different drivers handle geometries differently:

| Driver | Geometry Format | Example |
|--------|----------------|---------|
| GeoJSON | Native GeoJSON | `{"type": "Point", "coordinates": [x, y]}` |
| CSV | WKT (Well-Known Text) | `"POINT(x y)"` |
| Shapefile | Binary format | (not human-readable) |
| GeoPackage | Binary format | (not human-readable) |
| Parquet | GeoArrow/WKB | (columnar binary) |

**WKT Examples**:
```
POINT(x y)
LINESTRING(x1 y1, x2 y2, x3 y3)
POLYGON((x1 y1, x2 y2, x3 y3, x1 y1))
MULTIPOINT((x1 y1), (x2 y2))
```

## Common Driver Combinations

### GeoJSON ↔ CSV
```bash
# GeoJSON to CSV
geoetl-cli convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV

# CSV to GeoJSON
geoetl-cli convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column wkt
```

### Round-trip Testing
```bash
# Original → CSV → Back
geoetl-cli convert -i original.geojson -o temp.csv \
  --input-driver GeoJSON --output-driver CSV

geoetl-cli convert -i temp.csv -o recovered.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column geometry
```

## Future Driver Features

Coming in future releases:

### Phase 2 (Q2 2026)
- Driver auto-detection from file extensions
- More format drivers (10-15 total)
- Enhanced error messages

### Phase 3 (Q3 2026)
- Advanced driver options
- Custom driver plugins
- Format-specific optimizations

### Phase 4 (Q4 2026)
- Cloud storage drivers (S3, Azure, GCS)
- Database connection pooling
- Streaming data sources

## Driver Development

Want to contribute a driver? See:

- [Contributing Guide](https://github.com/geoyogesh/geoetl/blob/main/docs/DEVELOPMENT.md)
- [DataFusion Integration Guide](https://github.com/geoyogesh/geoetl/blob/main/docs/DATAFUSION_GEOSPATIAL_FORMAT_INTEGRATION_GUIDE.md)
- [Driver Implementation ADR](https://github.com/geoyogesh/geoetl/blob/main/docs/adr/0001-high-level-architecture.md)

## Quick Reference

```bash
# List all drivers
geoetl-cli drivers

# Search for specific driver
geoetl-cli drivers | grep -i "parquet"

# Check driver capabilities
geoetl-cli drivers | grep -i "csv"

# Get command help
geoetl-cli convert --help
```

## Key Takeaways

🎯 **What you learned**:
- What drivers are and how they work
- Which drivers are currently available
- How to check driver capabilities
- How to use drivers in commands
- Planned future drivers

🚀 **Skills unlocked**:
- Choosing the right format for your use case
- Understanding driver limitations
- Planning data workflows

## Next Steps

Continue learning:

👉 **Next: [Working with CSV](./working-with-csv)** - CSV-specific operations and tips

Or explore:
- [Common Operations](./common-operations) - Essential commands and workflows

## Need Help?

- **Command help**: `geoetl-cli drivers`
- **GitHub Issues**: [Report problems](https://github.com/geoyogesh/geoetl/issues)
- **GitHub Discussions**: [Ask questions](https://github.com/geoyogesh/geoetl/discussions)
