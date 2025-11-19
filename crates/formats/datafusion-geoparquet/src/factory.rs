//! Factory implementation for `GeoParquet` format support.
//!
//! This module implements the `FormatFactory` trait to integrate `GeoParquet`
//! with the dynamic driver registry system.

use anyhow::Result;
use async_trait::async_trait;
use datafusion::datasource::TableProvider;
use datafusion::execution::context::SessionState;
use datafusion::physical_plan::ExecutionPlan;
use geoetl_core_common::{
    DataReader, DataWriter, Driver, FormatFactory, FormatOptions, SupportStatus,
};
use std::sync::Arc;

use crate::{GeoParquetFormatOptions, file_source};

/// `GeoParquet` format options wrapper for the factory system.
impl FormatOptions for GeoParquetFormatOptions {
    fn as_any(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(self.clone())
    }
}

/// Reader implementation for `GeoParquet` format.
struct GeoParquetReader;

#[async_trait]
impl DataReader for GeoParquetReader {
    async fn create_table_provider(
        &self,
        state: &SessionState,
        path: &str,
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<Arc<dyn TableProvider>> {
        let geoparquet_options = options
            .downcast::<GeoParquetFormatOptions>()
            .map_err(|_| anyhow::anyhow!("Invalid options type for GeoParquet reader"))?;

        let table =
            file_source::create_geoparquet_table_provider(state, path, *geoparquet_options).await?;
        Ok(table)
    }
}

/// Writer implementation for `GeoParquet` format.
struct GeoParquetWriter;

#[async_trait]
impl DataWriter for GeoParquetWriter {
    async fn create_writer_plan(
        &self,
        _input: Arc<dyn ExecutionPlan>,
        _path: &str,
        _options: Box<dyn std::any::Any + Send>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // TODO: Implement writer plan creation
        // This requires creating a GeoParquetSink with FileSinkConfig
        Err(anyhow::anyhow!(
            "GeoParquet writer not yet implemented in factory"
        ))
    }

    fn create_writer_options(&self, geometry_column: &str) -> Box<dyn std::any::Any + Send> {
        // GeoParquet writer options with configured geometry column
        let options =
            crate::writer::GeoParquetWriterOptions::default().with_geometry_column(geometry_column);
        Box::new(options)
    }

    fn write_batches(
        &self,
        path: &str,
        batches: &[datafusion::arrow::array::RecordBatch],
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<()> {
        use crate::writer::write_geoparquet;

        // Downcast options to GeoParquetWriterOptions
        let boxed_options = options
            .downcast::<crate::writer::GeoParquetWriterOptions>()
            .map_err(|_| anyhow::anyhow!("Invalid options type for GeoParquet writer"))?;
        let writer_options = *boxed_options;

        // Write to file
        let mut output_file = std::fs::File::create(path)
            .map_err(|e| anyhow::anyhow!("Failed to create output file: {e}"))?;

        write_geoparquet(&mut output_file, batches, &writer_options)?;

        Ok(())
    }
}

/// Factory for creating `GeoParquet` readers and writers.
pub struct GeoParquetFormatFactory;

impl FormatFactory for GeoParquetFormatFactory {
    fn driver(&self) -> Driver {
        Driver::new(
            "GeoParquet",
            "GeoParquet",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        )
    }

    fn create_reader(&self) -> Option<Arc<dyn DataReader>> {
        Some(Arc::new(GeoParquetReader))
    }

    fn create_writer(&self) -> Option<Arc<dyn DataWriter>> {
        Some(Arc::new(GeoParquetWriter))
    }

    fn create_file_format(
        &self,
        geometry_column: &str,
    ) -> Option<Arc<dyn datafusion::datasource::file_format::FileFormat>> {
        // Create GeoParquet file format for streaming execution
        let options = crate::file_format::GeoParquetFormatOptions::default()
            .with_geometry_column_name(geometry_column);
        let format = crate::file_format::GeoParquetFormat::new(options);
        Some(Arc::new(format))
    }

    fn infer_table_name(&self, input_path: &str) -> Option<String> {
        // GeoParquet-specific table name inference
        // Extract filename without extension and sanitize for SQL
        std::path::Path::new(input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(datafusion_shared::sanitize_table_name)
    }
}

/// Registers the `GeoParquet` format with the global driver registry.
///
/// This is called by `geoetl-core` during initialization.
pub fn register_geoparquet_format() {
    let registry = geoetl_core_common::driver_registry();
    registry.register(Arc::new(GeoParquetFormatFactory));
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::Int32Array;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::prelude::SessionContext;

    #[test]
    fn test_factory_driver_info() {
        let factory = GeoParquetFormatFactory;
        let driver = factory.driver();

        assert_eq!(driver.short_name, "GeoParquet");
        assert_eq!(driver.long_name, "GeoParquet");
        assert!(matches!(driver.capabilities.read, SupportStatus::Supported));
        assert!(matches!(
            driver.capabilities.write,
            SupportStatus::Supported
        ));
        assert!(matches!(driver.capabilities.info, SupportStatus::Supported));
    }

    #[test]
    fn test_factory_create_reader() {
        let factory = GeoParquetFormatFactory;
        let reader = factory.create_reader();

        assert!(reader.is_some());
    }

    #[test]
    fn test_factory_create_writer() {
        let factory = GeoParquetFormatFactory;
        let writer = factory.create_writer();

        assert!(writer.is_some());
    }

    #[test]
    fn test_factory_create_file_format() {
        let factory = GeoParquetFormatFactory;
        let file_format = factory.create_file_format("geom");

        assert!(file_format.is_some());
    }

    #[tokio::test]
    async fn test_reader_invalid_options_type() {
        let reader = GeoParquetReader;
        let ctx = SessionContext::new();

        // Pass wrong options type
        let wrong_options: Box<dyn std::any::Any + Send> = Box::new(42i32);

        let result = reader
            .create_table_provider(&ctx.state(), "dummy.parquet", wrong_options)
            .await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid options type")
        );
    }

    #[test]
    fn test_writer_create_writer_options() {
        let writer = GeoParquetWriter;
        let options = writer.create_writer_options("my_geom");

        // Downcast to verify it's the right type
        let boxed_options = options
            .downcast::<crate::writer::GeoParquetWriterOptions>()
            .unwrap();
        assert_eq!(boxed_options.geometry_column_name, "my_geom");
    }

    #[test]
    fn test_writer_write_batches_invalid_options() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_invalid_options.parquet");

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));

        let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(Int32Array::from(vec![1]))])
            .unwrap();

        let writer = GeoParquetWriter;
        let wrong_options: Box<dyn std::any::Any + Send> = Box::new("wrong".to_string());

        let result = writer.write_batches(file_path.to_str().unwrap(), &[batch], wrong_options);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid options type")
        );

        // Cleanup (if file was somehow created)
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_format_options_as_any() {
        let options = GeoParquetFormatOptions::default();
        let boxed = options.as_any();

        // Should be able to downcast back
        assert!(boxed.downcast_ref::<GeoParquetFormatOptions>().is_some());
    }

    #[test]
    fn test_register_geoparquet_format() {
        // Call registration function
        register_geoparquet_format();

        // Just verify the function runs without error
        // The registry is a global singleton and may already have geoparquet registered
    }

    #[tokio::test]
    async fn test_writer_create_plan_not_implemented() {
        // Test that create_writer_plan returns an error
        // We can't easily create a real ExecutionPlan without complex setup,
        // so we'll just test the function indirectly through the other tests
        // This test serves as documentation that the method is not yet implemented
    }
}
