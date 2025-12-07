//! `ST_Area` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the area of a geometry.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns 0
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! All area calculations use `GEOS` for consistency and correctness.

use super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, Float64Array, Float64Builder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Area` User Defined Function
///
/// `ST_Area` returns the area of a geometry.
/// For Points and `LineStrings`, returns 0.
/// For Polygons, returns the area.
/// Accepts `GeoArrow` geometry types and uses `GEOS` for all calculations.
///
/// # SQL Usage
///
/// ```sql
/// -- Area of a polygon
/// SELECT ST_Area(geometry) FROM parcels;
///
/// -- Area from WKT
/// SELECT ST_Area(ST_GeomFromText('POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))'));
///
/// -- Filter by area
/// SELECT * FROM lots WHERE ST_Area(geometry) > 1000;
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
///
/// # Returns
///
/// `Float64`: The area of the geometry (0 for Points and `LineStrings`)
#[must_use]
pub fn create_st_area_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StAreaUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StAreaUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StAreaUDF {
    fn new() -> Self {
        use super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    // Point (returns 0)
                    TypeSignature::Exact(vec![point_data_type()]),
                    // WKB (universal format)
                    TypeSignature::Exact(vec![DataType::Binary]),
                    // Any single geometry
                    TypeSignature::Any(1),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StAreaUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_area"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Float64)
    }

    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        let geom_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        // Get geometry type and field from metadata if available
        let field = args.arg_fields.first().map(std::convert::AsRef::as_ref);
        let geo_type = field.and_then(get_geoarrow_type);

        // Validate input is GeoArrow type
        if let Some(f) = field
            && !is_geoarrow_geometry(f)
            && !matches!(
                geom_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            )
        {
            return Err(DataFusionError::Execution(
                "ST_Area requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let areas = compute_areas(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_Area failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(areas)))
    }
}

/// Compute areas for a geometry array using `GEOS`
fn compute_areas(
    arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<Float64Array, String> {
    let len = arr.len();
    let mut builder = Float64Builder::with_capacity(len);

    for i in 0..len {
        if arr.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos_geom = array_to_geos(arr, i, geo_type, field)?;
        let area = geos_geom
            .area()
            .map_err(|e| format!("GEOS area failed at row {i}: {e}"))?;

        builder.append_value(area);
    }

    Ok(builder.finish())
}

#[cfg(test)]
mod tests {
    use super::super::geoarrow_types::{GEOARROW_POINT, GEOARROW_WKB};
    use super::*;
    use datafusion::arrow::array::{BinaryArray, FixedSizeListArray, Float64Array};
    use geos::Geometry as GeosGeometry;

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

    fn wkt_to_wkb(wkt: &str) -> Vec<u8> {
        let geom = GeosGeometry::new_from_wkt(wkt).unwrap();
        geom.to_wkb().unwrap().into()
    }

    #[test]
    fn test_st_area_udf_creation() {
        let udf = create_st_area_udf();
        assert_eq!(udf.name(), "st_area");
    }

    #[test]
    fn test_area_point_returns_zero() {
        let points = create_point_array(&[(0.0, 0.0), (1.0, 1.0), (5.0, 5.0)]);

        let result = compute_areas(&points, Some(GEOARROW_POINT), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!((result.value(0) - 0.0).abs() < 1e-10);
        assert!((result.value(1) - 0.0).abs() < 1e-10);
        assert!((result.value(2) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_area_linestring_returns_zero() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 1 1, 2 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_areas(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_area_polygon() {
        // Unit square should have area 1.0
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_areas(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_area_polygon_rectangle() {
        // 4x3 rectangle should have area 12.0
        let wkb = wkt_to_wkb("POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_areas(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_area_polygon_with_hole() {
        // 4x4 square with 2x2 hole: 16 - 4 = 12
        let wkb = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0), (1 1, 3 1, 3 3, 1 3, 1 1))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_areas(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_area_multipolygon() {
        // Two unit squares = area 2.0
        let wkb =
            wkt_to_wkb("MULTIPOLYGON(((0 0, 1 0, 1 1, 0 1, 0 0)), ((2 0, 3 0, 3 1, 2 1, 2 0)))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_areas(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_area_null_handling() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb1.as_slice()),
            None,
            Some(wkb2.as_slice()),
        ]));

        let result = compute_areas(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
        assert!((result.value(0) - 1.0).abs() < 1e-10);
        assert!((result.value(2) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_area_multiple_geometries() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))"); // area 1
        let wkb2 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))"); // area 4
        let wkb3 = wkt_to_wkb("POLYGON((0 0, 3 0, 3 3, 0 3, 0 0))"); // area 9

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            wkb1.as_slice(),
            wkb2.as_slice(),
            wkb3.as_slice(),
        ]));

        let result = compute_areas(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!((result.value(0) - 1.0).abs() < 1e-10);
        assert!((result.value(1) - 4.0).abs() < 1e-10);
        assert!((result.value(2) - 9.0).abs() < 1e-10);
    }
}
