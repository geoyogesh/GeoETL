//! Display utilities for formatting CLI output.
//!
//! This module provides table row structures and formatting functions
//! for presenting geospatial data information in a human-readable format.

use tabled::{Table, Tabled};

use geoetl_core::types::DatasetInfo;

/// Table row representation for displaying geometry column information.
#[derive(Tabled)]
pub struct GeometryRow {
    /// Name of the geometry column.
    #[tabled(rename = "Column")]
    pub name: String,
    /// `GeoArrow` extension name for the geometry type.
    #[tabled(rename = "Extension")]
    pub extension: String,
    /// Coordinate Reference System information.
    #[tabled(rename = "CRS")]
    pub crs: String,
}

/// Table row representation for displaying field/column information.
#[derive(Tabled)]
pub struct FieldRow {
    /// Name of the field.
    #[tabled(rename = "Field")]
    pub name: String,
    /// Data type of the field.
    #[tabled(rename = "Type")]
    pub data_type: String,
    /// Whether the field can contain null values.
    #[tabled(rename = "Nullable")]
    pub nullable: String,
}

/// Table row representation for displaying driver information.
#[derive(Tabled)]
pub struct DriverRow {
    /// Short identifier for the driver (e.g., `GeoJSON`, `Parquet`).
    #[tabled(rename = "Short Name")]
    pub short_name: String,
    /// Full descriptive name of the driver format.
    #[tabled(rename = "Long Name")]
    pub long_name: String,
    /// Support status for reading dataset metadata and information.
    #[tabled(rename = "Info")]
    pub info: String,
    /// Support status for reading data from this format.
    #[tabled(rename = "Read")]
    pub read: String,
    /// Support status for writing data to this format.
    #[tabled(rename = "Write")]
    pub write: String,
}

/// Display dataset information in a formatted table.
///
/// This function presents dataset metadata, geometry columns, and field schema
/// in a human-readable table format written to standard output.
///
/// # Arguments
///
/// * `info` - The dataset information to display
pub fn display_dataset_info(info: &DatasetInfo) {
    // Display dataset path and driver
    println!("\nDataset: {}", info.dataset);
    println!("Driver: {} ({})", info.driver, info.driver_long_name);

    // Display geometry columns
    if !info.geometry_columns.is_empty() {
        println!("\n=== Geometry Columns ===");

        let geo_rows: Vec<GeometryRow> = info
            .geometry_columns
            .iter()
            .map(|g| GeometryRow {
                name: g.name.clone(),
                extension: g.extension.clone().unwrap_or_else(|| "N/A".to_string()),
                crs: g.crs.clone().unwrap_or_else(|| "N/A".to_string()),
            })
            .collect();

        let geo_table = Table::new(geo_rows).to_string();
        println!("{geo_table}");
    }

    // Display field schema
    if !info.fields.is_empty() {
        println!("\n=== Fields ===");

        let field_rows: Vec<FieldRow> = info
            .fields
            .iter()
            .map(|f| FieldRow {
                name: f.name.clone(),
                data_type: f.data_type.clone(),
                nullable: if f.nullable { "Yes" } else { "No" }.to_string(),
            })
            .collect();

        let field_table = Table::new(field_rows).to_string();
        println!("{field_table}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geoetl_core::types::{DatasetInfo, FieldInfo, GeometryColumnInfo};

    #[test]
    fn test_geometry_row_creation() {
        let row = GeometryRow {
            name: "geom".to_string(),
            extension: "geoarrow.point".to_string(),
            crs: "EPSG:4326".to_string(),
        };
        assert_eq!(row.name, "geom");
        assert_eq!(row.extension, "geoarrow.point");
        assert_eq!(row.crs, "EPSG:4326");
    }

    #[test]
    fn test_field_row_creation() {
        let row = FieldRow {
            name: "id".to_string(),
            data_type: "Int32".to_string(),
            nullable: "Yes".to_string(),
        };
        assert_eq!(row.name, "id");
        assert_eq!(row.data_type, "Int32");
        assert_eq!(row.nullable, "Yes");
    }

    #[test]
    fn test_driver_row_creation() {
        let row = DriverRow {
            short_name: "GeoJSON".to_string(),
            long_name: "GeoJSON File Format".to_string(),
            info: "Yes".to_string(),
            read: "Yes".to_string(),
            write: "Yes".to_string(),
        };
        assert_eq!(row.short_name, "GeoJSON");
        assert_eq!(row.long_name, "GeoJSON File Format");
        assert_eq!(row.info, "Yes");
        assert_eq!(row.read, "Yes");
        assert_eq!(row.write, "Yes");
    }

    #[test]
    fn test_display_dataset_info_with_geometry() {
        let info = DatasetInfo {
            dataset: "test.geojson".to_string(),
            driver: "GeoJSON".to_string(),
            driver_long_name: "GeoJSON File Format".to_string(),
            geometry_columns: vec![GeometryColumnInfo {
                name: "geometry".to_string(),
                data_type: "Point".to_string(),
                extension: Some("geoarrow.point".to_string()),
                crs: Some("EPSG:4326".to_string()),
            }],
            fields: vec![FieldInfo {
                name: "id".to_string(),
                data_type: "Int32".to_string(),
                nullable: false,
            }],
        };

        // This test just ensures the function runs without panicking
        display_dataset_info(&info);
    }

    #[test]
    fn test_display_dataset_info_without_geometry() {
        let info = DatasetInfo {
            dataset: "test.csv".to_string(),
            driver: "CSV".to_string(),
            driver_long_name: "Comma Separated Values".to_string(),
            geometry_columns: vec![],
            fields: vec![
                FieldInfo {
                    name: "name".to_string(),
                    data_type: "String".to_string(),
                    nullable: true,
                },
                FieldInfo {
                    name: "value".to_string(),
                    data_type: "Float64".to_string(),
                    nullable: false,
                },
            ],
        };

        // This test just ensures the function runs without panicking
        display_dataset_info(&info);
    }

    #[test]
    fn test_display_dataset_info_with_na_fields() {
        let info = DatasetInfo {
            dataset: "test.geojson".to_string(),
            driver: "GeoJSON".to_string(),
            driver_long_name: "GeoJSON File Format".to_string(),
            geometry_columns: vec![GeometryColumnInfo {
                name: "geometry".to_string(),
                data_type: "Point".to_string(),
                extension: None,
                crs: None,
            }],
            fields: vec![],
        };

        // This test ensures None values are handled correctly (should show "N/A")
        display_dataset_info(&info);
    }

    #[test]
    fn test_display_dataset_info_empty() {
        let info = DatasetInfo {
            dataset: "empty.csv".to_string(),
            driver: "CSV".to_string(),
            driver_long_name: "Comma Separated Values".to_string(),
            geometry_columns: vec![],
            fields: vec![],
        };

        // This test ensures empty datasets are handled correctly
        display_dataset_info(&info);
    }
}
