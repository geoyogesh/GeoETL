//! Integration tests for CSV writer functionality

use std::fs;
use std::sync::Arc;

use arrow_array::{ArrayRef, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use datafusion::datasource::listing::ListingTableUrl;
use datafusion::datasource::physical_plan::{FileGroup, FileSinkConfig};
use datafusion::datasource::sink::DataSink;
use datafusion::logical_expr::dml::InsertOp;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion_csv::{CsvSink, CsvWriterOptions, write_csv_to_bytes};
use datafusion_execution::object_store::ObjectStoreUrl;
use datafusion_execution::{SendableRecordBatchStream, TaskContext};
use futures::stream;
use tempfile::TempDir;

#[ignore = "Requires proper object store integration"]
#[tokio::test]
async fn test_csv_sink_write_all() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap();

    // Pre-create the directory since sink expects it to exist
    fs::create_dir_all(output_path).unwrap();

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
    ]));

    // Create test data
    let id_array: ArrayRef = Arc::new(Int64Array::from(vec![1, 2, 3]));
    let name_array: ArrayRef = Arc::new(StringArray::from(vec![
        Some("Alice"),
        Some("Bob"),
        Some("Charlie"),
    ]));

    let batch = RecordBatch::try_new(schema.clone(), vec![id_array, name_array]).unwrap();

    // Create file sink config
    let config = FileSinkConfig {
        original_url: format!("file://{output_path}/output.csv"),
        object_store_url: ObjectStoreUrl::local_filesystem(),
        file_group: FileGroup::default(),
        table_paths: vec![ListingTableUrl::parse(format!("file://{output_path}")).unwrap()],
        output_schema: schema.clone(),
        table_partition_cols: vec![],
        insert_op: InsertOp::Append,
        keep_partition_by_columns: false,
        file_extension: "csv".to_string(),
    };

    let writer_options = CsvWriterOptions::default();
    let sink = CsvSink::new(config, writer_options);

    // Create a stream from the batch
    let stream: SendableRecordBatchStream = Box::pin(RecordBatchStreamAdapter::new(
        schema.clone(),
        stream::iter(vec![Ok(batch)]),
    ));

    // Write data
    let context = Arc::new(TaskContext::default());
    let row_count = sink.write_all(stream, &context).await.unwrap();

    assert_eq!(row_count, 3);

    // Verify file was created and contains data
    let file_path = format!("{output_path}/data.csv");
    assert!(fs::metadata(&file_path).is_ok());

    let contents = fs::read_to_string(&file_path).unwrap();
    assert!(contents.contains("id,name"));
    assert!(contents.contains("1,Alice"));
    assert!(contents.contains("2,Bob"));
    assert!(contents.contains("3,Charlie"));
}

#[test]
fn test_csv_writer_options_builder() {
    let options = CsvWriterOptions::new()
        .with_delimiter(b';')
        .with_header(false)
        .with_date_format("%Y-%m-%d")
        .with_null_value("NULL");

    assert_eq!(options.delimiter, b';');
    assert!(!options.has_header);
    assert_eq!(options.date_format, Some("%Y-%m-%d".to_string()));
    assert_eq!(options.null_value, "NULL");
}

#[test]
fn test_write_csv_with_various_delimiters() {
    let schema = Arc::new(Schema::new(vec![
        Field::new("a", DataType::Int64, false),
        Field::new("b", DataType::Int64, false),
    ]));

    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int64Array::from(vec![1, 2])) as ArrayRef,
            Arc::new(Int64Array::from(vec![3, 4])) as ArrayRef,
        ],
    )
    .unwrap();

    // Test semicolon delimiter
    let options = CsvWriterOptions::default().with_delimiter(b';');
    let result = write_csv_to_bytes(std::slice::from_ref(&batch), &options).unwrap();
    let csv_str = String::from_utf8(result).unwrap();
    assert!(csv_str.contains("a;b"));
    assert!(csv_str.contains("1;3"));

    // Test tab delimiter
    let options = CsvWriterOptions::default().with_delimiter(b'\t');
    let result = write_csv_to_bytes(std::slice::from_ref(&batch), &options).unwrap();
    let csv_str = String::from_utf8(result).unwrap();
    assert!(csv_str.contains("a\tb"));
}

#[test]
fn test_csv_sink_config_accessors() {
    let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));

    let config = FileSinkConfig {
        original_url: "file:///tmp/test.csv".to_string(),
        object_store_url: ObjectStoreUrl::local_filesystem(),
        file_group: FileGroup::default(),
        table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
        output_schema: schema.clone(),
        table_partition_cols: vec![],
        insert_op: InsertOp::Append,
        keep_partition_by_columns: false,
        file_extension: "csv".to_string(),
    };

    let writer_options = CsvWriterOptions::default();
    let sink = CsvSink::new(config.clone(), writer_options.clone());

    assert_eq!(sink.config().original_url, config.original_url);
    assert_eq!(sink.writer_options().delimiter, writer_options.delimiter);
}
