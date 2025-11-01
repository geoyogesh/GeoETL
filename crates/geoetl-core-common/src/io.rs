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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read or does not exist
    /// - The file format is invalid or corrupted
    /// - The options cannot be downcast to the expected type
    /// - Schema inference fails
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
    ///
    /// # Arguments
    ///
    /// * `input` - The input execution plan providing data
    /// * `path` - Output file path
    /// * `options` - Format-specific options (as dynamic trait object)
    ///
    /// # Returns
    ///
    /// An execution plan that writes data when executed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The output file cannot be created or written to
    /// - The options cannot be downcast to the expected type
    /// - The execution plan creation fails
    async fn create_writer_plan(
        &self,
        input: Arc<dyn ExecutionPlan>,
        path: &str,
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<Arc<dyn ExecutionPlan>>;

    /// Creates format-specific writer options (Factory pattern).
    ///
    /// This method encapsulates the creation of format-specific options,
    /// eliminating the need for match statements in calling code.
    ///
    /// # Arguments
    ///
    /// * `geometry_column` - Name of the geometry column (used by some formats)
    ///
    /// # Returns
    ///
    /// Format-specific options as a dynamic trait object
    fn create_writer_options(&self, geometry_column: &str) -> Box<dyn std::any::Any + Send>;

    /// Synchronously writes record batches to a file (Strategy pattern).
    ///
    /// This method provides a simpler synchronous interface for writing already-collected
    /// record batches, which is used by the ETL operations module.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path
    /// * `batches` - Record batches to write
    /// * `options` - Format-specific options (as dynamic trait object)
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The output file cannot be created or written to
    /// - The options cannot be downcast to the expected type
    /// - Geometry conversion fails (if applicable)
    /// - Writing data to the file fails
    fn write_batches(
        &self,
        path: &str,
        batches: &[datafusion::arrow::array::RecordBatch],
        options: Box<dyn std::any::Any + Send>,
    ) -> Result<()>;
}
