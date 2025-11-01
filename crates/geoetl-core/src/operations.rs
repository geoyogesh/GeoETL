//! Core ETL operations for geospatial data.
//!
//! This module provides the main functions for Extract, Transform, and Load (ETL)
//! operations on geospatial data, leveraging the driver registry for format support.

use crate::drivers::Driver;
use crate::types::{DatasetInfo, FieldInfo, GeometryColumnInfo};
use crate::utils::ArrowDataTypeExt;
use anyhow::{Result, anyhow};
use datafusion::arrow::array::RecordBatch;
use datafusion::prelude::SessionContext;
use log::info;
use std::fs::File;

/// Initialize a `DataFusion` session context and register a dataset.
///
/// This is a common entry point for all ETL operations that need to work with a dataset.
/// It creates a new session context, registers the dataset with the specified parameters,
/// and returns the context ready for use.
///
/// # Arguments
///
/// * `input` - Path to the input file
/// * `driver` - The driver responsible for reading the format
/// * `geometry_column` - Name of the geometry column (for CSV)
/// * `geometry_type` - Optional geometry type hint (for CSV)
///
/// # Returns
///
/// A `SessionContext` with the dataset registered as "dataset" table.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read or parsed.
/// - The driver format is not yet implemented.
async fn initialize_context(
    input: &str,
    driver: &Driver,
    geometry_column: &str,
    geometry_type: Option<&str>,
) -> Result<SessionContext> {
    let ctx = SessionContext::new();
    let table_name = "dataset";
    register_catalog(
        &ctx,
        input,
        driver,
        table_name,
        geometry_column,
        geometry_type,
    )
    .await?;
    Ok(ctx)
}

/// Prepare format-specific options for reading.
///
/// This helper function creates the appropriate options object for each format type,
/// encapsulating format-specific configuration logic.
///
/// # Arguments
///
/// * `driver_name` - The short name of the driver (e.g., "`CSV`", "`GeoJSON`")
/// * `geometry_column` - Name of the geometry column (used for CSV)
/// * `geometry_type` - Optional geometry type hint (used for CSV)
///
/// # Returns
///
/// A boxed `Any` containing the format-specific options, or an error if the driver is unknown.
fn prepare_reader_options(
    driver_name: &str,
    geometry_column: &str,
    geometry_type: Option<&str>,
) -> Result<Box<dyn std::any::Any + Send>> {
    match driver_name {
        "CSV" => {
            use datafusion_csv::CsvFormatOptions;
            let mut options = CsvFormatOptions::new();
            let geoarrow_type = parse_geometry_type(geometry_type.unwrap_or("Geometry"))?;
            options = options.with_geometry_from_wkt(geometry_column, geoarrow_type);
            Ok(Box::new(options))
        },
        "GeoJSON" => {
            use datafusion_geojson::GeoJsonFormatOptions;
            Ok(Box::new(GeoJsonFormatOptions::default()))
        },
        _ => Err(anyhow!("Unknown driver '{driver_name}'")),
    }
}

/// Register a dataset in the `DataFusion` catalog.
///
/// This function handles the registration of different data formats (`CSV`, `GeoJSON`, etc.)
/// into a `DataFusion` session context, making them available for SQL queries or conversion.
/// Uses the factory pattern to dynamically dispatch to the appropriate format reader.
///
/// # Arguments
///
/// * `ctx` - The `DataFusion` session context
/// * `input` - Path to the input file
/// * `driver` - The driver responsible for reading the format
/// * `table_name` - Name to register the table as
/// * `geometry_column` - Name of the geometry column (for CSV)
/// * `geometry_type` - Optional geometry type hint (for CSV)
///
/// # Returns
///
/// A `Result` indicating success or an error if registration fails.
async fn register_catalog(
    ctx: &SessionContext,
    input: &str,
    driver: &Driver,
    table_name: &str,
    geometry_column: &str,
    geometry_type: Option<&str>,
) -> Result<()> {
    // Get factory from global registry
    let registry = geoetl_core_common::driver_registry();
    let factory = registry
        .find_factory(driver.short_name)
        .ok_or_else(|| anyhow!("Driver '{}' not registered", driver.short_name))?;

    // Create reader strategy
    let reader = factory
        .create_reader()
        .ok_or_else(|| anyhow!("Driver '{}' does not support reading", driver.short_name))?;

    // Prepare format-specific options
    let options = prepare_reader_options(driver.short_name, geometry_column, geometry_type)?;

    // Use polymorphic dispatch - no switch statement needed!
    let table = reader
        .create_table_provider(&ctx.state(), input, options)
        .await
        .map_err(|e| {
            anyhow!(
                "Failed to read {} file '{}': {}",
                driver.short_name,
                input,
                e
            )
        })?;

    ctx.register_table(table_name, table)
        .map_err(|e| anyhow!("Failed to register table '{table_name}': {e}"))?;

    Ok(())
}

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
/// * `geometry_column` - Name of the geometry column (for CSV)
/// * `geometry_type` - Optional geometry type hint (for CSV)
///
/// # Returns
///
/// A `Result` indicating success or an error if the conversion fails.
///
/// # Errors
///
/// This function will return an error if:
/// - The file cannot be read or parsed.
/// - The file format is not yet implemented.
/// - The output file cannot be written.
///
/// # Note
///
/// Driver capability validation should be performed by the caller before invoking this function.
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

    // Initialize context and register dataset
    let ctx = initialize_context(input, input_driver, geometry_column, geometry_type).await?;

    // Collect batches from the registered table
    let table = ctx
        .table("dataset")
        .await
        .map_err(|e| anyhow!("Failed to get table: {e}"))?;
    let batches = table
        .collect()
        .await
        .map_err(|e| anyhow!("Failed to collect data: {e}"))?;

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

