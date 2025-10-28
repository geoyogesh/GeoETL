//! `GeoJSON` writer implementation for converting Arrow record batches to `GeoJSON` format

use std::io::Write as IoWrite;

use arrow_array::{Array, RecordBatch};
use arrow_schema::DataType;
use datafusion_common::{DataFusionError, Result};
use geoarrow_array::{GeoArrowArray, GeoArrowArrayAccessor};
use geojson::{Feature, FeatureCollection, GeoJson, JsonObject, JsonValue};
use geozero::ToJson;

/// Options for `GeoJSON` writing
#[derive(Debug, Clone)]
pub struct GeoJsonWriterOptions {
    /// Name of the geometry column (default: "geometry")
    pub geometry_column_name: String,
    /// Write as `FeatureCollection` (default: true)
    /// If false, writes as newline-delimited `GeoJSON` features
    pub feature_collection: bool,
    /// Pretty-print JSON output (default: false)
    pub pretty_print: bool,
}

impl Default for GeoJsonWriterOptions {
    fn default() -> Self {
        Self {
            geometry_column_name: "geometry".to_string(),
            feature_collection: true,
            pretty_print: false,
        }
    }
}

impl GeoJsonWriterOptions {
    /// Create new writer options with defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set geometry column name
    #[must_use]
    pub fn with_geometry_column(mut self, name: impl Into<String>) -> Self {
        self.geometry_column_name = name.into();
        self
    }

    /// Set whether to write as `FeatureCollection`
    #[must_use]
    pub fn with_feature_collection(mut self, feature_collection: bool) -> Self {
        self.feature_collection = feature_collection;
        self
    }

    /// Set whether to pretty-print JSON
    #[must_use]
    pub fn with_pretty_print(mut self, pretty_print: bool) -> Self {
        self.pretty_print = pretty_print;
        self
    }
}

/// Convert `GeoArrow` geometry to `GeoJSON` geometry using geozero
fn geoarrow_to_geojson_geometry(
    geom_array: &dyn Array,
    geom_field: &arrow_schema::Field,
    row_idx: usize,
) -> Result<Option<geojson::Geometry>> {
    use geoarrow_array::array::GeometryArray;

    // Try to convert from Arrow array to GeometryArray (supports all geometry types)
    let geometry_array_result = GeometryArray::try_from((geom_array, geom_field));

    match geometry_array_result {
        Ok(geom_arr) => {
            // Check if the value at this row is null
            if geom_arr.is_null(row_idx) {
                return Ok(None);
            }

            // Get the geometry scalar at this row index using GeoArrowArrayAccessor
            let geom = geom_arr
                .value(row_idx)
                .map_err(|e| DataFusionError::External(Box::new(e)))?;

            // Convert to GeoJSON string using geozero's ToJson trait
            let geojson_string = geom
                .to_json()
                .map_err(|e| DataFusionError::External(Box::new(e)))?;

            // Parse the GeoJSON string into a geojson::Geometry
            let geometry: geojson::Geometry = serde_json::from_str(&geojson_string)
                .map_err(|e| DataFusionError::External(Box::new(e)))?;

            Ok(Some(geometry))
        },
        Err(e) => {
            // Log the error for debugging
            eprintln!("DEBUG: Failed to convert to GeometryArray: {e:?}");
            // Not a GeoArrow geometry column
            Ok(None)
        },
    }
}

/// Convert Arrow value to JSON value
fn arrow_value_to_json(array: &dyn Array, row: usize) -> Result<JsonValue> {
    if array.is_null(row) {
        return Ok(JsonValue::Null);
    }

    match array.data_type() {
        DataType::Boolean => {
            let arr = array
                .as_any()
                .downcast_ref::<arrow_array::BooleanArray>()
                .ok_or_else(|| {
                    DataFusionError::Internal("Failed to downcast to BooleanArray".to_string())
                })?;
            Ok(JsonValue::Bool(arr.value(row)))
        },
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => {
            let arr = array
                .as_any()
                .downcast_ref::<arrow_array::Int64Array>()
                .or_else(|| {
                    array
                        .as_any()
                        .downcast_ref::<arrow_array::Int32Array>()
                        .map(|_| None::<&arrow_array::Int64Array>);
                    None
                });

            if let Some(arr) = arr {
                Ok(JsonValue::Number(arr.value(row).into()))
            } else {
                // Handle Int8, Int16, Int32
                let arr32 = array
                    .as_any()
                    .downcast_ref::<arrow_array::Int32Array>()
                    .ok_or_else(|| {
                        DataFusionError::Internal("Failed to downcast to Int32Array".to_string())
                    })?;
                Ok(JsonValue::Number(i64::from(arr32.value(row)).into()))
            }
        },
        DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => {
            #[allow(clippy::cast_possible_wrap)]
            if let Some(arr) = array.as_any().downcast_ref::<arrow_array::UInt64Array>() {
                Ok(JsonValue::Number((arr.value(row) as i64).into()))
            } else if let Some(arr) = array.as_any().downcast_ref::<arrow_array::UInt32Array>() {
                Ok(JsonValue::Number(i64::from(arr.value(row)).into()))
            } else {
                Err(DataFusionError::Internal(
                    "Failed to downcast unsigned integer array".to_string(),
                ))
            }
        },
        DataType::Float32 | DataType::Float64 => {
            let arr = array
                .as_any()
                .downcast_ref::<arrow_array::Float64Array>()
                .ok_or_else(|| {
                    DataFusionError::Internal("Failed to downcast to Float64Array".to_string())
                })?;
            let val = arr.value(row);
            Ok(serde_json::Number::from_f64(val).map_or(JsonValue::Null, JsonValue::Number))
        },
        DataType::Utf8 | DataType::LargeUtf8 => {
            let arr = array
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .ok_or_else(|| {
                    DataFusionError::Internal("Failed to downcast to StringArray".to_string())
                })?;
            Ok(JsonValue::String(arr.value(row).to_string()))
        },
        _ => Ok(JsonValue::String(format!("{array:?}"))),
    }
}

