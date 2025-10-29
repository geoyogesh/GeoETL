//! Physical execution for CSV reading
//!
//! This module implements the core CSV reading and parsing logic,
//! converting CSV data directly to Arrow `RecordBatches`.

use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;

use std::collections::HashMap;

use arrow_array::{ArrayRef, RecordBatch, RecordBatchOptions, StringArray};
use arrow_csv::reader::Format;
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use csv_async::{AsyncReaderBuilder, StringRecord as AsyncStringRecord};
use datafusion::datasource::listing::PartitionedFile;
use datafusion::datasource::physical_plan::{FileMeta, FileOpenFuture, FileOpener};
use datafusion::error::{DataFusionError, Result};
use datafusion_shared::{SourcePosition, SpatialFormatReadError};
use futures::{Stream, StreamExt, TryStreamExt};
use object_store::ObjectStore;
use tokio_util::io::StreamReader;

use crate::file_format::{CsvFormatOptions, GeometryColumnOptions};
use crate::geospatial;

/// CSV file opener that implements the `FileOpener` trait
#[derive(Clone)]
pub struct CsvOpener {
    /// CSV format options
    options: CsvFormatOptions,
    /// Schema for the CSV file
    schema: SchemaRef,
    /// Column projection (indices of columns to read)
    projection: Option<Vec<usize>>,
    /// Batch size for reading
    batch_size: usize,
    /// Object store for reading files
    object_store: Arc<dyn ObjectStore>,
}

