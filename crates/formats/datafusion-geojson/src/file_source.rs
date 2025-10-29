//! `GeoJSON` file source configuration and integration with `DataFusion` listing tables.

use std::any::Any;
use std::env;
use std::fmt;
use std::sync::Arc;

use arrow_schema::SchemaRef;
use datafusion::datasource::TableProvider;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::physical_plan::{
    FileGroup, FileOpener, FileScanConfig, FileSource, FileStream,
};
use datafusion::error::Result;
use datafusion::execution::TaskContext;
use datafusion::execution::context::SessionState;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::metrics::ExecutionPlanMetricsSet;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties, SendableRecordBatchStream,
};
use datafusion_common::{DataFusionError, Statistics};
use datafusion_physical_expr::EquivalenceProperties;
use datafusion_shared::SpatialFormatReadError;
use object_store::ObjectStore;
use object_store::aws::AmazonS3Builder;
use object_store::azure::MicrosoftAzureBuilder;
use object_store::gcp::GoogleCloudStorageBuilder;
use object_store::http::HttpBuilder;
use url::Url;

use crate::file_format::{GeoJsonFormat, GeoJsonFormatOptions, detect_file_extension};
use crate::physical_exec::GeoJsonOpener;

/// Builder for creating `GeoJSON` table providers.
pub struct GeoJsonSourceBuilder {
    path: String,
    options: GeoJsonFormatOptions,
}

impl GeoJsonSourceBuilder {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            options: GeoJsonFormatOptions::default(),
        }
    }

    #[must_use]
    pub fn with_options(mut self, options: GeoJsonFormatOptions) -> Self {
        self.options = options;
        self
    }

    /// # Errors
    ///
    /// Returns an error if the `DataFusion` listing table cannot be constructed, including
    /// object store registration or schema inference failures.
    pub async fn build(self, state: &SessionState) -> Result<Arc<dyn TableProvider>> {
        create_geojson_table_provider(state, &self.path, self.options).await
    }
}

/// Create a listing table provider for `GeoJSON` files.
pub async fn create_geojson_table_provider(
    state: &SessionState,
    path: &str,
    options: GeoJsonFormatOptions,
) -> Result<Arc<dyn TableProvider>> {
    let table_url = ListingTableUrl::parse(path)?;
    register_object_store_for_url(state, &table_url)?;

    let extension = resolve_extension(path, &options);

    let format = GeoJsonFormat::new(options.clone());
    let listing_options = ListingOptions::new(Arc::new(format)).with_file_extension(&extension);

    let config = ListingTableConfig::new(table_url)
        .with_listing_options(listing_options)
        .infer_schema(state)
        .await?;

    let table = ListingTable::try_new(config)?;

    Ok(Arc::new(table))
}

fn resolve_extension(path: &str, options: &GeoJsonFormatOptions) -> String {
    let default = options.file_extension_with_dot();
    if default == ".geojson" {
        if let Some(ext) = detect_file_extension(path) {
            let lowercase = ext.to_ascii_lowercase();
            match lowercase.as_str() {
                "geojson" | "json" | "jsonl" | "geojsonl" | "ndjson" => {
                    if lowercase.starts_with('.') {
                        lowercase
                    } else {
                        format!(".{lowercase}")
                    }
                },
                _ => ".geojson".to_string(),
            }
        } else {
            ".geojson".to_string()
        }
    } else {
        default
    }
}

#[derive(Debug, Clone)]
pub struct GeoJsonFileSource {
    options: GeoJsonFormatOptions,
    batch_size: Option<usize>,
    schema: Option<SchemaRef>,
    projection: Option<Vec<usize>>,
    statistics: Option<Statistics>,
    metrics: ExecutionPlanMetricsSet,
}

impl GeoJsonFileSource {
    pub fn new(options: GeoJsonFormatOptions) -> Self {
        Self {
            options,
            batch_size: None,
            schema: None,
            projection: None,
            statistics: None,
            metrics: ExecutionPlanMetricsSet::new(),
        }
    }

    fn resolve_schema(&self, base_config: &FileScanConfig) -> SchemaRef {
        self.schema
            .clone()
            .unwrap_or_else(|| base_config.file_schema.clone())
    }

    fn resolve_projection(&self, base_config: &FileScanConfig) -> Option<Vec<usize>> {
        self.projection
            .clone()
            .or_else(|| base_config.file_column_projection_indices())
    }

    fn resolve_batch_size(&self, base_config: &FileScanConfig) -> usize {
        self.batch_size
            .or(base_config.batch_size)
            .unwrap_or(self.options.batch_size)
    }
}

