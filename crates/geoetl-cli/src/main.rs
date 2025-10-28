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

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use tabled::{Table, Tabled};
use tracing::{Level, debug, info, warn};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;

use geoetl_core::drivers::get_available_drivers;

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
    },

    /// Displays information about a vector geospatial dataset.
    ///
    /// This command can show general information, detailed layer information,
    /// and statistics for each field within the dataset.
    Info {
        /// Path to the input geospatial dataset.
        #[arg(value_name = "DATASET")]
        input: String,

        /// Shows detailed information for each layer in the dataset.
        #[arg(long)]
        detailed: bool,

        /// Shows statistics (e.g., min, max, mean) for each field.
        #[arg(short, long)]
        stats: bool,
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
        } => {
            info!("Converting {input} to {output}");
            handle_convert(&input, &output, &input_driver, &output_driver).await?;
        },
        Commands::Info {
            input,
            detailed,
            stats,
        } => {
            info!("Displaying info for {input}");
            handle_info(&input, detailed, stats)?;
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
) -> Result<()> {
    info!("Validating convert command:");
    info!("Input: {input}");
    info!("Output: {output}");
    info!("Input driver: {input_driver_name}");
    info!("Output driver: {output_driver_name}");

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
    operations::convert(input, output, &input_driver, &output_driver).await?;
    info!("Conversion complete.");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // Placeholder until command execution is implemented
fn handle_info(input: &str, detailed: bool, stats: bool) -> Result<()> {
    info!("Info command:");
    info!("Input: {input}");
    debug!("Detailed: {detailed}");
    debug!("Stats: {stats}");
    warn!("Not yet implemented - Phase 1 development");
    Ok(())
}

/// Table row representation for displaying driver information.
///
/// This struct is used to format driver metadata into a human-readable table
/// using the [`tabled`] crate. Each field corresponds to a column in the output table.
#[derive(Tabled)]
struct DriverRow {
    /// Short identifier for the driver (e.g., `GeoJSON`, `Parquet`).
    #[tabled(rename = "Short Name")]
    short_name: String,
    /// Full descriptive name of the driver format.
    #[tabled(rename = "Long Name")]
    long_name: String,
    /// Support status for reading dataset metadata and information.
    #[tabled(rename = "Info")]
    info: String,
    /// Support status for reading data from this format.
    #[tabled(rename = "Read")]
    read: String,
    /// Support status for writing data to this format.
    #[tabled(rename = "Write")]
    write: String,
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
#[allow(clippy::unnecessary_wraps)] // Placeholder until command execution is implemented
#[allow(clippy::unnecessary_wraps)] // Placeholder until command execution is implemented
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
