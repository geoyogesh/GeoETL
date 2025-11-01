//! `DataFusion` CSV Integration
//!
//! This crate provides CSV file format support for Apache `DataFusion`.
//! It allows reading and querying CSV files using `DataFusion`'s SQL and `DataFrame` APIs.
//!
//! # Architecture
//!
//! The crate is organized into several modules following the datafusion-orc pattern:
//! - `file_format` - CSV format configuration and options
//! - `file_source` - CSV source builders and table providers
//! - `physical_exec` - Physical execution configuration
//! - `object_store_reader` - Object store integration utilities
//!
//! # Example
//!
//! ```no_run
//! use datafusion::prelude::*;
//! use datafusion_csv::SessionContextCsvExt;
//!
//! #[tokio::main]
//! async fn main() -> datafusion::error::Result<()> {
//!     let ctx = SessionContext::new();
//!
//!     // Register a CSV file as a table
//!     ctx.register_csv_file("my_table", "path/to/file.csv").await?;
//!
//!     // Query the CSV file
//!     let df = ctx.sql("SELECT * FROM my_table").await?;
//!     df.show().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod factory;
mod file_format;
mod file_source;
pub mod geospatial;
mod object_store_reader;
mod physical_exec;
mod sink;
mod writer;

// Re-export public types
pub use factory::register_csv_format;
pub use file_format::CsvFormatOptions;
pub use file_source::CsvSourceBuilder;
pub use object_store_reader::CsvFileMetadata;
pub use sink::{CsvSink, CsvWriterExec};
pub use writer::{CsvWriterOptions, write_csv, write_csv_to_bytes};

use datafusion::prelude::*;
use datafusion_common::Result;

/// Extension trait for `SessionContext` to add convenient CSV registration methods
#[allow(async_fn_in_trait)]
pub trait SessionContextCsvExt {
    /// Register a CSV file as a table with default options
    ///
    /// # Example
    ///
    /// ```no_run
    /// use datafusion::prelude::*;
    /// use datafusion_csv::SessionContextCsvExt;
    ///
    /// # async fn example() -> datafusion_common::Result<()> {
    /// let ctx = SessionContext::new();
    /// ctx.register_csv_file("users", "data/users.csv").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn register_csv_file(&self, name: &str, path: &str) -> Result<()>;

    /// Register a CSV file with custom delimiter
    ///
    /// # Example
    ///
    /// ```no_run
    /// use datafusion::prelude::*;
    /// use datafusion_csv::SessionContextCsvExt;
    ///
    /// # async fn example() -> datafusion_common::Result<()> {
    /// let ctx = SessionContext::new();
    /// // Register a TSV file with tab delimiter
    /// ctx.register_csv_with_delimiter("data", "file.tsv", b'\t').await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn register_csv_with_delimiter(
        &self,
        name: &str,
        path: &str,
        delimiter: u8,
    ) -> Result<()>;

    /// Register a CSV file with custom options
    ///
    /// # Example
    ///
    /// ```no_run
    /// use datafusion::prelude::*;
    /// use datafusion_csv::{SessionContextCsvExt, CsvFormatOptions};
    ///
    /// # async fn example() -> datafusion_common::Result<()> {
    /// let ctx = SessionContext::new();
    /// let options = CsvFormatOptions::new()
    ///     .with_delimiter(b';')
    ///     .with_has_header(false);
    ///
    /// ctx.register_csv_with_options("data", "file.csv", options).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn register_csv_with_options(
        &self,
        name: &str,
        path: &str,
        options: CsvFormatOptions,
    ) -> Result<()>;

    /// Read a CSV file into a `DataFrame` with default options
    ///
    /// # Example
    ///
    /// ```no_run
    /// use datafusion::prelude::*;
    /// use datafusion_csv::SessionContextCsvExt;
    ///
    /// # async fn example() -> datafusion_common::Result<()> {
    /// let ctx = SessionContext::new();
    /// let df = ctx.read_csv_file("data/users.csv").await?;
    /// df.show().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn read_csv_file(&self, path: &str) -> Result<DataFrame>;

    /// Read a CSV file into a `DataFrame` with custom options
    ///
    /// # Example
    ///
    /// ```no_run
    /// use datafusion::prelude::*;
    /// use datafusion_csv::{SessionContextCsvExt, CsvFormatOptions};
    ///
    /// # async fn example() -> datafusion_common::Result<()> {
    /// let ctx = SessionContext::new();
    /// let options = CsvFormatOptions::new().with_delimiter(b'\t');
    /// let df = ctx.read_csv_with_options("data.tsv", options).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn read_csv_with_options(
        &self,
        path: &str,
        options: CsvFormatOptions,
    ) -> Result<DataFrame>;
}

