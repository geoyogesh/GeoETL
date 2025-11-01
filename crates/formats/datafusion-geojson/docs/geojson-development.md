# GeoJSON - Development Guide

Implementation details and architecture of the DataFusion GeoJSON format integration with GeoArrow support.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Module Structure](#module-structure)
- [Core Components](#core-components)
- [GeoJSON Parsing](#geojson-parsing)
- [Schema Inference](#schema-inference)
- [Physical Execution](#physical-execution)
- [Performance Optimizations](#performance-optimizations)
- [Testing](#testing)
- [Contributing](#contributing)

## Architecture Overview

The `datafusion-geojson` crate provides native GeoJSON support for DataFusion by:

1. **Format Detection**: Identifying GeoJSON files by extension
2. **Parsing**: Converting JSON to structured features with geometries and properties
3. **Schema Inference**: Detecting property types and geometry structure
4. **GeoArrow Conversion**: Building GeoArrow geometry arrays from parsed geometries
5. **Streaming Execution**: Processing files in batches for memory efficiency

### Data Flow

```
GeoJSON File → Object Store → JSON Parser → Feature Extraction
                                              ↓
                                          Geometry → GeometryBuilder → GeoArrow Array
                                              ↓
                                          Properties → Arrow Builders → Arrow Arrays
                                              ↓
                                          RecordBatch Stream
```

## Module Structure

```
datafusion-geojson/
├── src/
│   ├── file_format.rs      # FileFormat trait, schema inference
│   ├── file_source.rs      # FileSource trait, table provider, cloud storage
│   ├── parser.rs           # GeoJSON parsing logic
│   ├── physical_exec.rs    # Physical execution, RecordBatch creation
│   └── lib.rs              # Public API, SessionContext extension
└── tests/
    ├── e2e.rs              # End-to-end integration tests
    └── e2e_data/           # Test GeoJSON files
        ├── natural-earth_cities.geojson
        ├── natural-earth_countries.geojson
        └── ...
```

## Core Components

### 1. File Format (`file_format.rs`)

Implements DataFusion's `FileFormat` trait:

```rust
#[derive(Debug, Clone)]
pub struct GeoJsonFormat {
    options: GeoJsonFormatOptions,
}

#[derive(Debug, Clone)]
pub struct GeoJsonFormatOptions {
    batch_size: usize,
    geometry_column_name: String,
    geometry_type: GeoArrowType,
    schema_infer_max_features: Option<usize>,
}

impl FileFormat for GeoJsonFormat {
    fn as_any(&self) -> &dyn Any { self }

    fn get_ext(&self) -> String {
        "geojson".to_string()
    }

    async fn infer_schema(
        &self,
        state: &dyn Session,
        store: &Arc<dyn ObjectStore>,
        objects: &[ObjectMeta],
    ) -> Result<SchemaRef> {
        // Schema inference from GeoJSON properties
    }

    fn file_source(&self) -> Arc<dyn FileSource> {
        Arc::new(GeoJsonSource::new(self.options.clone()))
    }
}
```

#### Schema Inference

Infers Arrow schema from GeoJSON properties:

```rust
async fn infer_schema_from_geojson(
    store: &Arc<dyn ObjectStore>,
    path: &Path,
    options: &GeoJsonFormatOptions,
) -> Result<SchemaRef> {
    // Read sample of file
    let bytes = read_geojson_sample(store, path, options).await?;

    // Parse features
    let features = parse_geojson_bytes(&bytes, options.schema_infer_max_features, "inference")?;

    // Infer property types
    let property_schema = infer_property_schema(&features)?;

    // Add geometry field
    let mut fields: Vec<Arc<Field>> = property_schema.fields().iter().cloned().collect();
    fields.push(Arc::new(Field::new(
        &options.geometry_column_name,
        options.geometry_type.to_data_type(),
        true, // nullable
    )));

    Ok(Arc::new(Schema::new(fields)))
}
```

Property type inference:

```rust
fn infer_property_schema(features: &[FeatureRecord]) -> Result<Schema> {
    let mut property_types: HashMap<String, HashSet<JsonValueType>> = HashMap::new();

    // Collect all property types across features
    for feature in features {
        for (key, value) in &feature.properties {
            property_types
                .entry(key.clone())
                .or_default()
                .insert(classify_json_type(value));
        }
    }

    // Convert to Arrow fields
    let fields: Vec<Arc<Field>> = property_types
        .into_iter()
        .map(|(name, types)| {
            let data_type = if types.contains(&JsonValueType::Number) {
                // Check if all numbers are integers
                if all_integers(&features, &name) {
                    DataType::Int64
                } else {
                    DataType::Float64
                }
            } else if types.contains(&JsonValueType::Boolean) {
                DataType::Boolean
            } else {
                DataType::Utf8 // Default to string
            };

            Arc::new(Field::new(name, data_type, true))
        })
        .collect();

    Ok(Schema::new(fields))
}
```

### 2. Parser (`parser.rs`)

Parses GeoJSON into structured feature records:

```rust
#[derive(Debug)]
pub struct FeatureRecord {
    pub properties: JsonObject,
    pub geometry: Option<Geometry>,
}

pub fn parse_geojson_bytes(
    data: &[u8],
    limit: Option<usize>,
    context: &str,
) -> Result<Vec<FeatureRecord>, SpatialFormatReadError> {
    // Try parsing as FeatureCollection first
    match parse_as_feature_collection(data, limit) {
        Ok(features) => return Ok(features),
        Err(fc_error) => {
            // Try parsing as GeoJSON sequence (newline-delimited)
            match parse_as_sequence(data, limit) {
                Ok(features) => return Ok(features),
                Err(seq_error) => {
                    // Return combined error
                    return Err(SpatialFormatReadError::Parse {
                        message: format!(
                            "Failed to parse GeoJSON as FeatureCollection: {fc_error}; \
                             also failed to parse as GeoJSON sequence: {seq_error}"
                        ),
                        position: None,
                        context: Some(context.to_string()),
                    });
                }
            }
        }
    }
}
```

FeatureCollection parsing:

```rust
fn parse_as_feature_collection(
    data: &[u8],
    limit: Option<usize>,
) -> Result<Vec<FeatureRecord>, Box<dyn std::error::Error>> {
    let geojson: GeoJson = serde_json::from_slice(data)?;

    let features = match geojson {
        GeoJson::FeatureCollection(fc) => {
            fc.features
                .into_iter()
                .take(limit.unwrap_or(usize::MAX))
                .map(feature_to_record)
                .collect()
        }
        GeoJson::Feature(feature) => {
            vec![feature_to_record(feature)]
        }
        GeoJson::Geometry(geom) => {
            // Single geometry → feature with no properties
            vec![FeatureRecord {
                properties: JsonObject::new(),
                geometry: Some(geom.try_into()?),
            }]
        }
    };

    Ok(features)
}
```

GeoJSON sequence parsing:

```rust
fn parse_as_sequence(
    data: &[u8],
    limit: Option<usize>,
) -> Result<Vec<FeatureRecord>, Box<dyn std::error::Error>> {
    let text = std::str::from_utf8(data)?;

    let mut features = Vec::new();
    for (line_num, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let geojson: GeoJson = serde_json::from_str(line)
            .map_err(|e| format!("Line {}: {}", line_num + 1, e))?;

        match geojson {
            GeoJson::FeatureCollection(fc) => {
                for feature in fc.features {
                    features.push(feature_to_record(feature));
                    if let Some(limit) = limit {
                        if features.len() >= limit {
                            return Ok(features);
                        }
                    }
                }
            }
            GeoJson::Feature(feature) => {
                features.push(feature_to_record(feature));
            }
            GeoJson::Geometry(geom) => {
                features.push(FeatureRecord {
                    properties: JsonObject::new(),
                    geometry: Some(geom.try_into()?),
                });
            }
        }

        if let Some(limit) = limit {
            if features.len() >= limit {
                break;
            }
        }
    }

    if features.is_empty() {
        return Err("No GeoJSON features found".into());
    }

    Ok(features)
}
```

Feature conversion:

```rust
fn feature_to_record(feature: geojson::Feature) -> FeatureRecord {
    let geometry = feature.geometry.map(|g| {
        Geometry::try_from(&g).expect("Failed to convert GeoJSON geometry")
    });

    let properties = feature.properties.unwrap_or_default();

    FeatureRecord {
        properties,
        geometry,
    }
}
```

### 3. Physical Execution (`physical_exec.rs`)

Converts parsed features to RecordBatches:

```rust
pub struct GeoJsonOpener {
    options: GeoJsonFormatOptions,
    projected_schema: SchemaRef,
    object_store: Arc<dyn ObjectStore>,
    batch_size: usize,
}

impl FileOpener for GeoJsonOpener {
    fn open(&self, file_meta: FileMeta) -> Result<FileOpenFuture> {
        let object_store = self.object_store.clone();
        let path = file_meta.object_meta.location.clone();
        let options = self.options.clone();
        let schema = self.projected_schema.clone();
        let batch_size = self.batch_size;

        Ok(Box::pin(async move {
            // Read file from object store
            let reader = object_store.get(&path).await?;
            let bytes = reader.bytes().await?;

            // Parse all features (could be optimized for streaming)
            let all_features = parse_geojson_bytes(
                &bytes,
                None,
                path.as_ref()
            )?;

            // Create stream of RecordBatches
            let stream = futures::stream::iter(
                all_features.chunks(batch_size).enumerate()
            ).map(move |(batch_idx, features)| {
                let source = Arc::from(format!("{}:batch{}", path, batch_idx));
                records_to_batch(&schema, &options, &source, features)
            });

            Ok(stream.boxed())
        }))
    }
}
```

RecordBatch creation:

```rust
fn records_to_batch(
    schema: &SchemaRef,
    options: &GeoJsonFormatOptions,
    source: &Arc<str>,
    records: &[FeatureRecord],
) -> Result<RecordBatch> {
    if schema.fields().is_empty() {
        return RecordBatch::try_new_with_options(
            schema.clone(),
            vec![],
            &RecordBatchOptions::new().with_row_count(Some(records.len())),
        )?;
    }

    let mut columns = Vec::with_capacity(schema.fields().len());

    for field in schema.fields() {
        if field.name() == &options.geometry_column_name {
            columns.push(build_geometry_array(records, options, source)?);
            continue;
        }

        let array = match field.data_type() {
            DataType::Boolean => build_boolean_array(field, records, source),
            DataType::Int64 => build_int64_array(field, records, source),
            DataType::Float64 => build_float64_array(field, records, source),
            DataType::Utf8 => Ok(build_utf8_array(field, records)),
            other => Err(DataFusionError::Plan(format!(
                "Unsupported data type {:?} for GeoJSON property '{}'",
                other, field.name()
            ))),
        }?;

        columns.push(array);
    }

    RecordBatch::try_new(schema.clone(), columns).map_err(Into::into)
}
```

Array builders for properties:

```rust
fn build_boolean_array(
    field: &Field,
    records: &[FeatureRecord],
    source: &Arc<str>,
) -> Result<ArrayRef> {
    let mut builder = BooleanBuilder::with_capacity(records.len());

    for record in records {
        match record.properties.get(field.name()) {
            Some(JsonValue::Bool(b)) => builder.append_value(*b),
            Some(JsonValue::Null) | None => builder.append_null(),
            Some(other) => {
                return Err(property_type_error(field, "bool", other, source));
            }
        }
    }

    Ok(Arc::new(builder.finish()))
}

fn build_int64_array(
    field: &Field,
    records: &[FeatureRecord],
    source: &Arc<str>,
) -> Result<ArrayRef> {
    let mut builder = Int64Builder::with_capacity(records.len());

    for record in records {
        match record.properties.get(field.name()) {
            Some(JsonValue::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    builder.append_value(i);
                } else {
                    return Err(property_type_error(field, "integer", &JsonValue::Number(n.clone()), source));
                }
            }
            Some(JsonValue::Null) | None => builder.append_null(),
            Some(other) => {
                return Err(property_type_error(field, "integer", other, source));
            }
        }
    }

    Ok(Arc::new(builder.finish()))
}

fn build_utf8_array(field: &Field, records: &[FeatureRecord]) -> ArrayRef {
    let mut builder = StringBuilder::with_capacity(records.len(), 1024);

    for record in records {
        match record.properties.get(field.name()) {
            Some(JsonValue::String(s)) => builder.append_value(s),
            Some(JsonValue::Null) | None => builder.append_null(),
            Some(other) => {
                // Convert other types to string
                builder.append_value(&other.to_string());
            }
        }
    }

    Arc::new(builder.finish())
}
```

Geometry array building:

```rust
fn build_geometry_array(
    records: &[FeatureRecord],
    options: &GeoJsonFormatOptions,
    source: &Arc<str>,
) -> Result<ArrayRef> {
    let mut builder = GeometryBuilder::new(options.geometry_type.clone());

    for feature in records {
        builder
            .push_geometry(feature.geometry.as_ref())
            .map_err(|err| {
                DataFusionError::from(SpatialFormatReadError::Parse {
                    message: format!("Failed to encode GeoJSON geometry: {err}"),
                    position: None,
                    context: Some(source.to_string()),
                })
            })?;
    }

    Ok(builder.finish().into_array_ref())
}
```

### 4. File Source (`file_source.rs`)

Cloud storage integration:

```rust
pub async fn create_geojson_table_provider(
    state: &SessionState,
    path: &str,
    options: GeoJsonFormatOptions,
) -> Result<Arc<dyn TableProvider>> {
    // Register object store for cloud URLs
    register_object_store_from_url(state, path).await?;

    // Parse URL
    let table_url = ListingTableUrl::parse(path)?;

    // Create listing options
    let listing_options = ListingOptions::new(Arc::new(GeoJsonFormat::new(options)))
        .with_file_extension("geojson")
        .with_file_extension("json");

    // Create table config
    let config = ListingTableConfig::new(table_url)
        .with_listing_options(listing_options);

    // Create and return table provider
    Ok(Arc::new(ListingTable::try_new(config)?))
}
```

Object store registration:

```rust
async fn register_object_store_from_url(
    state: &SessionState,
    url: &str,
) -> Result<()> {
    let parsed_url = ListingTableUrl::parse(url)?;

    match parsed_url.scheme() {
        "s3" | "s3a" => {
            let s3_store = AmazonS3Builder::from_env()
                .with_bucket_name(parsed_url.prefix().bucket_name().unwrap())
                .build()?;
            state.runtime_env().register_object_store(
                &parsed_url.object_store(),
                Arc::new(s3_store)
            );
        }
        "gs" | "gcs" => {
            let gcs_store = GoogleCloudStorageBuilder::from_env()
                .with_bucket_name(parsed_url.prefix().bucket_name().unwrap())
                .build()?;
            state.runtime_env().register_object_store(
                &parsed_url.object_store(),
                Arc::new(gcs_store)
            );
        }
        "az" | "adl" => {
            let azure_store = MicrosoftAzureBuilder::from_env()
                .with_container_name(parsed_url.prefix().bucket_name().unwrap())
                .build()?;
            state.runtime_env().register_object_store(
                &parsed_url.object_store(),
                Arc::new(azure_store)
            );
        }
        "http" | "https" => {
            let http_store = HttpBuilder::new()
                .with_url(url)
                .build()?;
            state.runtime_env().register_object_store(
                &parsed_url.object_store(),
                Arc::new(http_store)
            );
        }
        _ => {} // Local filesystem, no registration needed
    }

    Ok(())
}
```

## Performance Optimizations

### 1. Batch Processing

Process features in batches:

```rust
let batch_size = options.batch_size; // e.g., 8192

for chunk in all_features.chunks(batch_size) {
    let batch = records_to_batch(&schema, &options, &source, chunk)?;
    // Stream batch to DataFusion
}
```

### 2. Schema Inference Sampling

Limit features sampled for schema inference:

```rust
let sample_limit = options.schema_infer_max_features.unwrap_or(1000);
let features = parse_geojson_bytes(&bytes, Some(sample_limit), "inference")?;
```

### 3. Projection Pushdown

Only build arrays for projected columns:

```rust
let projection_indices = config.file_column_projection_indices();
let projected_schema = config.file_schema.project(&projection_indices)?;

// Only create builders for projected fields
for field in projected_schema.fields() {
    // Build only necessary columns
}
```

### 4. Streaming (Future Enhancement)

Current implementation loads entire file. Future improvement:

```rust
// Stream large GeoJSON sequences
let stream = BufReader::new(reader)
    .lines()
    .filter_map(|line| parse_geojson_line(line))
    .chunks(batch_size)
    .map(|chunk| records_to_batch(&schema, &options, &source, &chunk));
```

## Testing

### Unit Tests

Test parser components:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feature_collection() {
        let data = br#"{
            "type": "FeatureCollection",
            "features": [
                {"type":"Feature","geometry":{"type":"Point","coordinates":[1.0,2.0]},"properties":{"name":"A"}},
                {"type":"Feature","geometry":null,"properties":{"value":42}}
            ]
        }"#;

        let records = parse_geojson_bytes(data, None, "test").expect("parse");
        assert_eq!(records.len(), 2);
        assert!(records[0].geometry.is_some());
        assert_eq!(records[0].properties.get("name").unwrap(), "A");
        assert!(records[1].geometry.is_none());
    }

    #[test]
    fn test_parse_geojson_sequence() {
        let data = br#"{"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":{"id":1}}
{"type":"Feature","geometry":{"type":"Point","coordinates":[1,1]},"properties":{"id":2}}"#;

        let records = parse_geojson_bytes(data, Some(1), "seq").expect("sequence");
        assert_eq!(records.len(), 1);
    }
}
```

### Integration Tests

End-to-end tests with DataFusion:

```rust
#[tokio::test]
async fn test_read_cities_geojson() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_geojson_file("cities", "tests/e2e_data/natural-earth_cities.geojson")
        .await?;

    let df = ctx.sql(r"SELECT name, geometry FROM cities LIMIT 5").await?;
    let batches = df.collect().await?;

    assert!(!batches.is_empty());
    assert_eq!(batches[0].num_rows(), 5);

    Ok(())
}

#[tokio::test]
async fn test_cities_to_geoarrow_points() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_geojson_file("cities", "tests/e2e_data/natural-earth_cities.geojson")
        .await?;

    let df = ctx.sql(
        r#"SELECT geometry, name FROM cities WHERE name = 'Vatican City' LIMIT 1"#
    ).await?;

    let batches = df.collect().await?;
    let batch = &batches[0];
    let schema = batch.schema();
    let field = schema.field_with_name("geometry")?;
    let column = batch.column(schema.index_of("geometry")?).clone();

    // Verify it's a GeoArrow GeometryArray
    let geometry_array = GeometryArray::try_from((column.as_ref(), field))?;
    assert_eq!(geometry_array.len(), 1);

    // Extract and verify coordinates
    let first_geom = geometry_array.value(0)?;
    let geo_traits::GeometryType::Point(point) = first_geom.as_type() else {
        panic!("Expected point")
    };
    let coord = point.coord().unwrap();

    assert!((coord.x() - 12.453_386_5).abs() < 1e-6);
    assert!((coord.y() - 41.903_282_2).abs() < 1e-6);

    Ok(())
}
```

## Contributing

### Adding Features

To add new functionality:

1. **Update parser** (`parser.rs`) if needed
2. **Update schema inference** (`file_format.rs`)
3. **Update physical execution** (`physical_exec.rs`)
4. **Add tests**
5. **Update documentation**

### Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add doc comments for public APIs
- Include examples in doc comments

### Pull Request Checklist

- [ ] Tests added for new functionality
- [ ] Documentation updated (user guide and dev guide)
- [ ] `cargo test --package datafusion-geojson` passes
- [ ] `cargo clippy` passes
- [ ] Code formatted with `cargo fmt`
- [ ] E2E tests pass

## Performance Benchmarks

Typical performance:

| Operation | Throughput | Notes |
|-----------|------------|-------|
| GeoJSON parsing | ~50 MB/s | Single-threaded |
| Point geometry conversion | ~200k points/s | To GeoArrow |
| Polygon geometry conversion | ~20k polygons/s | Complexity dependent |
| Batch processing | 8192 rows optimal | Balance memory/throughput |

Run benchmarks:

```bash
cargo bench --package datafusion-geojson
```

## Debugging

Enable logging:

```rust
env_logger::init();
// Or
RUST_LOG=datafusion_geojson=debug cargo test
```

Debug points:

```rust
// In parser.rs
log::debug!("Parsing {} bytes as GeoJSON", data.len());
log::debug!("Found {} features", features.len());

// In physical_exec.rs
log::debug!("Building batch with {} features", records.len());
log::debug!("Schema: {:?}", schema);
```

## Resources

- [GeoJSON Specification (RFC 7946)](https://tools.ietf.org/html/rfc7946)
- [GeoArrow Specification](https://geoarrow.org/)
- [DataFusion Developer Guide](https://datafusion.apache.org/contributor-guide/)
- [serde_json Documentation](https://docs.rs/serde_json/)
- [geojson Crate](https://docs.rs/geojson/)
- [geo-types Documentation](https://docs.rs/geo-types/)
