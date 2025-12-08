//! `ST_X` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function extracts the X coordinate of a Point geometry.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `NULL` for non-Point geometries.

use super::super::geoarrow_types::{GEOARROW_POINT, get_geoarrow_type, is_geoarrow_geometry};
use super::super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, Float64Array, Float64Builder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_X` User Defined Function
///
/// `ST_X` returns the X coordinate of a Point geometry.
/// Returns `NULL` for non-Point geometries or empty points.
///
/// # SQL Usage
///
/// ```sql
/// -- Get X coordinate of a point
/// SELECT ST_X(ST_Point(10, 20));
/// -- Returns: 10.0
///
/// -- Extract X from geometry column
/// SELECT ST_X(location) FROM cities;
///
/// -- Filter by longitude
/// SELECT * FROM points WHERE ST_X(geom) > -122.0;
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
///
/// # Returns
///
/// `Float64`: The X coordinate (longitude), or `NULL` for non-Point geometries
#[must_use]
pub fn create_st_x_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StXUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StXUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StXUDF {
    fn new() -> Self {
        use super::super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![point_data_type()]),
                    TypeSignature::Exact(vec![DataType::Binary]),
                    TypeSignature::Any(1),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StXUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_x"
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

        let field = args.arg_fields.first().map(std::convert::AsRef::as_ref);
        let geo_type = field.and_then(get_geoarrow_type);

        if let Some(f) = field
            && !is_geoarrow_geometry(f)
            && !matches!(
                geom_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            )
        {
            return Err(DataFusionError::Execution(
                "ST_X requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_x(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_X failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute X coordinates for a geometry array using `GEOS`
fn compute_x(
    arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<Float64Array, String> {
    let len = arr.len();
    let mut builder = Float64Builder::with_capacity(len);

    // Optimization for native point arrays
    if geo_type == Some(GEOARROW_POINT) || matches!(arr.data_type(), DataType::FixedSizeList(_, 2))
    {
        let coords = arr
            .as_any()
            .downcast_ref::<datafusion::arrow::array::FixedSizeListArray>()
            .ok_or("Expected FixedSizeListArray for point")?;

        let values = coords
            .values()
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or("Expected Float64 coordinates")?;

        for i in 0..len {
            if arr.is_null(i) {
                builder.append_null();
            } else {
                builder.append_value(values.value(i * 2));
            }
        }
        return Ok(builder.finish());
    }

    // General case using GEOS
    for i in 0..len {
        if arr.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos_geom = array_to_geos(arr, i, geo_type, field)?;

        // Only return X for Point geometries
        let geometry_type_str = format!("{:?}", geos_geom.geometry_type());
        if !geometry_type_str.contains("Point") || geos_geom.is_empty().unwrap_or(true) {
            builder.append_null();
            continue;
        }

        let coord_seq = geos_geom
            .get_coord_seq()
            .map_err(|e| format!("Failed to get coordinate sequence at row {i}: {e}"))?;

        let x = coord_seq
            .get_x(0)
            .map_err(|e| format!("Failed to get X coordinate at row {i}: {e}"))?;

        builder.append_value(x);
    }

    Ok(builder.finish())
}

#[cfg(test)]
mod tests {
    use super::super::super::geoarrow_types::{GEOARROW_POINT, GEOARROW_WKB};
    use super::*;
    use datafusion::arrow::array::{BinaryArray, FixedSizeListArray, Float64Array};
    use geos::Geometry as GeosGeometry;

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
    fn test_st_x_udf_creation() {
        let udf = create_st_x_udf();
        assert_eq!(udf.name(), "st_x");
    }

    #[test]
    fn test_x_native_points() {
        let points = create_point_array(&[(10.0, 20.0), (-122.4, 37.8), (0.0, 0.0)]);

        let result = compute_x(&points, Some(GEOARROW_POINT), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!((result.value(0) - 10.0).abs() < 1e-10);
        assert!((result.value(1) - (-122.4)).abs() < 1e-10);
        assert!((result.value(2) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_x_wkb_point() {
        let wkb = wkt_to_wkb("POINT(42.5 17.3)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_x(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 42.5).abs() < 1e-10);
    }

    #[test]
    fn test_x_non_point_returns_null() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 1 1)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_x(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.is_null(0));
    }

    #[test]
    fn test_x_polygon_returns_null() {
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_x(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.is_null(0));
    }

    #[test]
    fn test_x_null_handling() {
        let wkb1 = wkt_to_wkb("POINT(5 10)");
        let wkb2 = wkt_to_wkb("POINT(15 20)");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb1.as_slice()),
            None,
            Some(wkb2.as_slice()),
        ]));

        let result = compute_x(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
        assert!((result.value(0) - 5.0).abs() < 1e-10);
        assert!((result.value(2) - 15.0).abs() < 1e-10);
    }
}
