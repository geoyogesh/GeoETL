//! Factory implementation for CSV format support.
//!
//! This module implements the `FormatFactory` trait to integrate CSV
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

use crate::{CsvFormatOptions, file_source};

/// CSV format options wrapper for the factory system.
impl FormatOptions for CsvFormatOptions {
    fn as_any(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(self.clone())
    }
}

/// Reader implementation for CSV format.
struct CsvReader;

#[async_trait]
impl DataReader for CsvReader {
    async fn create_table_provider(
        &self,
        state: &SessionState,
        path: &str,
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<Arc<dyn TableProvider>> {
        let csv_options = options
            .downcast::<CsvFormatOptions>()
            .map_err(|_| anyhow::anyhow!("Invalid options type for CSV reader"))?;

        let table = file_source::create_csv_table_provider(state, path, *csv_options).await?;
        Ok(table)
    }
}

/// Writer implementation for CSV format.
struct CsvWriter;

#[async_trait]
impl DataWriter for CsvWriter {
    async fn create_writer_plan(
        &self,
        _input: Arc<dyn ExecutionPlan>,
        _path: &str,
        _options: Box<dyn std::any::Any + Send>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // TODO: Implement writer plan creation
        // This requires creating a CsvSink with FileSinkConfig
        Err(anyhow::anyhow!("CSV writer not yet implemented in factory"))
    }
}

/// Factory for creating CSV readers and writers.
pub struct CsvFormatFactory;

impl FormatFactory for CsvFormatFactory {
    fn driver(&self) -> Driver {
        Driver::new(
            "CSV",
            "Comma Separated Value (.csv)",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        )
    }

    fn create_reader(&self) -> Option<Arc<dyn DataReader>> {
        Some(Arc::new(CsvReader))
    }

    fn create_writer(&self) -> Option<Arc<dyn DataWriter>> {
        Some(Arc::new(CsvWriter))
    }
}

/// Registers the CSV format with the global driver registry.
///
/// This is called by `geoetl-core` during initialization.
pub fn register_csv_format() {
    let registry = geoetl_core_common::driver_registry();
    registry.register(Arc::new(CsvFormatFactory));
}
