//! Core ETL operations for geospatial data.
//!
//! This module provides the main functions for Extract, Transform, and Load (ETL)
//! operations on geospatial data, leveraging the driver registry for format support.

use crate::drivers::Driver;
use crate::error::{self, DriverError, GeoEtlError};
use crate::types::{DatasetInfo, FieldInfo, GeometryColumnInfo};
use crate::utils::ArrowDataTypeExt;
use datafusion::prelude::SessionContext;
use log::{info, warn};

// Type alias for backward compatibility during migration
type Result<T> = std::result::Result<T, GeoEtlError>;

/// Initialize a `DataFusion` session context and register a dataset.
///
/// This is a common entry point for all ETL operations that need to work with a dataset.
/// It creates a new session context, registers the dataset with the specified parameters,
/// and returns the context ready for use along with the table name used for registration.
///
/// # Arguments
///
/// * `input` - Path to the input file
/// * `driver` - The driver responsible for reading the format
/// * `geometry_column` - Name of the geometry column (for CSV)
/// * `geometry_type` - Optional geometry type hint (for CSV)
/// * `batch_size` - Number of rows per batch for processing
/// * `table_name_override` - Optional custom table name (if not provided, inferred from filename)
///
/// # Returns
///
/// A tuple of (`SessionContext`, `table_name`) where the table name is either custom or inferred.
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
    batch_size: usize,
    read_partitions: usize,
    table_name_override: Option<&str>,
) -> Result<(SessionContext, String)> {
    use datafusion::prelude::SessionConfig;

    // Optimize for throughput with configurable parallel processing
    let config = SessionConfig::new()
        .with_target_partitions(read_partitions)
        .with_batch_size(batch_size);

    let ctx = SessionContext::new_with_config(config);

    // Use custom table name if provided, otherwise infer from filename using the factory
    let table_name = if let Some(custom_name) = table_name_override {
        info!("Using custom table name '{custom_name}'");
        custom_name.to_string()
    } else {
        // Get factory from global registry to use format-specific table name inference
        let registry = geoetl_core_common::driver_registry();
        let factory =
            registry
                .find_factory(driver.short_name)
                .ok_or_else(|| DriverError::NotRegistered {
                    driver: driver.short_name.to_string(),
                })?;

        // Try to infer table name from the format, fallback to "dataset" if inference fails
        let inferred = factory
            .infer_table_name(input)
            .unwrap_or_else(|| "dataset".to_string());
        info!("Registering input as table '{inferred}'");
        inferred
    };

    register_catalog(
        &ctx,
        input,
        driver,
        &table_name,
        geometry_column,
        geometry_type,
        batch_size,
    )
    .await?;
    Ok((ctx, table_name))
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
/// * `batch_size` - Number of rows per batch for processing
///
/// # Returns
///
/// A boxed `Any` containing the format-specific options, or an error if the driver is unknown.
fn prepare_reader_options(
    driver_name: &str,
    geometry_column: &str,
    geometry_type: Option<&str>,
    batch_size: usize,
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
            let options = GeoJsonFormatOptions::default().with_batch_size(batch_size);
            Ok(Box::new(options))
        },
        "GeoParquet" => {
            use datafusion_geoparquet::GeoParquetFormatOptions;
            let options = GeoParquetFormatOptions::default()
                .with_batch_size(batch_size)
                .with_geometry_column_name(geometry_column);
            Ok(Box::new(options))
        },
        _ => Err(DriverError::NotRegistered {
            driver: driver_name.to_string(),
        }
        .into()),
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
/// * `batch_size` - Number of rows per batch for processing
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
    batch_size: usize,
) -> Result<()> {
    // Get factory from global registry
    let registry = geoetl_core_common::driver_registry();
    let factory =
        registry
            .find_factory(driver.short_name)
            .ok_or_else(|| DriverError::NotRegistered {
                driver: driver.short_name.to_string(),
            })?;

    // Create reader strategy
    let reader = factory
        .create_reader()
        .ok_or_else(|| DriverError::OperationNotSupported {
            driver: driver.short_name.to_string(),
            operation: "reading".to_string(),
        })?;

    // Prepare format-specific options
    let options = prepare_reader_options(
        driver.short_name,
        geometry_column,
        geometry_type,
        batch_size,
    )?;

    // Use polymorphic dispatch - no switch statement needed!
    let table = reader
        .create_table_provider(&ctx.state(), input, options)
        .await
        .map_err(|e| {
            GeoEtlError::Io(error::IoError::Read {
                format: driver.short_name.to_string(),
                path: input.into(),
                source: e.into(),
            })
        })?;

    ctx.register_table(table_name, table).map_err(|e| {
        GeoEtlError::from(anyhow::anyhow!(
            "Failed to register table '{table_name}': {e}"
        ))
    })?;

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
            return Err(error::FormatError::UnsupportedGeometryType {
                geometry_type: geom_type_str.to_string(),
            }
            .into());
        },
    };
    Ok(geoarrow_type)
}

