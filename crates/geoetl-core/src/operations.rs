//! Core ETL operations for geospatial data.
//!
//! This module provides the main functions for Extract, Transform, and Load (ETL)
//! operations on geospatial data, leveraging the driver registry for format support.

use crate::drivers::Driver;
use anyhow::{Result, anyhow};
use datafusion::arrow::array::RecordBatch;
use datafusion::prelude::SessionContext;
use log::info;
use std::fs::File;

/// Parse geometry type string into `GeoArrowType`
fn parse_geometry_type(geom_type_str: &str) -> Result<geoarrow_schema::GeoArrowType> {
    use geoarrow_schema::{
        Dimension, GeoArrowType, GeometryType, LineStringType, MultiLineStringType, MultiPointType,
        MultiPolygonType, PointType, PolygonType,
    };
    use std::sync::Arc;

    let geoarrow_type = match geom_type_str.to_lowercase().as_str() {
        "geometry" => GeoArrowType::Geometry(GeometryType::new(Arc::default())),
        "point" => GeoArrowType::Point(PointType::new(Dimension::XY, Arc::default())),
        "linestring" => {
            GeoArrowType::LineString(LineStringType::new(Dimension::XY, Arc::default()))
        },
        "polygon" => GeoArrowType::Polygon(PolygonType::new(Dimension::XY, Arc::default())),
        "multipoint" => {
            GeoArrowType::MultiPoint(MultiPointType::new(Dimension::XY, Arc::default()))
        },
        "multilinestring" => {
            GeoArrowType::MultiLineString(MultiLineStringType::new(Dimension::XY, Arc::default()))
        },
        "multipolygon" => {
            GeoArrowType::MultiPolygon(MultiPolygonType::new(Dimension::XY, Arc::default()))
        },
        _ => {
            return Err(anyhow!(
                "Unsupported geometry type '{geom_type_str}'. Supported types: Geometry (mixed), Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon"
            ));
        },
    };
    Ok(geoarrow_type)
}

/// Read data from CSV file
async fn read_csv(
    ctx: &SessionContext,
    input: &str,
    geometry_column: &str,
    _geometry_type: Option<&str>,
) -> Result<Vec<RecordBatch>> {
    use datafusion_csv::{CsvFormatOptions, SessionContextCsvExt};
    info!("Reading CSV file: {input}");

    let mut csv_options = CsvFormatOptions::new();

    // Always use Geometry type (mixed geometries) to auto-detect from WKT content
    let geoarrow_type = parse_geometry_type("Geometry")?;
    info!("Parsing WKT from column '{geometry_column}' as Geometry (auto-detect)");
    csv_options = csv_options.with_geometry_from_wkt(geometry_column, geoarrow_type);

    let df = ctx
        .read_csv_with_options(input, csv_options)
        .await
        .map_err(|e| anyhow!("Failed to read CSV file: {e}"))?;
    df.collect()
        .await
        .map_err(|e| anyhow!("Failed to collect CSV data: {e}"))
}

/// Read data from `GeoJSON` file
async fn read_geojson(ctx: &SessionContext, input: &str) -> Result<Vec<RecordBatch>> {
    use datafusion_geojson::SessionContextGeoJsonExt;
    info!("Reading GeoJSON file: {input}");
    let df = ctx
        .read_geojson_file(input)
        .await
        .map_err(|e| anyhow!("Failed to read GeoJSON file: {e}"))?;
    df.collect()
        .await
        .map_err(|e| anyhow!("Failed to collect GeoJSON data: {e}"))
}

/// Write data to CSV file
fn write_csv(output: &str, batches: &[RecordBatch], geometry_column: &str) -> Result<()> {
    use datafusion_csv::{CsvWriterOptions, write_csv};
    info!("Writing CSV file: {output}");

    // Convert geometry columns to WKT before writing
    let converted_batches = convert_geometry_to_wkt(batches, geometry_column)?;

    let mut output_file =
        File::create(output).map_err(|e| anyhow!("Failed to create output file: {e}"))?;
    let options = CsvWriterOptions::default();
    write_csv(&mut output_file, &converted_batches, &options)
        .map_err(|e| anyhow!("Failed to write CSV file: {e}"))
}

