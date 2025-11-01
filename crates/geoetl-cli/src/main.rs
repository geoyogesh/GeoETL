//! Command-line interface for `GeoETL`, a high-performance geospatial data processing tool.
//!
//! This binary provides a user-friendly CLI to interact with the [`geoetl_core`] library,
//! enabling users to perform geospatial ETL (Extract, Transform, Load) operations on
//! vector data formats.
//!
//! # Architecture
//!
//! The CLI is built using [`clap`] for argument parsing and [`tracing`] for structured logging.
//! It currently acts as a thin façade that parses arguments, configures logging, and delegates
//! to command handlers. The full ETL pipeline implementation is under active development.
//!
//! # Available Commands
//!
//! - `convert` - Convert data between geospatial formats
//! - `info` - Display dataset information and metadata
//! - `drivers` - List all available format drivers and their capabilities

mod display;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use tabled::Table;
use tracing::{Level, info};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;

use geoetl_core::drivers::get_available_drivers;

use display::{DriverRow, display_dataset_info};

#[derive(Parser)]
#[command(
    name = "geoetl",
    version,
    about = "Modern vector geospatial ETL in Rust",
    long_about = "GeoETL is a high-performance CLI tool for spatial data conversion and processing.\n\
                  Built to be 5-10x faster than GDAL with distributed processing support."
)]
/// Command-line arguments and options for the `GeoETL` CLI.
///
/// This struct defines the top-level CLI interface, including global flags for
/// logging verbosity and the subcommand to execute.
struct Cli {
    /// Enable verbose (INFO level) logging output.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Enable debug (DEBUG level) logging output with detailed diagnostics.
    #[arg(short, long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands for the `GeoETL` CLI.
///
/// Each variant represents a distinct operation that can be performed on
/// geospatial datasets, such as format conversion, metadata inspection, or
/// driver enumeration.
#[derive(Subcommand)]
enum Commands {
    /// Converts data between different vector geospatial formats.
    ///
    /// This command takes an input dataset and converts it to an output dataset,
    /// specifying the input and output drivers.
    Convert {
        /// Path to the input geospatial dataset.
        #[arg(short, long, value_name = "DATASET")]
        input: String,

        /// Path for the output geospatial dataset.
        #[arg(short, long, value_name = "DATASET")]
        output: String,

        /// The driver to use for reading the input dataset (e.g., "`GeoJSON`", "`Parquet`").
        #[arg(long, value_name = "DRIVER")]
        input_driver: String,

        /// The driver to use for writing the output dataset (e.g., "`GeoJSON`", "`Parquet`").
        #[arg(long, value_name = "DRIVER")]
        output_driver: String,

        /// Name of the geometry column in the input dataset (default: "geometry").
        /// For CSV files, this should be the column containing WKT geometry strings.
        #[arg(long, value_name = "COLUMN", default_value = "geometry")]
        geometry_column: String,

        /// Geometry type for the input geometry column (e.g., "`Point`", "`LineString`", "`Polygon`").
        /// Only required when converting from CSV with WKT geometries to `GeoJSON`.
        #[arg(long, value_name = "TYPE")]
        geometry_type: Option<String>,
    },

    /// Displays information about a vector geospatial dataset.
    ///
    /// This command shows general information, detailed layer information,
    /// and statistics for each field within the dataset.
    Info {
        /// Path to the input geospatial dataset.
        #[arg(value_name = "DATASET")]
        input: String,

        /// Input driver (e.g., `GeoJSON`, `CSV`, `Parquet`).
        #[arg(short = 'f', long, value_name = "DRIVER")]
        driver: String,

        /// Name of the geometry column in the input dataset.
        /// For CSV files, this should be the column containing WKT geometry strings.
        /// Required for CSV format, optional for other formats (defaults to "geometry").
        #[arg(long, value_name = "COLUMN")]
        geometry_column: Option<String>,

        /// Geometry type for the input geometry column (e.g., "`Point`", "`LineString`", "`Polygon`").
        /// Only used when reading CSV files with WKT geometries.
        #[arg(long, value_name = "TYPE")]
        geometry_type: Option<String>,
    },