/// Convert a record batch to `GeoJSON` features
///
/// # Errors
///
/// Returns an error if the geometry column is not found or if type conversion fails
pub fn batch_to_features(
    batch: &RecordBatch,
    options: &GeoJsonWriterOptions,
) -> Result<Vec<Feature>> {
    let schema = batch.schema();
    let num_rows = batch.num_rows();

    // Find geometry column index
    let geom_idx = schema
        .fields()
        .iter()
        .position(|f| f.name() == &options.geometry_column_name)
        .ok_or_else(|| {
            DataFusionError::Plan(format!(
                "Geometry column '{}' not found in schema",
                options.geometry_column_name
            ))
        })?;

    let mut features = Vec::with_capacity(num_rows);

    for row_idx in 0..num_rows {
        let mut properties = JsonObject::new();

        // Extract properties (all columns except geometry)
        for (col_idx, field) in schema.fields().iter().enumerate() {
            if col_idx == geom_idx {
                continue; // Skip geometry column for properties
            }

            let column = batch.column(col_idx);
            let value = arrow_value_to_json(column.as_ref(), row_idx)?;
            properties.insert(field.name().clone(), value);
        }

        // Extract geometry - use geoarrow_to_geojson_geometry helper
        let geom_column = batch.column(geom_idx);
        let geom_field = schema.field(geom_idx);
        let geometry = geoarrow_to_geojson_geometry(geom_column.as_ref(), geom_field, row_idx)?;

        let feature = Feature {
            bbox: None,
            geometry,
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };

        features.push(feature);
    }

    Ok(features)
}

/// Write record batches to `GeoJSON` format
///
/// # Errors
///
/// Returns an error if writing to the output fails or if `GeoJSON` serialization fails
pub fn write_geojson<W: IoWrite>(
    writer: &mut W,
    batches: &[RecordBatch],
    options: &GeoJsonWriterOptions,
) -> Result<()> {
    if batches.is_empty() {
        return Ok(());
    }

    let mut all_features = Vec::new();

    for batch in batches {
        let features = batch_to_features(batch, options)?;
        all_features.extend(features);
    }

    if options.feature_collection {
        let collection = FeatureCollection {
            bbox: None,
            features: all_features,
            foreign_members: None,
        };

        let geojson = GeoJson::FeatureCollection(collection);
        let json_str = if options.pretty_print {
            serde_json::to_string_pretty(&geojson)
        } else {
            serde_json::to_string(&geojson)
        }
        .map_err(|e| DataFusionError::External(Box::new(e)))?;

        writer
            .write_all(json_str.as_bytes())
            .map_err(|e| DataFusionError::External(Box::new(e)))?;
    } else {
        // Newline-delimited GeoJSON
        for feature in all_features {
            let geojson = GeoJson::Feature(feature);
            let json_str = serde_json::to_string(&geojson)
                .map_err(|e| DataFusionError::External(Box::new(e)))?;

            writer
                .write_all(json_str.as_bytes())
                .map_err(|e| DataFusionError::External(Box::new(e)))?;
            writer
                .write_all(b"\n")
                .map_err(|e| DataFusionError::External(Box::new(e)))?;
        }
    }

    Ok(())
}

/// Write record batches to `GeoJSON` bytes
///
/// # Errors
///
/// Returns an error if `GeoJSON` serialization fails
pub fn write_geojson_to_bytes(
    batches: &[RecordBatch],
    options: &GeoJsonWriterOptions,
) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    write_geojson(&mut buffer, batches, options)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{ArrayRef, Int64Array, StringArray};
    use arrow_schema::{Field, Schema};
    use std::sync::Arc;

    fn create_test_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("geometry", DataType::Utf8, true), // Placeholder for geometry
        ]));

        let id_array: ArrayRef = Arc::new(Int64Array::from(vec![1, 2, 3]));
        let name_array: ArrayRef =
            Arc::new(StringArray::from(vec![Some("Alice"), Some("Bob"), None]));
        let geom_array: ArrayRef = Arc::new(StringArray::from(vec![
            Some("POINT(0 0)"),
            Some("POINT(1 1)"),
            Some("POINT(2 2)"),
        ]));

        RecordBatch::try_new(schema, vec![id_array, name_array, geom_array]).unwrap()
    }

    #[test]
    fn test_write_feature_collection() {
        let batch = create_test_batch();
        let options = GeoJsonWriterOptions::default();

        let result = write_geojson_to_bytes(&[batch], &options).unwrap();
        let json_str = String::from_utf8(result).unwrap();

        assert!(json_str.contains("\"type\":\"FeatureCollection\""));
        assert!(json_str.contains("\"features\""));
    }

    #[test]
    fn test_write_newline_delimited() {
        let batch = create_test_batch();
        let options = GeoJsonWriterOptions::default().with_feature_collection(false);

        let result = write_geojson_to_bytes(&[batch], &options).unwrap();
        let json_str = String::from_utf8(result).unwrap();

        let lines: Vec<&str> = json_str.lines().collect();
        assert_eq!(lines.len(), 3); // 3 features
        assert!(lines[0].contains("\"type\":\"Feature\""));
    }

    #[test]
    fn test_empty_batches() {
        let batches: Vec<RecordBatch> = vec![];
        let options = GeoJsonWriterOptions::default();

        let result = write_geojson_to_bytes(&batches, &options).unwrap();
        assert!(result.is_empty());
    }
}
