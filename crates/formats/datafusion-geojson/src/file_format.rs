//! `GeoJSON` file format configuration and `DataFusion` integration.
#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema, SchemaRef};
use async_trait::async_trait;
use datafusion::datasource::file_format::FileFormat;
use datafusion::datasource::file_format::file_compression_type::FileCompressionType;
use datafusion::datasource::physical_plan::{FileScanConfig, FileSource};
use datafusion::error::Result;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_common::Statistics;
use datafusion_session::Session;
use datafusion_shared::SpatialFormatReadError;
use geoarrow_schema::{CoordType, GeometryType};
use object_store::{ObjectMeta, ObjectStore};

use crate::file_source::{GeoJsonExec, GeoJsonFileSource};
use crate::parser::{FeatureRecord, parse_geojson_bytes};

/// Options controlling `GeoJSON` reading behaviour.
#[derive(Debug, Clone)]
pub struct GeoJsonFormatOptions {
    /// Maximum number of features to sample for schema inference.
    pub schema_infer_max_features: Option<usize>,
    /// Target batch size when producing record batches.
    pub batch_size: usize,
    /// File extension to look for when listing datasets.
    pub file_extension: String,
    /// Name of the geometry column in the output schema.
    pub geometry_column_name: String,
    /// `GeoArrow` geometry type to emit.
    pub geometry_type: GeometryType,
}

impl Default for GeoJsonFormatOptions {
    fn default() -> Self {
        Self {
            schema_infer_max_features: Some(1024),
            batch_size: 8192,
            file_extension: ".geojson".to_string(),
            geometry_column_name: "geometry".to_string(),
            geometry_type: GeometryType::new(Arc::default())
                .with_coord_type(CoordType::Interleaved),
        }
    }
}

impl GeoJsonFormatOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_schema_infer_max_features(mut self, limit: Option<usize>) -> Self {
        self.schema_infer_max_features = limit;
        self
    }

    #[must_use]
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    #[must_use]
    pub fn with_file_extension(mut self, extension: impl Into<String>) -> Self {
        self.file_extension = extension.into();
        self
    }

    #[must_use]
    pub fn with_geometry_column_name(mut self, name: impl Into<String>) -> Self {
        self.geometry_column_name = name.into();
        self
    }

    #[must_use]
    pub fn with_geometry_type(mut self, geometry_type: GeometryType) -> Self {
        self.geometry_type = geometry_type;
        self
    }

    pub(crate) fn file_extension_with_dot(&self) -> String {
        if self.file_extension.starts_with('.') {
            self.file_extension.clone()
        } else {
            format!(".{}", self.file_extension)
        }
    }
}

/// `GeoJSON` [`FileFormat`] implementation for `DataFusion`.
#[derive(Debug, Clone)]
pub struct GeoJsonFormat {
    options: GeoJsonFormatOptions,
}

impl GeoJsonFormat {
    pub fn new(options: GeoJsonFormatOptions) -> Self {
        Self { options }
    }
}

impl Default for GeoJsonFormat {
    fn default() -> Self {
        Self::new(GeoJsonFormatOptions::default())
    }
}

#[async_trait]
impl FileFormat for GeoJsonFormat {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_ext(&self) -> String {
        self.options.file_extension_with_dot()
    }

    fn get_ext_with_compression(&self, _c: &FileCompressionType) -> Result<String> {
        Ok(self.get_ext())
    }

    fn compression_type(&self) -> Option<FileCompressionType> {
        None
    }

    async fn infer_schema(
        &self,
        _state: &dyn Session,
        store: &Arc<dyn ObjectStore>,
        objects: &[ObjectMeta],
    ) -> Result<SchemaRef> {
        if objects.is_empty() {
            return Ok(Arc::new(Schema::empty()));
        }

        let object = &objects[0];
        let location = object.location.clone();

        let bytes = store
            .get(&object.location)
            .await
            .map_err(|err| {
                datafusion::error::DataFusionError::from(SpatialFormatReadError::Io {
                    source: std::io::Error::other(err),
                    context: Some(location.to_string()),
                })
            })?
            .bytes()
            .await
            .map_err(|err| {
                datafusion::error::DataFusionError::from(SpatialFormatReadError::Io {
                    source: std::io::Error::other(err),
                    context: Some(location.to_string()),
                })
            })?;

        let records = parse_geojson_bytes(
            &bytes,
            self.options.schema_infer_max_features,
            location.to_string(),
        )
        .map_err(datafusion::error::DataFusionError::from)?;

        let schema = infer_schema_from_records(&records, &self.options);

        Ok(Arc::new(schema))
    }

