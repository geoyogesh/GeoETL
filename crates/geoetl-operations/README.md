# GeoETL Operations

Spatial operations and User Defined Functions (UDFs) for DataFusion, powered by GEOS.

## Overview

This crate provides spatial operations (like `ST_Distance`, `ST_Buffer`, etc.) as UDFs that can be used in DataFusion SQL queries within GeoETL.

## Features

### Currently Implemented

**Construction Functions** (convert data to GeoArrow geometry):
- **ST_Point** / **ST_MakePoint**: Create point geometry from X, Y coordinates
- **ST_GeomFromText**: Parse WKT (Well-Known Text) to geometry
- **ST_GeomFromWKB**: Validate and tag WKB (Well-Known Binary) as geometry

**Spatial Operations**:
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

### Example: Using Construction Functions

```sql
-- Create points from coordinates
SELECT ST_Point(lon, lat) as geometry FROM table;

-- Parse WKT strings
SELECT ST_GeomFromText('POINT(1 2)');

-- Calculate distance from a fixed point
SELECT name, ST_Distance(geometry, ST_Point(0, 0)) as dist_from_origin
FROM cities;
```

## Architecture

```
geoetl-operations/
├── src/
│   ├── lib.rs                      # Public API exports
│   └── spatial_udf/
│       ├── mod.rs                  # UDF registration
│       ├── geoarrow_types.rs       # GeoArrow type utilities
│       ├── st_point.rs             # ST_Point construction
│       ├── st_geomfromtext.rs      # ST_GeomFromText construction
│       ├── st_geomfromwkb.rs       # ST_GeomFromWKB construction
│       └── st_distance.rs          # ST_Distance operation
```

### Function Categories

1. **Construction Functions**: Convert input data to GeoArrow geometry types
   - Input: Coordinates, WKT strings, WKB binary
   - Output: GeoArrow geometry (`geoarrow.point`, `geoarrow.wkb`, etc.)

2. **Spatial Operations**: Compute results from geometry inputs
   - Input: GeoArrow geometry types
   - Output: Scalar values (Float64, Boolean, etc.) or new geometries

### GeoArrow Types

GeoArrow uses Arrow extension types with metadata to represent geospatial data:

| Type | Arrow Storage | Extension Name |
|------|---------------|----------------|
| Point | `FixedSizeList<Float64, 2>` | `geoarrow.point` |
| WKB | `Binary` | `geoarrow.wkb` |
| Mixed Geometry | `Union` | `geoarrow.geometry` |

Extension metadata is stored in the Arrow Field's metadata map:
```
ARROW:extension:name -> "geoarrow.point"
```

## Best Practices for Adding New Operations

### 1. Choose the Right Function Category

**Construction Function** (e.g., `ST_Point`, `ST_GeomFromText`):
- Converts non-geometry input to GeoArrow geometry
- Returns geometry with proper extension metadata
- Use `geoarrow_types.rs` helpers for output field creation

**Spatial Operation** (e.g., `ST_Distance`, `ST_Buffer`):
- Accepts GeoArrow geometry inputs
- Returns scalar values or new geometries
- Should handle multiple GeoArrow input types

### 2. File Structure Pattern

Create a new file `src/spatial_udf/st_<function>.rs`:

