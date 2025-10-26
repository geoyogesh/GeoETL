//! CSV file source configuration and execution
//!
//! This module provides the execution plan for reading CSV files,
//! using our independent CSV reader implementation.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use arrow_schema::SchemaRef;
use datafusion::datasource::TableProvider;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::physical_plan::FileScanConfig;
use datafusion::error::Result;
use datafusion::execution::TaskContext;
use datafusion::execution::context::SessionState;
use datafusion::physical_plan::metrics::ExecutionPlanMetricsSet;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionMode, ExecutionPlan, PlanProperties,
    SendableRecordBatchStream,
};
use datafusion_common::project_schema;
use datafusion_physical_expr::EquivalenceProperties;
use object_store::http::HttpBuilder;
use url::Url;

use crate::file_format::{CsvFormat, CsvFormatOptions, detect_file_extension};
use crate::physical_exec::CsvOpener;

/// CSV source builder for creating table providers
pub struct CsvSourceBuilder {
    path: String,
    options: CsvFormatOptions,
}

impl CsvSourceBuilder {
    /// Create a new CSV source builder
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            options: CsvFormatOptions::default(),
        }
    }

    /// Set CSV format options
    #[must_use]
    pub fn with_options(mut self, options: CsvFormatOptions) -> Self {
        self.options = options;
        self
    }

    /// Set delimiter
    #[must_use]
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.options = self.options.with_delimiter(delimiter);
        self
    }

    /// Set whether file has header
    #[must_use]
    pub fn with_has_header(mut self, has_header: bool) -> Self {
        self.options = self.options.with_has_header(has_header);
        self
    }

    /// Build the table provider
    ///
    /// # Errors
    ///
    /// Returns an error if the object store registration or listing table setup fails.
    pub async fn build(self, state: &SessionState) -> Result<Arc<dyn TableProvider>> {
        create_csv_table_provider(state, &self.path, self.options).await
    }
}

/// Create a CSV table provider from a path and options
pub async fn create_csv_table_provider(
    state: &SessionState,
    path: &str,
    options: CsvFormatOptions,
) -> Result<Arc<dyn TableProvider>> {
    // Register HTTP object store if the URL is HTTP/HTTPS
    if path.starts_with("http://") || path.starts_with("https://") {
        register_http_object_store(state, path)?;
    }

    let table_url = ListingTableUrl::parse(path)?;

    // Auto-detect file extension if not explicitly set as non-csv
    let extension = if options.file_extension == ".csv" {
        detect_file_extension(path).map_or_else(
            || ".csv".to_string(),
            |ext| {
                if ext.starts_with('.') {
                    ext
                } else {
                    format!(".{ext}")
                }
            },
        )
    } else {
        options.file_extension_with_dot()
    };

    let format = CsvFormat::new(options);
    let listing_options = ListingOptions::new(Arc::new(format)).with_file_extension(&extension);

    let config = ListingTableConfig::new(table_url)
        .with_listing_options(listing_options)
        .infer_schema(state)
        .await?;

    let table = ListingTable::try_new(config)?;

    Ok(Arc::new(table))
}

/// Register HTTP object store for the given URL
fn register_http_object_store(state: &SessionState, url_str: &str) -> Result<()> {
    let url = Url::parse(url_str).map_err(|e| {
        datafusion_common::DataFusionError::Execution(format!("Failed to parse URL: {e}"))
    })?;

    // Extract the base URL (scheme + host + port)
    let host = url.host_str().ok_or_else(|| {
        datafusion_common::DataFusionError::Execution("URL has no host".to_string())
    })?;

    let authority = if let Some(port) = url.port() {
        format!("{host}:{port}")
    } else if let Some(default_port) = url.port_or_known_default() {
        format!("{host}:{default_port}")
    } else {
        host.to_string()
    };

    let base_url = format!("{}://{}", url.scheme(), authority);

    // Build HTTP object store
    let http_store = HttpBuilder::new()
        .with_url(base_url.clone())
        .build()
        .map_err(|e| {
            datafusion_common::DataFusionError::Execution(format!(
                "Failed to create HTTP object store: {e}"
            ))
        })?;

    // Register the object store
    let object_store_url = Url::parse(&base_url).unwrap();
    state
        .runtime_env()
        .register_object_store(&object_store_url, Arc::new(http_store));

    Ok(())
}

