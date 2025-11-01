# GeoJSON - User Guide

Complete guide for reading and querying GeoJSON files using DataFusion and GeoArrow.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Basic Usage](#basic-usage)
- [GeoJSON Formats](#geojson-formats)
- [Configuration Options](#configuration-options)
- [Advanced Usage](#advanced-usage)
- [Cloud Storage](#cloud-storage)
- [Performance Tips](#performance-tips)
- [Troubleshooting](#troubleshooting)

## Overview

The `datafusion-geojson` crate provides native GeoJSON format support for Apache DataFusion. It reads GeoJSON files and converts geometries to GeoArrow format for efficient columnar processing.

### Key Features

- **Native GeoJSON Support**: Read FeatureCollections, Features, and Geometries
- **GeoJSON Sequence**: Support for newline-delimited GeoJSON
- **GeoArrow Output**: Geometries stored in efficient columnar format
- **Schema Inference**: Automatic property type detection
- **Cloud Storage**: Native support for S3, Azure Blob, GCS, and HTTP
- **SQL Queries**: Full DataFusion SQL support
- **Streaming**: Process large files without loading entirely into memory

### Supported Geometry Types

- Point
- LineString
- Polygon
- MultiPoint
- MultiLineString
- MultiPolygon
- GeometryCollection (stored as generic Geometry type)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
datafusion = "50.0"
datafusion-geojson = { path = "../path/to/datafusion-geojson" }
tokio = { version = "1.28", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

```rust
use datafusion::prelude::*;
use datafusion_geojson::SessionContextGeoJsonExt;

#[tokio::main]
async fn main() -> datafusion::error::Result<()> {
    let ctx = SessionContext::new();

    // Register a GeoJSON file
    ctx.register_geojson_file("cities", "data/cities.geojson").await?;

    // Query with SQL
    let df = ctx.sql("SELECT name, geometry FROM cities WHERE name = 'Paris'").await?;
    df.show().await?;

    Ok(())
}
```

## Basic Usage

### Reading GeoJSON Files

#### Simple Registration

```rust
use datafusion::prelude::*;
use datafusion_geojson::SessionContextGeoJsonExt;

#[tokio::main]
async fn main() -> datafusion::error::Result<()> {
    let ctx = SessionContext::new();

    // Register with default options
    ctx.register_geojson_file("places", "data/places.geojson").await?;

    // Query the data
    let df = ctx.sql("SELECT * FROM places WHERE population > 100000").await?;
    df.show().await?;

    Ok(())
}
```

#### Direct DataFrame Creation

```rust
use datafusion_geojson::SessionContextGeoJsonExt;

// Read directly into DataFrame
let df = ctx.read_geojson_file("data/cities.geojson").await?;

// Use DataFrame API
let result = df
    .filter(col("country").eq(lit("France")))?
    .select_columns(&["name", "geometry"])?
    .collect()
    .await?;
```

### Custom Configuration

```rust
use datafusion_geojson::{SessionContextGeoJsonExt, GeoJsonFormatOptions};

let options = GeoJsonFormatOptions::default()
    .with_batch_size(4096)
    .with_geometry_column_name("geom");

ctx.register_geojson_with_options(
    "places",
    "data/places.geojson",
    options
).await?;
```

## GeoJSON Formats

### FeatureCollection

Standard GeoJSON format with multiple features:

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "Point",
        "coordinates": [2.3522, 48.8566]
      },
      "properties": {
        "name": "Paris",
        "country": "France",
        "population": 2161000
      }
    }
  ]
}
```

Usage:

```rust
ctx.register_geojson_file("cities", "data/cities.geojson").await?;
```

### Single Feature

A GeoJSON file containing just one Feature:

```json
{
  "type": "Feature",
  "geometry": {
    "type": "Polygon",
    "coordinates": [[...]]
  },
  "properties": {
    "name": "France"
  }
}
```

### Single Geometry

A GeoJSON file containing just geometry:

```json
{
  "type": "Point",
  "coordinates": [2.3522, 48.8566]
}
```

### GeoJSON Sequence (Newline-Delimited)

Features separated by newlines for streaming:

```json
{"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":{"id":1}}
{"type":"Feature","geometry":{"type":"Point","coordinates":[1,1]},"properties":{"id":2}}
{"type":"Feature","geometry":{"type":"Point","coordinates":[2,2]},"properties":{"id":3}}
```

The parser automatically detects and handles GeoJSON sequences.

## Configuration Options

### GeoJsonFormatOptions

```rust
use datafusion_geojson::GeoJsonFormatOptions;
use geoarrow_schema::{CoordType, GeometryType};
use std::sync::Arc;

let options = GeoJsonFormatOptions::default()
    .with_batch_size(8192)              // Rows per batch
    .with_geometry_column_name("geom")  // Custom geometry column name
    .with_geometry_type(                // Specify geometry type
        GeometryType::new(Arc::default())
            .with_coord_type(CoordType::Interleaved)
    )
    .with_schema_infer_max_features(Some(1000)); // Limit schema sampling
```

### Batch Size

Control memory usage and throughput:

```rust
let options = GeoJsonFormatOptions::default()
    .with_batch_size(4096);  // Smaller batches = less memory
```

### Geometry Column Name

Customize the geometry column name:

```rust
let options = GeoJsonFormatOptions::default()
    .with_geometry_column_name("location");

// Query using custom name
let df = ctx.sql("SELECT name, location FROM places").await?;
```

### Schema Inference Sampling

Limit how many features are sampled for schema inference:

```rust
let options = GeoJsonFormatOptions::default()
    .with_schema_infer_max_features(Some(100));  // Sample first 100 features
```

## Advanced Usage

### DataFrame API

```rust
use datafusion::prelude::*;

let df = ctx.read_geojson_file("data/cities.geojson").await?;

// Complex query with DataFrame API
let result = df
    .filter(col("population").gt(lit(1_000_000)))?
    .select_columns(&["name", "country", "geometry"])?
    .sort(vec![col("population").sort(false, true)])?
    .limit(0, Some(10))?
    .collect()
    .await?;
```

### Extracting Geometry Data

After querying, extract GeoArrow geometry arrays:

```rust
use geoarrow_array::array::GeometryArray;
use geo_traits::{GeometryTrait, PointTrait, CoordTrait};
use std::convert::TryFrom;

let df = ctx.sql("SELECT geometry, name FROM cities LIMIT 1").await?;
let batches = df.collect().await?;

let batch = &batches[0];
let schema = batch.schema();
let field = schema.field_with_name("geometry")?;
let column = batch.column(schema.index_of("geometry")?).clone();

// Convert to GeometryArray
let geometry_array = GeometryArray::try_from((column.as_ref(), field))?;

// Access first geometry
let first_geom = geometry_array.value(0)?;

// Extract point coordinates
let geo_traits::GeometryType::Point(point) = first_geom.as_type() else {
    panic!("Expected point");
};
let coord = point.coord().unwrap();
println!("Coordinates: ({}, {})", coord.x(), coord.y());
```

### Joining GeoJSON with Other Data

```rust
// Register multiple tables
ctx.register_geojson_file("cities", "data/cities.geojson").await?;
ctx.register_csv_file("population", "data/population.csv").await?;

// Join them
let df = ctx.sql(
    "SELECT c.name, c.geometry, p.year, p.count
     FROM cities c
     JOIN population p ON c.id = p.city_id
     WHERE p.year = 2023"
).await?;
```

### Filtering by Bounding Box

```rust
// Filter cities within a bounding box
let df = ctx.sql(
    "SELECT name, geometry
     FROM cities
     WHERE latitude BETWEEN 48.0 AND 49.0
       AND longitude BETWEEN 2.0 AND 3.0"
).await?;
```

### Aggregations

```rust
// Count features by property
let df = ctx.sql(
    "SELECT country, COUNT(*) as city_count
     FROM cities
     GROUP BY country
     ORDER BY city_count DESC"
).await?;
```

## Cloud Storage

GeoJSON reader automatically supports cloud storage:

### Amazon S3

```rust
ctx.register_geojson_file(
    "s3_data",
    "s3://my-bucket/data/cities.geojson"
).await?;
```

Configure credentials:
```bash
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_REGION=us-east-1
```

### Google Cloud Storage

```rust
ctx.register_geojson_file(
    "gcs_data",
    "gs://my-bucket/data/cities.geojson"
).await?;
```

### Azure Blob Storage

```rust
ctx.register_geojson_file(
    "azure_data",
    "az://my-container/data/cities.geojson"
).await?;
```

### HTTP/HTTPS

```rust
ctx.register_geojson_file(
    "remote_data",
    "https://example.com/data/cities.geojson"
).await?;
```

## Performance Tips

### 1. Optimize Batch Size

```rust
// For large files with memory available
let options = GeoJsonFormatOptions::default().with_batch_size(16384);

// For memory-constrained environments
let options = GeoJsonFormatOptions::default().with_batch_size(2048);
```

### 2. Use Projection Pushdown

Only select columns you need:

```rust
// Good: Only reads necessary columns
let df = ctx.sql("SELECT name, country FROM cities").await?;

// Less efficient: Reads all properties
let df = ctx.sql("SELECT * FROM cities").await?;
```

### 3. Filter Early

Apply filters before expensive operations:

```rust
// Good: Filter first
let df = ctx.sql(
    "SELECT name FROM cities
     WHERE country = 'France'
     ORDER BY population DESC"
).await?;
```

### 4. Limit Schema Inference

For very large files:

```rust
let options = GeoJsonFormatOptions::default()
    .with_schema_infer_max_features(Some(100));
```

### 5. Use GeoJSON Sequence for Large Files

For very large datasets, use newline-delimited GeoJSON which supports streaming better than FeatureCollection.

## Troubleshooting

### Schema Inference Issues

If schema inference fails or is slow:

```rust
// Increase sampling
let options = GeoJsonFormatOptions::default()
    .with_schema_infer_max_features(Some(1000));

// Or decrease for faster inference
let options = GeoJsonFormatOptions::default()
    .with_schema_infer_max_features(Some(50));
```

### Memory Issues

Reduce batch size:

```rust
let options = GeoJsonFormatOptions::default()
    .with_batch_size(1024);
```

### Geometry Type Mismatches

Ensure consistent geometry types in your GeoJSON:

```json
// All features should have same geometry type
{"type":"Feature","geometry":{"type":"Point",...},"properties":{...}}
{"type":"Feature","geometry":{"type":"Point",...},"properties":{...}}

// Avoid mixing types in same file:
{"type":"Feature","geometry":{"type":"Point",...},"properties":{...}}
{"type":"Feature","geometry":{"type":"Polygon",...},"properties":{...}}
```

### Null Geometries

The reader handles null geometries correctly:

```json
{
  "type": "Feature",
  "geometry": null,
  "properties": {"name": "Unknown Location"}
}
```

### Cloud Storage Authentication

Verify credentials are set:

```bash
# S3
echo $AWS_ACCESS_KEY_ID

# GCS
echo $GOOGLE_APPLICATION_CREDENTIALS

# Azure
echo $AZURE_STORAGE_ACCOUNT
```

### Invalid GeoJSON

Common issues:

1. **Missing required fields**:
```json
// Bad: Missing "type"
{"geometry": {...}, "properties": {...}}

// Good:
{"type": "Feature", "geometry": {...}, "properties": {...}}
```

2. **Invalid coordinates**:
```json
// Bad: Coordinates not in [longitude, latitude] order
{"type": "Point", "coordinates": [48.8566, 2.3522]}  // latitude, longitude

// Good:
{"type": "Point", "coordinates": [2.3522, 48.8566]}  // longitude, latitude
```

## Examples

### Complete Example: City Query

```rust
use datafusion::prelude::*;
use datafusion_geojson::SessionContextGeoJsonExt;

#[tokio::main]
async fn main() -> datafusion::error::Result<()> {
    let ctx = SessionContext::new();

    // Register GeoJSON file
    ctx.register_geojson_file("cities", "data/world-cities.geojson").await?;

    // Find major European cities
    let df = ctx.sql(
        "SELECT name, country, population, geometry
         FROM cities
         WHERE population > 1000000
           AND latitude BETWEEN 35.0 AND 70.0
           AND longitude BETWEEN -10.0 AND 40.0
         ORDER BY population DESC
         LIMIT 20"
    ).await?;

    df.show().await?;

    Ok(())
}
```

### Example: Multiple File Query

```rust
// Register multiple GeoJSON files
ctx.register_geojson_file("cities", "data/cities.geojson").await?;
ctx.register_geojson_file("countries", "data/countries.geojson").await?;

// Query across both
let df = ctx.sql(
    "SELECT ci.name as city, co.name as country
     FROM cities ci
     JOIN countries co ON ci.country_code = co.iso_code
     WHERE ci.population > 5000000"
).await?;
```

### Example: Aggregation and Grouping

```rust
ctx.register_geojson_file("cities", "data/cities.geojson").await?;

let df = ctx.sql(
    "SELECT
        country,
        COUNT(*) as num_cities,
        AVG(population) as avg_population,
        MAX(population) as max_population
     FROM cities
     GROUP BY country
     HAVING COUNT(*) > 10
     ORDER BY avg_population DESC"
).await?;
```

## Next Steps

- See [`geojson-development.md`](geojson-development.md) for implementation details
- Read the [GeoJSON Specification](https://geojson.org/)
- Explore [GeoArrow Specification](https://geoarrow.org/)
- Check [DataFusion SQL Reference](https://datafusion.apache.org/user-guide/sql/index.html)
