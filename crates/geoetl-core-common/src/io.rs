//! I/O traits for reading and writing geospatial data.
//!
//! This module defines the core traits that format implementations must provide
//! for reading and writing data through `DataFusion`.

use anyhow::Result;
use async_trait::async_trait;
use datafusion::datasource::TableProvider;
use datafusion::execution::context::SessionState;
use datafusion::physical_plan::ExecutionPlan;
use std::sync::Arc;

/// Trait for reading data from a geospatial format.
///
/// Implementations create `DataFusion` `TableProvider` instances that can be
/// queried using SQL or the `DataFrame` API.
#[async_trait]
pub trait DataReader: Send + Sync {
    /// Creates a table provider for the given file path.
    ///
    /// # Arguments
    ///
    /// * `state` - The `DataFusion` session state
    /// * `path` - Path to the data file
    /// * `options` - Format-specific options (as dynamic trait object)
    ///
    /// # Returns
    ///
    /// A `TableProvider` that can be registered with `DataFusion`
    async fn create_table_provider(
        &self,
        state: &SessionState,
        path: &str,
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<Arc<dyn TableProvider>>;
}

/// Trait for writing data to a geospatial format.
///
/// Implementations create `DataFusion` execution plans that write data to files.
#[async_trait]
pub trait DataWriter: Send + Sync {
    /// Creates an execution plan to write data.
    ///``
    /// # Arguments
    ///
    /// * `input` - The input execution plan providing data
    /// * `path` - Output file path
    /// * `options` - Format-specific options (as dynamic trait object)
    ///
    /// # Returns
    ///
    /// An execution plan that writes data when executed
    async fn create_writer_plan(
        &self,
        input: Arc<dyn ExecutionPlan>,
        path: &str,
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<Arc<dyn ExecutionPlan>>;
}
