//! Geospatial data parsing for CSV files
//!
//! This module provides functionality for parsing and converting geospatial data
//! from CSV format into `GeoArrow` arrays. It supports Well-Known Text (WKT)
//! geometry encoding and conversion to various `GeoArrow` geometry types.

use std::sync::Arc;

use arrow_array::{ArrayRef, builder::StringBuilder};
use csv_async::StringRecord as AsyncStringRecord;
use datafusion::error::{DataFusionError, Result};
use datafusion_shared::SpatialFormatReadError;
use geoarrow_array::GeoArrowArray;
use geoarrow_array::array::WktArray;
use geoarrow_array::cast::from_wkt;
use geoarrow_schema::WktType;

use crate::file_format::{GeometryColumnOptions, GeometrySource};

/// Build a geometry column from CSV records containing WKT geometries
///
/// This function takes CSV records and converts a specific column containing
/// Well-Known Text (WKT) geometries into a `GeoArrow` array of the appropriate type.
///
/// # Arguments
///
/// * `geometry` - Configuration for the geometry column including the target data type
/// * `column_idx` - The index of the column containing WKT strings in the CSV records
/// * `records` - The CSV records to process
///
/// # Returns
///
/// An `ArrayRef` containing the parsed geometry data in the requested `GeoArrow` format
///
/// # Errors
///
/// Returns an error if:
/// - The geometry source is not WKT (currently only WKT is supported)
/// - WKT parsing fails
/// - Conversion to the target geometry type fails
///
/// # Example
///
/// ```ignore
/// use datafusion_csv::geospatial::build_geometry_column;
/// use datafusion_csv::file_format::{GeometryColumnOptions, GeometryDataType};
///
/// let geometry_config = GeometryColumnOptions {
///     field_name: "location".to_string(),
///     data_type: GeometryDataType::Point,
///     source: GeometrySource::Wkt { column: "location".to_string() },
/// };
///
/// let array = build_geometry_column(&geometry_config, 0, &records)?;
/// ```
pub fn build_geometry_column(
    geometry: &GeometryColumnOptions,
    column_idx: usize,
    records: &[AsyncStringRecord],
) -> Result<ArrayRef> {
    // Validate that the geometry source is WKT
    let GeometrySource::Wkt { .. } = &geometry.source;

    // Build a string array from the WKT column
    let string_array = extract_wkt_strings(column_idx, records);

    // Convert WKT strings to the target GeoArrow geometry type
    convert_wkt_to_geoarrow(string_array, geometry)
}

/// Extract WKT strings from CSV records into an Arrow `StringArray`
///
/// This function processes CSV records and extracts the values from a specific
/// column, handling empty values and null cases appropriately.
///
/// # Arguments
///
/// * `column_idx` - The index of the column to extract
/// * `records` - The CSV records to process
///
/// # Returns
///
/// An Arrow `StringArray` containing the extracted WKT strings
fn extract_wkt_strings(
    column_idx: usize,
    records: &[AsyncStringRecord],
) -> arrow_array::StringArray {
    let mut builder = StringBuilder::with_capacity(records.len(), records.len() * 4);

    for record in records {
        match record.get(column_idx) {
            Some(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    builder.append_null();
                } else {
                    builder.append_value(trimmed);
                }
            },
            None => builder.append_null(),
        }
    }

    builder.finish()
}