```rust
//! `ST_Function` implementation - brief description
//!
//! Detailed documentation about what the function does.

use super::geoarrow_types::{...};  // Import type utilities
use datafusion::arrow::array::{...};
use datafusion::logical_expr::{ScalarUDF, ...};
// ... other imports

/// Create the `ST_Function` User Defined Function
///
/// # SQL Usage
///
/// ```sql
/// SELECT ST_Function(geometry) FROM table;
/// ```
///
/// # Arguments
///
/// - `geometry`: Description of input
///
/// # Returns
///
/// Description of output
#[must_use]
pub fn create_st_function_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StFunctionUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StFunctionUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StFunctionUDF {
    fn new() -> Self {
        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    // Define accepted type signatures
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StFunctionUDF {
    // Implement required trait methods
}

#[cfg(test)]
mod tests {
    // Unit tests
}
```

### 3. Handle GeoArrow Types Properly

**For Construction Functions** - Return geometry with metadata:

```rust
use super::geoarrow_types::{point_field, wkb_field};

fn return_field_from_args(&self, args: ReturnFieldArgs) -> Result<FieldRef> {
    let nullable = args.arg_fields[0].is_nullable();
    Ok(Arc::new(point_field(self.name(), nullable)))  // or wkb_field()
}
```

**For Spatial Operations** - Accept multiple geometry types:

```rust
use super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry, GEOARROW_POINT, GEOARROW_WKB};

fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
    // Get field metadata for type detection
    let field = args.arg_fields.first().map(std::convert::AsRef::as_ref);
    let geo_type = field.and_then(get_geoarrow_type);

    // Handle different geometry types
    match geo_type {
        Some(GEOARROW_POINT) => handle_point(...),
        Some(GEOARROW_WKB) => handle_wkb(...),
        Some(GEOARROW_GEOMETRY) => handle_mixed(...),
        _ => // fallback or error
    }
}
```

### 4. Convert GeoArrow to GEOS

Use this pattern to convert GeoArrow arrays to GEOS geometries:

```rust
use geos::{CoordSeq, Geom, Geometry as GeosGeometry};

// GeoArrow Point -> GEOS
fn point_to_geos(coords: &Float64Array, idx: usize) -> Result<GeosGeometry, String> {
    let offset = idx * 2;
    let x = coords.value(offset);
    let y = coords.value(offset + 1);

    let coord_seq = CoordSeq::new_from_vec(&[[x, y]])
        .map_err(|e| format!("Failed to create CoordSeq: {e}"))?;

    GeosGeometry::create_point(coord_seq)
        .map_err(|e| format!("Failed to create GEOS point: {e}"))
}

// GeoArrow WKB -> GEOS
fn wkb_to_geos(wkb_array: &BinaryArray, idx: usize) -> Result<GeosGeometry, String> {
    let wkb = wkb_array.value(idx);
    GeosGeometry::new_from_wkb(wkb)
        .map_err(|e| format!("Invalid WKB at row {idx}: {e}"))
}

// GeoArrow Geometry (Union) -> GEOS
fn geometry_to_geos(arr: &ArrayRef, idx: usize, field: &Field) -> Result<GeosGeometry, String> {
    use geoarrow_array::array::GeometryArray;
    use geozero::{CoordDimensions, ToWkb};

    let geom_arr = GeometryArray::try_from((arr.as_ref(), field))?;
    let geom = geom_arr.value(idx)?;
    let wkb_bytes = geom.to_wkb(CoordDimensions::xy())?;
    GeosGeometry::new_from_wkb(&wkb_bytes)
}
```

### 6. Handle Nulls Correctly

Always check for and propagate null values:

```rust
for i in 0..len {
    if array.is_null(i) {
        builder.append_null();
        continue;
    }

    // Process non-null value
    let result = compute_value(...)?;
    builder.append_value(result);
}
```

### 7. Registration

Export your function in `src/spatial_udf/mod.rs`:

```rust
mod st_function;
pub use st_function::create_st_function_udf;

pub fn register_spatial_udfs(ctx: &SessionContext) {
    ctx.register_udf(create_st_function_udf());
    // ... other UDFs
}
```

And in `src/lib.rs`:

```rust
pub use spatial_udf::{create_st_function_udf, register_spatial_udfs};
```

### 8. Testing

Write tests at multiple levels:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udf_creation() {
        let udf = create_st_function_udf();
        assert_eq!(udf.name(), "st_function");
    }

    #[test]
    fn test_core_logic() {
        // Test the underlying computation
    }

    #[test]
    fn test_null_handling() {
        // Ensure nulls propagate correctly
    }
}
```

Add integration tests in `geoetl-core/tests/` for end-to-end SQL query testing.

## Technical Details

### Geometry Format

GeoArrow types are preferred over raw WKB for better performance:
- **Point**: `FixedSizeList<Float64, 2>` - zero-copy coordinate access
- **WKB**: `Binary` - requires parsing for each operation
- **Mixed Geometry**: `Union` - from GeoJSON, requires field metadata

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
- **GEOS**: C++ library for computational geometry
- **DataFusion**: In-memory query engine
- **GeoArrow**: Geospatial extension for Apache Arrow
- **geozero**: Geometry format conversion

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

## License

Same license as GeoETL (MIT).
