//! `geoetl-cli` is the command-line interface for `GeoETL`, a high-performance tool for spatial data
//! conversion and processing.
//!
//! This binary provides a user-friendly interface to interact with the `geoetl-core` library,
//! allowing users to perform various geospatial ETL operations.
//!
//! Currently, this binary acts as a thin façade, parsing CLI arguments, configuring logging,
//! and delegating to placeholder command handlers. The full ETL pipeline implementation
//! is under active development.

use anyhow::Result;
use clap::{Parser, Subcommand};
use tabled::{Table, Tabled};
use tracing::{Level, debug, info, warn};
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
/// Top-level options accepted by the `geoetl` CLI.
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Supported `GeoETL` subcommands.
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

        /// The driver to use for reading the input dataset (e.g., "GeoJSON", "Parquet").
        #[arg(long, value_name = "DRIVER")]
        input_driver: String,

        /// The driver to use for writing the output dataset (e.g., "GeoJSON", "Parquet").
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

/// Entrypoint for the `geoetl` CLI.
fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging based on verbosity flags
    let log_level = if cli.debug {
        Level::DEBUG
    } else if cli.verbose {
        Level::INFO
    } else {
        Level::WARN
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
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
            handle_convert(&input, &output, &input_driver, &output_driver)?;
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

/// Handles the `convert` subcommand. Currently a placeholder.
///
/// This function will eventually orchestrate the ETL process for converting
/// geospatial data from an input format to an output format using specified drivers.
///
/// # Arguments
///
/// * `input` - The path to the input dataset.
/// * `output` - The path to the output dataset.
/// * `input_driver` - The name of the driver to use for reading the input.
/// * `output_driver` - The name of the driver to use for writing the output.
///
/// # Errors
///
/// This function currently returns a `Result` but does not perform any actual
/// conversion, so it will always return `Ok(())`.
#[allow(clippy::unnecessary_wraps)] // Placeholder until command execution is implemented
fn handle_convert(
    input: &str,
    output: &str,
    input_driver: &str,
    output_driver: &str,
) -> Result<()> {
    info!("Convert command:");
    info!("Input: {input}");
    info!("Output: {output}");
    info!("Input driver: {input_driver}");
    info!("Output driver: {output_driver}");
    warn!("Not yet implemented - Phase 1 development");
    Ok(())
}

/// Handles the `info` subcommand. Currently a placeholder.
///
/// This function will eventually display detailed information about a geospatial dataset,
/// including layer details and field statistics.
///
/// # Arguments
///
/// * `input` - The path to the input dataset.
/// * `detailed` - Whether to show detailed layer information.
/// * `stats` - Whether to show statistics for each field.
///
/// # Errors
///
/// This function currently returns a `Result` but does not perform any actual
/// information retrieval, so it will always return `Ok(())`.
#[allow(clippy::unnecessary_wraps)] // Placeholder until command execution is implemented
fn handle_info(input: &str, detailed: bool, stats: bool) -> Result<()> {
    info!("Info command:");
    info!("Input: {input}");
    debug!("Detailed: {detailed}");
    debug!("Stats: {stats}");
    warn!("Not yet implemented - Phase 1 development");
    Ok(())
}

/// A helper struct used to format and display driver metadata in a table.
#[derive(Tabled)]
struct DriverRow {
    /// The short name of the driver.
    #[tabled(rename = "Short Name")]
    short_name: String,
    /// The long, descriptive name of the driver.
    #[tabled(rename = "Long Name")]
    long_name: String,
    /// The support status for providing dataset information.
    #[tabled(rename = "Info")]
    info: String,
    /// The support status for reading data from the driver.
    #[tabled(rename = "Read")]
    read: String,
    /// The support status for writing data to the driver.
    #[tabled(rename = "Write")]
    write: String,
}

/// Handles the `drivers` subcommand, emitting a formatted table of available drivers.
///
/// This function retrieves the list of all known drivers from `geoetl-core` and
/// presents their capabilities in a human-readable table format to standard output.
///
/// # Errors
///
/// This function currently returns a `Result` but does not perform any operations
/// that would typically fail, so it will always return `Ok(())`.
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