// Note: write_with_driver was removed as it's no longer used.
// All writing now goes through streaming execution via DataSink.

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
/// * `sql_query` - Optional SQL query to apply to the input dataset
/// * `table_name` - Optional table name override (if not provided, inferred from filename)
/// * `batch_size` - Number of rows per batch for processing (default: 8192)
/// * `read_partitions` - Number of partitions for reading (default: 1)
/// * `write_partitions` - Number of partitions for writing (default: 1, overridden by format)
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
/// - The SQL query is invalid or fails to execute.
///
/// # Note
///
/// Driver capability validation should be performed by the caller before invoking this function.
#[allow(clippy::too_many_arguments)]
pub async fn convert(
    input: &str,
    output: &str,
    input_driver: &Driver,
    output_driver: &Driver,
    geometry_column: &str,
    geometry_type: Option<&str>,
    sql_query: Option<&str>,
    table_name_override: Option<&str>,
    batch_size: Option<usize>,
    read_partitions: Option<usize>,
    write_partitions: Option<usize>,
) -> Result<()> {
    use datafusion::datasource::physical_plan::{FileGroup, FileSinkConfig};
    use datafusion::logical_expr::dml::InsertOp;
    use datafusion::physical_plan::collect;

    let batch_size = batch_size.unwrap_or(8192); // Default to standard DataFusion batch size
    let read_partitions = read_partitions.unwrap_or(1); // Default to single partition
    let mut write_partitions = write_partitions.unwrap_or(1); // Default to single partition

    // CSV and GeoJSON only support single-partition writes
    if matches!(output_driver.short_name, "CSV" | "GeoJSON") && write_partitions > 1 {
        warn!(
            "{} format only supports single-partition writes. Overriding write_partitions from {} to 1.",
            output_driver.short_name, write_partitions
        );
        write_partitions = 1;
    }

    info!("Starting conversion:");
    info!("Input: {input} (Driver: {})", input_driver.short_name);
    info!("Output: {output} (Driver: {})", output_driver.short_name);
    info!("Batch size: {batch_size}");
    info!("Read partitions: {read_partitions}");
    info!("Write partitions: {write_partitions}");

    // Initialize context and register dataset
    let (ctx, table_name) = initialize_context(
        input,
        input_driver,
        geometry_column,
        geometry_type,
        batch_size,
        read_partitions,
        table_name_override,
    )
    .await?;

    // Get the table - either from SQL query or default SELECT *
    let table = if let Some(query) = sql_query {
        info!("Executing SQL query: {query}");
        ctx.sql(query)
            .await
            .map_err(|e| GeoEtlError::from(anyhow::anyhow!("SQL query execution failed: {e}")))?
    } else {
        ctx.table(&table_name)
            .await
            .map_err(|e| GeoEtlError::from(anyhow::anyhow!("Failed to get table: {e}")))?
    };

    // Use streaming execution via DataSink - NO fallback!
    let registry = geoetl_core_common::driver_registry();
    let factory = registry
        .find_factory(output_driver.short_name)
        .ok_or_else(|| {
            GeoEtlError::Io(error::IoError::Write {
                format: output_driver.short_name.to_string(),
                path: output.into(),
                source: Box::new(GeoEtlError::Driver(DriverError::NotRegistered {
                    driver: output_driver.short_name.to_string(),
                })),
            })
        })?;

    let file_format = factory.create_file_format(geometry_column).ok_or_else(|| {
        GeoEtlError::from(anyhow::anyhow!(
            "Streaming not supported for driver: {}",
            output_driver.short_name
        ))
    })?;

    info!("Using streaming execution via DataSink");

    // Get the logical plan from the table
    let logical_plan = table
        .into_optimized_plan()
        .map_err(|e| GeoEtlError::from(anyhow::anyhow!("Failed to create logical plan: {e}")))?;

    // Create physical plan
    let state = ctx.state();
    let physical_plan = state
        .create_physical_plan(&logical_plan)
        .await
        .map_err(|e| GeoEtlError::from(anyhow::anyhow!("Failed to create physical plan: {e}")))?;

    // Parse output path
    let table_path = datafusion::datasource::listing::ListingTableUrl::parse(output)
        .map_err(|e| GeoEtlError::from(anyhow::anyhow!("Invalid output path: {e}")))?;

    // Create FileSinkConfig
    // Use Overwrite mode for all formats in convert operations
    // This ensures we replace the output file if it already exists
    let config = FileSinkConfig {
        original_url: output.to_string(),
        object_store_url: table_path.object_store(),
        file_group: FileGroup::default(),
        table_paths: vec![table_path],
        output_schema: physical_plan.schema(),
        table_partition_cols: vec![],
        insert_op: InsertOp::Overwrite,
        keep_partition_by_columns: false,
        file_extension: output_driver.short_name.to_lowercase(),
    };

    // Create the writer plan using the file format
    let writer_plan = file_format
        .create_writer_physical_plan(physical_plan, &state, config, None)
        .await
        .map_err(|e| GeoEtlError::from(anyhow::anyhow!("Failed to create writer plan: {e}")))?;

    // Execute the writer plan (streams data through DataSink without collecting)
    let result = collect(writer_plan, state.task_ctx()).await.map_err(|e| {
        GeoEtlError::from(anyhow::anyhow!("Failed to execute streaming writer: {e}"))
    })?;

    // Extract row count from result
    if let Some(batch) = result.first()
        && batch.num_columns() > 0
        && let Some(count_array) = batch
            .column(0)
            .as_any()
            .downcast_ref::<datafusion::arrow::array::UInt64Array>()
    {
        let total_rows = count_array.value(0);
        info!("Total rows written: {total_rows}");
    }

    info!("Conversion completed successfully using streaming execution");
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
/// * `batch_size` - Number of rows per batch for processing (default: 8192)
/// * `read_partitions` - Number of partitions for reading (default: 1)
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
    batch_size: Option<usize>,
    read_partitions: Option<usize>,
) -> Result<DatasetInfo> {
    let batch_size = batch_size.unwrap_or(8192); // Default to standard DataFusion batch size
    let read_partitions = read_partitions.unwrap_or(1); // Default to single partition

    info!("Reading dataset information:");
    info!("Input: {input} (Driver: {})", input_driver.short_name);
    info!("Batch size: {batch_size}");
    info!("Read partitions: {read_partitions}");

    // Initialize context and register dataset
    // Note: info command doesn't use custom table names, always infers from filename
    let (ctx, table_name) = initialize_context(
        input,
        input_driver,
        geometry_column,
        geometry_type,
        batch_size,
        read_partitions,
        None, // No table name override for info command
    )
    .await?;

    // Build dataset info using context
    let dataset_info =
        build_dataset_info_from_context(&ctx, &table_name, input, input_driver).await?;

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
        .map_err(|e| GeoEtlError::from(anyhow::anyhow!("Failed to get table: {e}")))?;

    let schema = table.schema();
    let arrow_schema = schema.as_arrow();

    // Try to parse GeoParquet metadata from schema-level metadata
    let geo_metadata_json = arrow_schema
        .metadata()
        .get("geo")
        .and_then(|geo_json| serde_json::from_str::<serde_json::Value>(geo_json).ok());

    // Find and collect geometry column information
    let mut geometry_column_info = Vec::new();
    for field in arrow_schema.fields() {
        let metadata = field.metadata();
        if metadata.contains_key("ARROW:extension:name") {
            let extension_name = metadata.get("ARROW:extension:name").unwrap();
            if extension_name.starts_with("geoarrow") {
                // First try to get CRS from field-level metadata (GeoArrow style)
                let mut crs = metadata.get("ARROW:extension:metadata").cloned();

                // If not found and we have GeoParquet metadata, try to get CRS from there
                if crs.is_none()
                    && let Some(ref geo_meta) = geo_metadata_json
                    && let Some(columns) = geo_meta.get("columns")
                    && let Some(column_meta) = columns.get(field.name())
                {
                    if let Some(crs_value) = column_meta.get("crs") {
                        // If crs is null in the JSON, skip it (means no CRS assigned)
                        if !crs_value.is_null() {
                            crs = Some(crs_value.to_string());
                        }
                    } else {
                        // No CRS field means default WGS84 per GeoParquet spec
                        crs = Some("WGS84".to_string());
                    }
                }

                geometry_column_info.push(GeometryColumnInfo {
                    name: field.name().to_string(),
                    data_type: format!("{:?}", field.data_type()),
                    extension: Some(extension_name.clone()),
                    crs,
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
            None, // sql_query
            None, // table_name_override
            None, // batch_size
            None, // read_partitions
            None, // write_partitions
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
            None, // sql_query
            None, // table_name_override
            None, // batch_size
            None, // read_partitions
            None, // write_partitions
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
            None, // sql_query
            None, // table_name_override
            None, // batch_size
            None, // read_partitions
            None, // write_partitions
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
            None, // sql_query
            None, // table_name_override
            None, // batch_size
            None, // read_partitions
            None, // write_partitions
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        // After refactoring, unregistered drivers produce an IoError with NotRegistered as source
        // This provides better context about what operation failed
        match &err {
            GeoEtlError::Io(error::IoError::Write { source, .. }) => {
                // The source should be the original DriverError::NotRegistered
                // We need to downcast to check the concrete error type
                let source_err = source.downcast_ref::<GeoEtlError>();
                assert!(source_err.is_some(), "Source should be a GeoEtlError");
                assert!(
                    matches!(
                        source_err.unwrap(),
                        GeoEtlError::Driver(DriverError::NotRegistered { .. })
                    ),
                    "Source should be DriverError::NotRegistered"
                );
            },
            _ => panic!("Expected IoError::Write, got: {err:?}"),
        }
        assert!(err.to_string().contains("GML"));
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
            None, // sql_query
            None, // table_name_override
            None, // batch_size
            None, // read_partitions
            None, // write_partitions
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
            None, // sql_query
            None, // table_name_override
            None, // batch_size
            None, // read_partitions
            None, // write_partitions
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
            None, // sql_query
            None, // table_name_override
            None, // batch_size
            None, // read_partitions
            None, // write_partitions
        )
        .await;

        assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
        assert!(output_path.exists(), "Output file was not created");

        Ok(())
    }
}