/// CSV execution plan that uses our independent CSV reader
#[derive(Debug, Clone)]
pub struct CsvExec {
    /// File scan configuration
    config: FileScanConfig,
    /// CSV format options
    options: CsvFormatOptions,
    /// Execution metrics
    metrics: ExecutionPlanMetricsSet,
    /// Plan properties
    properties: PlanProperties,
}

impl CsvExec {
    #[must_use]
    pub fn new(config: FileScanConfig, options: CsvFormatOptions) -> Self {
        // Calculate the projected schema
        let projected_schema = if let Some(ref proj) = config.projection {
            project_schema(&config.file_schema, Some(proj)).unwrap()
        } else {
            config.file_schema.clone()
        };

        let properties = PlanProperties::new(
            EquivalenceProperties::new(projected_schema.clone()),
            datafusion::physical_plan::Partitioning::UnknownPartitioning(config.file_groups.len()),
            ExecutionMode::Bounded,
        );

        Self {
            config,
            options,
            metrics: ExecutionPlanMetricsSet::new(),
            properties,
        }
    }

    fn projected_schema(&self) -> SchemaRef {
        if let Some(ref proj) = self.config.projection {
            project_schema(&self.config.file_schema, Some(proj)).unwrap()
        } else {
            self.config.file_schema.clone()
        }
    }
}

impl DisplayAs for CsvExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                let file_count: usize = self.config.file_groups.iter().map(Vec::len).sum();
                write!(f, "CsvExec: file_groups={{count={file_count}}}")
            },
        }
    }
}

impl ExecutionPlan for CsvExec {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "CsvExec"
    }

    fn schema(&self) -> SchemaRef {
        self.projected_schema()
    }

    fn properties(&self) -> &datafusion::physical_plan::PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let object_store_url = self.config.object_store_url.clone();
        let object_store = context.runtime_env().object_store(&object_store_url)?;

        let opener = CsvOpener::new(
            self.options.clone(),
            self.config.file_schema.clone(),
            self.config.projection.clone(),
            object_store,
        )
        .with_batch_size(self.options.batch_size);

        // Open files using our CSV opener
        let stream = datafusion::datasource::physical_plan::FileStream::new(
            &self.config,
            partition,
            opener,
            &self.metrics,
        )?;

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::datasource::physical_plan::FileScanConfig;
    use datafusion::execution::context::SessionContext;
    use datafusion_execution::object_store::ObjectStoreUrl;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_csv_source_builder_detects_extension() -> datafusion::error::Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("data.tsv");

        let mut file = File::create(&csv_path).unwrap();
        writeln!(file, "id\tname\tvalue").unwrap();
        writeln!(file, "1\tAlice\t100").unwrap();
        writeln!(file, "2\tBob\t200").unwrap();

        let ctx = SessionContext::new();
        let provider = CsvSourceBuilder::new(csv_path.to_str().unwrap())
            .with_delimiter(b'\t')
            .build(&ctx.state())
            .await?;

        let schema = provider.schema();
        assert_eq!(schema.fields().len(), 3);
        assert_eq!(schema.field(0).name(), "id");

        Ok(())
    }

    #[tokio::test]
    async fn test_csv_source_builder_without_header() -> datafusion::error::Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("data_no_header.csv");

        let mut file = File::create(&csv_path).unwrap();
        writeln!(file, "1,Alice").unwrap();
        writeln!(file, "2,Bob").unwrap();

        let ctx = SessionContext::new();
        let provider = CsvSourceBuilder::new(csv_path.to_str().unwrap())
            .with_has_header(false)
            .build(&ctx.state())
            .await?;

        let schema = provider.schema();
        assert_eq!(schema.field(0).name(), "column_0");
        assert_eq!(schema.field(1).name(), "column_1");

        Ok(())
    }

    #[tokio::test]
    async fn test_register_http_object_store_registers_store() {
        let ctx = SessionContext::new();
        register_http_object_store(&ctx.state(), "https://example.com/data.csv").unwrap();

        let result = ctx
            .state()
            .runtime_env()
            .object_store(ObjectStoreUrl::parse("https://example.com").unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_csv_exec_projection_schema() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, true),
            Field::new("name", DataType::Utf8, true),
        ]));
        let object_store_url = ObjectStoreUrl::local_filesystem();
        let config =
            FileScanConfig::new(object_store_url, schema.clone()).with_projection(Some(vec![1]));

        let exec = CsvExec::new(config, CsvFormatOptions::default());
        assert_eq!(exec.name(), "CsvExec");
        assert_eq!(exec.schema().fields().len(), 1);
        assert_eq!(exec.schema().field(0).name(), "name");
    }
}