    /// Lists all available geospatial drivers and their capabilities.
    ///
    /// This command provides an overview of which formats can be read from,
    /// written to, and provide metadata information.
    Drivers,
}

/// Entry point for the `GeoETL` command-line interface.
///
/// This function parses command-line arguments, configures the logging system based on
/// verbosity flags, and dispatches to the appropriate command handler.
///
/// # Errors
///
/// Returns an error if command execution fails or if the logging system cannot be initialized.
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging based on verbosity flags
    let log_level = if cli.debug {
        Level::DEBUG
    } else if cli.verbose {
        Level::INFO
    } else {
        Level::WARN
    };

    // Bridge logs from the `log` crate to the `tracing` ecosystem.
    LogTracer::init()?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true) // Show module paths for better context
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Execute the command
    match cli.command {
        Commands::Convert {
            input,
            output,
            input_driver,
            output_driver,
            geometry_column,
            geometry_type,
        } => {
            info!("Converting {input} to {output}");
            handle_convert(
                &input,
                &output,
                &input_driver,
                &output_driver,
                &geometry_column,
                geometry_type.as_deref(),
            )
            .await?;
        },
        Commands::Info {
            input,
            driver,
            geometry_column,
            geometry_type,
        } => {
            info!("Displaying info for {input}");
            handle_info(
                &input,
                &driver,
                geometry_column.as_deref(),
                geometry_type.as_deref(),
            )
            .await?;
        },
        Commands::Drivers => {
            handle_drivers()?;
        },
    }

    Ok(())
}

use geoetl_core::drivers;
use geoetl_core::operations;

async fn handle_convert(
    input: &str,
    output: &str,
    input_driver_name: &str,
    output_driver_name: &str,
    geometry_column: &str,
    geometry_type: Option<&str>,
) -> Result<()> {
    info!("Validating convert command:");
    info!("Input: {input}");
    info!("Output: {output}");
    info!("Input driver: {input_driver_name}");
    info!("Output driver: {output_driver_name}");
    info!("Geometry column: {geometry_column}");
    if let Some(geom_type) = geometry_type {
        info!("Geometry type: {geom_type}");
    }

    let input_driver = drivers::find_driver(input_driver_name)
        .ok_or_else(|| anyhow!("Input driver '{input_driver_name}' not found."))?;

    if !input_driver.capabilities.read.is_supported() {
        return Err(anyhow!(
            "Input driver '{input_driver_name}' does not support reading."
        ));
    }

    let output_driver = drivers::find_driver(output_driver_name)
        .ok_or_else(|| anyhow!("Output driver '{output_driver_name}' not found."))?;

    if !output_driver.capabilities.write.is_supported() {
        return Err(anyhow!(
            "Output driver '{output_driver_name}' does not support writing."
        ));
    }

    info!("Convert command:");
    operations::convert(
        input,
        output,
        &input_driver,
        &output_driver,
        geometry_column,
        geometry_type,
    )
    .await?;
    info!("Conversion complete.");
    Ok(())
}

