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
use datafusion::arrow::array::RecordBatch;

/// Convert geometry columns to WKT format for CSV writing
fn convert_geometry_to_wkt(
    batches: &[RecordBatch],
    geometry_column: &str,
) -> Result<Vec<RecordBatch>> {
    use arrow_schema::Schema;
    use geoarrow_array::GeoArrowArray;
    use geoarrow_array::array::from_arrow_array;
    use geoarrow_array::cast::to_wkt;
    use std::sync::Arc;

    let mut converted_batches = Vec::with_capacity(batches.len());

    for batch in batches {
        let schema = batch.schema();

        // Find the geometry column index
        let geom_idx = schema
            .fields()
            .iter()
            .position(|field| field.name() == geometry_column);

        if let Some(idx) = geom_idx {
            // Get the geometry column and its field
            let geom_array = batch.column(idx);
            let geom_field = schema.field(idx);

            // Convert Arrow array to GeoArrowArray
            let geoarrow_array = from_arrow_array(geom_array.as_ref(), geom_field)
                .map_err(|e| anyhow::anyhow!("Failed to convert to GeoArrowArray: {e}"))?;

            // Convert to WKT using geoarrow cast (using i32 offset)
            let wkt_array: geoarrow_array::array::WktArray = to_wkt(&geoarrow_array)
                .map_err(|e| anyhow::anyhow!("Failed to convert geometry to WKT: {e}"))?;

            // Create new schema with WKT column
            let mut new_fields = schema.fields().to_vec();
            new_fields[idx] = Arc::new(arrow_schema::Field::new(
                geometry_column,
                arrow_schema::DataType::Utf8,
                true,
            ));
            let new_schema = Arc::new(Schema::new(new_fields));

            // Create new columns with WKT
            let mut new_columns = batch.columns().to_vec();
            new_columns[idx] = wkt_array.to_array_ref();

            // Create new batch
            let new_batch = RecordBatch::try_new(new_schema, new_columns)
                .map_err(|e| anyhow::anyhow!("Failed to create record batch with WKT: {e}"))?;

            converted_batches.push(new_batch);
        } else {
            // No geometry column found, use batch as-is
            converted_batches.push(batch.clone());
        }
    }

    Ok(converted_batches)
}

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

    fn create_writer_options(&self, geometry_column: &str) -> Box<dyn std::any::Any + Send> {
        // CSV writer options - returns tuple of (CsvWriterOptions, geometry_column)
        // The geometry column is needed for WKT conversion
        let options = crate::writer::CsvWriterOptions::default();
        Box::new((options, geometry_column.to_string()))
    }

    fn write_batches(
        &self,
        path: &str,
        batches: &[datafusion::arrow::array::RecordBatch],
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<()> {
        use crate::writer::write_csv;

        // Downcast options to (CsvWriterOptions, String) tuple
        // The string is the geometry column name for WKT conversion
        let boxed_options = options
            .downcast::<(crate::writer::CsvWriterOptions, String)>()
            .map_err(|_| anyhow::anyhow!("Invalid options type for CSV writer"))?;
        let (writer_options, geometry_column) = *boxed_options;

        // Convert geometry columns to WKT before writing
        let converted_batches = convert_geometry_to_wkt(batches, &geometry_column)?;

        // Write to file
        let mut output_file = std::fs::File::create(path)
            .map_err(|e| anyhow::anyhow!("Failed to create output file: {e}"))?;

        write_csv(&mut output_file, &converted_batches, &writer_options)?;

        Ok(())
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
