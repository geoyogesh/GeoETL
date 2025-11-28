//! `GeoArrow` type definitions and utilities for `DataFusion` integration
//!
//! This module provides utilities for working with `GeoArrow` extension types
//! in `DataFusion` UDFs. `GeoArrow` uses Arrow extension types to represent
//! geospatial data with proper metadata.
//!
//! # Extension Type Names
//!
//! - `geoarrow.point` - Point geometry
//! - `geoarrow.linestring` - `LineString` geometry
//! - `geoarrow.polygon` - Polygon geometry
//! - `geoarrow.multipoint` - `MultiPoint` geometry
//! - `geoarrow.multilinestring` - `MultiLineString` geometry
//! - `geoarrow.multipolygon` - `MultiPolygon` geometry
//! - `geoarrow.geometry` - Mixed geometry types
//! - `geoarrow.wkb` - WKB encoded geometry
//! - `geoarrow.wkt` - WKT encoded geometry

use arrow_schema::{DataType, Field};
use std::collections::HashMap;
use std::sync::Arc;

/// `GeoArrow` extension type name for Point geometry
pub const GEOARROW_POINT: &str = "geoarrow.point";
/// `GeoArrow` extension type name for `LineString` geometry
pub const GEOARROW_LINESTRING: &str = "geoarrow.linestring";
/// `GeoArrow` extension type name for Polygon geometry
pub const GEOARROW_POLYGON: &str = "geoarrow.polygon";
/// `GeoArrow` extension type name for `MultiPoint` geometry
pub const GEOARROW_MULTIPOINT: &str = "geoarrow.multipoint";
/// `GeoArrow` extension type name for `MultiLineString` geometry
pub const GEOARROW_MULTILINESTRING: &str = "geoarrow.multilinestring";
/// `GeoArrow` extension type name for `MultiPolygon` geometry
pub const GEOARROW_MULTIPOLYGON: &str = "geoarrow.multipolygon";
/// `GeoArrow` extension type name for mixed geometry
pub const GEOARROW_GEOMETRY: &str = "geoarrow.geometry";
/// `GeoArrow` extension type name for WKB encoded geometry
pub const GEOARROW_WKB: &str = "geoarrow.wkb";

/// Arrow metadata key for extension type name
pub const EXTENSION_NAME_KEY: &str = "ARROW:extension:name";
/// Arrow metadata key for extension type metadata (JSON)
#[allow(dead_code)]
pub const EXTENSION_METADATA_KEY: &str = "ARROW:extension:metadata";

/// Create the Arrow `DataType` for a `GeoArrow` Point (interleaved coordinates)
///
/// `GeoArrow` Point is stored as `FixedSizeList<Float64, 2>` for 2D points
/// containing [x, y] coordinates.
#[must_use]
pub fn point_data_type() -> DataType {
    DataType::FixedSizeList(Arc::new(Field::new("xy", DataType::Float64, false)), 2)
}

/// Create an Arrow Field for a `GeoArrow` Point with extension metadata
#[must_use]
pub fn point_field(name: &str, nullable: bool) -> Field {
    let metadata: HashMap<String, String> =
        [(EXTENSION_NAME_KEY.to_string(), GEOARROW_POINT.to_string())]
            .into_iter()
            .collect();

    Field::new(name, point_data_type(), nullable).with_metadata(metadata)
}

/// Create the Arrow `DataType` for `GeoArrow` WKB
///
/// WKB is stored as Binary (variable-length byte array)
#[must_use]
pub fn wkb_data_type() -> DataType {
    DataType::Binary
}

/// Create an Arrow Field for `GeoArrow` WKB with extension metadata
#[must_use]
pub fn wkb_field(name: &str, nullable: bool) -> Field {
    let metadata: HashMap<String, String> =
        [(EXTENSION_NAME_KEY.to_string(), GEOARROW_WKB.to_string())]
            .into_iter()
            .collect();

    Field::new(name, wkb_data_type(), nullable).with_metadata(metadata)
}

/// Check if a Field has `GeoArrow` Point extension type
#[must_use]
#[allow(dead_code)]
pub fn is_geoarrow_point(field: &Field) -> bool {
    field
        .metadata()
        .get(EXTENSION_NAME_KEY)
        .is_some_and(|v| v == GEOARROW_POINT)
}

/// Check if a Field has `GeoArrow` WKB extension type
#[must_use]
#[allow(dead_code)]
pub fn is_geoarrow_wkb(field: &Field) -> bool {
    field
        .metadata()
        .get(EXTENSION_NAME_KEY)
        .is_some_and(|v| v == GEOARROW_WKB)
}

/// Check if a Field has any `GeoArrow` geometry extension type
#[must_use]
pub fn is_geoarrow_geometry(field: &Field) -> bool {
    field.metadata().get(EXTENSION_NAME_KEY).is_some_and(|v| {
        v == GEOARROW_POINT
            || v == GEOARROW_LINESTRING
            || v == GEOARROW_POLYGON
            || v == GEOARROW_MULTIPOINT
            || v == GEOARROW_MULTILINESTRING
            || v == GEOARROW_MULTIPOLYGON
            || v == GEOARROW_GEOMETRY
            || v == GEOARROW_WKB
    })
}

/// Get the `GeoArrow` extension type name from a Field, if present
#[must_use]
pub fn get_geoarrow_type(field: &Field) -> Option<&str> {
    field.metadata().get(EXTENSION_NAME_KEY).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_data_type() {
        let dt = point_data_type();
        assert!(matches!(dt, DataType::FixedSizeList(_, 2)));
    }

    #[test]
    fn test_point_field_metadata() {
        let field = point_field("geometry", true);
        assert!(is_geoarrow_point(&field));
        assert!(is_geoarrow_geometry(&field));
        assert_eq!(get_geoarrow_type(&field), Some(GEOARROW_POINT));
    }

    #[test]
    fn test_wkb_field_metadata() {
        let field = wkb_field("geometry", true);
        assert!(is_geoarrow_wkb(&field));
        assert!(is_geoarrow_geometry(&field));
        assert_eq!(get_geoarrow_type(&field), Some(GEOARROW_WKB));
    }

    #[test]
    fn test_non_geoarrow_field() {
        let field = Field::new("x", DataType::Float64, false);
        assert!(!is_geoarrow_point(&field));
        assert!(!is_geoarrow_geometry(&field));
        assert_eq!(get_geoarrow_type(&field), None);
    }
}