impl FileSource for GeoJsonFileSource {
    fn create_file_opener(
        &self,
        object_store: Arc<dyn ObjectStore>,
        base_config: &FileScanConfig,
        _partition: usize,
    ) -> Arc<dyn FileOpener> {
        let schema = self.resolve_schema(base_config);
        let projection = self.resolve_projection(base_config);
        let batch_size = self.resolve_batch_size(base_config);

        let opener = GeoJsonOpener::new(self.options.clone(), schema, projection, object_store)
            .with_batch_size(batch_size);

        Arc::new(opener)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn with_batch_size(&self, batch_size: usize) -> Arc<dyn FileSource> {
        let mut source = self.clone();
        source.batch_size = Some(batch_size);
        Arc::new(source)
    }

    fn with_schema(&self, schema: SchemaRef) -> Arc<dyn FileSource> {
        let mut source = self.clone();
        source.schema = Some(schema);
        Arc::new(source)
    }

    fn with_projection(&self, config: &FileScanConfig) -> Arc<dyn FileSource> {
        let mut source = self.clone();
        source.projection = config.file_column_projection_indices();
        Arc::new(source)
    }

    fn with_statistics(&self, statistics: Statistics) -> Arc<dyn FileSource> {
        let mut source = self.clone();
        source.statistics = Some(statistics);
        Arc::new(source)
    }

    fn metrics(&self) -> &ExecutionPlanMetricsSet {
        &self.metrics
    }

    fn statistics(&self) -> datafusion_common::Result<Statistics> {
        self.statistics.clone().ok_or_else(|| {
            DataFusionError::Internal("GeoJSON file source statistics not initialized".to_string())
        })
    }

    fn file_type(&self) -> &'static str {
        "geojson"
    }

    fn fmt_extra(&self, t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, ", geometry_column={}", self.options.geometry_column_name)
            },
            DisplayFormatType::TreeRender => Ok(()),
        }
    }
}

/// Execution plan for reading `GeoJSON` files.
#[derive(Debug, Clone)]
pub struct GeoJsonExec {
    config: FileScanConfig,
    properties: PlanProperties,
}

impl GeoJsonExec {
    pub fn new(config: FileScanConfig) -> Self {
        let projected_schema = config.projected_schema();
        let file_groups = config.file_groups.len();
        let properties = PlanProperties::new(
            EquivalenceProperties::new(projected_schema),
            datafusion::physical_plan::Partitioning::UnknownPartitioning(file_groups),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );

        Self { config, properties }
    }

    fn projected_schema(&self) -> SchemaRef {
        self.config.projected_schema()
    }
}

impl DisplayAs for GeoJsonExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                let count: usize = self.config.file_groups.iter().map(FileGroup::len).sum();
                write!(f, "GeoJsonExec: file_groups={{count={count}}}")
            },
            DisplayFormatType::TreeRender => Ok(()),
        }
    }
}

impl ExecutionPlan for GeoJsonExec {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "GeoJsonExec"
    }

    fn schema(&self) -> SchemaRef {
        self.projected_schema()
    }

    fn properties(&self) -> &PlanProperties {
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

        let opener =
            self.config
                .file_source
                .create_file_opener(object_store, &self.config, partition);

        let stream = FileStream::new(
            &self.config,
            partition,
            opener,
            self.config.file_source.metrics(),
        )?;

        Ok(Box::pin(stream))
    }
}

fn register_object_store_for_url(state: &SessionState, table_url: &ListingTableUrl) -> Result<()> {
    let url = table_url.get_url();
    match url.scheme() {
        "s3" | "s3a" => register_s3_object_store(state, table_url),
        "gs" => register_gcs_object_store(state, table_url),
        "az" | "adl" | "azure" | "abfs" | "abfss" => register_azure_object_store(state, table_url),
        "http" | "https" => {
            if let Some(host) = url.host_str()
                && is_azure_blob_host(host)
            {
                return register_azure_object_store(state, table_url);
            }
            register_http_object_store(state, url.as_str())
        },
        _ => Ok(()),
    }
}