impl SessionContextCsvExt for SessionContext {
    async fn register_csv_file(&self, name: &str, path: &str) -> Result<()> {
        let options = CsvFormatOptions::default();
        self.register_csv_with_options(name, path, options).await
    }

    async fn register_csv_with_delimiter(
        &self,
        name: &str,
        path: &str,
        delimiter: u8,
    ) -> Result<()> {
        let options = CsvFormatOptions::default().with_delimiter(delimiter);
        self.register_csv_with_options(name, path, options).await
    }

    async fn register_csv_with_options(
        &self,
        name: &str,
        path: &str,
        options: CsvFormatOptions,
    ) -> Result<()> {
        let table = file_source::create_csv_table_provider(&self.state(), path, options).await?;
        self.register_table(name, table)?;
        Ok(())
    }

    async fn read_csv_file(&self, path: &str) -> Result<DataFrame> {
        let options = CsvFormatOptions::default();
        self.read_csv_with_options(path, options).await
    }

    async fn read_csv_with_options(
        &self,
        path: &str,
        options: CsvFormatOptions,
    ) -> Result<DataFrame> {
        let table = file_source::create_csv_table_provider(&self.state(), path, options).await?;
        self.read_table(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_register_csv_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("test.csv");

        // Create a simple CSV file
        let mut file = File::create(&csv_path).unwrap();
        writeln!(file, "id,name,value").unwrap();
        writeln!(file, "1,Alice,100").unwrap();
        writeln!(file, "2,Bob,200").unwrap();

        let ctx = SessionContext::new();
        ctx.register_csv_file("test_table", csv_path.to_str().unwrap())
            .await?;

        let df = ctx.sql("SELECT * FROM test_table WHERE id = 1").await?;
        let batches = df.collect().await?;

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_read_csv_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("test.csv");

        // Create a simple CSV file
        let mut file = File::create(&csv_path).unwrap();
        writeln!(file, "id,name,value").unwrap();
        writeln!(file, "1,Alice,100").unwrap();
        writeln!(file, "2,Bob,200").unwrap();
        writeln!(file, "3,Charlie,300").unwrap();

        let ctx = SessionContext::new();
        let df = ctx.read_csv_file(csv_path.to_str().unwrap()).await?;

        let batches = df.collect().await?;
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_custom_delimiter() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("test.csv");

        // Create a CSV file with tab delimiter
        let mut file = File::create(&csv_path).unwrap();
        writeln!(file, "id\tname\tvalue").unwrap();
        writeln!(file, "1\tAlice\t100").unwrap();
        writeln!(file, "2\tBob\t200").unwrap();

        let ctx = SessionContext::new();
        ctx.register_csv_with_delimiter("test_table", csv_path.to_str().unwrap(), b'\t')
            .await?;

        let df = ctx.sql("SELECT * FROM test_table").await?;
        let batches = df.collect().await?;

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_custom_options() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("test.csv");

        // Create a CSV file
        let mut file = File::create(&csv_path).unwrap();
        writeln!(file, "id,name,value").unwrap();
        writeln!(file, "1,Alice,100").unwrap();

        let ctx = SessionContext::new();
        let options = CsvFormatOptions::new().with_batch_size(1024);

        ctx.register_csv_with_options("test_table", csv_path.to_str().unwrap(), options)
            .await?;

        let df = ctx.sql("SELECT * FROM test_table").await?;
        let batches = df.collect().await?;

        assert_eq!(batches[0].num_rows(), 1);

        Ok(())
    }
}
