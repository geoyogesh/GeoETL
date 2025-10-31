//! Physical execution for `GeoJSON` reading.
//!
//! This module wires `GeoJSON` parsing into `DataFusion`'s `FileOpener` abstraction and produces
//! `GeoArrow`-backed record batches.

use std::sync::Arc;

use arrow_array::builder::{BooleanBuilder, Float64Builder, Int64Builder, StringBuilder};
use arrow_array::{ArrayRef, RecordBatch, RecordBatchOptions};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use datafusion::datasource::listing::PartitionedFile;
use datafusion::datasource::physical_plan::{FileMeta, FileOpenFuture, FileOpener};
use datafusion::error::{DataFusionError, Result};
use datafusion_shared::SpatialFormatReadError;
use futures::TryStreamExt;
use geoarrow_array::GeoArrowArray;
use geoarrow_array::builder::GeometryBuilder;
use object_store::ObjectStore;

use crate::file_format::GeoJsonFormatOptions;
use crate::parser::{FeatureRecord, describe_value, parse_geojson_bytes};

/// `GeoJSON` file opener that produces record batches using `GeoArrow` arrays.
#[derive(Clone)]
pub struct GeoJsonOpener {
    options: GeoJsonFormatOptions,
    schema: SchemaRef,
    projection: Option<Vec<usize>>,
    batch_size: usize,
    object_store: Arc<dyn ObjectStore>,
}

impl GeoJsonOpener {
    pub fn new(
        options: GeoJsonFormatOptions,
        schema: SchemaRef,
        projection: Option<Vec<usize>>,
        object_store: Arc<dyn ObjectStore>,
    ) -> Self {
        Self {
            options,
            schema,
            projection,
            batch_size: 8192,
            object_store,
        }
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
}

impl FileOpener for GeoJsonOpener {
    fn open(&self, file_meta: FileMeta, _file: PartitionedFile) -> Result<FileOpenFuture> {
        let opener = self.clone();
        let object_store = Arc::clone(&self.object_store);

        Ok(Box::pin(async move {
            let location = file_meta.location();
            let source_path: Arc<str> = Arc::from(location.to_string());

            let get_result = object_store.get(location).await.map_err(|err| {
                DataFusionError::from(SpatialFormatReadError::Io {
                    source: std::io::Error::other(err),
                    context: Some(source_path.to_string()),
                })
            })?;

            let bytes = get_result.bytes().await.map_err(|err| {
                DataFusionError::from(SpatialFormatReadError::Io {
                    source: std::io::Error::other(err),
                    context: Some(source_path.to_string()),
                })
            })?;

            let records = parse_geojson_bytes(&bytes, None, source_path.to_string())
                .map_err(DataFusionError::from)?;

            let output_schema = if let Some(ref proj) = opener.projection {
                let fields: Vec<Field> = proj
                    .iter()
                    .map(|i| opener.schema.field(*i).clone())
                    .collect();
                Arc::new(Schema::new(fields))
            } else {
                opener.schema.clone()
            };

            let state = GeoJsonReadState {
                schema: output_schema,
                options: opener.options.clone(),
                features: Arc::new(records),
                batch_size: opener.batch_size,
                offset: 0,
                source: Arc::clone(&source_path),
            };

            let stream = futures::stream::try_unfold(state, |mut state| async move {
                if state.offset >= state.features.len() {
                    return Ok(None);
                }

                let end = (state.offset + state.batch_size).min(state.features.len());
                let slice = &state.features[state.offset..end];
                let batch = records_to_batch(&state.schema, &state.options, &state.source, slice)?;
                state.offset = end;
                Ok(Some((batch, state)))
            })
            .into_stream();

            Ok(Box::pin(stream) as _)
        }))
    }
}

struct GeoJsonReadState {
    schema: SchemaRef,
    options: GeoJsonFormatOptions,
    features: Arc<Vec<FeatureRecord>>,
    batch_size: usize,
    offset: usize,
    source: Arc<str>,
}

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
        )
        .map_err(|err| {
            DataFusionError::from(SpatialFormatReadError::Parse {
                message: format!("Failed to create empty RecordBatch: {err}"),
                position: None,
                context: Some(source.to_string()),
            })
        });
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
            other => Err(DataFusionError::from(SpatialFormatReadError::Parse {
                message: format!(
                    "Unsupported data type {other:?} for GeoJSON property '{}'",
                    field.name()
                ),
                position: None,
                context: Some(source.to_string()),
            })),
        }?;

        columns.push(array);
    }

    RecordBatch::try_new(schema.clone(), columns).map_err(|err| {
        DataFusionError::from(SpatialFormatReadError::Parse {
            message: format!("Failed to build record batch: {err}"),
            position: None,
            context: Some(source.to_string()),
        })
    })
}