fn register_http_object_store(state: &SessionState, url_str: &str) -> Result<()> {
    let url = Url::parse(url_str).map_err(|e| {
        DataFusionError::from(SpatialFormatReadError::Parse {
            message: format!("Failed to parse URL: {e}"),
            position: None,
            context: Some(url_str.to_string()),
        })
    })?;

    let host = url.host_str().ok_or_else(|| {
        DataFusionError::from(SpatialFormatReadError::Parse {
            message: "URL has no host".to_string(),
            position: None,
            context: Some(url_str.to_string()),
        })
    })?;

    let authority = if let Some(port) = url.port() {
        format!("{host}:{port}")
    } else if let Some(default_port) = url.port_or_known_default() {
        format!("{host}:{default_port}")
    } else {
        host.to_string()
    };

    let base_url = format!("{}://{}", url.scheme(), authority);

    let http_store = HttpBuilder::new()
        .with_url(base_url.clone())
        .build()
        .map_err(|e| {
            DataFusionError::from(SpatialFormatReadError::Io {
                source: std::io::Error::other(e),
                context: Some(base_url.clone()),
            })
        })?;

    let object_store_url = Url::parse(&base_url).unwrap();
    state
        .runtime_env()
        .register_object_store(&object_store_url, Arc::new(http_store));

    Ok(())
}

fn register_s3_object_store(state: &SessionState, table_url: &ListingTableUrl) -> Result<()> {
    let url = table_url.get_url();
    let url_string = url.to_string();
    let bucket = url.host_str().ok_or_else(|| {
        DataFusionError::from(SpatialFormatReadError::Parse {
            message: "S3 URL has no bucket".to_string(),
            position: None,
            context: Some(url_string.clone()),
        })
    })?;

    let mut builder = AmazonS3Builder::from_env()
        .with_url(url_string.clone())
        .with_bucket_name(bucket.to_string());

    let region = env::var("AWS_REGION")
        .or_else(|_| env::var("AWS_DEFAULT_REGION"))
        .unwrap_or_else(|_| "us-east-1".to_string());
    builder = builder.with_region(region);

    let has_access_key = env::var("AWS_ACCESS_KEY_ID").is_ok();
    let has_secret_key = env::var("AWS_SECRET_ACCESS_KEY").is_ok();
    if !(has_access_key && has_secret_key) {
        builder = builder.with_skip_signature(true);
    }

    let s3_store = builder.build().map_err(|e| {
        DataFusionError::from(SpatialFormatReadError::Io {
            source: std::io::Error::other(e),
            context: Some(url_string.clone()),
        })
    })?;

    let object_store_url = table_url.object_store();
    state
        .runtime_env()
        .register_object_store(object_store_url.as_ref(), Arc::new(s3_store));

    Ok(())
}

fn register_gcs_object_store(state: &SessionState, table_url: &ListingTableUrl) -> Result<()> {
    let url = table_url.get_url();
    let url_string = url.to_string();
    let bucket = url.host_str().ok_or_else(|| {
        DataFusionError::from(SpatialFormatReadError::Parse {
            message: "GCS URL has no bucket".to_string(),
            position: None,
            context: Some(url_string.clone()),
        })
    })?;

    let mut builder = GoogleCloudStorageBuilder::from_env()
        .with_url(url_string.clone())
        .with_bucket_name(bucket.to_string());

    if !gcp_credentials_configured() {
        builder = builder.with_skip_signature(true);
    }

    let gcs_store = builder.build().map_err(|e| {
        DataFusionError::from(SpatialFormatReadError::Io {
            source: std::io::Error::other(e),
            context: Some(url_string.clone()),
        })
    })?;

    let object_store_url = table_url.object_store();
    state
        .runtime_env()
        .register_object_store(object_store_url.as_ref(), Arc::new(gcs_store));

    Ok(())
}

fn register_azure_object_store(state: &SessionState, table_url: &ListingTableUrl) -> Result<()> {
    let url = table_url.get_url();
    let url_string = url.to_string();

    let mut builder = MicrosoftAzureBuilder::from_env().with_url(url_string.clone());

    if !azure_credentials_configured() {
        builder = builder.with_skip_signature(true);
    }

    let azure_store = builder.build().map_err(|e| {
        DataFusionError::from(SpatialFormatReadError::Io {
            source: std::io::Error::other(e),
            context: Some(url_string.clone()),
        })
    })?;

    let object_store_url = table_url.object_store();
    state
        .runtime_env()
        .register_object_store(object_store_url.as_ref(), Arc::new(azure_store));

    Ok(())
}

fn is_azure_blob_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host.ends_with("blob.core.windows.net")
        || host.ends_with("dfs.core.windows.net")
        || host.ends_with("blob.fabric.microsoft.com")
        || host.ends_with("dfs.fabric.microsoft.com")
}

