//! Core ETL operations for geospatial data.
//!
//! This module provides the main functions for Extract, Transform, and Load (ETL)
//! operations on geospatial data, leveraging the driver registry for format support.

use crate::drivers::Driver;
use anyhow::{Result, anyhow};
use datafusion::prelude::SessionContext;
use log::info;
use std::fs::File;

/// Performs a geospatial data conversion from an input format to an output format.
///
/// This function orchestrates the reading of data from the `input` path using the
/// `input_driver` and writing it to the `output` path using the `output_driver`.
///
/// # Arguments
///
/// * `input` - The path to the input geospatial data file.
/// * `output` - The path where the converted geospatial data will be written.
/// * `input_driver` - The driver responsible for reading the input format.
/// * `output_driver` - The driver responsible for writing the output format.
///
/// # Returns
///
/// A `Result` indicating success or an error if the conversion fails.
///
/// # Errors
///
/// This function will return an error if:
/// - The `input_driver` does not support reading.
/// - The `output_driver` does not support writing.
/// - The actual conversion logic (once implemented) encounters an error.
pub async fn convert(
    input: &str,
    output: &str,
    input_driver: &Driver,
    output_driver: &Driver,
) -> Result<()> {
    info!("Starting conversion:");
    info!("Input: {} (Driver: {})", input, input_driver.short_name);
    info!("Output: {} (Driver: {})", output, output_driver.short_name);

    // Basic validation (already done in CLI, but good to have here too)
    if !input_driver.capabilities.read.is_supported() {
        return Err(anyhow!(
            "Input driver \'{}\' does not support reading.",
            input_driver.short_name
        ));
    }
    if !output_driver.capabilities.write.is_supported() {
        return Err(anyhow!(
            "Output driver \'{}\' does not support writing.",
            output_driver.short_name
        ));
    }

    // Create DataFusion session context
    let ctx = SessionContext::new();

    // Read data based on input driver
    let batches = match input_driver.short_name {
        "CSV" => {
            use datafusion_csv::SessionContextCsvExt;
            info!("Reading CSV file: {input}");
            let df = ctx
                .read_csv_file(input)
                .await
                .map_err(|e| anyhow!("Failed to read CSV file: {e}"))?;
            df.collect()
                .await
                .map_err(|e| anyhow!("Failed to collect CSV data: {e}"))?
        },
        "GeoJSON" => {
            use datafusion_geojson::SessionContextGeoJsonExt;
            info!("Reading GeoJSON file: {input}");
            let df = ctx
                .read_geojson_file(input)
                .await
                .map_err(|e| anyhow!("Failed to read GeoJSON file: {e}"))?;
            df.collect()
                .await
                .map_err(|e| anyhow!("Failed to collect GeoJSON data: {e}"))?
        },
        _ => {
            return Err(anyhow!(
                "Input driver '{}' is not yet implemented for conversion",
                input_driver.short_name
            ));
        },
    };

    info!("Read {} record batch(es)", batches.len());
    let total_rows: usize = batches
        .iter()
        .map(datafusion::arrow::array::RecordBatch::num_rows)
        .sum();
    info!("Total rows: {total_rows}");

    // Write data based on output driver
    match output_driver.short_name {
        "CSV" => {
            use datafusion_csv::{CsvWriterOptions, write_csv};
            info!("Writing CSV file: {output}");
            let mut output_file =
                File::create(output).map_err(|e| anyhow!("Failed to create output file: {e}"))?;
            let options = CsvWriterOptions::default();
            write_csv(&mut output_file, &batches, &options)
                .map_err(|e| anyhow!("Failed to write CSV file: {e}"))?;
        },
        "GeoJSON" => {
            use datafusion_geojson::{GeoJsonWriterOptions, write_geojson};
            info!("Writing GeoJSON file: {output}");
            let mut output_file =
                File::create(output).map_err(|e| anyhow!("Failed to create output file: {e}"))?;
            let options = GeoJsonWriterOptions::default();
            write_geojson(&mut output_file, &batches, &options)
                .map_err(|e| anyhow!("Failed to write GeoJSON file: {e}"))?;
        },
        _ => {
            return Err(anyhow!(
                "Output driver '{}' is not yet implemented for conversion",
                output_driver.short_name
            ));
        },
    }

    info!("Conversion completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::{Driver, SupportStatus};
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper function to create test CSV data
    fn create_test_csv(path: &std::path::Path) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        writeln!(file, "id,name,value")?;
        writeln!(file, "1,Alice,100")?;
        writeln!(file, "2,Bob,200")?;
        writeln!(file, "3,Charlie,300")?;
        Ok(())
    }

    /// Helper function to create test `GeoJSON` data
    fn create_test_geojson(path: &std::path::Path) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        writeln!(
            file,
            r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [-74.0060, 40.7128]
      }},
      "properties": {{
        "name": "New York",
        "population": 8336817
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [-118.2437, 34.0522]
      }},
      "properties": {{
        "name": "Los Angeles",
        "population": 3979576
      }}
    }}
  ]
}}"#
        )?;
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_csv_to_csv() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("input.csv");
        let output_path = temp_dir.path().join("output.csv");

        // Create test input file
        create_test_csv(&input_path).unwrap();

        let input_driver = Driver::new(
            "CSV",
            "Comma Separated Value (.csv)",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );
        let output_driver = Driver::new(
            "CSV",
            "Comma Separated Value (.csv)",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );

        let result = convert(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            &input_driver,
            &output_driver,
        )
        .await;

        assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
        assert!(output_path.exists(), "Output file was not created");

        // Verify output contains data
        let output_content = std::fs::read_to_string(&output_path).unwrap();
        assert!(output_content.contains("id,name,value"));
        assert!(output_content.contains("Alice"));
        assert!(output_content.contains("Bob"));
        assert!(output_content.contains("Charlie"));

        Ok(())
    }

    #[tokio::test]
    async fn test_convert_geojson_to_geojson() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("input.geojson");
        let output_path = temp_dir.path().join("output.geojson");

        // Create test input file
        create_test_geojson(&input_path).unwrap();

        let input_driver = Driver::new(
            "GeoJSON",
            "GeoJSON",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );
        let output_driver = Driver::new(
            "GeoJSON",
            "GeoJSON",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );

        let result = convert(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            &input_driver,
            &output_driver,
        )
        .await;

        assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
        assert!(output_path.exists(), "Output file was not created");

        // Verify output is valid JSON
        let output_content = std::fs::read_to_string(&output_path).unwrap();
        assert!(output_content.contains("FeatureCollection"));
        assert!(output_content.contains("New York"));
        assert!(output_content.contains("Los Angeles"));

        Ok(())
    }

    #[tokio::test]
    async fn test_convert_unsupported_input_read() -> Result<()> {
        let input_driver = Driver::new(
            "GML",
            "Geography Markup Language",
            SupportStatus::NotSupported,
            SupportStatus::NotSupported,
            SupportStatus::NotSupported,
        );
        let output_driver = Driver::new(
            "GeoJSON",
            "GeoJSON",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );

        let result = convert("input.gml", "output.geojson", &input_driver, &output_driver).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Input driver 'GML' does not support reading."
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_unsupported_output_write() -> Result<()> {
        let input_driver = Driver::new(
            "CSV",
            "Comma Separated Value (.csv)",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );
        let output_driver = Driver::new(
            "GML",
            "Geography Markup Language",
            SupportStatus::NotSupported,
            SupportStatus::NotSupported,
            SupportStatus::NotSupported,
        );

        let result = convert("input.csv", "output.gml", &input_driver, &output_driver).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Output driver 'GML' does not support writing."
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_invalid_csv() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("invalid.csv");
        let output_path = temp_dir.path().join("output.csv");

        // Create invalid CSV file
        let mut file = File::create(&input_path).unwrap();
        writeln!(file, "id,name,value").unwrap();
        writeln!(file, "1,Alice").unwrap(); // Missing column
        writeln!(file, "invalid,data,here,extra").unwrap(); // Extra column

        let input_driver = Driver::new(
            "CSV",
            "Comma Separated Value (.csv)",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );
        let output_driver = Driver::new(
            "CSV",
            "Comma Separated Value (.csv)",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );

        let result = convert(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            &input_driver,
            &output_driver,
        )
        .await;

        // This might succeed or fail depending on CSV parser tolerance
        // Either outcome is acceptable for malformed data
        if result.is_err() {
            assert!(result.unwrap_err().to_string().contains("Failed to"));
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_unimplemented_driver() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("input.shp");
        let output_path = temp_dir.path().join("output.shp");

        let input_driver = Driver::new(
            "ESRI Shapefile",
            "ESRI Shapefile / DBF",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );
        let output_driver = Driver::new(
            "ESRI Shapefile",
            "ESRI Shapefile / DBF",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );

        let result = convert(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            &input_driver,
            &output_driver,
        )
        .await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("is not yet implemented for conversion")
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_empty_csv() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("empty.csv");
        let output_path = temp_dir.path().join("output.csv");

        // Create empty CSV with just headers
        let mut file = File::create(&input_path).unwrap();
        writeln!(file, "id,name,value").unwrap();

        let input_driver = Driver::new(
            "CSV",
            "Comma Separated Value (.csv)",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );
        let output_driver = Driver::new(
            "CSV",
            "Comma Separated Value (.csv)",
            SupportStatus::Supported,
            SupportStatus::Supported,
            SupportStatus::Supported,
        );

        let result = convert(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            &input_driver,
            &output_driver,
        )
        .await;

        assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
        assert!(output_path.exists(), "Output file was not created");

        Ok(())
    }
}