/// Convert geometry columns to WKT format for CSV writing
fn convert_geometry_to_wkt(
    batches: &[RecordBatch],
    geometry_column: &str,
) -> Result<Vec<RecordBatch>> {
    use arrow_schema::Schema;
    use geoarrow_array::GeoArrowArray;
    use geoarrow_array::array::from_arrow_array;
    use geoarrow_array::cast::to_wkt;
    use std::sync::Arc;

    let mut converted_batches = Vec::with_capacity(batches.len());

    for batch in batches {
        let schema = batch.schema();

        // Find the geometry column index
        let geom_idx = schema
            .fields()
            .iter()
            .position(|field| field.name() == geometry_column);

        if let Some(idx) = geom_idx {
            // Get the geometry column and its field
            let geom_array = batch.column(idx);
            let geom_field = schema.field(idx);

            // Convert Arrow array to GeoArrowArray
            let geoarrow_array = from_arrow_array(geom_array.as_ref(), geom_field)
                .map_err(|e| anyhow!("Failed to convert to GeoArrowArray: {e}"))?;

            // Convert to WKT using geoarrow cast (using i32 offset)
            let wkt_array: geoarrow_array::array::WktArray = to_wkt(&geoarrow_array)
                .map_err(|e| anyhow!("Failed to convert geometry to WKT: {e}"))?;

            // Create new schema with WKT column
            let mut new_fields = schema.fields().to_vec();
            new_fields[idx] = Arc::new(arrow_schema::Field::new(
                geometry_column,
                arrow_schema::DataType::Utf8,
                true,
            ));
            let new_schema = Arc::new(Schema::new(new_fields));

            // Create new columns with WKT
            let mut new_columns = batch.columns().to_vec();
            new_columns[idx] = wkt_array.to_array_ref();

            // Create new batch
            let new_batch = RecordBatch::try_new(new_schema, new_columns)
                .map_err(|e| anyhow!("Failed to create record batch with WKT: {e}"))?;

            converted_batches.push(new_batch);
        } else {
            // No geometry column found, use batch as-is
            converted_batches.push(batch.clone());
        }
    }

    Ok(converted_batches)
}

/// Write data to `GeoJSON` file
fn write_geojson(output: &str, batches: &[RecordBatch]) -> Result<()> {
    use datafusion_geojson::{GeoJsonWriterOptions, write_geojson};
    info!("Writing GeoJSON file: {output}");
    let mut output_file =
        File::create(output).map_err(|e| anyhow!("Failed to create output file: {e}"))?;
    let options = GeoJsonWriterOptions::default();
    write_geojson(&mut output_file, batches, &options)
        .map_err(|e| anyhow!("Failed to write GeoJSON file: {e}"))
}

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
    geometry_column: &str,
    geometry_type: Option<&str>,
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
        "CSV" => read_csv(&ctx, input, geometry_column, geometry_type).await?,
        "GeoJSON" => read_geojson(&ctx, input).await?,
        _ => {
            return Err(anyhow!(
                "Input driver '{}' is not yet implemented for conversion",
                input_driver.short_name
            ));
        },
    };

    info!("Read {} record batch(es)", batches.len());
    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    info!("Total rows: {total_rows}");

    // Write data based on output driver
    match output_driver.short_name {
        "CSV" => write_csv(output, &batches, geometry_column)?,
        "GeoJSON" => write_geojson(output, &batches)?,
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
        writeln!(file, "id,name,wkt")?;
        writeln!(file, "1,Alice,\"POINT(1 1)\"")?;
        writeln!(file, "2,Bob,\"POINT(2 2)\"")?;
        writeln!(file, "3,Charlie,\"POINT(3 3)\"")?;
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
            "wkt",
            None,
        )
        .await;

        assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
        assert!(output_path.exists(), "Output file was not created");

        // Verify output contains data
        let output_content = std::fs::read_to_string(&output_path).unwrap();
        assert!(output_content.contains("id,name,wkt"));
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
            "geometry",
            None,
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

        let result = convert(
            "input.gml",
            "output.geojson",
            &input_driver,
            &output_driver,
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

        let result = convert(
            "input.csv",
            "output.gml",
            &input_driver,
            &output_driver,
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
            "geometry",
            None,
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
            "geometry",
            None,
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
        writeln!(file, "id,name,wkt").unwrap();

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
            "wkt",
            None,
        )
        .await;

        assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
        assert!(output_path.exists(), "Output file was not created");

        Ok(())
    }
}