fn build_boolean_array(
    field: &Field,
    records: &[FeatureRecord],
    source: &Arc<str>,
) -> Result<ArrayRef> {
    let mut builder = BooleanBuilder::with_capacity(records.len());

    for feature in records {
        match feature.properties.get(field.name()) {
            Some(geojson::JsonValue::Bool(value)) => builder.append_value(*value),
            Some(geojson::JsonValue::Null) | None => builder.append_null(),
            Some(other) => return Err(property_type_error(field, "bool", other, source)),
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

    for feature in records {
        match feature.properties.get(field.name()) {
            Some(geojson::JsonValue::Number(value)) => {
                if let Some(int_val) = value.as_i64() {
                    builder.append_value(int_val);
                } else {
                    return Err(property_type_error(
                        field,
                        "integer",
                        &geojson::JsonValue::Number(value.clone()),
                        source,
                    ));
                }
            },
            Some(geojson::JsonValue::Null) | None => builder.append_null(),
            Some(other) => return Err(property_type_error(field, "integer", other, source)),
        }
    }

    Ok(Arc::new(builder.finish()))
}

fn build_float64_array(
    field: &Field,
    records: &[FeatureRecord],
    source: &Arc<str>,
) -> Result<ArrayRef> {
    let mut builder = Float64Builder::with_capacity(records.len());

    for feature in records {
        match feature.properties.get(field.name()) {
            Some(geojson::JsonValue::Number(value)) => {
                if let Some(float_val) = value.as_f64() {
                    builder.append_value(float_val);
                } else {
                    return Err(property_type_error(
                        field,
                        "float",
                        &geojson::JsonValue::Number(value.clone()),
                        source,
                    ));
                }
            },
            Some(geojson::JsonValue::Null) | None => builder.append_null(),
            Some(other) => return Err(property_type_error(field, "float", other, source)),
        }
    }

    Ok(Arc::new(builder.finish()))
}

fn build_utf8_array(field: &Field, records: &[FeatureRecord]) -> ArrayRef {
    let mut builder = StringBuilder::with_capacity(records.len(), records.len() * 4);

    for feature in records {
        match feature.properties.get(field.name()) {
            Some(geojson::JsonValue::String(value)) => builder.append_value(value),
            Some(geojson::JsonValue::Number(value)) => builder.append_value(value.to_string()),
            Some(geojson::JsonValue::Bool(value)) => {
                builder.append_value(if *value { "true" } else { "false" });
            },
            Some(geojson::JsonValue::Array(value)) => {
                builder.append_value(geojson::JsonValue::Array(value.clone()).to_string());
            },
            Some(geojson::JsonValue::Object(value)) => {
                builder.append_value(geojson::JsonValue::Object(value.clone()).to_string());
            },
            Some(geojson::JsonValue::Null) | None => builder.append_null(),
        }
    }

    Arc::new(builder.finish())
}

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

fn property_type_error(
    field: &Field,
    expected: &str,
    actual: &geojson::JsonValue,
    source: &Arc<str>,
) -> DataFusionError {
    DataFusionError::from(SpatialFormatReadError::Parse {
        message: format!(
            "Property '{}' expected {expected}, but found {}",
            field.name(),
            describe_value(actual)
        ),
        position: None,
        context: Some(source.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::FeatureRecord;
    use arrow_array::Array;
    use geo_types::{Geometry, Point};
    use geoarrow_schema::{CoordType, GeometryType};

    fn make_feature_records() -> Vec<FeatureRecord> {
        vec![
            FeatureRecord {
                properties: [
                    ("bool_col".to_string(), geojson::JsonValue::Bool(true)),
                    ("int_col".to_string(), serde_json::json!(42)),
                    ("float_col".to_string(), serde_json::json!(2.5)),
                    (
                        "string_col".to_string(),
                        geojson::JsonValue::String("hello".into()),
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
                geometry: Some(Geometry::Point(Point::new(1.0, 2.0))),
            },
            FeatureRecord {
                properties: [
                    ("bool_col".to_string(), geojson::JsonValue::Null),
                    ("int_col".to_string(), geojson::JsonValue::Null),
                    ("float_col".to_string(), geojson::JsonValue::Null),
                    ("string_col".to_string(), geojson::JsonValue::Null),
                ]
                .iter()
                .cloned()
                .collect(),
                geometry: None,
            },
        ]
    }

    #[test]
    fn test_build_boolean_array() {
        let records = make_feature_records();
        let field = Field::new("bool_col", DataType::Boolean, true);
        let source = Arc::from("test");

        let array = build_boolean_array(&field, &records, &source).expect("build bool array");
        let bool_array = array
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();

        assert_eq!(bool_array.len(), 2);
        assert!(bool_array.value(0));
        assert!(bool_array.is_null(1));
    }

    #[test]
    fn test_build_boolean_array_type_error() {
        let records = vec![FeatureRecord {
            properties: [(
                "bool_col".to_string(),
                geojson::JsonValue::String("not a bool".into()),
            )]
            .iter()
            .cloned()
            .collect(),
            geometry: None,
        }];
        let field = Field::new("bool_col", DataType::Boolean, true);
        let source = Arc::from("test");

        let err = build_boolean_array(&field, &records, &source).unwrap_err();
        assert!(err.to_string().contains("expected bool"));
    }

    #[test]
    fn test_build_int64_array() {
        let records = make_feature_records();
        let field = Field::new("int_col", DataType::Int64, true);
        let source = Arc::from("test");

        let array = build_int64_array(&field, &records, &source).expect("build int64 array");
        let int_array = array
            .as_any()
            .downcast_ref::<arrow_array::Int64Array>()
            .unwrap();

        assert_eq!(int_array.len(), 2);
        assert_eq!(int_array.value(0), 42);
        assert!(int_array.is_null(1));
    }

    #[test]
    fn test_build_int64_array_type_error() {
        let records = vec![FeatureRecord {
            properties: [(
                "int_col".to_string(),
                geojson::JsonValue::String("not an int".into()),
            )]
            .iter()
            .cloned()
            .collect(),
            geometry: None,
        }];
        let field = Field::new("int_col", DataType::Int64, true);
        let source = Arc::from("test");

        let err = build_int64_array(&field, &records, &source).unwrap_err();
        assert!(err.to_string().contains("expected integer"));
    }

    #[test]
    fn test_build_int64_array_float_error() {
        let records = vec![FeatureRecord {
            properties: [("int_col".to_string(), serde_json::json!(2.5))]
                .iter()
                .cloned()
                .collect(),
            geometry: None,
        }];
        let field = Field::new("int_col", DataType::Int64, true);
        let source = Arc::from("test");

        let err = build_int64_array(&field, &records, &source).unwrap_err();
        assert!(err.to_string().contains("expected integer"));
    }

    #[test]
    fn test_build_float64_array() {
        let records = make_feature_records();
        let field = Field::new("float_col", DataType::Float64, true);
        let source = Arc::from("test");

        let array = build_float64_array(&field, &records, &source).expect("build float64 array");
        let float_array = array
            .as_any()
            .downcast_ref::<arrow_array::Float64Array>()
            .unwrap();

        assert_eq!(float_array.len(), 2);
        assert!((float_array.value(0) - 2.5).abs() < 1e-10);
        assert!(float_array.is_null(1));
    }

    #[test]
    fn test_build_float64_array_type_error() {
        let records = vec![FeatureRecord {
            properties: [(
                "float_col".to_string(),
                geojson::JsonValue::String("not a float".into()),
            )]
            .iter()
            .cloned()
            .collect(),
            geometry: None,
        }];
        let field = Field::new("float_col", DataType::Float64, true);
        let source = Arc::from("test");

        let err = build_float64_array(&field, &records, &source).unwrap_err();
        assert!(err.to_string().contains("expected float"));
    }

    #[test]
    fn test_build_utf8_array() {
        let records = make_feature_records();
        let field = Field::new("string_col", DataType::Utf8, true);

        let array = build_utf8_array(&field, &records);
        let string_array = array
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap();

        assert_eq!(string_array.len(), 2);
        assert_eq!(string_array.value(0), "hello");
        assert!(string_array.is_null(1));
    }

    #[test]
    fn test_build_utf8_array_with_number() {
        let records = vec![FeatureRecord {
            properties: [("string_col".to_string(), serde_json::json!(123))]
                .iter()
                .cloned()
                .collect(),
            geometry: None,
        }];
        let field = Field::new("string_col", DataType::Utf8, true);

        let array = build_utf8_array(&field, &records);
        let string_array = array
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap();

        assert_eq!(string_array.len(), 1);
        assert_eq!(string_array.value(0), "123");
    }

    #[test]
    fn test_build_utf8_array_with_bool() {
        let records = vec![
            FeatureRecord {
                properties: [("string_col".to_string(), geojson::JsonValue::Bool(true))]
                    .iter()
                    .cloned()
                    .collect(),
                geometry: None,
            },
            FeatureRecord {
                properties: [("string_col".to_string(), geojson::JsonValue::Bool(false))]
                    .iter()
                    .cloned()
                    .collect(),
                geometry: None,
            },
        ];
        let field = Field::new("string_col", DataType::Utf8, true);

        let array = build_utf8_array(&field, &records);
        let string_array = array
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap();

        assert_eq!(string_array.len(), 2);
        assert_eq!(string_array.value(0), "true");
        assert_eq!(string_array.value(1), "false");
    }

    #[test]
    fn test_build_utf8_array_with_array() {
        let records = vec![FeatureRecord {
            properties: [("string_col".to_string(), serde_json::json!([1, 2, 3]))]
                .iter()
                .cloned()
                .collect(),
            geometry: None,
        }];
        let field = Field::new("string_col", DataType::Utf8, true);

        let array = build_utf8_array(&field, &records);
        let string_array = array
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap();

        assert_eq!(string_array.len(), 1);
        assert!(string_array.value(0).contains('['));
    }

    #[test]
    fn test_build_utf8_array_with_object() {
        let records = vec![FeatureRecord {
            properties: [(
                "string_col".to_string(),
                serde_json::json!({"key": "value"}),
            )]
            .iter()
            .cloned()
            .collect(),
            geometry: None,
        }];
        let field = Field::new("string_col", DataType::Utf8, true);

        let array = build_utf8_array(&field, &records);
        let string_array = array
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap();

        assert_eq!(string_array.len(), 1);
        assert!(string_array.value(0).contains("key"));
    }

    #[test]
    fn test_build_geometry_array() {
        let records = make_feature_records();
        let options = GeoJsonFormatOptions::new()
            .with_geometry_column_name("geometry")
            .with_geometry_type(
                GeometryType::new(Arc::default()).with_coord_type(CoordType::Interleaved),
            );
        let source = Arc::from("test");

        let array =
            build_geometry_array(&records, &options, &source).expect("build geometry array");
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_records_to_batch_empty_schema() {
        let records = make_feature_records();
        let schema = Arc::new(Schema::new(Vec::<Field>::new()));
        let options = GeoJsonFormatOptions::new();
        let source = Arc::from("test");

        let batch = records_to_batch(&schema, &options, &source, &records).expect("build batch");
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 0);
    }

    #[test]
    fn test_records_to_batch_with_columns() {
        let records = make_feature_records();
        let schema = Arc::new(Schema::new(vec![
            Field::new("bool_col", DataType::Boolean, true),
            Field::new("int_col", DataType::Int64, true),
            Field::new("float_col", DataType::Float64, true),
            Field::new("string_col", DataType::Utf8, true),
        ]));
        let options = GeoJsonFormatOptions::new();
        let source = Arc::from("test");

        let batch = records_to_batch(&schema, &options, &source, &records).expect("build batch");
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 4);
    }

    #[test]
    fn test_records_to_batch_unsupported_type() {
        let records = make_feature_records();
        let schema = Arc::new(Schema::new(vec![Field::new(
            "unsupported",
            DataType::Binary,
            true,
        )]));
        let options = GeoJsonFormatOptions::new();
        let source = Arc::from("test");

        let err = records_to_batch(&schema, &options, &source, &records).unwrap_err();
        assert!(err.to_string().contains("Unsupported data type"));
    }

    #[test]
    fn test_geojson_opener_with_batch_size() {
        let options = GeoJsonFormatOptions::new();
        let schema = Arc::new(Schema::new(vec![Field::new(
            "geometry",
            DataType::Binary,
            false,
        )]));
        let object_store = Arc::new(object_store::memory::InMemory::new());

        let opener = GeoJsonOpener::new(options, schema, None, object_store).with_batch_size(1024);

        assert_eq!(opener.batch_size, 1024);
    }
}
