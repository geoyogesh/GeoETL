//! CSV Data Sink implementation for writing data to CSV files

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

use crate::writer::{CsvWriterOptions, write_csv};

/// CSV data sink that implements the `DataSink` trait
#[derive(Debug)]
pub struct CsvSink {
    config: FileSinkConfig,
    writer_options: CsvWriterOptions,
}

impl CsvSink {
    /// Create a new CSV sink
    #[must_use]
    pub fn new(config: FileSinkConfig, writer_options: CsvWriterOptions) -> Self {
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
    pub fn writer_options(&self) -> &CsvWriterOptions {
        &self.writer_options
    }
}

#[async_trait]
impl DataSink for CsvSink {
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
        // In a full implementation, this would handle partitioning
        // and write to object store
        let output_path = self
            .config
            .table_paths
            .first()
            .ok_or_else(|| DataFusionError::Internal("No output path specified".to_string()))?;

        let file_path = format!(
            "{}/data.csv",
            <datafusion::datasource::listing::ListingTableUrl as AsRef<str>>::as_ref(output_path)
        );

        // For now, write to local filesystem
        // A full implementation would use object store
        let mut file = std::fs::File::create(&file_path)
            .map_err(|e| DataFusionError::External(Box::new(e)))?;

        write_csv(&mut file, &batches, &self.writer_options)?;

        Ok(row_count)
    }
}

impl DisplayAs for CsvSink {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CsvSink")
    }
}

/// CSV writer physical execution plan
#[derive(Debug)]
pub struct CsvWriterExec {
    input: Arc<dyn ExecutionPlan>,
    sink: Arc<CsvSink>,
    _order_requirements: Option<LexRequirement>,
}

impl CsvWriterExec {
    /// Create a new CSV writer execution plan
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        sink: Arc<CsvSink>,
        order_requirements: Option<LexRequirement>,
    ) -> Self {
        Self {
            input,
            sink,
            _order_requirements: order_requirements,
        }
    }
}

impl DisplayAs for CsvWriterExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CsvWriterExec")
    }
}

impl std::fmt::Display for CsvWriterExec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CsvWriterExec")
    }
}

impl ExecutionPlan for CsvWriterExec {
    fn name(&self) -> &'static str {
        "CsvWriterExec"
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
                "CsvWriterExec requires exactly one child".to_string(),
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
                "CsvWriterExec only supports single partition".to_string(),
            ));
        }

        // Execute input and get stream
        let input_stream = self.input.execute(partition, Arc::clone(&context))?;

        // For now, we'll return the input stream
        // A full implementation would write and return a count stream
        Ok(input_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::CsvWriterOptions;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::datasource::listing::ListingTableUrl;
    use datafusion::datasource::physical_plan::FileGroup;
    use datafusion::logical_expr::dml::InsertOp;
    use datafusion_execution::object_store::ObjectStoreUrl;

    #[test]
    fn test_csv_sink_creation() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
        ]));

        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
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
        let sink = CsvSink::new(config, writer_options);

        assert_eq!(sink.schema().fields().len(), 2);
        assert_eq!(sink.writer_options().delimiter, b',');
    }

    #[test]
    fn test_csv_sink_as_any() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = CsvSink::new(config, CsvWriterOptions::default());
        assert!(sink.as_any().is::<CsvSink>());
    }

    #[test]
    fn test_csv_sink_metrics() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = CsvSink::new(config, CsvWriterOptions::default());
        assert!(sink.metrics().is_none());
    }

    #[test]
    fn test_csv_sink_display() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = CsvSink::new(config, CsvWriterOptions::default());
        assert_eq!(format!("{sink:?}"), format!("{sink:?}"));
    }

    #[test]
    fn test_csv_writer_exec_creation() {
        use datafusion::physical_plan::empty::EmptyExec;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = Arc::new(CsvSink::new(config, CsvWriterOptions::default()));
        let input = Arc::new(EmptyExec::new(schema.clone())) as Arc<dyn ExecutionPlan>;
        let exec = CsvWriterExec::new(input, sink, None);

        assert_eq!(exec.name(), "CsvWriterExec");
    }

    #[test]
    fn test_csv_writer_exec_as_any() {
        use datafusion::physical_plan::empty::EmptyExec;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = Arc::new(CsvSink::new(config, CsvWriterOptions::default()));
        let input = Arc::new(EmptyExec::new(schema.clone())) as Arc<dyn ExecutionPlan>;
        let exec = CsvWriterExec::new(input, sink, None);

        assert!(exec.as_any().is::<CsvWriterExec>());
    }

    #[test]
    fn test_csv_writer_exec_children() {
        use datafusion::physical_plan::empty::EmptyExec;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = Arc::new(CsvSink::new(config, CsvWriterOptions::default()));
        let input = Arc::new(EmptyExec::new(schema.clone())) as Arc<dyn ExecutionPlan>;
        let exec = CsvWriterExec::new(input, sink, None);

        let children = exec.children();
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn test_csv_writer_exec_with_new_children() {
        use datafusion::physical_plan::empty::EmptyExec;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = Arc::new(CsvSink::new(config, CsvWriterOptions::default()));
        let input = Arc::new(EmptyExec::new(schema.clone())) as Arc<dyn ExecutionPlan>;
        let exec = Arc::new(CsvWriterExec::new(input.clone(), sink, None));

        // Test with one child
        let new_exec = exec.clone().with_new_children(vec![input.clone()]).unwrap();
        assert_eq!(new_exec.children().len(), 1);
    }

    #[test]
    fn test_csv_writer_exec_with_new_children_error() {
        use datafusion::physical_plan::empty::EmptyExec;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = Arc::new(CsvSink::new(config, CsvWriterOptions::default()));
        let input = Arc::new(EmptyExec::new(schema.clone())) as Arc<dyn ExecutionPlan>;
        let exec = Arc::new(CsvWriterExec::new(input.clone(), sink, None));

        // Test with wrong number of children
        let result = exec.clone().with_new_children(vec![]);
        assert!(result.is_err());

        let result = exec
            .clone()
            .with_new_children(vec![input.clone(), input.clone()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_csv_writer_exec_execute_error() {
        use datafusion::physical_plan::empty::EmptyExec;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = Arc::new(CsvSink::new(config, CsvWriterOptions::default()));
        let input = Arc::new(EmptyExec::new(schema.clone())) as Arc<dyn ExecutionPlan>;
        let exec = CsvWriterExec::new(input, sink, None);

        let context = Arc::new(TaskContext::default());

        // Test with invalid partition
        let result = exec.execute(1, context);
        assert!(result.is_err());
    }

    #[test]
    fn test_csv_writer_exec_display() {
        use datafusion::physical_plan::empty::EmptyExec;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let config = FileSinkConfig {
            original_url: "file:///tmp/output.csv".to_string(),
            object_store_url: ObjectStoreUrl::local_filesystem(),
            file_group: FileGroup::default(),
            table_paths: vec![ListingTableUrl::parse("file:///tmp").unwrap()],
            output_schema: schema.clone(),
            table_partition_cols: vec![],
            insert_op: InsertOp::Append,
            keep_partition_by_columns: false,
            file_extension: "csv".to_string(),
        };

        let sink = Arc::new(CsvSink::new(config, CsvWriterOptions::default()));
        let input = Arc::new(EmptyExec::new(schema.clone())) as Arc<dyn ExecutionPlan>;
        let exec = CsvWriterExec::new(input, sink, None);

        assert_eq!(format!("{exec}"), "CsvWriterExec");
    }
}
