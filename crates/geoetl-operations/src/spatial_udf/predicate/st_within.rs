//! `ST_Within` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function tests if geometry A is completely within geometry B.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `true` if A is within B (all points of A lie inside B, and at least
//! one interior point of A lies in B's interior), `false` otherwise.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, BooleanBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Within` User Defined Function
///
/// `ST_Within` returns true if geometry A is completely within geometry B.
/// This is the inverse of `ST_Contains` (i.e., `ST_Within(A, B) == ST_Contains(B, A)`).
///
/// # SQL Usage
///
/// ```sql
/// -- Check if point is within polygon
/// SELECT ST_Within(location, boundary) FROM properties;
///
/// -- Filter points within a region
/// SELECT * FROM sensors
/// WHERE ST_Within(location, ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'));
///
/// -- Find devices within each zone
/// SELECT d.name, z.zone_name FROM devices d, zones z
/// WHERE ST_Within(d.location, z.boundary);
/// ```
///
/// # Arguments
///
/// - `geometry_a`: The `GeoArrow` geometry to test if within
/// - `geometry_b`: The containing `GeoArrow` geometry
///
/// # Returns
///
/// `Boolean`: `true` if A is within B, `false` otherwise
#[must_use]
pub fn create_st_within_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StWithinUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StWithinUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StWithinUDF {
    fn new() -> Self {
        use super::super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    // Point-to-Point
                    TypeSignature::Exact(vec![point_data_type(), point_data_type()]),
                    // WKB-to-WKB
                    TypeSignature::Exact(vec![DataType::Binary, DataType::Binary]),
                    // Mixed types
                    TypeSignature::Any(2),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StWithinUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_within"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Boolean)
    }

    #[allow(clippy::similar_names)]
    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        // Determine target length from arrays (scalars need to be broadcast)
        let len = args
            .args
            .iter()
            .map(|arg| match arg {
                ColumnarValue::Array(arr) => arr.len(),
                ColumnarValue::Scalar(_) => 1,
            })
            .max()
            .unwrap_or(1);

        let geom_a_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array_of_size(len)?,
        };

        let geom_b_array = match &args.args[1] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array_of_size(len)?,
        };

        // Get geometry types and fields from metadata if available
        let field_a = args.arg_fields.first().map(std::convert::AsRef::as_ref);
        let field_b = args.arg_fields.get(1).map(std::convert::AsRef::as_ref);
        let type_a = field_a.and_then(get_geoarrow_type);
        let type_b = field_b.and_then(get_geoarrow_type);

        // Validate inputs are GeoArrow types
        if let (Some(f_a), Some(f_b)) = (field_a, field_b)
            && (!is_geoarrow_geometry(f_a) || !is_geoarrow_geometry(f_b))
            && (!matches!(
                geom_a_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            ) || !matches!(
                geom_b_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            ))
        {
            return Err(DataFusionError::Execution(
                "ST_Within requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        // Validate arrays have same length
        if geom_a_array.len() != geom_b_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_Within: geometry arrays must have same length: {} vs {}",
                geom_a_array.len(),
                geom_b_array.len()
            )));
        }

        let results = compute_within(
            &geom_a_array,
            &geom_b_array,
            type_a,
            type_b,
            field_a,
            field_b,
        )
        .map_err(|e| DataFusionError::Execution(format!("ST_Within failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute within predicate for two geometry arrays using `GEOS`
fn compute_within(
    arr_a: &ArrayRef,
    arr_b: &ArrayRef,
    type_a: Option<&str>,
    type_b: Option<&str>,
    field_a: Option<&arrow_schema::Field>,
    field_b: Option<&arrow_schema::Field>,
) -> Result<datafusion::arrow::array::BooleanArray, String> {
    let len = arr_a.len();
    let mut builder = BooleanBuilder::with_capacity(len);

    for i in 0..len {
        if arr_a.is_null(i) || arr_b.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos_a = array_to_geos(arr_a, i, type_a, field_a)?;
        let geos_b = array_to_geos(arr_b, i, type_b, field_b)?;

        let within = geos_a
            .within(&geos_b)
            .map_err(|e| format!("GEOS within failed at row {i}: {e}"))?;

        builder.append_value(within);
    }

    Ok(builder.finish())
}

#[cfg(test)]
mod tests {
    use super::super::super::geoarrow_types::GEOARROW_WKB;
    use super::*;
    use datafusion::arrow::array::BinaryArray;
    use geos::Geometry as GeosGeometry;

    fn wkt_to_wkb(wkt: &str) -> Vec<u8> {
        let geom = GeosGeometry::new_from_wkt(wkt).unwrap();
        geom.to_wkb().unwrap().into()
    }

    #[test]
    fn test_st_within_udf_creation() {
        let udf = create_st_within_udf();
        assert_eq!(udf.name(), "st_within");
    }

    #[test]
    fn test_within_point_in_polygon() {
        let point_inside_wkb = wkt_to_wkb("POINT(5 5)");
        let point_outside_wkb = wkt_to_wkb("POINT(20 20)");
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");

        let point_arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            point_inside_wkb.as_slice(),
            point_outside_wkb.as_slice(),
        ]));
        let polygon_arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            polygon_wkb.as_slice(),
            polygon_wkb.as_slice(),
        ]));

        let result = compute_within(
            &point_arr,
            &polygon_arr,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.value(0), "Point inside should be within polygon");
        assert!(
            !result.value(1),
            "Point outside should not be within polygon"
        );
    }

    #[test]
    fn test_within_polygon_in_polygon() {
        let small_inside = wkt_to_wkb("POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))");
        let small_outside = wkt_to_wkb("POLYGON((30 30, 40 30, 40 40, 30 40, 30 30))");
        let large_polygon = wkt_to_wkb("POLYGON((0 0, 20 0, 20 20, 0 20, 0 0))");

        let small_arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            small_inside.as_slice(),
            small_outside.as_slice(),
        ]));
        let large_arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            large_polygon.as_slice(),
            large_polygon.as_slice(),
        ]));

        let result = compute_within(
            &small_arr,
            &large_arr,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 2);
        assert!(
            result.value(0),
            "Small polygon inside should be within large"
        );
        assert!(
            !result.value(1),
            "Small polygon outside should not be within large"
        );
    }

    #[test]
    fn test_within_null_handling() {
        let point_wkb = wkt_to_wkb("POINT(0.5 0.5)");
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");

        let point_arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(point_wkb.as_slice()),
            None,
            Some(point_wkb.as_slice()),
        ]));

        let polygon_arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(polygon_wkb.as_slice()),
            Some(polygon_wkb.as_slice()),
            None,
        ]));

        let result = compute_within(
            &point_arr,
            &polygon_arr,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(result.is_null(2));
    }

    #[test]
    fn test_within_inverse_of_contains() {
        // ST_Within(A, B) should equal ST_Contains(B, A)
        let point_wkb = wkt_to_wkb("POINT(5 5)");
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");

        let point_arr: ArrayRef = Arc::new(BinaryArray::from(vec![point_wkb.as_slice()]));
        let polygon_arr: ArrayRef = Arc::new(BinaryArray::from(vec![polygon_wkb.as_slice()]));

        let within_result = compute_within(
            &point_arr,
            &polygon_arr,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(
            within_result.value(0),
            "Point should be within polygon (inverse of contains)"
        );
    }

    #[test]
    fn test_within_same_point() {
        // A point is within itself
        let point_wkb = wkt_to_wkb("POINT(5 5)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![point_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![point_wkb.as_slice()]));

        let result = compute_within(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(result.value(0), "Point should be within itself");
    }
}
