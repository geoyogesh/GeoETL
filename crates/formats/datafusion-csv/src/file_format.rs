//! CSV file format configuration and handling
//!
//! This module provides CSV format configuration options and implements
//! the `DataFusion` `FileFormat` trait for independent CSV reading.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use arrow_schema::{Schema, SchemaRef};
use async_trait::async_trait;
use datafusion::datasource::file_format::FileFormat;
use datafusion::datasource::physical_plan::FileScanConfig;
use datafusion::error::Result;
use datafusion::execution::context::SessionState;
use datafusion::physical_plan::{ExecutionPlan, PhysicalExpr, Statistics};
use object_store::{ObjectMeta, ObjectStore};

use crate::file_source::CsvExec;
use crate::physical_exec;

/// CSV format configuration options
#[derive(Debug, Clone)]
pub struct CsvFormatOptions {
    /// Whether the CSV file has a header row (default: true)
    pub has_header: bool,
    /// The delimiter character (default: b',')
    pub delimiter: u8,
    /// Maximum number of rows to read for schema inference
    pub schema_infer_max_rec: Option<usize>,
    /// Batch size for reading (default: 8192)
    pub batch_size: usize,
    /// File extension to look for (default: ".csv")
    pub file_extension: String,
}

impl Default for CsvFormatOptions {
    fn default() -> Self {
        Self {
            has_header: true,
            delimiter: b',',
            schema_infer_max_rec: Some(1000),
            batch_size: 8192,
            file_extension: ".csv".to_string(),
        }
    }
}

impl CsvFormatOptions {
    /// Create new CSV format options with defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether the CSV has a header row
    #[must_use]
    pub fn with_has_header(mut self, has_header: bool) -> Self {
        self.has_header = has_header;
        self
    }

    /// Set the delimiter character
    #[must_use]
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Set maximum records for schema inference
    #[must_use]
    pub fn with_schema_infer_max_rec(mut self, max_rec: Option<usize>) -> Self {
        self.schema_infer_max_rec = max_rec;
        self
    }

    /// Set batch size for reading
    #[must_use]
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Set file extension
    #[must_use]
    pub fn with_file_extension(mut self, ext: impl Into<String>) -> Self {
        self.file_extension = ext.into();
        self
    }

    /// Get file extension with leading dot
    pub(crate) fn file_extension_with_dot(&self) -> String {
        if self.file_extension.starts_with('.') {
            self.file_extension.clone()
        } else {
            format!(".{extension}", extension = self.file_extension)
        }
    }
}

/// Independent CSV file format implementation
#[derive(Debug, Clone)]
pub struct CsvFormat {
    options: CsvFormatOptions,
}

impl CsvFormat {
    #[must_use]
    pub fn new(options: CsvFormatOptions) -> Self {
        Self { options }
    }
}

impl Default for CsvFormat {
    fn default() -> Self {
        Self::new(CsvFormatOptions::default())
    }
}

impl fmt::Display for CsvFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CSV")
    }
}

#[async_trait]
impl FileFormat for CsvFormat {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_ext(&self) -> String {
        self.options.file_extension_with_dot()
    }

    fn get_ext_with_compression(
        &self,
        _c: &datafusion::datasource::file_format::file_compression_type::FileCompressionType,
    ) -> Result<String> {
        Ok(self.get_ext())
    }

    async fn infer_schema(
        &self,
        _state: &SessionState,
        store: &Arc<dyn ObjectStore>,
        objects: &[ObjectMeta],
    ) -> Result<SchemaRef> {
        if objects.is_empty() {
            return Ok(Arc::new(Schema::empty()));
        }

        // Read the first file to infer schema
        let obj = &objects[0];
        let bytes = store
            .get(&obj.location)
            .await
            .map_err(|e| datafusion::error::DataFusionError::External(Box::new(e)))?
            .bytes()
            .await
            .map_err(|e| datafusion::error::DataFusionError::External(Box::new(e)))?;

        // Use our independent schema inference
        let schema = physical_exec::infer_schema(&bytes, &self.options)?;

        Ok(Arc::new(schema))
    }

