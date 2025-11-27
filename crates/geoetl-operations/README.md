# GeoETL Operations

Spatial operations and User Defined Functions (UDFs) for DataFusion, powered by GEOS.

## Overview

This crate provides spatial operations (like `ST_Distance`, `ST_Buffer`, etc.) as UDFs that can be used in DataFusion SQL queries within GeoETL.

## Features

### Currently Implemented

- **ST_Distance**: Calculate the minimum distance between two geometries

### Planned

- ST_Buffer
- ST_Contains
- ST_Intersects
- ST_Area
- ST_Length
- And more...

## Usage

The spatial UDFs are automatically registered when using GeoETL's convert operation with the `--sql` flag.

### Example: Calculate Distances Between Points (Self-Join)

```bash
geoetl convert \
  --input points.geojson \
  --output distances.csv \
  --sql "SELECT
           a.name as from_name,
           b.name as to_name,
           ST_Distance(a.geometry, b.geometry) as distance
         FROM points a
         CROSS JOIN points b
         WHERE a.name < b.name
         ORDER BY distance"
```

### Current Limitations

**Note:** ST_Distance currently works best with self-joins (comparing geometries from the same table).  Cross-table joins may encounter metadata issues as DataFusion UDFs receive columns without full schema context.

**Supported:**
- ✅ Self-joins: `SELECT ST_Distance(a.geom, b.geom) FROM table a, table b`
- ✅ Same table comparisons with aliasing

**Known Issues:**
- ⚠️ GeoArrow geometry columns may lose extension metadata in UDF context
- ⚠️ Cross-table joins between different datasources may fail

We're working on improving metadata preservation in future releases.

## Technical Details

### Geometry Format

Geometries must be in **WKB (Well-Known Binary)** format as `Binary` Arrow arrays. GeoETL automatically handles the conversion from:
- GeoJSON geometries
- GeoParquet geometries
- CSV WKT strings (when using `--geometry-column`)

### Distance Units

The distance returned by `ST_Distance` depends on the Coordinate Reference System (CRS):
- **Projected CRS** (e.g., UTM, State Plane): Returns distance in the units of the projection (usually meters)
- **Geographic CRS** (e.g., WGS84, EPSG:4326): Returns distance in degrees (not recommended for distance calculations)

For accurate distance calculations on geographic coordinates, consider:
1. Using a projected CRS appropriate for your region
2. Converting to a projected CRS before calculating distances
3. Using geodesic distance functions (planned for future releases)

## Dependencies

This crate uses:
- **GEOS 10.0**: C++ library for computational geometry
- **DataFusion 50.3**: In-memory query engine
- **GeoArrow**: Geospatial extension for Apache Arrow

### System Requirements

You need GEOS installed on your system:

**macOS:**
```bash
brew install geos
```

**Ubuntu/Debian:**
```bash
sudo apt-get install libgeos-dev
```

**Windows:**
Download from https://libgeos.org or use vcpkg

## Architecture

```
geoetl-operations/
├── src/
│   ├── lib.rs              # Public API
│   └── spatial_udf/
│       ├── mod.rs          # UDF registration
│       └── st_distance.rs  # ST_Distance implementation
```

The module follows this pattern:
1. Receive geometry columns as Arrow `BinaryArray` (WKB format)
2. Convert WKB to GEOS geometries
3. Perform spatial operation using GEOS
4. Return results as Arrow arrays

## Contributing

To add a new spatial function:

1. Create a new file in `src/spatial_udf/` (e.g., `st_buffer.rs`)
2. Implement the `ScalarUDFImpl` trait
3. Export the function in `src/spatial_udf/mod.rs`
4. Register it in the `register_spatial_udfs()` function

See `st_distance.rs` as a reference implementation.

## License

Same license as GeoETL (MIT).
