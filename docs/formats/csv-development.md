# CSV with GeoArrow - Development Guide

Implementation details and architecture of the DataFusion CSV format integration with GeoArrow support.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Module Structure](#module-structure)
- [Core Components](#core-components)
- [WKT Geometry Parsing](#wkt-geometry-parsing)
- [Schema Inference](#schema-inference)
- [Physical Execution](#physical-execution)
- [Performance Optimizations](#performance-optimizations)
- [Testing](#testing)
- [Contributing](#contributing)

## Architecture Overview

The `datafusion-csv` crate extends DataFusion's CSV capabilities with geospatial support by:

1. **Schema Enhancement**: Detecting and declaring geometry columns with GeoArrow types
2. **WKT Parsing**: Converting Well-Known Text to GeoArrow arrays during reads
3. **Streaming Execution**: Processing CSV data in batches with geometry conversion
4. **Object Store Integration**: Supporting local and cloud storage

### Data Flow

```
CSV File → Object Store → CSV Parser → WKT Parser → GeoArrow Builder → RecordBatch
                                    ↓
                                Properties → Arrow Arrays (String, Int64, etc.)
```

## Module Structure

```
datafusion-csv/
├── src/
│   ├── file_format.rs      # FileFormat implementation, schema inference
│   ├── file_source.rs      # FileSource implementation, table provider
│   ├── geospatial.rs       # WKT geometry parsing logic
│   ├── object_store_reader.rs  # Object store utilities
│   ├── physical_exec.rs    # Physical execution, batch processing
│   └── lib.rs              # Public API, SessionContext extension
└── tests/
    ├── e2e_spatial.rs      # End-to-end spatial tests
    └── e2e_non_spatial.rs  # Non-spatial CSV tests
```

## Core Components

### 1. File Format (`file_format.rs`)

Implements DataFusion's `FileFormat` trait:

```rust
pub struct CsvFormat {
    options: CsvFormatOptions,
}

impl FileFormat for CsvFormat {
    fn as_any(&self) -> &dyn Any { self }
    fn get_ext(&self) -> String { "csv".to_string() }

    async fn infer_schema(
        &self,
        state: &dyn Session,
        store: &Arc<dyn ObjectStore>,
        objects: &[ObjectMeta],
    ) -> Result<SchemaRef> {
        // Schema inference with geometry detection
    }

    fn file_source(&self) -> Arc<dyn FileSource> {
        Arc::new(CsvSource::new(self.options.clone()))
    }
}
```

#### CsvFormatOptions

Configuration structure for CSV reading:

```rust
pub struct CsvFormatOptions {
    delimiter: u8,
    has_header: bool,
    batch_size: usize,
    schema_infer_max_records: Option<usize>,
    geometry_options: Option<GeometryColumnOptions>,
}

pub struct GeometryColumnOptions {
    column_name: String,
    source_format: GeometrySourceFormat,  // WKT
    target_type: GeoArrowType,
}
```

Builder pattern implementation:

```rust
impl CsvFormatOptions {
    pub fn with_geometry_from_wkt(
        mut self,
        column_name: impl Into<String>,
        geometry_type: GeoArrowType,
    ) -> Self {
        self.geometry_options = Some(GeometryColumnOptions {
            column_name: column_name.into(),
            source_format: GeometrySourceFormat::WKT,
            target_type: geometry_type,
        });
        self
    }
}
```

### 2. File Source (`file_source.rs`)

Implements `FileSource` trait and creates table providers:

```rust
pub struct CsvSource {
    options: CsvFormatOptions,
    metrics: ExecutionPlanMetricsSet,
}

impl FileSource for CsvSource {
    fn create_file_opener(
        &self,
        object_store: Arc<dyn ObjectStore>,
        config: &FileScanConfig,
        partition: usize,
    ) -> Arc<dyn FileOpener> {
        Arc::new(CsvOpener::new(
            object_store,
            config.clone(),
            self.options.clone(),
        ))
    }
}
```

Object store registration for cloud storage:

```rust
pub(crate) async fn register_object_store_from_url(
    state: &SessionState,
    url: &str,
) -> Result<()> {
    let url = ListingTableUrl::parse(url)?;

    match url.scheme() {
        "s3" | "s3a" => register_s3_store(state, &url).await?,
        "gs" | "gcs" => register_gcs_store(state, &url).await?,
        "az" | "adl" => register_azure_store(state, &url).await?,
        "http" | "https" => register_http_store(state, &url).await?,
        _ => {} // Local filesystem
    }

    Ok(())
}
```

### 3. Geospatial Module (`geospatial.rs`)

WKT parsing and GeoArrow conversion:

```rust
pub fn parse_wkt_to_geoarrow(
    wkt_column: &StringArray,
    geometry_type: &GeoArrowType,
) -> Result<Arc<dyn Array>> {
    match geometry_type {
        GeoArrowType::Point(point_type) => {
            parse_wkt_points(wkt_column, point_type)
        }
        GeoArrowType::LineString(ls_type) => {
            parse_wkt_linestrings(wkt_column, ls_type)
        }
        GeoArrowType::Polygon(poly_type) => {
            parse_wkt_polygons(wkt_column, poly_type)
        }
        GeoArrowType::MultiPolygon(mp_type) => {
            parse_wkt_multipolygons(wkt_column, mp_type)
        }
        // ... other types
    }
}
```

Point parsing implementation:

```rust
fn parse_wkt_points(
    wkt_column: &StringArray,
    point_type: &PointType,
) -> Result<Arc<dyn Array>> {
    let mut builder = PointBuilder::with_capacity_and_options(
        wkt_column.len(),
        point_type.coord_type(),
        point_type.metadata().clone(),
    );

    for i in 0..wkt_column.len() {
        if wkt_column.is_null(i) {
            builder.push_null();
        } else {
            let wkt_str = wkt_column.value(i);
            let geom = wkt::Wkt::from_str(wkt_str)
                .map_err(|e| DataFusionError::External(Box::new(e)))?;

            match geom.item {
                wkt::Geometry::Point(point) => {
                    builder.push_point(Some(&geo_types::Point::new(
                        point.x.unwrap(),
                        point.y.unwrap(),
                    )));
                }
                _ => return Err(DataFusionError::Plan(
                    format!("Expected POINT, got {:?}", geom.item)
                )),
            }
        }
    }

    Ok(builder.finish().into_array_ref())
}
```

### 4. Physical Execution (`physical_exec.rs`)

Implements `FileOpener` for streaming CSV reads:

```rust
pub struct CsvOpener {
    object_store: Arc<dyn ObjectStore>,
    config: FileScanConfig,
    options: CsvFormatOptions,
}

impl FileOpener for CsvOpener {
    fn open(&self, file_meta: FileMeta) -> Result<FileOpenFuture> {
        let object_store = self.object_store.clone();
        let path = file_meta.object_meta.location.clone();
        let options = self.options.clone();
        let projected_schema = self.config.file_schema.clone();

        Ok(Box::pin(async move {
            // Open file from object store
            let reader = object_store.get(&path).await?;
            let stream = reader.into_stream();

            // Create CSV reader
            let csv_reader = ReaderBuilder::new(projected_schema.clone())
                .with_delimiter(options.delimiter)
                .with_batch_size(options.batch_size)
                .build_decoder();

            // Stream RecordBatches with geometry conversion
            let batch_stream = csv_reader
                .decode(stream)
                .and_then(move |batch| {
                    convert_geometry_columns(batch, &options.geometry_options)
                });

            Ok(batch_stream.boxed())
        }))
    }
}
```

Geometry column conversion:

```rust
async fn convert_geometry_columns(
    batch: RecordBatch,
    geom_options: &Option<GeometryColumnOptions>,
) -> Result<RecordBatch> {
    let Some(options) = geom_options else {
        return Ok(batch); // No geometry columns
    };

    let schema = batch.schema();
    let geom_idx = schema.index_of(&options.column_name)?;

    // Get WKT column as StringArray
    let wkt_column = batch
        .column(geom_idx)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| DataFusionError::Plan(
            format!("Geometry column '{}' is not string type", options.column_name)
        ))?;

    // Parse WKT to GeoArrow
    let geometry_array = parse_wkt_to_geoarrow(wkt_column, &options.target_type)?;

    // Replace WKT column with geometry array
    let mut columns: Vec<Arc<dyn Array>> = batch.columns().to_vec();
    columns[geom_idx] = geometry_array;

    // Update schema with GeoArrow type
    let mut fields: Vec<Arc<Field>> = schema.fields().iter().cloned().collect();
    fields[geom_idx] = Arc::new(Field::new(
        &options.column_name,
        options.target_type.to_data_type(),
        true,
    ));
    let new_schema = Arc::new(Schema::new(fields));

    RecordBatch::try_new(new_schema, columns)
}
```

## Schema Inference

Schema inference detects geometry columns and assigns GeoArrow types:

```rust
async fn infer_schema_with_geometry(
    store: &Arc<dyn ObjectStore>,
    path: &Path,
    options: &CsvFormatOptions,
) -> Result<SchemaRef> {
    // Read sample for inference
    let sample_size = options.schema_infer_max_records.unwrap_or(100);
    let reader = store.get(path).await?;
    let bytes = reader.bytes().await?;

    // Infer base schema from CSV
    let mut schema = infer_csv_schema(&bytes, options.delimiter, options.has_header, sample_size)?;

    // If geometry column specified, update its type
    if let Some(geom_opts) = &options.geometry_options {
        let geom_idx = schema.index_of(&geom_opts.column_name)?;

        let mut fields: Vec<Arc<Field>> = schema.fields().iter().cloned().collect();
        fields[geom_idx] = Arc::new(Field::new(
            &geom_opts.column_name,
            geom_opts.target_type.to_data_type(),
            true, // nullable
        ));

        schema = Arc::new(Schema::new(fields));
    }

    Ok(schema)
}
```

## Performance Optimizations

### 1. Projection Pushdown

Only read required columns:

```rust
impl FileOpener for CsvOpener {
    fn open(&self, file_meta: FileMeta) -> Result<FileOpenFuture> {
        // Get projection from config
        let projection = self.config.file_column_projection_indices();

        // Build CSV reader with projection
        let csv_reader = ReaderBuilder::new(self.config.file_schema.clone())
            .with_projection(projection.cloned())
            .build();

        // Only specified columns are read and decoded
    }
}
```

### 2. Batch Processing

Process CSV in configurable batch sizes:

```rust
let options = CsvFormatOptions::default()
    .with_batch_size(8192); // Process 8192 rows at a time
```

### 3. Streaming Reads

Use object_store's streaming API:

```rust
let reader = object_store.get(&path).await?;
let byte_stream = reader.into_stream(); // Streaming, not buffering entire file

let csv_stream = ReaderBuilder::new(schema)
    .build_decoder()
    .decode(byte_stream); // Process chunks as they arrive
```

### 4. Zero-Copy WKT Parsing

Minimize allocations during geometry parsing:

```rust
// Parse WKT directly into GeoArrow builders
// Builders pre-allocate based on batch size
let mut builder = PointBuilder::with_capacity(batch_size);

for wkt_str in wkt_column.iter() {
    // Parse and push in one step, no intermediate allocations
    let point = parse_wkt_point(wkt_str)?;
    builder.push_point(Some(&point));
}
```

## Testing

### Unit Tests

Test individual components:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wkt_point_parsing() {
        let wkt = "POINT(-86.64301145 32.5350198)";
        let geom = parse_wkt_geometry(wkt).unwrap();

        match geom {
            Geometry::Point(pt) => {
                assert!((pt.x() - (-86.64301145)).abs() < 1e-8);
                assert!((pt.y() - 32.5350198).abs() < 1e-8);
            }
            _ => panic!("Expected Point"),
        }
    }

    #[test]
    fn test_geometry_column_conversion() {
        let wkt_array = StringArray::from(vec![
            Some("POINT(0 0)"),
            Some("POINT(1 1)"),
            None,
        ]);

        let point_type = PointType::new(Dimension::XY, Arc::default());
        let geom_array = parse_wkt_points(&wkt_array, &point_type).unwrap();

        assert_eq!(geom_array.len(), 3);
        assert!(geom_array.is_valid(0));
        assert!(geom_array.is_valid(1));
        assert!(geom_array.is_null(2));
    }
}
```

### Integration Tests

End-to-end tests with DataFusion:

```rust
#[tokio::test]
async fn test_csv_with_wkt_geometries() -> Result<()> {
    let ctx = SessionContext::new();

    let options = CsvFormatOptions::default()
        .with_geometry_from_wkt(
            "geometry",
            GeoArrowType::Point(PointType::new(Dimension::XY, Arc::default()))
        );

    ctx.register_csv_with_options(
        "places",
        "tests/data/places.csv",
        options
    ).await?;

    let df = ctx.sql("SELECT * FROM places").await?;
    let batches = df.collect().await?;

    // Verify geometry column is GeoArrow Point array
    let schema = batches[0].schema();
    let geom_field = schema.field_with_name("geometry")?;
    assert!(matches!(
        GeoArrowType::try_from(geom_field.data_type()),
        Ok(GeoArrowType::Point(_))
    ));

    Ok(())
}
```

### Test Data

Example CSV with WKT:

```csv
id,name,population,geometry
1,Montgomery,200000,POINT(-86.64301145 32.5350198)
2,New York,8336000,POINT(-73.98546518 40.7484284)
3,Los Angeles,3980000,POINT(-118.2436849 34.0522342)
```

## Contributing

### Adding New Geometry Types

To add support for a new geometry type:

1. **Add parsing function** in `geospatial.rs`:

```rust
fn parse_wkt_multilinestrings(
    wkt_column: &StringArray,
    mls_type: &MultiLineStringType,
) -> Result<Arc<dyn Array>> {
    let mut builder = MultiLineStringBuilder::with_capacity_and_options(
        wkt_column.len(),
        mls_type.coord_type(),
        mls_type.metadata().clone(),
    );

    for i in 0..wkt_column.len() {
        if wkt_column.is_null(i) {
            builder.push_null();
        } else {
            let wkt_str = wkt_column.value(i);
            let geom = parse_wkt(wkt_str)?;

            match geom {
                Geometry::MultiLineString(mls) => {
                    builder.push_multi_line_string(Some(&mls))?;
                }
                _ => return Err(/* type error */),
            }
        }
    }

    Ok(builder.finish().into_array_ref())
}
```

2. **Add match arm** in `parse_wkt_to_geoarrow`:

```rust
GeoArrowType::MultiLineString(mls_type) => {
    parse_wkt_multilinestrings(wkt_column, mls_type)
}
```

3. **Add tests**:

```rust
#[tokio::test]
async fn test_multilinestring_parsing() -> Result<()> {
    let ctx = SessionContext::new();

    let options = CsvFormatOptions::default()
        .with_geometry_from_wkt(
            "geometry",
            GeoArrowType::MultiLineString(
                MultiLineStringType::new(Dimension::XY, Arc::default())
            )
        );

    // Test with data...
    Ok(())
}
```

### Code Style

Follow Rust conventions:
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add documentation comments for public APIs
- Include examples in doc comments

### Pull Request Checklist

- [ ] Tests added for new functionality
- [ ] Documentation updated
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] Code formatted with `cargo fmt`
- [ ] Performance benchmarks (if applicable)

## Performance Benchmarks

Typical performance characteristics:

| Operation | Throughput | Notes |
|-----------|------------|-------|
| CSV reading (no geometry) | ~500 MB/s | Depends on column count |
| WKT Point parsing | ~100k points/s | Single-threaded |
| WKT Polygon parsing | ~10k polygons/s | Complexity dependent |
| Batch processing | ~8192 rows optimal | Balance memory/throughput |

Benchmark with:

```bash
cargo bench --package datafusion-csv
```

## Debugging

Enable detailed logging:

```rust
env_logger::init();

// Or with specific module:
RUST_LOG=datafusion_csv=debug cargo test
```

Common debug points:

```rust
// In physical_exec.rs
log::debug!("Processing batch: {} rows", batch.num_rows());
log::debug!("Geometry column: {}", geom_column_name);

// In geospatial.rs
log::debug!("Parsing WKT: {}", wkt_str);
log::debug!("Parsed geometry type: {:?}", geom.type_name());
```

## Resources

- [DataFusion Developer Guide](https://datafusion.apache.org/contributor-guide/)
- [Arrow CSV Reader](https://docs.rs/arrow-csv/)
- [GeoArrow Specification](https://geoarrow.org/)
- [WKT Crate Documentation](https://docs.rs/wkt/)
- [geo-types Documentation](https://docs.rs/geo-types/)
