# CSV with GeoArrow - User Guide

Complete guide for reading and querying CSV files with geospatial data using DataFusion and GeoArrow.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Basic Usage](#basic-usage)
- [Geometry Formats](#geometry-formats)
- [Configuration Options](#configuration-options)
- [Advanced Usage](#advanced-usage)
- [Cloud Storage](#cloud-storage)
- [Performance Tips](#performance-tips)
- [Troubleshooting](#troubleshooting)

## Overview

The `datafusion-csv` crate provides seamless integration between CSV files containing geospatial data and Apache DataFusion's SQL query engine. It converts geometry data into GeoArrow format for efficient columnar processing.

### Key Features

- **WKT Geometry Parsing**: Read Well-Known Text (WKT) geometries and convert to GeoArrow
- **Flexible Schema**: Automatic schema inference with configurable options
- **Cloud Storage**: Native support for S3, Azure Blob, GCS, and HTTP
- **SQL Queries**: Full DataFusion SQL support for spatial data
- **DataFrame API**: Fluent, type-safe API for data manipulation
- **Performance**: Streaming reads with configurable batch sizes

### Supported Geometry Types

- Point
- LineString
- Polygon
- MultiPoint
- MultiLineString
- MultiPolygon
- GeometryCollection

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
datafusion = "50.0"
datafusion-csv = { path = "../path/to/datafusion-csv" }
tokio = { version = "1.28", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

```rust
use datafusion::prelude::*;
use datafusion_csv::{SessionContextCsvExt, CsvFormatOptions};
use geoarrow_schema::{Dimension, GeoArrowType, PointType};
use std::sync::Arc;

#[tokio::main]
async fn main() -> datafusion::error::Result<()> {
    let ctx = SessionContext::new();

    // Configure CSV to parse WKT geometries as Points
    let options = CsvFormatOptions::default()
        .with_geometry_from_wkt(
            "Geolocation",  // Column name containing WKT
            GeoArrowType::Point(PointType::new(Dimension::XY, Arc::default()))
        );

    // Register the CSV file
    ctx.register_csv_with_options("places", "data/places.csv", options).await?;

    // Query with SQL
    let df = ctx.sql("SELECT * FROM places WHERE name = 'New York'").await?;
    df.show().await?;

    Ok(())
}
```

## Basic Usage

### Reading Simple CSV Files

For CSV files without geometry:

```rust
use datafusion::prelude::*;
use datafusion_csv::SessionContextCsvExt;

#[tokio::main]
async fn main() -> datafusion::error::Result<()> {
    let ctx = SessionContext::new();

    // Simple registration
    ctx.register_csv_file("users", "data/users.csv").await?;

    // Query the data
    let df = ctx.sql("SELECT name, age FROM users WHERE age > 30").await?;
    df.show().await?;

    Ok(())
}
```

### Reading CSV with Custom Delimiter

```rust
use datafusion_csv::SessionContextCsvExt;

// TSV file with tab delimiter
ctx.register_csv_with_delimiter("data", "file.tsv", b'\t').await?;

// Pipe-delimited file
ctx.register_csv_with_delimiter("data", "file.psv", b'|').await?;
```

### Reading CSV with Geospatial Data

```rust
use datafusion_csv::CsvFormatOptions;
use geoarrow_schema::{Dimension, GeoArrowType, PointType};
use std::sync::Arc;

// CSV with WKT Point geometries
let options = CsvFormatOptions::default()
    .with_geometry_from_wkt(
        "geometry",  // Column containing WKT
        GeoArrowType::Point(PointType::new(Dimension::XY, Arc::default()))
    );

ctx.register_csv_with_options("cities", "data/cities.csv", options).await?;
```

## Geometry Formats

### Well-Known Text (WKT)

WKT is a text-based geometry format. Examples:

```
POINT(1.0 2.0)
LINESTRING(0 0, 1 1, 2 2)
POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))
MULTIPOINT((0 0), (1 1), (2 2))
```

CSV example with WKT:

```csv
id,name,geometry
1,Location A,POINT(-86.64301145 32.5350198)
2,Location B,POINT(-73.98546518 40.7484284)
3,Location C,POINT(-122.419418 37.774929)
```

### Parsing WKT Geometries

#### Point Geometries

```rust
use geoarrow_schema::{Dimension, GeoArrowType, PointType};

let options = CsvFormatOptions::default()
    .with_geometry_from_wkt(
        "geometry",
        GeoArrowType::Point(PointType::new(Dimension::XY, Arc::default()))
    );
```

#### Polygon Geometries

```rust
use geoarrow_schema::{Dimension, GeoArrowType, PolygonType};

let options = CsvFormatOptions::default()
    .with_geometry_from_wkt(
        "geometry",
        GeoArrowType::Polygon(PolygonType::new(Dimension::XY, Arc::default()))
    );
```

#### MultiPolygon Geometries

```rust
use geoarrow_schema::{Dimension, GeoArrowType, MultiPolygonType};

let options = CsvFormatOptions::default()
    .with_geometry_from_wkt(
        "boundary",
        GeoArrowType::MultiPolygon(MultiPolygonType::new(Dimension::XY, Arc::default()))
    );
```

## Configuration Options

### CSV Format Options

```rust
use datafusion_csv::CsvFormatOptions;

let options = CsvFormatOptions::new()
    .with_delimiter(b',')              // Default: comma
    .with_has_header(true)             // Default: true
    .with_batch_size(8192)             // Default: 8192 rows
    .with_schema_infer_max_records(100); // Sample size for schema inference
```

### Schema Inference

Control how many rows are sampled for schema inference:

```rust
let options = CsvFormatOptions::default()
    .with_schema_infer_max_records(1000);  // Sample first 1000 rows
```

### Batch Size

Configure how many rows are processed at once:

```rust
let options = CsvFormatOptions::default()
    .with_batch_size(4096);  // Smaller batches use less memory
```

## Advanced Usage

### DataFrame API

Use the DataFrame API for programmatic queries:

```rust
use datafusion::prelude::*;

let df = ctx.read_csv("data/cities.csv").await?;

// Filter, select, and aggregate
let result = df
    .filter(col("population").gt(lit(1_000_000)))?
    .select_columns(&["name", "country", "population"])?
    .sort(vec![col("population").sort(false, true)])?
    .limit(0, Some(10))?
    .collect()
    .await?;
```

### Combining with Geometry Parsing

```rust
use datafusion_csv::CsvFormatOptions;
use geoarrow_schema::{Dimension, GeoArrowType, PointType};

let options = CsvFormatOptions::default()
    .with_geometry_from_wkt(
        "location",
        GeoArrowType::Point(PointType::new(Dimension::XY, Arc::default()))
    )
    .with_batch_size(4096);

let df = ctx.read_csv_with_options("data/places.csv", options).await?;

let result = df
    .filter(col("city").eq(lit("Boston")))?
    .select_columns(&["name", "location"])?
    .collect()
    .await?;
```

### Working with Multiple Files

Register multiple CSV files:

```rust
// Register individual files
ctx.register_csv_file("cities_us", "data/us_cities.csv").await?;
ctx.register_csv_file("cities_eu", "data/eu_cities.csv").await?;

// Query across both tables
let df = ctx.sql(
    "SELECT * FROM cities_us
     UNION ALL
     SELECT * FROM cities_eu
     WHERE population > 500000"
).await?;
```

Or use glob patterns:

```rust
// Register all CSV files in a directory
ctx.register_csv("cities", "data/cities/*.csv").await?;
```

### Spatial Queries

After parsing geometries, you can query them:

```rust
// Example: Find all locations near a point
let df = ctx.sql(
    "SELECT name, geometry
     FROM locations
     WHERE latitude BETWEEN 40.0 AND 41.0
       AND longitude BETWEEN -74.0 AND -73.0"
).await?;
```

## Cloud Storage

The CSV reader automatically supports cloud storage through `object_store`.

### Amazon S3

```rust
ctx.register_csv_with_options(
    "s3_data",
    "s3://my-bucket/data/cities.csv",
    options
).await?;
```

Configure S3 credentials via environment variables:
```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_REGION=us-east-1
```

### Google Cloud Storage

```rust
ctx.register_csv_with_options(
    "gcs_data",
    "gs://my-bucket/data/cities.csv",
    options
).await?;
```

### Azure Blob Storage

```rust
ctx.register_csv_with_options(
    "azure_data",
    "az://my-container/data/cities.csv",
    options
).await?;
```

### HTTP/HTTPS

```rust
ctx.register_csv_with_options(
    "remote_data",
    "https://example.com/data/cities.csv",
    options
).await?;
```

## Performance Tips

### 1. Optimize Batch Size

Larger batches improve throughput but use more memory:

```rust
// For large files with ample memory
let options = CsvFormatOptions::default().with_batch_size(16384);

// For memory-constrained environments
let options = CsvFormatOptions::default().with_batch_size(2048);
```

### 2. Use Projection Pushdown

Only select columns you need:

```rust
// Good: Only reads necessary columns
let df = ctx.sql("SELECT name, population FROM cities").await?;

// Avoid: Reads all columns
let df = ctx.sql("SELECT * FROM cities").await?;
```

### 3. Schema Inference Sampling

For very large files, limit schema inference sampling:

```rust
let options = CsvFormatOptions::default()
    .with_schema_infer_max_records(100);  // Sample only 100 rows
```

### 4. Predicate Pushdown

Use filters early in the query:

```rust
// Good: Filter early
let df = ctx.sql(
    "SELECT name FROM cities WHERE country = 'USA'"
).await?;

// Less efficient: Filter after selecting all data
let df = ctx.sql(
    "SELECT name FROM cities"
).await?.filter(col("country").eq(lit("USA")))?;
```

## Troubleshooting

### Schema Inference Errors

If schema inference fails:

```rust
// Increase sampling
let options = CsvFormatOptions::default()
    .with_schema_infer_max_records(1000);
```

### Geometry Parsing Errors

Ensure WKT column name matches:

```rust
// Check your CSV column name
let options = CsvFormatOptions::default()
    .with_geometry_from_wkt(
        "geometry",  // Must match CSV header exactly
        GeoArrowType::Point(PointType::new(Dimension::XY, Arc::default()))
    );
```

### Memory Issues

Reduce batch size:

```rust
let options = CsvFormatOptions::default().with_batch_size(1024);
```

### Cloud Storage Authentication

Verify credentials are set:

```bash
# For S3
echo $AWS_ACCESS_KEY_ID
echo $AWS_SECRET_ACCESS_KEY

# For GCS
echo $GOOGLE_APPLICATION_CREDENTIALS
```

## Examples

### Complete Example: Spatial Query

```rust
use datafusion::prelude::*;
use datafusion_csv::{SessionContextCsvExt, CsvFormatOptions};
use geoarrow_schema::{Dimension, GeoArrowType, PointType};
use std::sync::Arc;

#[tokio::main]
async fn main() -> datafusion::error::Result<()> {
    let ctx = SessionContext::new();

    // Configure geometry parsing
    let options = CsvFormatOptions::default()
        .with_geometry_from_wkt(
            "location",
            GeoArrowType::Point(PointType::new(Dimension::XY, Arc::default()))
        );

    // Register CSV with spatial data
    ctx.register_csv_with_options(
        "restaurants",
        "data/restaurants.csv",
        options
    ).await?;

    // Find restaurants in a bounding box
    let df = ctx.sql(
        "SELECT name, cuisine, rating
         FROM restaurants
         WHERE latitude BETWEEN 40.7 AND 40.8
           AND longitude BETWEEN -74.0 AND -73.9
           AND rating >= 4.0
         ORDER BY rating DESC
         LIMIT 10"
    ).await?;

    df.show().await?;

    Ok(())
}
```

## Next Steps

- See [`csv-development.md`](csv-development.md) for implementation details
- Read the [DataFusion SQL Reference](https://datafusion.apache.org/user-guide/sql/index.html)
- Explore [GeoArrow Specification](https://geoarrow.org/)
