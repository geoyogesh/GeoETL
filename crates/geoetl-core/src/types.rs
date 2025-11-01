//! Data types for geospatial ETL operations.
//!
//! This module defines the data structures used to represent dataset information,
//! geometry columns, and field schemas.

/// Information about a dataset.
#[derive(Debug, Clone)]
pub struct DatasetInfo {
    /// Path to the dataset
    pub dataset: String,
    /// Driver name
    pub driver: String,
    /// Driver long name
    pub driver_long_name: String,
    /// Geometry columns information
    pub geometry_columns: Vec<GeometryColumnInfo>,
    /// Schema fields
    pub fields: Vec<FieldInfo>,
}

/// Information about a geometry column.
#[derive(Debug, Clone)]
pub struct GeometryColumnInfo {
    /// Column name
    pub name: String,
    /// Data type description
    pub data_type: String,
    /// Extension name (e.g., "geoarrow.geometry")
    pub extension: Option<String>,
    /// CRS information
    pub crs: Option<String>,
}

/// Information about a field/column.
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Field name
    pub name: String,
    /// Data type
    pub data_type: String,
    /// Whether the field is nullable
    pub nullable: bool,
}