    async fn infer_stats(
        &self,
        _state: &dyn Session,
        _store: &Arc<dyn ObjectStore>,
        table_schema: SchemaRef,
        _object: &ObjectMeta,
    ) -> Result<Statistics> {
        Ok(Statistics::new_unknown(&table_schema))
    }

    async fn create_physical_plan(
        &self,
        _state: &dyn Session,
        conf: FileScanConfig,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let exec = GeoJsonExec::new(conf);
        Ok(Arc::new(exec))
    }

    fn file_source(&self) -> Arc<dyn FileSource> {
        Arc::new(GeoJsonFileSource::new(self.options.clone()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferredScalarType {
    Null,
    Boolean,
    Int64,
    Float64,
    Utf8,
}

impl InferredScalarType {
    fn update(self, value: &geojson::JsonValue) -> Self {
        use geojson::JsonValue;
        match value {
            JsonValue::Null => self,
            JsonValue::Bool(_) => match self {
                Self::Null | Self::Boolean => Self::Boolean,
                _ => Self::Utf8,
            },
            JsonValue::Number(n) => {
                let is_int = n.is_i64();
                match self {
                    Self::Null | Self::Int64 => {
                        if is_int {
                            Self::Int64
                        } else {
                            Self::Float64
                        }
                    },
                    Self::Float64 => Self::Float64,
                    _ => Self::Utf8,
                }
            },
            JsonValue::String(_) | JsonValue::Array(_) | JsonValue::Object(_) => Self::Utf8,
        }
    }

    fn to_datatype(self) -> DataType {
        match self {
            Self::Null | Self::Utf8 => DataType::Utf8,
            Self::Boolean => DataType::Boolean,
            Self::Int64 => DataType::Int64,
            Self::Float64 => DataType::Float64,
        }
    }
}

fn infer_schema_from_records(records: &[FeatureRecord], options: &GeoJsonFormatOptions) -> Schema {
    let mut inferred: BTreeMap<String, InferredScalarType> = BTreeMap::new();

    for record in records {
        for (key, value) in &record.properties {
            let entry = inferred
                .entry(key.clone())
                .or_insert(InferredScalarType::Null);
            *entry = entry.update(value);
        }
    }

    let mut fields: Vec<Field> = inferred
        .into_iter()
        .map(|(name, ty)| Field::new(name, ty.to_datatype(), true))
        .collect();

    let geometry_field = options
        .geometry_type
        .to_field(options.geometry_column_name.clone(), true);
    fields.push(geometry_field);

    Schema::new(fields)
}

/// Helper to detect file extensions from a provided path.
pub(crate) fn detect_file_extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::FeatureRecord;
    use geojson::{JsonObject, JsonValue};
    use serde_json::Number;

    #[test]
    fn options_helpers() {
        let options = GeoJsonFormatOptions::new()
            .with_batch_size(256)
            .with_file_extension("jsonl")
            .with_geometry_column_name("geom");

        assert_eq!(options.batch_size, 256);
        assert_eq!(options.file_extension_with_dot(), ".jsonl");
        assert_eq!(options.geometry_column_name, "geom");
    }

    #[test]
    fn infer_schema_from_properties() {
        let mut props_a = JsonObject::new();
        props_a.insert("name".to_string(), JsonValue::String("A".to_string()));
        props_a.insert("value".to_string(), JsonValue::Number(1.into()));

        let mut props_b = JsonObject::new();
        props_b.insert("name".to_string(), JsonValue::String("B".to_string()));
        props_b.insert("active".to_string(), JsonValue::Bool(true));
        props_b.insert(
            "value".to_string(),
            JsonValue::Number(Number::from_f64(1.5).unwrap()),
        );

        let records = vec![
            FeatureRecord {
                properties: props_a,
                geometry: None,
            },
            FeatureRecord {
                properties: props_b,
                geometry: None,
            },
        ];

        let schema = infer_schema_from_records(&records, &GeoJsonFormatOptions::default());

        assert_eq!(schema.fields().len(), 4);
        assert_eq!(schema.field(0).name(), "active");
        assert_eq!(schema.field(0).data_type(), &DataType::Boolean);
        assert_eq!(schema.field(1).name(), "name");
        assert_eq!(schema.field(2).data_type(), &DataType::Float64);
        assert_eq!(schema.field(3).name(), "geometry");
    }
}
