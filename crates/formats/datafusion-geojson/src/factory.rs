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
}

/// Registers the `GeoJSON` format with the global driver registry.
///
/// This is called by `geoetl-core` during initialization.
pub fn register_geojson_format() {
    let registry = geoetl_core_common::driver_registry();
    registry.register(Arc::new(GeoJsonFormatFactory));
}