fn azure_credentials_configured() -> bool {
    const AZURE_VARS: &[&str] = &[
        "AZURE_STORAGE_CONNECTION_STRING",
        "AZURE_STORAGE_ACCOUNT_KEY",
        "AZURE_STORAGE_ACCESS_KEY",
        "AZURE_STORAGE_MASTER_KEY",
        "AZURE_STORAGE_SAS",
        "AZURE_STORAGE_SAS_KEY",
        "AZURE_STORAGE_BEARER_TOKEN",
        "AZURE_STORAGE_TOKEN",
        "AZURE_STORAGE_CLIENT_SECRET",
        "AZURE_CLIENT_SECRET",
        "AZURE_STORAGE_CLIENT_ID",
        "AZURE_CLIENT_ID",
        "AZURE_STORAGE_TENANT_ID",
        "AZURE_TENANT_ID",
    ];
    any_env_var_set(AZURE_VARS)
}

fn gcp_credentials_configured() -> bool {
    const GCP_VARS: &[&str] = &[
        "GOOGLE_APPLICATION_CREDENTIALS",
        "GOOGLE_SERVICE_ACCOUNT",
        "GOOGLE_SERVICE_ACCOUNT_PATH",
        "SERVICE_ACCOUNT",
        "GOOGLE_SERVICE_ACCOUNT_KEY",
        "SERVICE_ACCOUNT_KEY",
        "GOOGLE_APPLICATION_CREDENTIALS_JSON",
    ];
    any_env_var_set(GCP_VARS)
}

fn any_env_var_set(keys: &[&str]) -> bool {
    keys.iter()
        .any(|key| env::var(key).map(|v| !v.is_empty()).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::datasource::physical_plan::FileScanConfigBuilder;
    use datafusion::execution::context::SessionContext;
    use datafusion_execution::object_store::ObjectStoreUrl;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn source_builder_detects_extension() -> datafusion::error::Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let geojson_path = temp_dir.path().join("data.geojsonl");

        let mut file = File::create(&geojson_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"Feature","geometry":{{"type":"Point","coordinates":[0,0]}},"properties":{{"name":"A"}}}}"#
        )
        .unwrap();

        let ctx = SessionContext::new();
        let provider = GeoJsonSourceBuilder::new(geojson_path.to_str().unwrap())
            .build(&ctx.state())
            .await?;

        let schema = provider.schema();
        assert_eq!(schema.fields().len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn register_http_object_store_registers_store() {
        let ctx = SessionContext::new();
        register_http_object_store(&ctx.state(), "https://example.com/data.geojson").unwrap();

        let result = ctx
            .state()
            .runtime_env()
            .object_store(ObjectStoreUrl::parse("https://example.com").unwrap());
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn register_s3_object_store_registers_store() {
        let ctx = SessionContext::new();
        let table_url = ListingTableUrl::parse("s3://test-bucket/data.geojson").unwrap();
        register_s3_object_store(&ctx.state(), &table_url).unwrap();

        let result = ctx
            .state()
            .runtime_env()
            .object_store(ObjectStoreUrl::parse("s3://test-bucket").unwrap());
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn register_gcs_object_store_registers_store() {
        let ctx = SessionContext::new();
        let table_url = ListingTableUrl::parse("gs://test-bucket/data.geojson").unwrap();
        register_gcs_object_store(&ctx.state(), &table_url).unwrap();

        let result = ctx
            .state()
            .runtime_env()
            .object_store(ObjectStoreUrl::parse("gs://test-bucket").unwrap());
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn register_azure_object_store_registers_store() {
        let ctx = SessionContext::new();
        let table_url =
            ListingTableUrl::parse("https://account.blob.core.windows.net/container/data.geojson")
                .unwrap();
        register_azure_object_store(&ctx.state(), &table_url).unwrap();

        let result = ctx
            .state()
            .runtime_env()
            .object_store(ObjectStoreUrl::parse("https://account.blob.core.windows.net").unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn exec_projection_schema() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, true),
            Field::new("value", DataType::Float64, true),
        ]));
        let object_store_url = ObjectStoreUrl::local_filesystem();
        let file_source = Arc::new(GeoJsonFileSource::new(GeoJsonFormatOptions::default()));
        let config = FileScanConfigBuilder::new(object_store_url, schema.clone(), file_source)
            .with_projection(Some(vec![0]))
            .build();

        let exec = GeoJsonExec::new(config);
        assert_eq!(exec.schema().fields().len(), 1);
        assert_eq!(exec.schema().field(0).name(), "name");
    }
}