    async fn infer_stats(
        &self,
        _state: &SessionState,
        _store: &Arc<dyn ObjectStore>,
        table_schema: SchemaRef,
        _object: &ObjectMeta,
    ) -> Result<Statistics> {
        // Return unknown statistics for now
        Ok(Statistics::new_unknown(&table_schema))
    }

    async fn create_physical_plan(
        &self,
        _state: &SessionState,
        conf: FileScanConfig,
        _filters: Option<&Arc<dyn PhysicalExpr>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Create our custom CSV execution plan
        let exec = CsvExec::new(conf, self.options.clone());
        Ok(Arc::new(exec))
    }
}

/// Helper to detect file extension from path
pub(crate) fn detect_file_extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::datasource::file_format::file_compression_type::FileCompressionType;
    use datafusion::datasource::physical_plan::FileScanConfig;
    use datafusion::execution::context::SessionContext;
    use datafusion_execution::object_store::ObjectStoreUrl;
    use object_store::ObjectStore;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use std::sync::Arc;

    #[test]
    fn test_csv_format_options_helpers() {
        let options = CsvFormatOptions::new()
            .with_has_header(false)
            .with_delimiter(b';')
            .with_schema_infer_max_rec(Some(42))
            .with_batch_size(256)
            .with_file_extension("tsv");

        assert!(!options.has_header);
        assert_eq!(options.delimiter, b';');
        assert_eq!(options.schema_infer_max_rec, Some(42));
        assert_eq!(options.batch_size, 256);
        assert_eq!(options.file_extension_with_dot(), ".tsv");

        let with_dot = CsvFormatOptions::new().with_file_extension(".csv");
        assert_eq!(with_dot.file_extension_with_dot(), ".csv");

        assert_eq!(
            detect_file_extension("data/file.csv"),
            Some("csv".to_string())
        );

        let format = CsvFormat::default();
        assert_eq!(format!("{format}"), "CSV");

        let ext = format
            .get_ext_with_compression(&FileCompressionType::UNCOMPRESSED)
            .unwrap();
        assert_eq!(ext, ".csv");
    }

    #[tokio::test]
    async fn test_infer_schema_empty_objects() {
        let ctx = SessionContext::new();
        let format = CsvFormat::default();
        let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

        let schema = format
            .infer_schema(&ctx.state(), &store, &[])
            .await
            .expect("schema inference");
        assert_eq!(schema.fields().len(), 0);
    }

    #[tokio::test]
    async fn test_infer_schema_from_object_store() {
        let ctx = SessionContext::new();
        let format = CsvFormat::default();
        let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

        let data = b"name,active,score\nAlice,true,1.5\nBob,false,3.0".to_vec();
        let location = Path::from("data/test.csv");
        store
            .put(&location, data.clone().into())
            .await
            .expect("write object");
        let meta = store.head(&location).await.expect("object metadata");

        let schema = format
            .infer_schema(&ctx.state(), &store, std::slice::from_ref(&meta))
            .await
            .expect("schema inference");

        assert_eq!(schema.fields().len(), 3);
        assert_eq!(schema.field(0).name(), "name");
        assert_eq!(schema.field(1).data_type(), &DataType::Boolean);
        assert_eq!(schema.field(2).data_type(), &DataType::Float64);

        let stats = format
            .infer_stats(&ctx.state(), &store, schema.clone(), &meta)
            .await
            .expect("statistics");
        assert!(matches!(
            stats.num_rows,
            datafusion_common::stats::Precision::Absent
        ));
    }

    #[tokio::test]
    async fn test_create_physical_plan_returns_csv_exec() {
        let ctx = SessionContext::new();
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, true),
            Field::new("name", DataType::Utf8, true),
        ]));
        let object_store_url = ObjectStoreUrl::local_filesystem();
        let config = FileScanConfig::new(object_store_url, schema.clone());

        let format = CsvFormat::default();
        let plan = format
            .create_physical_plan(&ctx.state(), config, None)
            .await
            .expect("physical plan");

        assert_eq!(plan.name(), "CsvExec");
        let csv_exec = plan.as_any().downcast_ref::<CsvExec>().expect("CsvExec");
        assert_eq!(csv_exec.schema().fields().len(), 2);
        assert_eq!(csv_exec.schema().field(0).name(), "id");
    }
}