/// Convert WKT strings to a `GeoArrow` array
///
/// This function takes an Arrow `StringArray` containing WKT geometries and
/// converts it to the appropriate `GeoArrow` geometry type as specified in
/// the geometry configuration.
///
/// # Arguments
///
/// * `string_array` - Arrow `StringArray` containing WKT geometry strings
/// * `geometry` - Configuration specifying the target `GeoArrow` geometry type
///
/// # Returns
///
/// An `ArrayRef` containing the converted geometry data
///
/// # Errors
///
/// Returns an error if:
/// - WKT array construction fails
/// - Geometry conversion fails (e.g., invalid WKT or type mismatch)
fn convert_wkt_to_geoarrow(
    string_array: arrow_array::StringArray,
    geometry: &GeometryColumnOptions,
) -> Result<ArrayRef> {
    // Create WktArray from the StringArray
    let wkt_type = WktType::new(Arc::default());
    let wkt_array = WktArray::from((string_array, wkt_type));

    // Convert WKT to the target GeoArrow type
    let geometry_array = from_wkt(&wkt_array, geometry.geoarrow_type.clone()).map_err(|err| {
        DataFusionError::from(SpatialFormatReadError::Parse {
            message: format!(
                "Failed to decode WKT geometry for column '{}': {err}",
                geometry.field_name
            ),
            position: None,
            context: Some(format!("geometry column '{}'", geometry.field_name)),
        })
    })?;

    Ok(geometry_array.into_array_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_format::GeometrySource;
    use arrow_array::Array;
    use csv_async::StringRecord;
    use geoarrow_schema::{Dimension, PointType};
    use std::sync::Arc;

    #[test]
    fn test_extract_wkt_strings() {
        let records = vec![
            StringRecord::from(vec!["POINT(0 0)", "data1"]),
            StringRecord::from(vec!["POINT(1 1)", "data2"]),
            StringRecord::from(vec!["  ", "data3"]), // Empty/whitespace
            StringRecord::from(vec!["POINT(2 2)", "data4"]),
        ];

        let array = extract_wkt_strings(0, &records);

        assert_eq!(array.len(), 4);
        assert_eq!(array.value(0), "POINT(0 0)");
        assert_eq!(array.value(1), "POINT(1 1)");
        assert!(array.is_null(2)); // Empty should become null
        assert_eq!(array.value(3), "POINT(2 2)");
    }

    #[test]
    fn test_build_geometry_column_points() {
        let geometry = GeometryColumnOptions {
            field_name: "location".to_string(),
            geoarrow_type: geoarrow_schema::GeoArrowType::Point(PointType::new(
                Dimension::XY,
                Arc::default(),
            )),
            source: GeometrySource::Wkt {
                column: "location".to_string(),
            },
        };

        let records = vec![
            StringRecord::from(vec!["POINT(0 0)"]),
            StringRecord::from(vec!["POINT(1 1)"]),
        ];

        let result = build_geometry_column(&geometry, 0, &records);
        assert!(
            result.is_ok(),
            "Should successfully parse WKT points: {:?}",
            result.err()
        );

        let array = result.unwrap();
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_build_geometry_column_with_nulls() {
        let geometry = GeometryColumnOptions {
            field_name: "location".to_string(),
            geoarrow_type: geoarrow_schema::GeoArrowType::Point(PointType::new(
                Dimension::XY,
                Arc::default(),
            )),
            source: GeometrySource::Wkt {
                column: "location".to_string(),
            },
        };

        let records = vec![
            StringRecord::from(vec!["POINT(0 0)"]),
            StringRecord::from(vec![""]), // Empty string
            StringRecord::from(vec!["POINT(2 2)"]),
        ];

        let result = build_geometry_column(&geometry, 0, &records);
        assert!(result.is_ok(), "Should handle null values gracefully");

        let array = result.unwrap();
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_build_geometry_column_invalid_wkt() {
        let geometry = GeometryColumnOptions {
            field_name: "location".to_string(),
            geoarrow_type: geoarrow_schema::GeoArrowType::Point(PointType::new(
                Dimension::XY,
                Arc::default(),
            )),
            source: GeometrySource::Wkt {
                column: "location".to_string(),
            },
        };

        let records = vec![StringRecord::from(vec!["INVALID WKT"])];

        let result = build_geometry_column(&geometry, 0, &records);
        assert!(result.is_err(), "Should fail on invalid WKT");

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Failed to decode WKT geometry"),
            "Error should mention WKT decoding failure"
        );
        assert!(
            error_msg.contains("location"),
            "Error should include column name"
        );
    }
}
