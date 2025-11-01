# Implementing Geospatial File Format Integrations

## Introduction

This guide provides comprehensive instructions for integrating custom geospatial file formats into DataFusion, enabling efficient querying and processing of geospatial data using SQL and DataFrame APIs.

### Version Compatibility

This guide is based on the following versions:

| Component | Version | Notes |
|-----------|---------|-------|
| **DataFusion** | 50.1.0 | Core query engine |
| **Arrow** | 56.0 | Columnar memory format |
| **geoarrow-rs** | 0.6.1 | GeoArrow implementation (`geoarrow-schema`, `geoarrow-array`, `geoarrow-cast`) |
| **object_store** | 0.12 | Storage abstraction layer |

### Format-Specific Documentation

For detailed usage and implementation guides for supported formats:

- **CSV with WKT Geometries**: See [`crates/formats/datafusion-csv/docs/csv-user-guide.md`](../crates/formats/datafusion-csv/docs/csv-user-guide.md) and [`crates/formats/datafusion-csv/docs/csv-development.md`](../crates/formats/datafusion-csv/docs/csv-development.md)
  - Reference Implementation: [`crates/formats/datafusion-csv`](../crates/formats/datafusion-csv/)
- **GeoJSON**: See [`crates/formats/datafusion-geojson/docs/geojson-user-guide.md`](../crates/formats/datafusion-geojson/docs/geojson-user-guide.md) and [`crates/formats/datafusion-geojson/docs/geojson-development.md`](../crates/formats/datafusion-geojson/docs/geojson-development.md)
  - Reference Implementation: [`crates/formats/datafusion-geojson`](../crates/formats/datafusion-geojson/)

These guides provide concrete examples, API references, and best practices for working with these formats in production. The reference implementations demonstrate all concepts covered in this guide.

### What You'll Learn

- How to implement DataFusion's core traits (`FileFormat`, `FileSource`, `FileOpener`)
- How to convert your format's geometry data into GeoArrow-compatible Arrow arrays
- How to leverage projection and predicate pushdown for optimal query performance
- How to create a user-friendly API for your format integration

### Prerequisites

- Familiarity with Rust and async programming
- Basic understanding of Apache Arrow and columnar data processing
- Knowledge of your geospatial file format's structure
- Understanding of the DataFusion query execution model

### Key Technologies

- **DataFusion**: A fast, extensible query engine built on Apache Arrow
- **GeoArrow**: A specification for encoding geospatial data in Apache Arrow columnar format
- **object_store**: Rust crate providing unified I/O abstraction for local and cloud storage
- **geoarrow-rs**: Rust implementation of GeoArrow with array builders and utilities

### Essential Resources