impl CsvOpener {
    pub fn new(
        options: CsvFormatOptions,
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

impl FileOpener for CsvOpener {
    fn open(&self, file_meta: FileMeta, _file: PartitionedFile) -> Result<FileOpenFuture> {
        let opener = self.clone();
        let object_store = Arc::clone(&self.object_store);

        Ok(Box::pin(async move {
            let location = file_meta.location();
            let source_path: Arc<str> = Arc::from(location.to_string());
            let get_result = object_store.get(location).await.map_err(|e| {
                DataFusionError::from(SpatialFormatReadError::Io {
                    source: std::io::Error::other(e),
                    context: Some(source_path.to_string()),
                })
            })?;

            let byte_stream = get_result
                .into_stream()
                .map(|result| result.map_err(std::io::Error::other));
            let reader = StreamReader::new(byte_stream);

            let mut builder = AsyncReaderBuilder::new();
            builder
                .delimiter(opener.options.delimiter)
                .has_headers(opener.options.has_header);

            let record_stream = builder.create_reader(reader).into_records();
            let record_stream: BoxedCsvRecordStream = Box::pin(record_stream);

            let output_schema = if let Some(ref proj) = opener.projection {
                let fields: Vec<Field> = proj
                    .iter()
                    .map(|i| opener.schema.field(*i).clone())
                    .collect();
                Arc::new(Schema::new(fields))
            } else {
                opener.schema.clone()
            };

            let batch_size = opener.batch_size;
            let state = CsvReadState {
                records: record_stream,
                schema: output_schema,
                record_buffer: Vec::with_capacity(batch_size),
                opener,
                source: Arc::clone(&source_path),
            };

            let stream = futures::stream::try_unfold(state, |mut state| async move {
                state.record_buffer.clear();

                while state.record_buffer.len() < state.opener.batch_size {
                    match state.records.as_mut().next().await {
                        Some(Ok(record)) => state.record_buffer.push(record),
                        Some(Err(err)) => {
                            return Err(DataFusionError::from(csv_error_to_spatial(
                                &err,
                                &state.source,
                            )));
                        },
                        None => break,
                    }
                }

                if state.record_buffer.is_empty() {
                    Ok(None)
                } else {
                    let batch = records_to_batch(
                        &state.schema,
                        &state.opener,
                        &state.source,
                        &state.record_buffer,
                    )?;
                    Ok(Some((batch, state)))
                }
            })
            .into_stream();

            Ok(Box::pin(stream) as _)
        }))
    }
}

type BoxedCsvRecordStream = Pin<
    Box<
        dyn Stream<Item = std::result::Result<AsyncStringRecord, csv_async::Error>>
            + Send
            + 'static,
    >,
>;

struct CsvReadState {
    records: BoxedCsvRecordStream,
    schema: SchemaRef,
    opener: CsvOpener,
    record_buffer: Vec<AsyncStringRecord>,
    source: Arc<str>,
}

fn records_to_batch(
    schema: &SchemaRef,
    opener: &CsvOpener,
    source: &Arc<str>,
    records: &[AsyncStringRecord],
) -> Result<RecordBatch> {
    if records.is_empty() {
        return Err(DataFusionError::from(SpatialFormatReadError::Other {
            message: format!("No records to convert while reading {source}"),
        }));
    }

    let column_indices: Vec<usize> = if let Some(proj) = &opener.projection {
        proj.clone()
    } else {
        (0..opener.schema.fields().len()).collect()
    };

    if column_indices.is_empty() {
        return RecordBatch::try_new_with_options(
            schema.clone(),
            vec![],
            &RecordBatchOptions::new().with_row_count(Some(records.len())),
        )
        .map_err(|e| {
            DataFusionError::from(SpatialFormatReadError::Parse {
                message: format!("Failed to create empty RecordBatch: {e}"),
                position: None,
                context: Some(source.to_string()),
            })
        });
    }

    let geometry_lookup: HashMap<&str, &GeometryColumnOptions> = opener
        .options
        .geometry_columns
        .iter()
        .map(|geom| (geom.field_name.as_str(), geom))
        .collect();

    let mut columns: Vec<ArrayRef> = Vec::with_capacity(column_indices.len());

    for &actual_idx in &column_indices {
        let field = opener.schema.field(actual_idx);

        if let Some(geometry) = geometry_lookup.get(field.name().as_str()) {
            let array = geospatial::build_geometry_column(geometry, actual_idx, records)?;
            columns.push(array);
            continue;
        }

        let column_data: Vec<Option<&str>> = records
            .iter()
            .map(|record| record.get(actual_idx))
            .collect();

        let array = build_array(field, &column_data);
        columns.push(array);
    }

    RecordBatch::try_new(schema.clone(), columns).map_err(|e| {
        DataFusionError::from(SpatialFormatReadError::Parse {
            message: format!("Failed to create RecordBatch: {e}"),
            position: None,
            context: Some(source.to_string()),
        })
    })
}

fn csv_error_to_spatial(err: &csv_async::Error, source: &Arc<str>) -> SpatialFormatReadError {
    let mut position = SourcePosition::default();

    if let Some(csv_pos) = err.position() {
        position.line = Some(csv_pos.line());
        position.byte_offset = Some(csv_pos.byte());
        position.record = Some(csv_pos.record());
    }

    if let Some(field) = csv_error_field(err) {
        position.field = Some(field);
        position.column = Some(field);
    }

    let position = if position.is_empty() {
        None
    } else {
        Some(position)
    };

    SpatialFormatReadError::Parse {
        message: err.to_string(),
        position,
        context: Some(source.to_string()),
    }
}

fn csv_error_field(err: &csv_async::Error) -> Option<u64> {
    match err.kind() {
        csv_async::ErrorKind::Utf8 { err, .. } => Some((err.field() as u64) + 1),
        _ => None,
    }
}

fn build_array(field: &Field, data: &[Option<&str>]) -> ArrayRef {
    match field.data_type() {
        DataType::Utf8 => {
            let array: StringArray = data.iter().copied().collect();
            Arc::new(array)
        },
        DataType::Int64 => {
            use arrow_array::Int64Array;
            let array: Int64Array = data
                .iter()
                .map(|v| v.and_then(|s| s.parse::<i64>().ok()))
                .collect();
            Arc::new(array)
        },
        DataType::Float64 => {
            use arrow_array::Float64Array;
            let array: Float64Array = data
                .iter()
                .map(|v| v.and_then(|s| s.parse::<f64>().ok()))
                .collect();
            Arc::new(array)
        },
        DataType::Boolean => {
            use arrow_array::BooleanArray;
            let array: BooleanArray = data
                .iter()
                .map(|v| v.and_then(|s| s.parse::<bool>().ok()))
                .collect();
            Arc::new(array)
        },
        _ => {
            // Default to string for unsupported types
            let array: StringArray = data.iter().copied().collect();
            Arc::new(array)
        },
    }
}

/// Infer schema from CSV file with type detection
pub fn infer_schema(bytes: &[u8], options: &CsvFormatOptions) -> Result<Schema> {
    let format = Format::default()
        .with_header(options.has_header)
        .with_delimiter(options.delimiter);

    let (inferred_schema, _) = format
        .infer_schema(Cursor::new(bytes), options.schema_infer_max_rec)
        .map_err(|e| {
            DataFusionError::from(SpatialFormatReadError::SchemaInference {
                message: format!("Failed to infer schema: {e}"),
                context: None,
            })
        })?;

    if inferred_schema.fields().is_empty() {
        return Err(DataFusionError::from(
            SpatialFormatReadError::SchemaInference {
                message: "Cannot infer schema from empty file".to_string(),
                context: None,
            },
        ));
    }

    let schema = sanitize_schema_types(&inferred_schema);
    let schema = apply_geometry_overrides(schema, options)?;

    if options.has_header {
        Ok(schema)
    } else {
        Ok(rename_fields_without_header(&schema))
    }
}

fn sanitize_schema_types(schema: &Schema) -> Schema {
    let metadata = schema.metadata().clone();
    let fields: Vec<Field> = schema
        .fields()
        .iter()
        .map(|field_ref| {
            let field = field_ref.as_ref().clone();
            let adjusted_type = match field.data_type() {
                DataType::Boolean => DataType::Boolean,
                DataType::Float64 | DataType::Float32 => DataType::Float64,
                DataType::Int64
                | DataType::Int32
                | DataType::Int16
                | DataType::Int8
                | DataType::UInt64
                | DataType::UInt32
                | DataType::UInt16
                | DataType::UInt8 => DataType::Int64,
                _ => DataType::Utf8,
            };

            if adjusted_type == *field.data_type() {
                field
            } else {
                field.with_data_type(adjusted_type)
            }
        })
        .collect();

    Schema::new_with_metadata(fields, metadata)
}

fn rename_fields_without_header(schema: &Schema) -> Schema {
    let metadata = schema.metadata().clone();
    let fields: Vec<Field> = schema
        .fields()
        .iter()
        .enumerate()
        .map(|(idx, field)| field.as_ref().clone().with_name(format!("column_{idx}")))
        .collect();
    Schema::new_with_metadata(fields, metadata)
}

fn apply_geometry_overrides(schema: Schema, options: &CsvFormatOptions) -> Result<Schema> {
    if options.geometry_columns.is_empty() {
        return Ok(schema);
    }

    let mut fields: Vec<Arc<Field>> = schema.fields().iter().cloned().collect();
    for geometry in &options.geometry_columns {
        let position = fields
            .iter()
            .position(|field| field.name() == &geometry.field_name)
            .ok_or_else(|| {
                DataFusionError::from(SpatialFormatReadError::SchemaInference {
                    message: format!(
                        "Geometry column '{}' was not found in the inferred schema",
                        geometry.field_name
                    ),
                    context: Some("geometry override".to_string()),
                })
            })?;

        let nullable = fields[position].is_nullable();
        fields[position] = Arc::new(
            geometry
                .geoarrow_type
                .to_field(&geometry.field_name, nullable),
        );
    }

    Ok(Schema::new_with_metadata(fields, schema.metadata().clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_schema() {
        let csv_data = b"name,age,city\nAlice,30,NYC\nBob,25,LA";
        let options = CsvFormatOptions::default();

        let schema = infer_schema(csv_data, &options).unwrap();

        assert_eq!(schema.fields().len(), 3);
        assert_eq!(schema.field(0).name(), "name");
        assert_eq!(schema.field(1).name(), "age");
        assert_eq!(schema.field(2).name(), "city");
    }
}