/// Get information about a geospatial dataset.
///
/// This function reads a geospatial file and returns structured information about it, including:
/// - Dataset path and driver
/// - Geometry column information (name, extension, CRS)
/// - Field schema (name, data type, nullable status)
///
/// # Arguments
///
/// * `input` - The path to the input geospatial data file.
/// * `input_driver` - The driver responsible for reading the input format.
/// * `geometry_column` - Name of the geometry column (for CSV)
/// * `geometry_type` - Optional geometry type hint (for CSV)
///
/// # Returns
///
/// A `Result` containing `DatasetInfo` or an error if the info operation fails.
///
/// # Errors
///
/// This function will return an error if:
/// - The file cannot be read or parsed.
/// - The file format is not yet implemented.
///
/// # Note
///
/// Driver capability validation should be performed by the caller before invoking this function.
pub async fn info(
    input: &str,
    input_driver: &Driver,
    geometry_column: &str,
    geometry_type: Option<&str>,
) -> Result<DatasetInfo> {
    info!("Reading dataset information:");
    info!("Input: {} (Driver: {})", input, input_driver.short_name);

    // Initialize context and register dataset
    let ctx = initialize_context(input, input_driver, geometry_column, geometry_type).await?;

    // Build dataset info using context
    let dataset_info =
        build_dataset_info_from_context(&ctx, "dataset", input, input_driver).await?;

    Ok(dataset_info)
}

/// Build dataset information structure using `DataFusion` context.
async fn build_dataset_info_from_context(
    ctx: &SessionContext,
    table_name: &str,
    input: &str,
    driver: &Driver,
) -> Result<DatasetInfo> {
    // Get the table schema from the context
    let table = ctx
        .table(table_name)
        .await
        .map_err(|e| anyhow!("Failed to get table: {e}"))?;

    let schema = table.schema();
    let arrow_schema = schema.as_arrow();

    // Find and collect geometry column information
    let mut geometry_column_info = Vec::new();
    for field in arrow_schema.fields() {
        let metadata = field.metadata();
        if metadata.contains_key("ARROW:extension:name") {
            let extension_name = metadata.get("ARROW:extension:name").unwrap();
            if extension_name.starts_with("geoarrow") {
                geometry_column_info.push(GeometryColumnInfo {
                    name: field.name().to_string(),
                    data_type: format!("{:?}", field.data_type()),
                    extension: Some(extension_name.clone()),
                    crs: metadata.get("ARROW:extension:metadata").cloned(),
                });
            }
        }
    }

    // Collect field information
    let mut field_infos = Vec::new();
    for field in arrow_schema.fields() {
        // Skip geometry columns in field listing
        let metadata = field.metadata();
        let is_geometry = metadata.contains_key("ARROW:extension:name")
            && metadata
                .get("ARROW:extension:name")
                .is_some_and(|s| s.starts_with("geoarrow"));

        if is_geometry {
            continue;
        }

        field_infos.push(FieldInfo {
            name: field.name().to_string(),
            data_type: field.data_type().format(),
            nullable: field.is_nullable(),
        });
    }

    Ok(DatasetInfo {
        dataset: input.to_string(),
        driver: driver.short_name.to_string(),
        driver_long_name: driver.long_name.to_string(),
        geometry_columns: geometry_column_info,
        fields: field_infos,
    })
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
        // Initialize format drivers
        crate::init::initialize();

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
        // Initialize format drivers
        crate::init::initialize();

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
        // Initialize format drivers
        crate::init::initialize();

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
        let error_msg = result.unwrap_err().to_string();
        // After factory refactoring, unregistered drivers produce a "not registered" error
        assert!(
            error_msg.contains("not registered") || error_msg.contains("does not support reading"),
            "Unexpected error message: {error_msg}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_unsupported_output_write() -> Result<()> {
        // Initialize format drivers
        crate::init::initialize();

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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("is not yet implemented for conversion")
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_invalid_csv() -> Result<()> {
        // Initialize format drivers
        crate::init::initialize();

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
        // Initialize format drivers
        crate::init::initialize();

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
        let error_msg = result.unwrap_err().to_string();
        // After factory refactoring, unregistered drivers produce a "not registered" error
        assert!(
            error_msg.contains("not registered"),
            "Unexpected error message: {error_msg}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_empty_csv() -> Result<()> {
        // Initialize format drivers
        crate::init::initialize();

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