async fn handle_info(
    input: &str,
    driver_name: &str,
    geometry_column: Option<&str>,
    geometry_type: Option<&str>,
) -> Result<()> {
    info!("Info command:");
    info!("Input: {input}");
    info!("Driver: {driver_name}");

    // Resolve the input path to an absolute path
    let input_path = std::path::Path::new(input);
    let absolute_path = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(input_path)
    };

    // Convert to string for use with operations
    let resolved_input = absolute_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid path: could not convert path to string"))?;

    // Verify file exists
    if !absolute_path.exists() {
        return Err(anyhow!(
            "File not found: {input}\nResolved path: {resolved_input}"
        ));
    }

    // Find the specified driver
    let driver = drivers::find_driver(driver_name).ok_or_else(|| {
        anyhow!(
            "Driver '{driver_name}' not found. Use 'geoetl-cli drivers' to list available drivers."
        )
    })?;

    // Validate driver supports info or read operations
    if !driver.capabilities.info.is_supported() && !driver.capabilities.read.is_supported() {
        return Err(anyhow!(
            "Driver '{}' does not support info or read operations.",
            driver.short_name
        ));
    }

    // Validate geometry column is provided for CSV
    let geometry_col = if driver.short_name == "CSV" {
        geometry_column.ok_or_else(|| {
            anyhow!(
                "The --geometry-column argument is required for CSV files.\n\
                 Example: geoetl-cli info {input} -f CSV --geometry-column wkt"
            )
        })?
    } else {
        geometry_column.unwrap_or("geometry")
    };

    // Get dataset information
    let dataset_info =
        operations::info(resolved_input, &driver, geometry_col, geometry_type).await?;

    // Display dataset information using tables
    display_dataset_info(&dataset_info);

    Ok(())
}

/// Handles the `drivers` subcommand by displaying a formatted table of available drivers.
///
/// Retrieves all drivers with at least one supported operation from the driver registry
/// and presents their capabilities (info, read, write) in a human-readable table format
/// written to standard output.
///
/// # Errors
///
/// This function returns a `Result` for consistency with other command handlers,
/// but does not currently perform any operations that fail, so it always returns `Ok(())`.
#[allow(clippy::unnecessary_wraps)]
fn handle_drivers() -> Result<()> {
    let drivers = get_available_drivers();

    println!("\nAvailable Drivers ({} total):\n", drivers.len());

    let rows: Vec<DriverRow> = drivers
        .iter()
        .map(|d| DriverRow {
            short_name: d.short_name.to_string(),
            long_name: d.long_name.to_string(),
            info: d.capabilities.info.as_str().to_string(),
            read: d.capabilities.read.as_str().to_string(),
            write: d.capabilities.write.as_str().to_string(),
        })
        .collect();

    let table = Table::new(rows).to_string();
    println!("{table}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_convert_valid_drivers() -> Result<()> {
        // This test relies on the `operations::convert` being a placeholder
        // that returns Ok(()). Once actual conversion is implemented, this
        // test might need to be updated to mock the conversion.
        let input_driver_name = "CSV";
        let output_driver_name = "GeoJSON";

        let result = handle_convert(
            "input.csv",
            "output.geojson",
            input_driver_name,
            output_driver_name,
            "geometry",
            None,
        )
        .await;
        assert!(result.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_convert_invalid_input_driver() -> Result<()> {
        let input_driver_name = "NonExistentDriver";
        let output_driver_name = "GeoJSON";

        let result = handle_convert(
            "input.csv",
            "output.geojson",
            input_driver_name,
            output_driver_name,
            "geometry",
            None,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Input driver 'NonExistentDriver' not found."
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_convert_input_driver_no_read_support() -> Result<()> {
        let input_driver_name = "GML"; // GML does not support read
        let output_driver_name = "GeoJSON";

        let result = handle_convert(
            "input.gml",
            "output.geojson",
            input_driver_name,
            output_driver_name,
            "geometry",
            None,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Input driver 'GML' does not support reading."
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_convert_invalid_output_driver() -> Result<()> {
        let input_driver_name = "CSV";
        let output_driver_name = "NonExistentDriver";

        let result = handle_convert(
            "input.csv",
            "output.geojson",
            input_driver_name,
            output_driver_name,
            "geometry",
            None,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Output driver 'NonExistentDriver' not found."
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_convert_output_driver_no_write_support() -> Result<()> {
        let input_driver_name = "CSV";
        let output_driver_name = "GML"; // GML does not support write

        let result = handle_convert(
            "input.csv",
            "output.gml",
            input_driver_name,
            output_driver_name,
            "geometry",
            None,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Output driver 'GML' does not support writing."
        );
        Ok(())
    }
}
