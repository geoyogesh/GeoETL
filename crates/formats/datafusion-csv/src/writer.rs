//! CSV writer implementation for converting Arrow record batches to CSV format

use std::io::Write as IoWrite;

use arrow_array::RecordBatch;
use arrow_csv::WriterBuilder;
use datafusion_common::{DataFusionError, Result};

/// Options for CSV writing
#[derive(Debug, Clone)]
pub struct CsvWriterOptions {
    /// Column delimiter (default: b',')
    pub delimiter: u8,
    /// Whether to write header row (default: true)
    pub has_header: bool,
    /// Date format string (default: None)
    pub date_format: Option<String>,
    /// Datetime format string (default: None)
    pub datetime_format: Option<String>,
    /// Timestamp format string (default: None)
    pub timestamp_format: Option<String>,
    /// Time format string (default: None)
    pub time_format: Option<String>,
    /// Null value representation (default: empty string)
    pub null_value: String,
}

impl Default for CsvWriterOptions {
    fn default() -> Self {
        Self {
            delimiter: b',',
            has_header: true,
            date_format: None,
            datetime_format: None,
            timestamp_format: None,
            time_format: None,
            null_value: String::new(),
        }
    }
}

impl CsvWriterOptions {
    /// Create new writer options with defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set column delimiter
    #[must_use]
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Set whether to write header row
    #[must_use]
    pub fn with_header(mut self, has_header: bool) -> Self {
        self.has_header = has_header;
        self
    }

    /// Set date format string
    #[must_use]
    pub fn with_date_format(mut self, format: impl Into<String>) -> Self {
        self.date_format = Some(format.into());
        self
    }

    /// Set datetime format string
    #[must_use]
    pub fn with_datetime_format(mut self, format: impl Into<String>) -> Self {
        self.datetime_format = Some(format.into());
        self
    }

    /// Set timestamp format string
    #[must_use]
    pub fn with_timestamp_format(mut self, format: impl Into<String>) -> Self {
        self.timestamp_format = Some(format.into());
        self
    }

    /// Set time format string
    #[must_use]
    pub fn with_time_format(mut self, format: impl Into<String>) -> Self {
        self.time_format = Some(format.into());
        self
    }

    /// Set null value representation
    #[must_use]
    pub fn with_null_value(mut self, null_value: impl Into<String>) -> Self {
        self.null_value = null_value.into();
        self
    }
}

/// Write record batches to CSV format
///
/// # Errors
///
/// Returns an error if writing to the output fails or if CSV serialization fails
pub fn write_csv<W: IoWrite>(
    writer: &mut W,
    batches: &[RecordBatch],
    options: &CsvWriterOptions,
) -> Result<()> {
    if batches.is_empty() {
        return Ok(());
    }

    let mut builder = WriterBuilder::new()
        .with_delimiter(options.delimiter)
        .with_header(options.has_header);

    if let Some(ref format) = options.date_format {
        builder = builder.with_date_format(format.clone());
    }
    if let Some(ref format) = options.datetime_format {
        builder = builder.with_datetime_format(format.clone());
    }
    if let Some(ref format) = options.timestamp_format {
        builder = builder.with_timestamp_format(format.clone());
    }
    if let Some(ref format) = options.time_format {
        builder = builder.with_time_format(format.clone());
    }
    if !options.null_value.is_empty() {
        builder = builder.with_null(options.null_value.clone());
    }

    let mut csv_writer = builder.build(writer);

    for batch in batches {
        csv_writer
            .write(batch)
            .map_err(|e| DataFusionError::External(Box::new(e)))?;
    }

    Ok(())
}

/// Write record batches to CSV bytes
///
/// # Errors
///
/// Returns an error if CSV serialization fails
pub fn write_csv_to_bytes(batches: &[RecordBatch], options: &CsvWriterOptions) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    write_csv(&mut buffer, batches, options)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;

    fn create_test_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("value", DataType::Float64, true),
            Field::new("active", DataType::Boolean, true),
        ]));

        let id_array: ArrayRef = Arc::new(Int64Array::from(vec![1, 2, 3]));
        let name_array: ArrayRef =
            Arc::new(StringArray::from(vec![Some("Alice"), Some("Bob"), None]));
        let value_array: ArrayRef =
            Arc::new(Float64Array::from(vec![Some(10.5), None, Some(30.2)]));
        let active_array: ArrayRef = Arc::new(BooleanArray::from(vec![
            Some(true),
            Some(false),
            Some(true),
        ]));

        RecordBatch::try_new(
            schema,
            vec![id_array, name_array, value_array, active_array],
        )
        .unwrap()
    }

    #[test]
    fn test_write_csv_with_header() {
        let batch = create_test_batch();
        let options = CsvWriterOptions::default();

        let result = write_csv_to_bytes(&[batch], &options).unwrap();
        let csv_str = String::from_utf8(result).unwrap();

        assert!(csv_str.starts_with("id,name,value,active\n"));
        assert!(csv_str.contains("1,Alice,10.5,true"));
        assert!(csv_str.contains("2,Bob,,false"));
        assert!(csv_str.contains("3,,30.2,true"));
    }

    #[test]
    fn test_write_csv_without_header() {
        let batch = create_test_batch();
        let options = CsvWriterOptions::default().with_header(false);

        let result = write_csv_to_bytes(&[batch], &options).unwrap();
        let csv_str = String::from_utf8(result).unwrap();

        assert!(!csv_str.starts_with("id,name"));
        assert!(csv_str.starts_with("1,Alice,10.5,true"));
    }

    #[test]
    fn test_write_csv_custom_delimiter() {
        let batch = create_test_batch();
        let options = CsvWriterOptions::default().with_delimiter(b';');

        let result = write_csv_to_bytes(&[batch], &options).unwrap();
        let csv_str = String::from_utf8(result).unwrap();

        assert!(csv_str.starts_with("id;name;value;active\n"));
        assert!(csv_str.contains("1;Alice;10.5;true"));
    }

    #[test]
    fn test_write_csv_custom_null() {
        let batch = create_test_batch();
        let options = CsvWriterOptions::default().with_null_value("NULL");

        let result = write_csv_to_bytes(&[batch], &options).unwrap();
        let csv_str = String::from_utf8(result).unwrap();

        assert!(csv_str.contains("2,Bob,NULL,false"));
        assert!(csv_str.contains("3,NULL,30.2,true"));
    }

    #[test]
    fn test_write_empty_batches() {
        let batches: Vec<RecordBatch> = vec![];
        let options = CsvWriterOptions::default();

        let result = write_csv_to_bytes(&batches, &options).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_write_multiple_batches() {
        let batch1 = create_test_batch();
        let batch2 = create_test_batch();
        let options = CsvWriterOptions::default();

        let result = write_csv_to_bytes(&[batch1, batch2], &options).unwrap();
        let csv_str = String::from_utf8(result).unwrap();

        // Header should only appear once
        let header_count = csv_str.matches("id,name,value,active").count();
        assert_eq!(header_count, 1);

        // Should have 6 data rows (3 from each batch)
        let lines: Vec<&str> = csv_str.lines().collect();
        assert_eq!(lines.len(), 7); // 1 header + 6 data rows
    }
}
