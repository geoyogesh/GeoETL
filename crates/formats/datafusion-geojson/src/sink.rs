//! `GeoJSON` Data Sink implementation for writing data to `GeoJSON` files

use std::sync::Arc;

use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::datasource::physical_plan::FileSinkConfig;
use datafusion::datasource::sink::DataSink;
use datafusion::physical_plan::metrics::MetricsSet;
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan};
use datafusion_common::{DataFusionError, Result};
use datafusion_execution::{SendableRecordBatchStream, TaskContext};
use datafusion_physical_expr::LexRequirement;
use futures::StreamExt;

use crate::writer::{GeoJsonWriterOptions, write_geojson};

/// `GeoJSON` data sink that implements the `DataSink` trait
#[derive(Debug)]
pub struct GeoJsonSink {
    config: FileSinkConfig,
    writer_options: GeoJsonWriterOptions,
}

impl GeoJsonSink {
    /// Create a new `GeoJSON` sink
    #[must_use]
    pub fn new(config: FileSinkConfig, writer_options: GeoJsonWriterOptions) -> Self {
        Self {
            config,
            writer_options,
        }
    }

    /// Get the sink configuration
    #[must_use]
    pub fn config(&self) -> &FileSinkConfig {
        &self.config
    }

    /// Get writer options
    #[must_use]
    pub fn writer_options(&self) -> &GeoJsonWriterOptions {
        &self.writer_options
    }
}

#[async_trait]
impl DataSink for GeoJsonSink {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn metrics(&self) -> Option<MetricsSet> {
        None
    }

    fn schema(&self) -> &SchemaRef {
        self.config.output_schema()
    }

    async fn write_all(
        &self,
        mut data: SendableRecordBatchStream,
        _context: &Arc<TaskContext>,
    ) -> Result<u64> {
        let mut batches = Vec::new();
        let mut row_count = 0u64;

        // Collect all batches from the stream
        while let Some(batch_result) = data.next().await {
            let batch = batch_result?;
            row_count += batch.num_rows() as u64;
            batches.push(batch);
        }

        // Write to output - for now write to a single file
        let output_path = self
            .config
            .table_paths
            .first()
            .ok_or_else(|| DataFusionError::Internal("No output path specified".to_string()))?;

        let file_path = format!(
            "{}/data.geojson",
            <datafusion::datasource::listing::ListingTableUrl as AsRef<str>>::as_ref(output_path)
        );

        // For now, write to local filesystem
        let mut file = std::fs::File::create(&file_path)
            .map_err(|e| DataFusionError::External(Box::new(e)))?;

        write_geojson(&mut file, &batches, &self.writer_options)?;

        Ok(row_count)
    }
}

impl DisplayAs for GeoJsonSink {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "GeoJsonSink")
    }
}

/// `GeoJSON` writer physical execution plan
#[derive(Debug)]
pub struct GeoJsonWriterExec {
    input: Arc<dyn ExecutionPlan>,
    sink: Arc<GeoJsonSink>,
    _order_requirements: Option<LexRequirement>,
}

impl GeoJsonWriterExec {
    /// Create a new `GeoJSON` writer execution plan
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        sink: Arc<GeoJsonSink>,
        order_requirements: Option<LexRequirement>,
    ) -> Self {
        Self {
            input,
            sink,
            _order_requirements: order_requirements,
        }
    }
}

impl DisplayAs for GeoJsonWriterExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "GeoJsonWriterExec")
    }
}

impl std::fmt::Display for GeoJsonWriterExec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "GeoJsonWriterExec")
    }
}

impl ExecutionPlan for GeoJsonWriterExec {
    fn name(&self) -> &'static str {
        "GeoJsonWriterExec"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn properties(&self) -> &datafusion::physical_plan::PlanProperties {
        self.input.properties()
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if children.len() != 1 {
            return Err(DataFusionError::Internal(
                "GeoJsonWriterExec requires exactly one child".to_string(),
            ));
        }

        #[allow(clippy::used_underscore_binding)]
        Ok(Arc::new(Self {
            input: Arc::clone(&children[0]),
            sink: Arc::clone(&self.sink),
            _order_requirements: self._order_requirements.clone(),
        }))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        if partition != 0 {
            return Err(DataFusionError::Internal(
                "GeoJsonWriterExec only supports single partition".to_string(),
            ));
        }

        // Execute input and get stream
        let input_stream = self.input.execute(partition, Arc::clone(&context))?;

        // For now, we'll return the input stream
        Ok(input_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::GeoJsonWriterOptions;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::datasource::listing::ListingTableUrl;
    use datafusion::datasource::physical_plan::FileGroup;
    use datafusion::logical_expr::dml::InsertOp;
    use datafusion_execution::object_store::ObjectStoreUrl;

    #[test]
    fn test_geojson_sink_creation() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("geometry", DataType::Utf8, true),
        ]));

        let config = FileSinkConfig {
            original_url: "file:///tmp/output.geojson".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "geojson".to_string(),
        };

        let writer_options = GeoJsonWriterOptions::default();
        let sink = GeoJsonSink::new(config, writer_options);

        assert_eq!(sink.schema().fields().len(), 3);
        assert_eq!(sink.writer_options().geometry_column_name, "geometry");
    }
}
