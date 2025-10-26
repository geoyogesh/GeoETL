//! Physical execution for CSV reading
//!
//! This module implements the core CSV reading and parsing logic,
//! converting CSV data directly to Arrow `RecordBatches`.

use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;

use arrow::datatypes::SchemaRef;
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use arrow_array::{ArrayRef, StringArray};
use arrow_schema::{DataType, Field, Schema};
use csv_async::{AsyncReaderBuilder, StringRecord as AsyncStringRecord};
use datafusion::datasource::physical_plan::{FileMeta, FileOpenFuture, FileOpener};
use datafusion::error::{DataFusionError, Result};
use futures::{Stream, StreamExt, TryStreamExt};
use object_store::ObjectStore;
use tokio_util::io::StreamReader;

use crate::file_format::CsvFormatOptions;

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
    fn open(&self, file_meta: FileMeta) -> Result<FileOpenFuture> {
        let opener = self.clone();
        let object_store = self.object_store.clone();

        Ok(Box::pin(async move {
            let location = file_meta.location();
            let get_result = object_store
                .get(location)
                .await
                .map_err(|e| DataFusionError::Execution(format!("Failed to read file: {e}")))?;

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
            };

            let stream = futures::stream::try_unfold(state, |mut state| async move {
                state.record_buffer.clear();

                while state.record_buffer.len() < state.opener.batch_size {
                    match state.records.as_mut().next().await {
                        Some(Ok(record)) => state.record_buffer.push(record),
                        Some(Err(err)) => {
                            return Err(DataFusionError::Execution(format!(
                                "CSV parse error: {err}"
                            )));
                        },
                        None => break,
                    }
                }

                if state.record_buffer.is_empty() {
                    Ok(None)
                } else {
                    let batch =
                        records_to_batch(&state.schema, &state.opener, &state.record_buffer)?;
                    Ok(Some((batch, state)))
                }
            })
            .map_err(|e| ArrowError::ExternalError(Box::new(e)))
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
}

fn records_to_batch(
    schema: &SchemaRef,
    opener: &CsvOpener,
    records: &[AsyncStringRecord],
) -> Result<RecordBatch> {
    if records.is_empty() {
        return Err(DataFusionError::Execution(
            "No records to convert".to_string(),
        ));
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
            &arrow::record_batch::RecordBatchOptions::new().with_row_count(Some(records.len())),
        )
        .map_err(|e| {
            DataFusionError::Execution(format!("Failed to create empty RecordBatch: {e}"))
        });
    }

    let mut columns: Vec<ArrayRef> = Vec::with_capacity(column_indices.len());

    for &actual_idx in &column_indices {
        let field = opener.schema.field(actual_idx);
        let column_data: Vec<Option<&str>> = records
            .iter()
            .map(|record| record.get(actual_idx))
            .collect();

        let array = build_array(field, &column_data);
        columns.push(array);
    }

    RecordBatch::try_new(schema.clone(), columns)
        .map_err(|e| DataFusionError::Execution(format!("Failed to create RecordBatch: {e}")))
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
    let cursor = Cursor::new(bytes);
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(options.delimiter)
        .has_headers(options.has_header)
        .from_reader(cursor);

    let mut first_data_record: Option<csv::StringRecord> = None;

    let headers: Vec<String> = if options.has_header {
        reader
            .headers()
            .map_err(|e| DataFusionError::Execution(format!("Failed to read headers: {e}")))?
            .iter()
            .map(str::to_string)
            .collect()
    } else {
        // Generate column names if no header
        if let Some(Ok(record)) = reader.records().next() {
            let column_names = (0..record.len()).map(|i| format!("column_{i}")).collect();
            first_data_record = Some(record);
            column_names
        } else {
            return Err(DataFusionError::Execution(
                "Cannot infer schema from empty file".to_string(),
            ));
        }
    };

    // Sample records to infer types
    let max_records = options.schema_infer_max_rec.unwrap_or(1000);
    let mut sample_records: Vec<csv::StringRecord> = Vec::new();

    if let Some(record) = first_data_record.filter(|_| max_records > 0) {
        sample_records.push(record);
    }

    for result in reader.records() {
        if sample_records.len() >= max_records {
            break;
        }
        if let Ok(record) = result {
            sample_records.push(record);
        }
    }

    // Infer type for each column
    let num_columns = headers.len();
    let mut fields: Vec<Field> = Vec::with_capacity(num_columns);

    for (col_idx, name) in headers.into_iter().enumerate() {
        let data_type = infer_column_type(&sample_records, col_idx);
        fields.push(Field::new(name, data_type, true));
    }

    Ok(Schema::new(fields))
}

/// Infer the data type of a column by sampling values
fn infer_column_type(records: &[csv::StringRecord], col_idx: usize) -> DataType {
    let mut has_float = false;
    let mut has_int = false;
    let mut has_bool = false;
    let mut total_values = 0;

    for record in records.iter().take(100) {
        if let Some(value) = record.get(col_idx) {
            let value = value.trim();
            if value.is_empty() {
                continue;
            }

            total_values += 1;

            // Check if it's a boolean
            if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
                has_bool = true;
                continue;
            }

            // Check if it's a float
            if value.parse::<f64>().is_ok() {
                if value.contains('.') {
                    has_float = true;
                } else {
                    has_int = true;
                }
            }
        }
    }

    // Prioritize type inference: Bool > Float > Int > String
    if total_values == 0 {
        return DataType::Utf8;
    }

    if has_bool && !has_int && !has_float {
        DataType::Boolean
    } else if has_float {
        DataType::Float64
    } else if has_int {
        DataType::Int64
    } else {
        DataType::Utf8
    }
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