- **[GeoArrow Rust Documentation](https://geoarrow.org/geoarrow-rs/rust/)**: **This is your primary reference** for working with geospatial arrays in Rust. It provides comprehensive API documentation for:
  - Array builders (`PointBuilder`, `LineStringBuilder`, `PolygonBuilder`, etc.)
  - Geometry array types and their memory layouts
  - Coordinate reference system (CRS) handling
  - Conversion utilities and interoperability with other geo libraries
  - Zero-copy operations and performance optimization techniques

  **You will reference this documentation frequently throughout your implementation.**

### Understanding the geoarrow-rs Crate Ecosystem

The geoarrow-rs project is organized into several focused crates. Understanding which ones you need is critical for efficient development:

#### Core Crates (Essential for Format Integration)

1. **`geoarrow-schema`** - [📚 docs](https://docs.rs/geoarrow-schema/)
   - **Purpose**: Defines geometry data types and GeoArrow metadata specifications
   - **When to use**:
     - Declaring geometry field types in your Arrow schema
     - Working with `GeometryDataType` enum (Point, LineString, Polygon, etc.)
     - Attaching CRS metadata to geometry columns
   - **Key types**: `GeometryDataType`, `Metadata`, field extension metadata
   - **You need this for**: Schema inference and geometry type declarations

2. **`geoarrow-array`** - [📚 docs](https://docs.rs/geoarrow-array/)
   - **Purpose**: Provides array implementations and builders for all GeoArrow geometry types
   - **When to use**:
     - Building geometry arrays from parsed features
     - Converting your format's geometries into Arrow arrays
     - Working with typed geometry arrays
   - **Key types**:
     - Builders: `PointBuilder`, `LineStringBuilder`, `PolygonBuilder`, `MultiPointBuilder`, `MultiLineStringBuilder`, `MultiPolygonBuilder`, `MixedGeometryBuilder`
     - Arrays: `PointArray`, `LineStringArray`, `PolygonArray`, etc.
     - Traits: `GeometryArrayBuilder`, `GeometryArray`
   - **You need this for**: The core of your geometry conversion logic

3. **`geoarrow-cast`** - [📚 docs](https://docs.rs/geoarrow-cast/)
   - **Purpose**: Conversion functions between different GeoArrow geometry types
   - **When to use**:
     - Converting between geometry types (e.g., Polygon to MultiPolygon)
     - Normalizing heterogeneous geometry collections
     - Type coercion during query processing
   - **Key functions**: `cast()`, type conversion utilities
   - **You need this for**: Handling geometry type conversions and schema compatibility

4. **`geoarrow`** (Amalgam Crate) - [📚 docs](https://docs.rs/geoarrow/)
   - **Purpose**: Convenience crate that re-exports items from `geoarrow-array`, `geoarrow-cast`, and `geoarrow-schema`
   - **When to use**: If you want a single dependency instead of adding three separate crates
   - **Trade-off**: Simpler dependency management but larger compile-time footprint
   - **Recommendation**: Use individual crates for more granular control and faster builds

#### Reference Implementation Crates

5. **`geoparquet`** - [📚 docs](https://docs.rs/geoparquet/)
   - **Purpose**: Complete GeoParquet format implementation
   - **When to use**: Study this as a reference implementation for your own format
   - **Value**: Shows best practices for:
     - Metadata handling
     - Predicate pushdown with spatial indexes
     - Efficient batching and streaming
     - Integration with DataFusion
   - **You need this for**: Learning by example from a production-quality implementation

6. **`geoarrow-flatgeobuf`** - [📚 docs](https://docs.rs/geoarrow-flatgeobuf/)
   - **Purpose**: FlatGeobuf format reading/writing
   - **When to use**: Another reference implementation showing format integration patterns
   - **Value**: Demonstrates spatial index usage and streaming reads

#### Typical Dependency Configuration

For most format integrations, you'll need:

```toml
[dependencies]
# Option 1: Individual crates (recommended for production)
geoarrow-schema = "0.6.1"
geoarrow-array = "0.6.1"
geoarrow-cast = "0.6.1"  # Optional, for type conversions

# Option 2: Amalgam crate (simpler for prototyping)
# geoarrow = "0.6.1"

# Study these as references
geoparquet = "0.6.1"  # Optional, for learning best practices
```

**Recommendation**: Start with individual crates (`geoarrow-schema`, `geoarrow-array`) for better compile times and explicit dependencies. Add `geoarrow-cast` only if you need geometry type conversions. Study `geoparquet` source code as a reference for advanced features like spatial indexing.

## 1. Core Concepts and DataFusion Traits

To integrate a new geospatial file format, you'll primarily interact with DataFusion's `datafusion-datasource` crate. The goal is to convert your format's data into a stream of Apache Arrow `RecordBatch`es, ensuring geometry columns adhere to the [GeoArrow](https://geoarrow.org/) memory layout.

### 1.1. The `FileFormat` Trait

This is the central entry point for DataFusion to understand your file format. It defines how to:

*   Identify your file by its extension (`get_ext`).
*   Infer its schema (`infer_schema`).
*   Infer its statistics (`infer_stats`).
*   Create the physical plan for reading it (`create_physical_plan`).
*   Provide a `FileSource` instance (`file_source`).

### 1.2. The `FileSource` Trait

`FileSource` acts as a factory for `FileOpener` instances. It holds configuration common to all files of a given format within a scan (e.g., batch size, metrics).

### 1.3. The `FileOpener` Trait

`FileOpener` is responsible for reading a *single* file. Its `open` method returns a `FileOpenFuture`, which resolves to a stream of `RecordBatch`es. This is where the actual parsing of your file format and conversion to Arrow happens.

### 1.4. `ExecutionPlan` and `DataSourceExec`

Ultimately, your `FileFormat` contributes to DataFusion's `ExecutionPlan`. Typically, `create_physical_plan` will return a `DataSourceExec`, which wraps your `FileSource` and handles the parallel execution of `FileOpener`s.

### 1.5. Geospatial Bridge: GeoArrow and `geoarrow-rs`

All geometry data read from your format *must* be converted into GeoArrow-compatible Arrow arrays. The `geoarrow-rs` crate provides the necessary array builders and types to achieve this, often enabling **zero-copy** operations where data can be directly mapped into Arrow arrays without expensive memory duplication.

#### Why GeoArrow Matters

GeoArrow provides a standardized, interoperable way to represent geospatial data in Apache Arrow's columnar memory format. This enables:
- **Interoperability**: Your format automatically works with any tool in the GeoArrow ecosystem
- **Performance**: Columnar layout enables vectorized operations and efficient CPU cache usage
- **Zero-copy**: Geometry data can be shared across language boundaries without serialization
- **Integration**: Seamless integration with DataFusion's query engine and optimization passes

#### Using the geoarrow-rs Documentation

The **[GeoArrow Rust Documentation](https://geoarrow.org/geoarrow-rs/rust/)** is essential for understanding:

1. **Which builder to use for your geometry type**:
   - `PointBuilder` for `POINT` geometries
   - `LineStringBuilder` for `LINESTRING` geometries
   - `PolygonBuilder` for `POLYGON` geometries
   - `MultiPointBuilder`, `MultiLineStringBuilder`, `MultiPolygonBuilder` for multi-geometries
   - `MixedGeometryBuilder` for heterogeneous geometry collections

2. **How to construct geometry arrays efficiently**:
   ```rust
   use geoarrow_array::array::PointBuilder;

   // See the docs for builder methods and capacity planning
   let mut builder = PointBuilder::with_capacity(1024);

   // Consult docs for the correct method signature
   builder.push_point(Some(&point));  // For Option<Point>
   // or
   builder.push_xy(x, y);  // For raw coordinates

   let point_array = builder.finish();
   ```

3. **Memory layout and data types**:
   - Understanding the Arrow data type returned by `.finish()`
   - How coordinates are stored (interleaved vs. separated)
   - Handling null/missing geometries

4. **Coordinate Reference Systems (CRS)**:
   - How to attach CRS metadata to geometry arrays
   - Converting between different CRS representations

5. **Performance optimization**:
   - Pre-allocating builder capacity
   - Batch operations
   - Zero-copy construction from existing buffers

**Always refer to the [geoarrow-rs API docs](https://geoarrow.org/geoarrow-rs/rust/) when working with geometry builders and arrays.** The documentation includes examples, method signatures, and important notes about memory safety and performance.

### 1.6. I/O Abstraction: `object_store`

All file I/O should be performed using the `object_store` crate. This ensures your format can seamlessly read from local files, S3, Azure Blob Storage, GCS, and other supported object stores.

## 2. Component Interaction Diagram

This diagram illustrates the high-level flow and interaction between the various components when DataFusion processes a query involving a custom geospatial file format.

```mermaid
graph TD
    A[User Application <br> (SessionContext API)] --- B[FileFormat Trait <br> (e.g., SimpleGeoFormat)]
    B -- Creates --> C[FileSource Trait <br> (e.g., SimpleGeoSource)]
    C -- Creates per-file --> D[FileOpener Trait <br> (e.g., SimpleGeoOpener)]
    D -- Reads via --> E[ObjectStore <br> (Local, S3, GCS, HTTP, etc.)]
    subgraph DataFusion Query Planner
        F[FileScanConfig <br> (Projection, Filters)]
    end
    E -- Raw Bytes --> G[Format-Specific Parser <br> (e.g., SimpleGeoReader)]
    G -- Features/Records --> H[GeoArrow Builders <br> (e.g., PointBuilder, StringBuilder)]
    H -- Arrow Arrays --> I[RecordBatch Stream <br> (Output to DataFusion)]
    F --- D
    D --- I
```

### Explanation of Components:

*   **User Application (SessionContext API)**: This is where the user interacts with DataFusion, typically by registering tables and executing SQL queries or DataFrame operations. The `SessionContext` extension trait (e.g., `SessionContextSimpleGeoExt`) simplifies this interaction.
*   **`FileFormat` Trait (e.g., `SimpleGeoFormat`)**: Your implementation of this trait is registered with DataFusion. It's responsible for providing metadata about your file type, such as its schema and statistics. When DataFusion needs to read data, it asks the `FileFormat` to provide a `FileSource`.
*   **`FileSource` Trait (e.g., `SimpleGeoSource`)**: This trait acts as a factory. For each file (or partition of a file) that DataFusion needs to read, the `FileSource` creates a `FileOpener` instance. It also holds common configuration like batch size.
*   **`FileOpener` Trait (e.g., `SimpleGeoOpener`)**: This is where the per-file reading logic resides. For each file, DataFusion calls its `open()` method, which returns a `Future` that yields a stream of `RecordBatch`es.
*   **`FileScanConfig`**: This object is passed down to the `FileOpener`. It contains crucial information from DataFusion's query planner, such as:
    *   **Projection**: Which columns are actually needed by the query.
    *   **Filters**: Predicates that can potentially be pushed down to the file reader.
*   **`ObjectStore`**: This is DataFusion's abstraction for reading data from various storage systems. Your `FileOpener` uses this to fetch raw bytes from the file. This abstraction inherently supports various storage backends including **local disk files**, **cloud object stores (S3, GCS, Azure Blob)**, and **HTTP/HTTPS URLs**, without requiring format-specific implementations for each.
*   **Format-Specific Parser (e.g., `SimpleGeoReader`)**: This is your custom logic (or an external library) that understands the internal structure of your geospatial file format. It takes raw bytes and parses them into meaningful features or records.
*   **GeoArrow Builders (e.g., `PointBuilder`, `StringBuilder`)**: As features are parsed, their attribute data is fed into Arrow array builders (e.g., `StringBuilder` for text, `UInt32Builder` for IDs), and their geometry data is fed into `geoarrow-rs` builders (e.g., `PointBuilder`, `PolygonBuilder`).
*   **`RecordBatch` Stream**: Once a batch of features has been processed, the builders are finalized into Arrow arrays, which are then combined into a `RecordBatch`. The `FileOpener` returns a stream of these `RecordBatch`es to DataFusion for further processing.

## 3. Step-by-Step Implementation with Concrete Examples
Let's walk through implementing a hypothetical `SimpleGeo` format. This format will have a simple header, a few attribute columns (e.g., `name: Utf8`, `id: UInt32`), and a geometry column (e.g., `Point`).

### 3.1. Project Setup (`Cargo.toml`)

Create a new crate (e.g., `datafusion-simplegeo`) and add the necessary dependencies:

```toml
[package]
name = "datafusion-simplegeo"
version = "0.1.0"
edition = "2021"

[dependencies]
# DataFusion core components
datafusion = "50.0"
datafusion-common = "50.0"
datafusion-expr = "50.0"
datafusion-physical-plan = "50.0"
datafusion-datasource = "50.0"

# Apache Arrow for in-memory data
arrow = { version = "56.0", features = ["prettyprint"] }

# ============================================================================
# GeoArrow Crates - See https://geoarrow.org/geoarrow-rs/rust/
# ============================================================================

# geoarrow-schema: REQUIRED
# Provides geometry type definitions (GeometryDataType enum) and metadata
# You'll use this to declare geometry fields in your Arrow schema
geoarrow-schema = "0.6.1"

# geoarrow-array: REQUIRED
# Provides geometry array builders (PointBuilder, LineStringBuilder, etc.)
# This is the core crate for converting your format's geometries to Arrow arrays
geoarrow-array = "0.6.1"

# geoarrow-cast: OPTIONAL
# Only add if you need to convert between geometry types
# Example: Converting heterogeneous geometries to a uniform type
# geoarrow-cast = "0.6.1"

# Alternative: Use the amalgam crate instead (re-exports all three above)
# Trade-off: Simpler but larger compile footprint
# geoarrow = "0.6.1"

# ============================================================================
# Async utilities
# ============================================================================
async-trait = "0.1"
futures = { version = "0.3", default-features = false, features = ["std"] }
futures-util = { version = "0.3" }
tokio = { version = "1.28", features = ["macros", "rt-multi-thread"] }

# ============================================================================
# Object storage abstraction - handles local, S3, GCS, Azure, etc.
# ============================================================================
object_store = "0.12"

# ============================================================================
# Your format-specific parser
# ============================================================================
simplegeo-parser = { path = "./simplegeo-parser" } # Or from crates.io

[dev-dependencies]
# Optional: Study geoparquet as a reference implementation
# Uncomment to explore best practices for spatial indexing and optimization
# geoparquet = "0.6.1"
```

#### Dependency Selection Guide

**Must have**:
- `geoarrow-schema` - For declaring geometry types in your schema
- `geoarrow-array` - For building geometry arrays from your data

**Optional**:
- `geoarrow-cast` - Only if you need type conversions (e.g., Polygon → MultiPolygon)
- `geoarrow` (amalgam) - Alternative to individual crates; use for simpler dependency management

**For reference**:
- `geoparquet` - Study as a production-quality example (add as dev-dependency)

#### How the Crates Work Together

Here's a quick example showing how these crates integrate:

```rust
// From geoarrow-schema: Declare geometry types in your schema
use geoarrow_schema::GeometryDataType;
use arrow::datatypes::Field;

let geometry_field = Field::new(
    "geometry",
    GeometryDataType::Point.to_arrow_type(), // Schema definition
    true, // nullable
);

// From geoarrow-array: Build geometry arrays from your data
use geoarrow_array::array::PointBuilder;

let mut builder = PointBuilder::with_capacity(1024);
for feature in features {
    builder.push_xy(feature.x, feature.y); // Or push_point(point)
}
let point_array = builder.finish(); // Returns Arc<dyn GeometryArray>

// From geoarrow-cast: Convert between geometry types (if needed)
use geoarrow_cast::cast;
let multi_point_array = cast(&point_array, &GeometryDataType::MultiPoint)?;
```

Each crate has a specific role in the pipeline:
1. **geoarrow-schema** → Define what geometry types your schema uses
2. **geoarrow-array** → Build arrays from your parsed geometries
3. **geoarrow-cast** → Convert between types when necessary

### 3.2. Defining the `SimpleGeoFormat` (`FileFormat` Implementation)

This struct will implement the `FileFormat` trait. It's responsible for telling DataFusion about your format.

```rust
// src/file_format.rs

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use datafusion::arrow::datatypes::TimeUnit;
use datafusion::catalog::Session;
use datafusion::common::{DataFusionError, Result, Statistics};
use datafusion::datasource::file_format::FileFormat;
use datafusion::datasource::physical_plan::{FileScanConfig, FileSource};
use datafusion::physical_plan::ExecutionPlan;
use datafusion::datasource::source::DataSourceExec;
use object_store::{ObjectStore, ObjectMeta};
use futures::stream::{self, StreamExt, TryStreamExt};

use geoarrow_schema::GeometryDataType;

use crate::file_source::SimpleGeoSource;

#[derive(Debug, Default, Clone)]
pub struct SimpleGeoFormat;

#[async_trait]
impl FileFormat for SimpleGeoFormat {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_ext(&self) -> String {
        "sgeo".to_string() // Your file extension
    }

    // No compression for this example
    fn compression_type(&self) -> Option<datafusion::datasource::file_format::file_compression_type::FileCompressionType> {
        None
    }

    async fn infer_schema(
        &self,
        state: &dyn Session,
        store: &Arc<dyn ObjectStore>,
        objects: &[ObjectMeta],
    ) -> Result<SchemaRef> {
        if objects.is_empty() {
            return Err(DataFusionError::Plan("No objects to infer schema from".to_string()));
        }

        // For simplicity, we'll infer from the first object.
        // A robust implementation would merge schemas from multiple files.
        let first_object = &objects[0];
        let path = &first_object.location;

        // Read a small header or sample to infer schema
        // In a real scenario, you'd use your format's specific metadata reader
        let reader = store.get(path).await?;

        // Read first 1KB or entire file if smaller
        let header_size = std::cmp::min(1024, first_object.size);
        let bytes = reader.range(0..header_size).await?;

        // Hypothetical: Parse header to get schema
        // This would involve your `simplegeo-parser` crate
        let (attribute_fields, geometry_type) = 
            simplegeo_parser::infer_schema_from_header(&bytes)
                .map_err(|e| DataFusionError::External(Box::new(e)))?;

        let mut fields = attribute_fields;
        // Add the geometry field, ensuring it's GeoArrow compatible
        fields.push(Field::new(
            "geometry",
            geometry_type.to_arrow_type(), // e.g., GeometryDataType::Point.to_arrow_type()
            true, // nullable
        ));

        let schema = Schema::new(fields);
        Ok(Arc::new(schema))
    }

    async fn infer_stats(
        &self,
        _state: &dyn Session,
        _store: &Arc<dyn ObjectStore>,
        table_schema: SchemaRef,
        _object: &ObjectMeta,
    ) -> Result<Statistics> {
        // For this example, we return unknown statistics. 
        // A real implementation would read bounding boxes or other stats from file metadata.
        Ok(Statistics::new_unknown(&table_schema))
    }

    async fn create_physical_plan(
        &self,
        _state: &dyn Session,
        conf: FileScanConfig,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // This is the standard way to create the physical plan for file-based data sources.
        // DataSourceExec will use your FileSource to create FileOpeners.
        Ok(DataSourceExec::from_data_source(conf))
    }

    fn file_source(&self) -> Arc<dyn FileSource> {
        Arc::new(SimpleGeoSource::default())
    }
}
```

### 3.3. Implementing `SimpleGeoSource` (`FileSource` Implementation)

This struct provides configuration and creates `FileOpener` instances.

```rust
// src/file_source.rs

use std::any::Any;
use std::sync::Arc;

use arrow::datatypes::SchemaRef;
use datafusion::common::Statistics;
use datafusion::datasource::physical_plan::{FileOpener, FileScanConfig, FileSource};
use datafusion::physical_plan::metrics::ExecutionPlanMetricsSet;
use object_store::ObjectStore;

use crate::physical_exec::SimpleGeoOpener;

#[derive(Debug, Clone)]
pub struct SimpleGeoSource {
    metrics: ExecutionPlanMetricsSet,
    statistics: Statistics,
    batch_size: usize,
}

impl Default for SimpleGeoSource {
    fn default() -> Self {
        Self {
            metrics: ExecutionPlanMetricsSet::default(),
            statistics: Statistics::default(),
            batch_size: 1024, // Default batch size
        }
    }
}

impl FileSource for SimpleGeoSource {
    fn create_file_opener(
        &self,
        object_store: Arc<dyn ObjectStore>,
        config: &FileScanConfig,
        _partition: usize,
    ) -> Arc<dyn FileOpener> {
        // Create your specific FileOpener here
        Arc::new(SimpleGeoOpener::new(object_store, config, self.batch_size))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    // Implement other methods to pass configuration like batch_size, schema, projection, stats
    fn with_batch_size(&self, batch_size: usize) -> Arc<dyn FileSource> {
        Arc::new(Self { batch_size, ..self.clone() })
    }

    fn with_schema(&self, _schema: SchemaRef) -> Arc<dyn FileSource> {
        Arc::new(self.clone())
    }

    fn with_projection(&self, _config: &FileScanConfig) -> Arc<dyn FileSource> {
        Arc::new(self.clone())
    }

    fn with_statistics(&self, statistics: Statistics) -> Arc<dyn FileSource> {
        Arc::new(Self { statistics, ..self.clone() })
    }

    fn metrics(&self) -> &ExecutionPlanMetricsSet {
        &self.metrics
    }

    fn statistics(&self) -> datafusion::common::Result<Statistics> {
        Ok(self.statistics.clone())
    }

    fn file_type(&self) -> &str {
        "sgeo"
    }
}
```

### 3.4. Implementing `SimpleGeoOpener` (`FileOpener` Implementation)

This is where the core logic for reading a single file and converting it to `RecordBatch`es resides. This will involve your `simplegeo-parser` crate and `geoarrow-rs` builders.

```rust
// src/physical_exec.rs

use std::sync::Arc;

use arrow::array::{ArrayRef, UInt32Builder, StringBuilder};
use arrow::datatypes::SchemaRef;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::physical_plan::{FileMeta, FileOpenFuture, FileOpener, FileScanConfig};
use datafusion::error::{DataFusionError, Result};
use datafusion_datasource::PartitionedFile;
use object_store::ObjectStore;
use futures::StreamExt;
use futures::TryStreamExt;

// GeoArrow builders for geometry types
// See https://geoarrow.org/geoarrow-rs/rust/ for all available geometry builders:
// PointBuilder, LineStringBuilder, PolygonBuilder, MultiPointBuilder, etc.
use geoarrow_array::array::PointBuilder;
use geoarrow_array::GeometryArray;

// Hypothetical parser for SimpleGeo format
use simplegeo_parser::{SimpleGeoFeature, SimpleGeoReader};

// Enum to identify which field we're working with
#[derive(Debug, Clone)]
enum FieldMapping {
    Id,
    Name,
    Geometry,
}

pub(crate) struct SimpleGeoOpener {
    object_store: Arc<dyn ObjectStore>,
    projected_schema: SchemaRef,
    field_mappings: Vec<FieldMapping>,
    batch_size: usize,
}

impl SimpleGeoOpener {
    pub(crate) fn new(
        object_store: Arc<dyn ObjectStore>,
        config: &FileScanConfig,
        batch_size: usize,
    ) -> Self {
        let projection_indices = config
            .file_column_projection_indices()
            .unwrap_or_else(|| (0..config.file_schema.fields().len()).collect());

        let projected_schema = Arc::new(
            config.file_schema.project(&projection_indices).expect("schema projection failed")
        );

        // Build field mappings once during initialization
        let field_mappings: Vec<FieldMapping> = projected_schema
            .fields()
            .iter()
            .map(|field| match field.name().as_str() {
                "id" => FieldMapping::Id,
                "name" => FieldMapping::Name,
                "geometry" => FieldMapping::Geometry,
                _ => panic!("Unknown field: {}", field.name()),
            })
            .collect();

        Self {
            object_store,
            projected_schema,
            field_mappings,
            batch_size: config.batch_size.unwrap_or(batch_size),
        }
    }
}

impl FileOpener for SimpleGeoOpener {
    fn open(&self, file_meta: FileMeta, _partition: PartitionedFile) -> Result<FileOpenFuture> {
        let object_store = Arc::clone(&self.object_store);
        let path = file_meta.object_meta.location.clone();
        let projected_schema = Arc::clone(&self.projected_schema);
        let batch_size = self.batch_size;
        let field_mappings = self.field_mappings.clone();

        Ok(Box::pin(async move {
            // 1. Get a reader for the file from the object store
            let reader = object_store.get(&path).await?;
            let bytes = reader.bytes().await?; // Read all bytes for simplicity. For large files, use range reads.

            // 2. Use your format-specific parser to create an iterator over features
            let mut simple_geo_reader = SimpleGeoReader::new(bytes.as_ref())
                .map_err(|e| DataFusionError::External(Box::new(e)))?;

            // 3. Create a stream of RecordBatches
            let stream = futures::stream::try_unfold(
                simple_geo_reader,
                move |mut reader| async move {
                    // Create builders only for projected fields
                    // Using Option to conditionally create builders
                    let needs_id = field_mappings.iter().any(|m| matches!(m, FieldMapping::Id));
                    let needs_name = field_mappings.iter().any(|m| matches!(m, FieldMapping::Name));
                    let needs_geometry = field_mappings.iter().any(|m| matches!(m, FieldMapping::Geometry));

                    let mut id_builder = needs_id.then(|| UInt32Builder::with_capacity(batch_size));
                    let mut name_builder = needs_name.then(|| StringBuilder::with_capacity(batch_size));
                    let mut geometry_builder = needs_geometry.then(|| PointBuilder::with_capacity(batch_size));

                    let mut features_read = 0;
                    while features_read < batch_size {
                        match reader.next_feature() {
                            Ok(Some(feature)) => {
                                // Append to builders based on field mappings
                                if let Some(builder) = &mut id_builder {
                                    builder.append_value(feature.id);
                                }
                                if let Some(builder) = &mut name_builder {
                                    builder.append_value(&feature.name);
                                }
                                if let Some(builder) = &mut geometry_builder {
                                    builder.push_point(feature.geometry.as_point());
                                }
                                features_read += 1;
                            },
                            Ok(None) => break, // End of file
                            Err(e) => return Err(DataFusionError::External(Box::new(e))), // Parser error
                        }
                    }

                    if features_read == 0 {
                        Ok(None) // No more features
                    } else {
                        // Build columns in the order specified by field_mappings
                        let columns: Vec<ArrayRef> = field_mappings
                            .iter()
                            .map(|mapping| match mapping {
                                FieldMapping::Id => {
                                    Arc::new(id_builder.take().unwrap().finish()) as ArrayRef
                                },
                                FieldMapping::Name => {
                                    Arc::new(name_builder.take().unwrap().finish()) as ArrayRef
                                },
                                FieldMapping::Geometry => {
                                    Arc::new(geometry_builder.take().unwrap().finish()) as ArrayRef
                                },
                            })
                            .collect();

                        let record_batch = RecordBatch::try_new(projected_schema.clone(), columns)?;
                        Ok(Some((record_batch, reader)))
                    }
                },
            );
            Ok(stream.boxed())
        }))
    }
}
```

### 3.5. Projection Pushdown

Projection pushdown is handled by the `FileOpener`. The `FileScanConfig` passed to `SimpleGeoOpener::new` contains `file_column_projection_indices()`, which tells you exactly which columns (by their original schema index) are required by the query. You should use this to avoid reading and decoding unnecessary data from your `SimpleGeoReader`.

In the `SimpleGeoOpener::new` method:

```rust
// ... inside SimpleGeoOpener::new
        let projection_indices = config
            .file_column_projection_indices()
            .unwrap_or_else(|| (0..config.file_schema.fields().len()).collect());

        let projected_schema = Arc::new(
            config.file_schema.project(&projection_indices).expect("schema projection failed")
        );
// ...
```

And then in the `open` method, when building `RecordBatch`es, only create builders and append data for the columns present in `projected_schema` (which is derived from `projection_indices`).

### 3.6. Predicate Pushdown (Attribute and Spatial)

Predicate pushdown allows filtering data at the source, reducing the amount of data read and processed. The `FileScanConfig` contains `filters` that DataFusion's query planner believes can be pushed down to your format reader.

#### Attribute Predicate Pushdown

```rust
use datafusion::logical_expr::{Expr, BinaryExpr, Operator};
use datafusion::scalar::ScalarValue;

// Helper function to extract attribute filters
fn extract_attribute_filters(filters: &[Expr]) -> Vec<AttributeFilter> {
    let mut extracted_filters = Vec::new();

    for filter_expr in filters {
        if let Expr::BinaryExpr(BinaryExpr { left, op, right }) = filter_expr {
            // Handle simple equality: column = value
            if let (Expr::Column(col), Operator::Eq, Expr::Literal(val)) =
                (left.as_ref(), op, right.as_ref()) {
                match val {
                    ScalarValue::Utf8(Some(s)) => {
                        extracted_filters.push(AttributeFilter::StringEq {
                            column: col.name.clone(),
                            value: s.clone(),
                        });
                    }
                    ScalarValue::UInt32(Some(n)) => {
                        extracted_filters.push(AttributeFilter::U32Eq {
                            column: col.name.clone(),
                            value: *n,
                        });
                    }
                    _ => {}
                }
            }
            // Handle range predicates: column > value, column < value
            else if let (Expr::Column(col), op @ (Operator::Gt | Operator::Lt | Operator::GtEq | Operator::LtEq), Expr::Literal(val)) =
                (left.as_ref(), op, right.as_ref()) {
                if let ScalarValue::UInt32(Some(n)) = val {
                    extracted_filters.push(AttributeFilter::U32Range {
                        column: col.name.clone(),
                        operator: *op,
                        value: *n,
                    });
                }
            }
        }
    }

    extracted_filters
}

#[derive(Debug)]
enum AttributeFilter {
    StringEq { column: String, value: String },
    U32Eq { column: String, value: u32 },
    U32Range { column: String, operator: Operator, value: u32 },
}

// In your FileOpener::open method or SimpleGeoOpener::new:
let attribute_filters = extract_attribute_filters(&config.filters);

// Pass these filters to your reader
// simple_geo_reader.set_filters(attribute_filters);
```

#### Spatial Predicate Pushdown

Spatial predicate pushdown is more complex but can dramatically improve performance by using spatial indexes. Here's a concrete example of extracting bounding box filters:

```rust
use datafusion::logical_expr::{Expr, ScalarFunction};
use geo::Rect;

// Helper to extract bounding box from spatial predicates
fn extract_bbox_filter(filters: &[Expr]) -> Option<Rect<f64>> {
    for filter_expr in filters {
        // Look for function calls like ST_Intersects, ST_Within, etc.
        if let Expr::ScalarFunction(ScalarFunction { func, args }) = filter_expr {
            let func_name = func.name();

            // Check for bounding box optimizable functions
            if func_name == "st_intersects" || func_name == "st_within" {
                // args[0] is typically the geometry column
                // args[1] is typically the filter geometry
                if args.len() == 2 {
                    if let Expr::Literal(ScalarValue::Binary(Some(wkb_bytes))) = &args[1] {
                        // Parse WKB to extract bounding box
                        if let Ok(bbox) = parse_wkb_to_bbox(wkb_bytes) {
                            return Some(bbox);
                        }
                    }
                }
            }
        }
    }
    None
}

fn parse_wkb_to_bbox(wkb: &[u8]) -> Result<Rect<f64>, DataFusionError> {
    // Use a geometry library like geo or wkt to parse WKB
    // and extract the bounding box
    use wkb::wkb_to_geom;
    use geo::BoundingRect;

    let geom = wkb_to_geom(wkb)
        .map_err(|e| DataFusionError::External(Box::new(e)))?;

    geom.bounding_rect()
        .ok_or_else(|| DataFusionError::Plan("Failed to compute bounding box".to_string()))
}

// In your FileOpener implementation:
impl SimpleGeoOpener {
    pub(crate) fn new(
        object_store: Arc<dyn ObjectStore>,
        config: &FileScanConfig,
        batch_size: usize,
    ) -> Self {
        // ... existing code ...

        // Extract spatial filters
        let bbox_filter = extract_bbox_filter(&config.filters);

        Self {
            object_store,
            projected_schema,
            field_mappings,
            batch_size: config.batch_size.unwrap_or(batch_size),
            bbox_filter, // Store for use in open()
        }
    }
}

// When reading features, skip those outside the bounding box:
while features_read < batch_size {
    match reader.next_feature() {
        Ok(Some(feature)) => {
            // Apply spatial filter early
            if let Some(bbox) = &bbox_filter {
                if !feature.geometry.intersects_bbox(bbox) {
                    continue; // Skip this feature
                }
            }

            // Apply attribute filters
            if !passes_attribute_filters(&feature, &attribute_filters) {
                continue; // Skip this feature
            }

            // Append to builders
            // ... builder code ...
            features_read += 1;
        },
        Ok(None) => break,
        Err(e) => return Err(DataFusionError::External(Box::new(e))),
    }
}
```

#### Best Practices for Predicate Pushdown

1. **Extract filters during initialization**: Parse filter expressions once in `SimpleGeoOpener::new` rather than for every batch
2. **Use format-specific indexes**: If your format has a spatial index (like GeoParquet's R-tree), use the bounding box to query the index before reading features
3. **Apply filters early**: Check filters as soon as possible to avoid unnecessary deserialization
4. **Return accurate statistics**: Implement `infer_stats` with row counts and bounding boxes to help DataFusion's planner make better decisions
5. **Handle complex expressions**: Use DataFusion's `PhysicalExpr` for evaluating complex predicates that can't be pushed down natively

#### Example: Using Spatial Index

If your format has an embedded spatial index:

```rust
// In FileOpener::open, after opening the reader:
if let Some(bbox) = &bbox_filter {
    // Use the format's spatial index to get candidate feature IDs
    let candidate_ids = simple_geo_reader
        .query_spatial_index(bbox)
        .map_err(|e| DataFusionError::External(Box::new(e)))?;

    // Only read features that passed the spatial index check
    simple_geo_reader.set_feature_id_filter(candidate_ids);
}
```

This approach can reduce I/O dramatically when querying large files with good spatial locality.

### 3.7. `SessionContext` Extension Trait

This trait provides a convenient way for users to register your format with DataFusion.

#### Simplified Extension Trait Pattern (Recommended)

Based on the reference implementations ([`datafusion-geojson`](../crates/formats/datafusion-geojson/src/lib.rs) and [`datafusion-csv`](../crates/formats/datafusion-csv/src/lib.rs)), here's the recommended simplified pattern:

```rust
// src/lib.rs

use datafusion::prelude::*;
use datafusion_common::Result;

/// Extension trait for [`SessionContext`] that offers convenience helpers to
/// register or read `SimpleGeo` sources.
#[allow(async_fn_in_trait)]
pub trait SessionContextSimpleGeoExt {
    /// Register a `SimpleGeo` dataset as a table with default options.
    async fn register_simple_geo_file(&self, name: &str, path: &str) -> Result<()>;

    /// Register a `SimpleGeo` dataset with custom format options.
    async fn register_simple_geo_with_options(
        &self,
        name: &str,
        path: &str,
        options: SimpleGeoFormatOptions,
    ) -> Result<()>;

    /// Read a `SimpleGeo` dataset into a [`DataFrame`] with default options.
    async fn read_simple_geo_file(&self, path: &str) -> Result<DataFrame>;

    /// Read a `SimpleGeo` dataset into a [`DataFrame`] with custom format options.
    async fn read_simple_geo_with_options(
        &self,
        path: &str,
        options: SimpleGeoFormatOptions,
    ) -> Result<DataFrame>;
}

impl SessionContextSimpleGeoExt for SessionContext {
    async fn register_simple_geo_file(&self, name: &str, path: &str) -> Result<()> {
        let options = SimpleGeoFormatOptions::default();
        self.register_simple_geo_with_options(name, path, options).await
    }

    async fn register_simple_geo_with_options(
        &self,
        name: &str,
        path: &str,
        options: SimpleGeoFormatOptions,
    ) -> Result<()> {
        // Use your helper function to create a table provider
        let table = create_simple_geo_table_provider(&self.state(), path, options).await?;
        self.register_table(name, table)?;
        Ok(())
    }

    async fn read_simple_geo_file(&self, path: &str) -> Result<DataFrame> {
        let options = SimpleGeoFormatOptions::default();
        self.read_simple_geo_with_options(path, options).await
    }

    async fn read_simple_geo_with_options(
        &self,
        path: &str,
        options: SimpleGeoFormatOptions,
    ) -> Result<DataFrame> {
        let table = create_simple_geo_table_provider(&self.state(), path, options).await?;
        self.read_table(table)
    }
}

/// Helper function to create a table provider for SimpleGeo files.
///
/// This function handles:
/// - URL parsing and object store configuration
/// - ListingTable setup with your file format
/// - Schema inference
pub async fn create_simple_geo_table_provider(
    state: &SessionState,
    path: &str,
    options: SimpleGeoFormatOptions,
) -> Result<Arc<dyn TableProvider>> {
    use datafusion::datasource::listing::{ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl};

    let table_path = ListingTableUrl::parse(path)?;
    let file_format = Arc::new(SimpleGeoFormat::new(options));

    let listing_options = ListingOptions::new(file_format)
        .with_file_extension("sgeo");

    let config = ListingTableConfig::new(table_path)
        .with_listing_options(listing_options);

    let table = ListingTable::try_new(config)?;
    Ok(Arc::new(table))
}
```

**Key Advantages of This Pattern:**

1. **Simpler API**: Users don't need to work with `ReadOptions` or understand internal configuration
2. **Clear Intent**: Method names clearly indicate default vs. custom options
3. **Easier Testing**: Direct integration with `SessionContext` makes tests straightforward
4. **Less Boilerplate**: No need to implement `ReadOptions` trait
5. **Better Discoverability**: IDE autocomplete shows all available methods

**Note on `#[allow(async_fn_in_trait)]`**: This suppresses warnings about using async functions in traits without the `async_trait` macro. For DataFusion 50+, this is the preferred pattern since Rust now has native async trait support.

### 3.8. Usage Examples

Now that the format is implemented, here's how users can work with SimpleGeo files:

#### Basic Usage: Registering and Querying a Table

```rust
use datafusion::prelude::*;
use datafusion_simplegeo::SessionContextSimpleGeoExt;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    // Register a SimpleGeo file as a table with default options
    ctx.register_simple_geo_file("my_locations", "data/locations.sgeo").await?;

    // Query using SQL
    let df = ctx.sql("SELECT name, id FROM my_locations WHERE name = 'Central Park'").await?;
    df.show().await?;

    Ok(())
}
```

#### Reading Multiple Files

```rust
use datafusion::prelude::*;
use datafusion_simplegeo::SessionContextSimpleGeoExt;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    // Register a directory of SimpleGeo files
    ctx.register_simple_geo_file(
        "all_locations",
        "data/locations/"  // Directory containing multiple .sgeo files
    ).await?;

    // Query across all files
    let df = ctx.sql("SELECT COUNT(*) as total FROM all_locations").await?;
    df.show().await?;

    Ok(())
}
```

#### DataFrame API with Projection

```rust
use datafusion::prelude::*;
use datafusion_simplegeo::SessionContextSimpleGeoExt;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    // Read directly into a DataFrame (without registering)
    let df = ctx.read_simple_geo_file(
        "s3://my-bucket/data.sgeo"  // Works with S3, GCS, Azure, etc.
    ).await?;

    // Use DataFrame API to select specific columns (projection pushdown!)
    let result = df
        .select_columns(&["name"])?
        .filter(col("id").gt(lit(1000)))?
        .show()
        .await?;

    Ok(())
}
```

#### Working with Cloud Storage

```rust
use datafusion::prelude::*;
use datafusion_simplegeo::SessionContextSimpleGeoExt;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    // The object_store integration means cloud storage works automatically
    ctx.register_simple_geo_file(
        "s3_data",
        "s3://my-bucket/geospatial/cities.sgeo"
    ).await?;

    ctx.register_simple_geo_file(
        "gcs_data",
        "gs://my-bucket/geospatial/roads.sgeo"
    ).await?;

    // Join data from different cloud sources
    let df = ctx.sql(
        "SELECT s.name, g.id
         FROM s3_data s
         JOIN gcs_data g ON s.id = g.city_id"
    ).await?;

    df.show().await?;

    Ok(())
}
```

#### Custom Format Options

```rust
use datafusion::prelude::*;
use datafusion_simplegeo::{SessionContextSimpleGeoExt, SimpleGeoFormatOptions};

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = SessionContext::new();

    // Configure custom options
    let options = SimpleGeoFormatOptions::default()
        .with_batch_size(8192)
        .with_geometry_column_name("location");

    ctx.register_simple_geo_with_options(
        "custom_format",
        "data/file.sgeo",
        options
    ).await?;

    Ok(())
}
```

### 3.9. Testing

Thorough testing is crucial. You should write unit tests for your parser and integration tests that use DataFusion's `SessionContext` to read from your format and assert the correctness of the resulting `RecordBatch`es.

#### Example Integration Test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::prelude::*;
    use datafusion::assert_batches_eq;
    use datafusion_simplegeo::SessionContextSimpleGeoExt;

    #[tokio::test]
    async fn test_read_simple_geo() -> Result<()> {
        let ctx = SessionContext::new();

        ctx.register_simple_geo_file("test_data", "tests/data/sample.sgeo").await?;

        let df = ctx.sql("SELECT * FROM test_data ORDER BY id").await?;
        let batches = df.collect().await?;

        let expected = vec![
            "+----+---------------+",
            "| id | name          |",
            "+----+---------------+",
            "| 1  | Location A    |",
            "| 2  | Location B    |",
            "+----+---------------+",
        ];

        assert_batches_eq!(expected, &batches);
        Ok(())
    }

    #[tokio::test]
    async fn test_projection_pushdown() -> Result<()> {
        let ctx = SessionContext::new();

        let df = ctx.read_simple_geo_file("tests/data/sample.sgeo").await?;

        // Only select 'name' column - should only decode that column
        let result = df.select_columns(&["name"])?.collect().await?;

        assert_eq!(result[0].num_columns(), 1);
        assert_eq!(result[0].column(0).len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_custom_options() -> Result<()> {
        let ctx = SessionContext::new();

        let options = SimpleGeoFormatOptions::default()
            .with_batch_size(4096)
            .with_geometry_column_name("geom");

        ctx.register_simple_geo_with_options(
            "custom_test",
            "tests/data/sample.sgeo",
            options
        ).await?;

        let df = ctx.sql("SELECT geom FROM custom_test").await?;
        let batches = df.collect().await?;

        assert!(!batches.is_empty());
        Ok(())
    }
}
```

#### End-to-End Testing with Real-World Data

For comprehensive testing of your format integration, use real-world datasets that cover edge cases and multiple geometry types. The [Natural Earth datasets provided by GeoArrow](https://geoarrow.org/data.html) are ideal for this purpose.

##### Why Natural Earth for Testing?

Natural Earth datasets are **public domain**, small-sized, and contain real-world edge cases that help validate robust implementations:

1. **Multiple Geometry Types**: Points (cities), Polygons (countries), complex boundaries
2. **Edge Cases**:
   - **Antimeridian handling**: Countries like Fiji and Russia that cross the international date line
   - **Polar geometries**: Antarctica coverage near the South Pole
   - **Special coordinates**: Bounding boxes with `xmax < xmin` for antimeridian cases
3. **Format Availability**: Available in FlatGeobuf, GeoParquet, Arrow IPC, and standard Parquet
4. **Global Coverage**: Comprehensive geographic distribution for realistic testing

##### Available Natural Earth Datasets

| Dataset | Geometry Type | Description | Test Use Case |
|---------|--------------|-------------|---------------|
| `ne_10m_admin_0_countries` | Polygon | Country boundaries | Complex polygons, antimeridian handling |
| `ne_10m_populated_places` | Point | Global cities | Simple point geometries, attribute variety |
| `ne_10m_geographic_lines` | LineString | Geographic features | Line geometry handling |
| `ne_10m_geography_regions_polys` | Polygon | Geographic regions | Special cases (poles, antimeridian) |

##### Setting Up Test Data

**Download Natural Earth data** (choose format matching your implementation):

```bash
# Create test data directory
mkdir -p tests/data/naturalearth

# Download countries (Polygon geometries with edge cases)
curl -L "https://github.com/geoarrow/geoarrow-data/releases/download/latest/ne_10m_admin_0_countries.parquet" \
  -o tests/data/naturalearth/countries.parquet

# Download cities (Point geometries)
curl -L "https://github.com/geoarrow/geoarrow-data/releases/download/latest/ne_10m_populated_places.parquet" \
  -o tests/data/naturalearth/cities.parquet

# Or use FlatGeobuf format
curl -L "https://github.com/geoarrow/geoarrow-data/releases/download/latest/ne_10m_admin_0_countries.fgb" \
  -o tests/data/naturalearth/countries.fgb
```

You can also download these directly from:
- **GitHub**: https://github.com/geoarrow/geoarrow-data/releases/latest
- **GeoArrow Data Page**: https://geoarrow.org/data.html

##### Example: End-to-End Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use datafusion::prelude::*;
    use datafusion_simplegeo::SessionContextSimpleGeoExt;

    /// Test reading real-world Natural Earth countries data
    #[tokio::test]
    async fn test_naturalearth_countries() -> Result<()> {
        let ctx = SessionContext::new();

        // Convert Natural Earth Parquet to your format first (one-time setup)
        // Then register your format version
        ctx.register_simple_geo_file("countries", "tests/data/naturalearth/countries.sgeo").await?;

        // Query countries
        let df = ctx.sql(
            "SELECT name, iso_a3, pop_est
             FROM countries
             WHERE pop_est > 100000000
             ORDER BY pop_est DESC"
        ).await?;

        let batches = df.collect().await?;

        // Verify we got populous countries
        assert!(!batches.is_empty());
        assert!(batches[0].num_rows() > 5); // Should have China, India, USA, etc.

        Ok(())
    }

    /// Test antimeridian edge case with Fiji
    #[tokio::test]
    async fn test_antimeridian_handling() -> Result<()> {
        let ctx = SessionContext::new();

        ctx.register_simple_geo_file("countries", "tests/data/naturalearth/countries.sgeo").await?;

        // Query Fiji, which crosses the antimeridian (180° longitude)
        let df = ctx.sql(
            "SELECT name, geometry
             FROM countries
             WHERE iso_a3 = 'FJI'"
        ).await?;

        let batches = df.collect().await?;

        // Verify Fiji geometry was read correctly
        assert_eq!(batches[0].num_rows(), 1);

        // Validate geometry column exists and has data
        let geometry_col = batches[0].column_by_name("geometry").unwrap();
        assert_eq!(geometry_col.len(), 1);
        assert!(geometry_col.is_valid(0)); // Non-null geometry

        Ok(())
    }

    /// Test polar geometry with Antarctica
    #[tokio::test]
    async fn test_polar_geometry() -> Result<()> {
        let ctx = SessionContext::new();

        ctx.register_simple_geo_file("countries", "tests/data/naturalearth/countries.sgeo").await?;

        // Antarctica has polygon covering the South Pole
        let df = ctx.sql(
            "SELECT name, geometry
             FROM countries
             WHERE name = 'Antarctica'"
        ).await?;

        let batches = df.collect().await?;

        assert_eq!(batches[0].num_rows(), 1);

        // Validate polar geometry was handled correctly
        let geometry_col = batches[0].column_by_name("geometry").unwrap();
        assert!(geometry_col.is_valid(0));

        Ok(())
    }

    /// Test projection pushdown with Natural Earth data
    #[tokio::test]
    async fn test_projection_with_naturalearth() -> Result<()> {
        let ctx = SessionContext::new();

        let df = ctx.read_simple_geo_file("tests/data/naturalearth/countries.sgeo").await?;

        // Select only 2 columns from the many available
        let result = df
            .select_columns(&["name", "iso_a3"])?
            .collect()
            .await?;

        // Verify only requested columns are present
        assert_eq!(result[0].num_columns(), 2);
        assert!(result[0].schema().column_with_name("name").is_some());
        assert!(result[0].schema().column_with_name("iso_a3").is_some());
        assert!(result[0].schema().column_with_name("pop_est").is_none()); // Not projected

        Ok(())
    }

    /// Test spatial predicate pushdown
    #[tokio::test]
    async fn test_spatial_filter_naturalearth() -> Result<()> {
        let ctx = SessionContext::new();

        ctx.register_simple_geo_file("cities", "tests/data/naturalearth/cities.sgeo").await?;

        // Filter cities within a bounding box (Europe)
        let df = ctx.sql(
            "SELECT name, latitude, longitude
             FROM cities
             WHERE latitude BETWEEN 35.0 AND 70.0
             AND longitude BETWEEN -10.0 AND 40.0"
        ).await?;

        let batches = df.collect().await?;

        // Should find European cities
        assert!(!batches.is_empty());

        // Verify all cities are within the specified bounds
        // (Additional validation logic here)

        Ok(())
    }

    /// Test handling of multiple geometry types
    #[tokio::test]
    async fn test_mixed_geometry_types() -> Result<()> {
        let ctx = SessionContext::new();

        // Register both point and polygon datasets
        ctx.register_simple_geo_file("cities", "tests/data/naturalearth/cities.sgeo").await?;
        ctx.register_simple_geo_file("countries", "tests/data/naturalearth/countries.sgeo").await?;

        // Join cities with their countries
        let df = ctx.sql(
            "SELECT ci.name as city, co.name as country
             FROM cities ci
             JOIN countries co ON ci.iso_a2 = co.iso_a2
             WHERE ci.pop_max > 5000000
             ORDER BY ci.pop_max DESC
             LIMIT 10"
        ).await?;

        let batches = df.collect().await?;

        // Should get major cities with their countries
        assert!(!batches.is_empty());
        assert!(batches[0].num_rows() > 0);

        Ok(())
    }

    /// Benchmark performance with Natural Earth data
    #[tokio::test]
    #[ignore] // Run with: cargo test -- --ignored
    async fn benchmark_naturalearth_read_performance() -> Result<()> {
        use std::time::Instant;

        let ctx = SessionContext::new();

        let start = Instant::now();

        let df = ctx.read_simple_geo_file("tests/data/naturalearth/countries.sgeo").await?;

        let batches = df.collect().await?;
        let duration = start.elapsed();

        println!("Read {} rows in {:?}",
                 batches.iter().map(|b| b.num_rows()).sum::<usize>(),
                 duration);

        // Set performance expectations
        assert!(duration.as_millis() < 1000, "Reading should complete in <1s");

        Ok(())
    }
}
```

##### Test Data Management

**Add to `.gitignore`**:
```gitignore
# Test data (download via script)
tests/data/naturalearth/*.parquet
tests/data/naturalearth/*.fgb
tests/data/naturalearth/*.arrow
```

**Create download script** (`tests/download_test_data.sh`):
```bash
#!/bin/bash
set -e

TESTDATA_DIR="tests/data/naturalearth"
mkdir -p "$TESTDATA_DIR"

echo "Downloading Natural Earth test data..."

# Countries (Polygon geometries with edge cases)
curl -L "https://github.com/geoarrow/geoarrow-data/releases/download/latest/ne_10m_admin_0_countries.parquet" \
  -o "$TESTDATA_DIR/countries.parquet"

# Cities (Point geometries)
curl -L "https://github.com/geoarrow/geoarrow-data/releases/download/latest/ne_10m_populated_places.parquet" \
  -o "$TESTDATA_DIR/cities.parquet"

echo "Test data downloaded successfully!"
echo "Convert to your format:"
echo "  cargo run --example convert_naturalearth"
```

**Add to CI/CD** (`.github/workflows/test.yml` or `.circleci/config.yml`):
```yaml
- run:
    name: Download test data
    command: |
      chmod +x tests/download_test_data.sh
      tests/download_test_data.sh

- run:
    name: Run integration tests
    command: cargo test --test integration_tests
```

##### Test Coverage Checklist

Use Natural Earth data to validate:

- ✅ **Geometry Types**: Points, Lines, Polygons, MultiPolygons
- ✅ **Edge Cases**: Antimeridian crossing, polar regions, special coordinates
- ✅ **Attributes**: Various data types (string, integer, float)
- ✅ **Projection Pushdown**: Column selection optimization
- ✅ **Predicate Pushdown**: Spatial and attribute filtering
- ✅ **Null Handling**: Missing/optional attribute values
- ✅ **Large Datasets**: Performance with thousands of features
- ✅ **Join Operations**: Multi-table queries
- ✅ **Bounding Boxes**: Spatial statistics and metadata

##### Best Practices for Test Data

1. **Version Control**: Pin specific data versions in your download script
2. **Size Management**: Keep test datasets small (<10MB) for fast CI/CD
3. **Format Conversion**: Convert Natural Earth data to your format as part of test setup
4. **Cache in CI**: Cache downloaded test data to speed up CI pipeline
5. **Document Edge Cases**: Comment tests explaining which edge case they validate

By testing with Natural Earth data, you ensure your format integration handles real-world complexities and edge cases that synthetic test data might miss.

## 4. Best Practices and Antipatterns

Adhering to best practices and avoiding common antipatterns is crucial for building performant, robust, and maintainable DataFusion file format integrations.

### 4.1. Best Practices

*   **Leverage `object_store` for all I/O**: Always use `object_store` for reading file data. This provides seamless support for various storage backends (local, S3, GCS, Azure Blob, etc.) and handles asynchronous operations efficiently. Avoid direct usage of `std::fs` or other blocking I/O.
*   **Efficient Compression Handling**:
    *   **External Compression**: `object_store` can often handle external compression (e.g., `.gz`, `.zip` files) transparently. If your format files are compressed at the filesystem level, `object_store` will typically decompress them before passing the raw bytes to your parser.
    *   **Internal Compression**: If your file format has its own internal compression (e.g., GeoParquet using Snappy or Gzip within its blocks), your format-specific parser is responsible for handling this decompression efficiently.
*   **Prioritize Zero-Copy Operations**: Whenever possible, avoid unnecessary memory allocations and copies. GeoArrow and Apache Arrow are designed for zero-copy. If your format's internal memory layout can be directly mapped to Arrow arrays (e.g., by pointing Arrow arrays to existing buffers), this can significantly boost performance.
*   **Embrace GeoArrow and Study the Documentation**: Convert your geospatial data into GeoArrow-compatible Arrow arrays as early as possible in the data processing pipeline. This ensures interoperability with the wider GeoArrow/Arrow ecosystem and allows DataFusion to apply optimized operations. **Regularly consult the [geoarrow-rs documentation](https://geoarrow.org/geoarrow-rs/rust/)** for:
    *   Choosing the right geometry array builder for your data
    *   Understanding memory layouts and performance characteristics
    *   Learning zero-copy construction techniques
    *   Handling coordinate reference systems correctly
    *   Optimizing builder capacity and batch sizes
*   **Implement Pushdown Optimizations**:
    *   **Projection Pushdown**: Only read and decode the columns (attributes and geometry) that are explicitly requested by the query. Use `FileScanConfig::file_column_projection_indices()` to determine the required columns.
    *   **Predicate Pushdown**: If your file format supports it (e.g., via internal indexes or metadata), push down filters (both attribute and spatial) to the reader. This significantly reduces the amount of data that needs to be transferred and processed by DataFusion.
    *   **Statistics**: Implement `infer_stats` to provide DataFusion with meaningful statistics (e.g., bounding boxes for geometries, min/max for attributes). This allows the query optimizer to make better decisions.
*   **Handle Schema Evolution and Merging**: If your format allows for schema variations across files (e.g., different columns or column order), ensure your `infer_schema` implementation can correctly merge these schemas into a unified `SchemaRef`.
*   **Batch Processing**: Always read and process data in batches (e.g., 1024-8192 rows per `RecordBatch`). This is fundamental to Arrow's columnar processing model and DataFusion's performance. Avoid processing one feature at a time.
*   **Asynchronous Operations**: Ensure all I/O and heavy computational tasks within your `FileOpener` are asynchronous. Rust's `async`/`await` and the `futures` crate are essential here.
*   **Robust Error Handling**: Use DataFusion's `Result` type and `DataFusionError` for consistent and informative error reporting. Convert errors from underlying parsing libraries into `DataFusionError::External`.
*   **Thorough Testing**: Write comprehensive unit tests for your format parser and integration tests that use DataFusion's `SessionContext` to verify correctness and performance.

### 4.2. Antipatterns to Avoid

*   **Reading Entire Files into Memory**: For large files, this will lead to out-of-memory errors and poor performance. Always use streaming or range reads via `object_store`.
*   **Unnecessary Data Copying**: Avoid copying data more than necessary. If your format parser produces data in a memory layout that can be directly used by Arrow (or GeoArrow), leverage that for zero-copy.
*   **Blocking I/O**: Performing synchronous file reads or CPU-bound work on the async runtime's main thread will block other tasks and severely degrade performance. Use `tokio::task::spawn_blocking` for CPU-bound work if it cannot be made async.
*   **Ignoring Pushdown Optimizations**: Failing to implement projection and predicate pushdown means DataFusion will read and process more data than necessary, leading to slow queries.
*   **Creating Custom Geometry Types**: Stick to the GeoArrow specification for geometry representation within Arrow arrays. Avoid inventing your own custom Arrow extensions for geometries, as this breaks interoperability.
*   **Inefficient Schema Inference**: Do not read the entire file just to infer the schema. Design your format or parser to extract schema information from a small header or metadata section.
*   **Direct File System Access**: Bypassing `object_store` for file access limits your format to local filesystems and prevents it from working with cloud storage.
*   **Single-Feature Processing**: Building `RecordBatch`es one feature at a time is inefficient. Always use batching.

## 5. Conclusion

By following these steps, you can create a robust and high-performance integration for your geospatial file format within DataFusion. The key is to leverage the existing DataFusion traits, the `object_store` abstraction for I/O, and `geoarrow-rs` for efficient geospatial data representation. Remember to prioritize projection and predicate pushdown to achieve optimal query performance.

### Key Takeaways

1. **DataFusion traits** (`FileFormat`, `FileSource`, `FileOpener`) provide the structure for integrating your format
2. **GeoArrow standardization** ensures your format works seamlessly with the broader ecosystem
3. **The [geoarrow-rs documentation](https://geoarrow.org/geoarrow-rs/rust/)** is your essential reference for working with geometry arrays - bookmark it and refer to it frequently
4. **Pushdown optimizations** (projection and predicates) are critical for query performance
5. **`object_store`** provides unified I/O across local and cloud storage

### Additional Resources

- [GeoArrow Specification](https://geoarrow.org/): Understanding the standard
- **[GeoArrow Rust API Documentation](https://geoarrow.org/geoarrow-rs/rust/)**: Your primary technical reference
- [DataFusion User Guide](https://datafusion.apache.org/): DataFusion concepts and APIs
- [Apache Arrow Format](https://arrow.apache.org/docs/format/Columnar.html): Understanding the columnar format
- [object_store crate docs](https://docs.rs/object_store/): I/O abstraction details

The geoarrow-rs documentation will be particularly valuable as you implement geometry conversion logic - it contains detailed examples, performance tips, and API references for all geometry types supported by GeoArrow.

### Quick Reference: geoarrow-rs Crates

| Crate | Required? | Purpose | Key Use Cases |
|-------|-----------|---------|---------------|
| [`geoarrow-schema`](https://docs.rs/geoarrow-schema/) | ✅ Yes | Geometry type definitions | Schema inference, declaring geometry fields |
| [`geoarrow-array`](https://docs.rs/geoarrow-array/) | ✅ Yes | Array builders and implementations | Building geometry arrays from parsed data |
| [`geoarrow-cast`](https://docs.rs/geoarrow-cast/) | ⚠️ Optional | Type conversion utilities | Converting between geometry types |
| [`geoarrow`](https://docs.rs/geoarrow/) | 🔄 Alternative | Amalgam of above three | Simpler dependency management |
| [`geoparquet`](https://docs.rs/geoparquet/) | 📚 Reference | Complete format implementation | Study for best practices |
| [`geoarrow-flatgeobuf`](https://docs.rs/geoarrow-flatgeobuf/) | 📚 Reference | FlatGeobuf integration | Example of spatial index usage |

**Legend**: ✅ Required | ⚠️ Add if needed | 🔄 Instead of individual crates | 📚 For learning

Start with `geoarrow-schema` and `geoarrow-array` - these are the core crates you'll use in every implementation.
