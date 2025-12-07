//! Shared GEOS helper utilities for spatial UDFs
//!
//! This module provides common functions for converting between `GeoArrow` arrays
//! and GEOS geometries, eliminating code duplication across spatial UDFs.

use super::geoarrow_types::{GEOARROW_GEOMETRY, GEOARROW_POINT, GEOARROW_WKB};
use datafusion::arrow::array::{Array, ArrayRef, BinaryArray, FixedSizeListArray, Float64Array};
use datafusion::arrow::datatypes::DataType;
use geoarrow_array::{GeoArrowArray, GeoArrowArrayAccessor};
use geos::{CoordSeq, Geom, Geometry as GeosGeometry};
use geozero::{CoordDimensions, ToWkb};

/// Convert a single geometry from a `GeoArrow` array to a GEOS geometry
///
/// This function handles all supported `GeoArrow` geometry types:
/// - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
/// - `geoarrow.wkb` (Binary)
/// - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
///
/// # Arguments
///
/// * `arr` - The Arrow array containing geometries
/// * `idx` - The index of the geometry to extract
/// * `geo_type` - Optional `GeoArrow` type hint from field metadata
/// * `field` - Optional Arrow field for metadata access
///
/// # Returns
///
/// A GEOS geometry or an error message
///
/// # Example
///
/// ```ignore
/// let geos_geom = array_to_geos(&arr, 0, Some(GEOARROW_WKB), None)?;
/// let area = geos_geom.area()?;
/// ```
pub fn array_to_geos(
    arr: &ArrayRef,
    idx: usize,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<GeosGeometry, String> {
    match geo_type {
        Some(GEOARROW_POINT) | None if matches!(arr.data_type(), DataType::FixedSizeList(_, 2)) => {
            // GeoArrow Point -> GEOS
            let points = arr
                .as_any()
                .downcast_ref::<FixedSizeListArray>()
                .ok_or("Expected FixedSizeList for Point")?;

            let coords = points
                .values()
                .as_any()
                .downcast_ref::<Float64Array>()
                .ok_or("Expected Float64 coordinates")?;

            let offset = idx * 2;
            let x = coords.value(offset);
            let y = coords.value(offset + 1);

            let coord_seq = CoordSeq::new_from_vec(&[[x, y]])
                .map_err(|e| format!("Failed to create CoordSeq: {e}"))?;

            GeosGeometry::create_point(coord_seq)
                .map_err(|e| format!("Failed to create GEOS point: {e}"))
        },
        Some(GEOARROW_WKB) | None if matches!(arr.data_type(), DataType::Binary) => {
            // GeoArrow WKB -> GEOS
            let wkb_array = arr
                .as_any()
                .downcast_ref::<BinaryArray>()
                .ok_or("Expected Binary for WKB")?;

            let wkb = wkb_array.value(idx);
            GeosGeometry::new_from_wkb(wkb).map_err(|e| format!("Invalid WKB at row {idx}: {e}"))
        },
        Some(GEOARROW_GEOMETRY) => {
            // GeoArrow mixed geometry (Union) -> GEOS via WKB
            geometry_array_to_geos(arr, idx, field)
        },
        Some(other) => Err(format!("Unsupported geometry type: {other}")),
        None => Err(format!(
            "Unknown geometry format: {:?}. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.",
            arr.data_type()
        )),
    }
}

/// Convert a `GeoArrow` geometry array element to GEOS via WKB
///
/// This handles the `geoarrow.geometry` Union type which contains mixed
/// geometry types (from `GeoJSON` parsing).
///
/// # Arguments
///
/// * `arr` - The Arrow array containing mixed geometries
/// * `idx` - The index of the geometry to extract
/// * `field` - The Arrow field with `GeoArrow` metadata (required)
///
/// # Returns
///
/// A GEOS geometry or an error message
pub fn geometry_array_to_geos(
    arr: &ArrayRef,
    idx: usize,
    field: Option<&arrow_schema::Field>,
) -> Result<GeosGeometry, String> {
    use geoarrow_array::array::GeometryArray;

    // Need field metadata to construct GeometryArray
    let field = field.ok_or("Field metadata required for geoarrow.geometry type")?;

    let geom_arr = GeometryArray::try_from((arr.as_ref(), field))
        .map_err(|e| format!("Failed to convert to GeometryArray: {e}"))?;

    if geom_arr.is_null(idx) {
        return Err(format!("Null geometry at row {idx}"));
    }

    // Get geometry and convert to WKB
    let geom = geom_arr
        .value(idx)
        .map_err(|e| format!("Failed to get geometry at row {idx}: {e}"))?;

    let wkb_bytes = geom
        .to_wkb(CoordDimensions::xy())
        .map_err(|e| format!("Failed to convert geometry to WKB at row {idx}: {e}"))?;

    GeosGeometry::new_from_wkb(&wkb_bytes).map_err(|e| format!("Invalid WKB at row {idx}: {e}"))
}

/// Convert a GEOS geometry to WKB bytes
///
/// # Arguments
///
/// * `geom` - The GEOS geometry to convert
///
/// # Returns
///
/// WKB bytes or an error message
#[allow(dead_code)] // Will be used when refactoring geometry-returning UDFs
pub fn geos_to_wkb(geom: &GeosGeometry) -> Result<Vec<u8>, String> {
    geom.to_wkb()
        .map(std::convert::Into::into)
        .map_err(|e| format!("Failed to convert geometry to WKB: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::BinaryArray;
    use std::sync::Arc;

    fn wkt_to_wkb(wkt: &str) -> Vec<u8> {
        let geom = GeosGeometry::new_from_wkt(wkt).unwrap();
        geom.to_wkb().unwrap().into()
    }

    /// Create a `GeoArrow` Point array from coordinate pairs
    fn create_point_array(coords: &[(f64, f64)]) -> ArrayRef {
        let len = coords.len();
        let mut values = Vec::with_capacity(len * 2);
        for (x, y) in coords {
            values.push(*x);
            values.push(*y);
        }

        let coords_array = Float64Array::from(values);
        let field = Arc::new(arrow_schema::Field::new("xy", DataType::Float64, false));
        let points = FixedSizeListArray::new(field, 2, Arc::new(coords_array), None);

        Arc::new(points)
    }

    #[test]
    fn test_array_to_geos_point() {
        let points = create_point_array(&[(1.0, 2.0), (3.0, 4.0)]);

        let geom0 = array_to_geos(&points, 0, Some(GEOARROW_POINT), None).unwrap();
        let geom1 = array_to_geos(&points, 1, Some(GEOARROW_POINT), None).unwrap();

        assert_eq!(geom0.geometry_type(), geos::GeometryTypes::Point);
        assert_eq!(geom1.geometry_type(), geos::GeometryTypes::Point);

        let cs0 = geom0.get_coord_seq().unwrap();
        assert!((cs0.get_x(0).unwrap() - 1.0).abs() < 1e-10);
        assert!((cs0.get_y(0).unwrap() - 2.0).abs() < 1e-10);

        let cs1 = geom1.get_coord_seq().unwrap();
        assert!((cs1.get_x(0).unwrap() - 3.0).abs() < 1e-10);
        assert!((cs1.get_y(0).unwrap() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_array_to_geos_wkb() {
        let wkb1 = wkt_to_wkb("POINT(5 10)");
        let wkb2 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice(), wkb2.as_slice()]));

        let geom0 = array_to_geos(&arr, 0, Some(GEOARROW_WKB), None).unwrap();
        let geom1 = array_to_geos(&arr, 1, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(geom0.geometry_type(), geos::GeometryTypes::Point);
        assert_eq!(geom1.geometry_type(), geos::GeometryTypes::Polygon);

        let area = geom1.area().unwrap();
        assert!((area - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_array_to_geos_wkb_auto_detect() {
        // Without explicit type hint, should detect from DataType::Binary
        let wkb = wkt_to_wkb("LINESTRING(0 0, 1 1, 2 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let geom = array_to_geos(&arr, 0, None, None).unwrap();
        assert_eq!(geom.geometry_type(), geos::GeometryTypes::LineString);
    }

    #[test]
    fn test_array_to_geos_point_auto_detect() {
        // Without explicit type hint, should detect from DataType::FixedSizeList
        let points = create_point_array(&[(7.0, 8.0)]);

        let geom = array_to_geos(&points, 0, None, None).unwrap();
        assert_eq!(geom.geometry_type(), geos::GeometryTypes::Point);

        let cs = geom.get_coord_seq().unwrap();
        assert!((cs.get_x(0).unwrap() - 7.0).abs() < 1e-10);
        assert!((cs.get_y(0).unwrap() - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_array_to_geos_invalid_wkb() {
        let invalid_wkb: ArrayRef = Arc::new(BinaryArray::from(vec![&[0u8, 1, 2, 3][..]]));

        let result = array_to_geos(&invalid_wkb, 0, Some(GEOARROW_WKB), None);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            err.contains("Invalid WKB"),
            "Expected 'Invalid WKB' error, got: {err}"
        );
    }

    #[test]
    fn test_array_to_geos_unsupported_type() {
        let points = create_point_array(&[(0.0, 0.0)]);

        let result = array_to_geos(&points, 0, Some("unsupported.type"), None);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            err.contains("Unsupported geometry type"),
            "Expected 'Unsupported geometry type' error, got: {err}"
        );
    }

    #[test]
    fn test_geos_to_wkb() {
        let geom = GeosGeometry::new_from_wkt("POINT(1 2)").unwrap();
        let wkb = geos_to_wkb(&geom).unwrap();

        // Verify roundtrip
        let geom2 = GeosGeometry::new_from_wkb(&wkb).unwrap();
        assert_eq!(geom2.geometry_type(), geos::GeometryTypes::Point);

        let cs = geom2.get_coord_seq().unwrap();
        assert!((cs.get_x(0).unwrap() - 1.0).abs() < 1e-10);
        assert!((cs.get_y(0).unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_geos_to_wkb_polygon() {
        let geom = GeosGeometry::new_from_wkt("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))").unwrap();
        let wkb = geos_to_wkb(&geom).unwrap();

        let geom2 = GeosGeometry::new_from_wkb(&wkb).unwrap();
        assert_eq!(geom2.geometry_type(), geos::GeometryTypes::Polygon);
        assert!((geom2.area().unwrap() - 4.0).abs() < 1e-10);
    }
}
