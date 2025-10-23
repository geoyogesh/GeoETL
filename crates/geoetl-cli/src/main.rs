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

#[derive(Subcommand)]
enum Commands {
    /// Convert between different vector formats
    Convert {
        /// Input dataset path
        #[arg(short, long, value_name = "DATASET")]
        input: String,

        /// Output dataset path
        #[arg(short, long, value_name = "DATASET")]
        output: String,

        /// Input driver
        #[arg(long, value_name = "DRIVER")]
        input_driver: String,

        /// Output driver
        #[arg(long, value_name = "DRIVER")]
        output_driver: String,
    },

    /// Display information about a vector dataset
    Info {
        /// Input dataset path
        #[arg(value_name = "DATASET")]
        input: String,

        /// Show detailed layer information
        #[arg(long)]
        detailed: bool,

        /// Show statistics for each field
        #[arg(short, long)]
        stats: bool,
    },

    /// List available drivers and their capabilities
    Drivers,
}

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
            info!("Converting {} to {}", input, output);
            handle_convert(input, output, input_driver, output_driver)?;
        },
        Commands::Info {
            input,
            detailed,
            stats,
        } => {
            info!("Displaying info for {}", input);
            handle_info(input, detailed, stats)?;
        },
        Commands::Drivers => {
            handle_drivers()?;
        },
    }

    Ok(())
}

fn handle_convert(
    input: String,
    output: String,
    input_driver: String,
    output_driver: String,
) -> Result<()> {
    info!("Convert command:");
    info!("Input: {}", input);
    info!("Output: {}", output);
    info!("Input driver: {}", input_driver);
    info!("Output driver: {}", output_driver);
    warn!("Not yet implemented - Phase 1 development");
    Ok(())
}

fn handle_info(input: String, detailed: bool, stats: bool) -> Result<()> {
    info!("Info command:");
    info!("Input: {}", input);
    debug!("Detailed: {}", detailed);
    debug!("Stats: {}", stats);
    warn!("Not yet implemented - Phase 1 development");
    Ok(())
}

#[derive(Tabled)]
struct DriverRow {
    #[tabled(rename = "Short Name")]
    short_name: String,
    #[tabled(rename = "Long Name")]
    long_name: String,
    #[tabled(rename = "Info")]
    info: String,
    #[tabled(rename = "Read")]
    read: String,
    #[tabled(rename = "Write")]
    write: String,
}

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
    println!("{}", table);

    Ok(())
}
