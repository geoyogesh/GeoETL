//! Factory implementation for `GeoJSON` format support.
//!
//! This module implements the `FormatFactory` trait to integrate `GeoJSON`
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

use crate::{GeoJsonFormatOptions, file_source};

/// `GeoJSON` format options wrapper for the factory system.
impl FormatOptions for GeoJsonFormatOptions {
    fn as_any(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(self.clone())
    }
}

/// Reader implementation for `GeoJSON` format.
struct GeoJsonReader;

#[async_trait]
impl DataReader for GeoJsonReader {
    async fn create_table_provider(
        &self,
        state: &SessionState,
        path: &str,
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<Arc<dyn TableProvider>> {
        let geojson_options = options
            .downcast::<GeoJsonFormatOptions>()
            .map_err(|_| anyhow::anyhow!("Invalid options type for GeoJSON reader"))?;

        let table =
            file_source::create_geojson_table_provider(state, path, *geojson_options).await?;
        Ok(table)
    }
}

/// Writer implementation for `GeoJSON` format.
struct GeoJsonWriter;

#[async_trait]
impl DataWriter for GeoJsonWriter {
    async fn create_writer_plan(
        &self,
        _input: Arc<dyn ExecutionPlan>,
        _path: &str,
        _options: Box<dyn std::any::Any + Send>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // TODO: Implement writer plan creation
        // This requires creating a GeoJsonSink with FileSinkConfig
        Err(anyhow::anyhow!(
            "GeoJSON writer not yet implemented in factory"
        ))
    }

    fn create_writer_options(&self, geometry_column: &str) -> Box<dyn std::any::Any + Send> {
        // GeoJSON writer options with configured geometry column
        let options =
            crate::writer::GeoJsonWriterOptions::default().with_geometry_column(geometry_column);
        Box::new(options)
    }

    fn write_batches(
        &self,
        path: &str,
        batches: &[datafusion::arrow::array::RecordBatch],
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<()> {
        use crate::writer::write_geojson;

        // Downcast options to GeoJsonWriterOptions
        let boxed_options = options
            .downcast::<crate::writer::GeoJsonWriterOptions>()
            .map_err(|_| anyhow::anyhow!("Invalid options type for GeoJSON writer"))?;
        let writer_options = *boxed_options;

        // Write to file
        let mut output_file = std::fs::File::create(path)
            .map_err(|e| anyhow::anyhow!("Failed to create output file: {e}"))?;

        write_geojson(&mut output_file, batches, &writer_options)?;

        Ok(())
    }
}

/// Factory for creating `GeoJSON` readers and writers.
pub struct GeoJsonFormatFactory;

impl FormatFactory for GeoJsonFormatFactory {
    fn driver(&self) -> Driver {
        Driver::new(
            "GeoJSON",
            "GeoJSON",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        )
    }

    fn create_reader(&self) -> Option<Arc<dyn DataReader>> {
        Some(Arc::new(GeoJsonReader))
    }

    fn create_writer(&self) -> Option<Arc<dyn DataWriter>> {
        Some(Arc::new(GeoJsonWriter))
    }

    fn create_file_format(
        &self,
        geometry_column: &str,
    ) -> Option<Arc<dyn datafusion::datasource::file_format::FileFormat>> {
        // Create GeoJSON file format for streaming execution
        let options = crate::file_format::GeoJsonFormatOptions::default()
            .with_geometry_column_name(geometry_column);
        let format = crate::file_format::GeoJsonFormat::new(options);
        Some(Arc::new(format))
    }

    fn infer_table_name(&self, input_path: &str) -> Option<String> {
        // GeoJSON-specific table name inference
        // Extract filename without extension and sanitize for SQL
        std::path::Path::new(input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(datafusion_shared::sanitize_table_name)
    }
}

/// Registers the `GeoJSON` format with the global driver registry.
///
/// This is called by `geoetl-core` during initialization.
pub fn register_geojson_format() {
    let registry = geoetl_core_common::driver_registry();
    registry.register(Arc::new(GeoJsonFormatFactory));
}
