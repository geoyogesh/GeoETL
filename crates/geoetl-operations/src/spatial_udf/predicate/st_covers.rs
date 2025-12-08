//! `ST_Covers` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function tests if geometry A covers geometry B (no point in B is
//! outside A). Similar to `ST_Contains` but also returns true when points of B
//! are on the boundary of A.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `true` if A covers B, `false` otherwise.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, BooleanBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Covers` User Defined Function
///
/// `ST_Covers` returns true if no point in geometry B is outside geometry A.
/// This is similar to `ST_Contains` but is less strict - it also returns true
/// when all points of B lie on the boundary of A.
///
/// The inverse of `ST_Covers(A, B)` is `ST_CoveredBy(B, A)`.
///
/// # SQL Usage
///
/// ```sql
/// -- Check if a polygon covers a point
/// SELECT ST_Covers(boundary, location) FROM properties;
///
/// -- Find all points covered by a region (including boundary)
/// SELECT * FROM sensors
/// WHERE ST_Covers(ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'), location);
///
/// -- Check if a polygon covers a line on its boundary
/// SELECT ST_Covers(
///     ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
///     ST_GeomFromText('LINESTRING(0 0, 10 0)')
/// );
/// ```
///
/// # Arguments
///
/// - `geometry_a`: The covering `GeoArrow` geometry
/// - `geometry_b`: The `GeoArrow` geometry to test if covered
///
/// # Returns
///
/// `Boolean`: `true` if A covers B, `false` otherwise
#[must_use]
pub fn create_st_covers_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StCoversUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StCoversUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StCoversUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StCoversUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_covers"
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
                "ST_Covers requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        // Validate arrays have same length
        if geom_a_array.len() != geom_b_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_Covers: geometry arrays must have same length: {} vs {}",
                geom_a_array.len(),
                geom_b_array.len()
            )));
        }

        let results = compute_covers(
            &geom_a_array,
            &geom_b_array,
            type_a,
            type_b,
            field_a,
            field_b,
        )
        .map_err(|e| DataFusionError::Execution(format!("ST_Covers failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute covers predicate for two geometry arrays using `GEOS`
fn compute_covers(
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

        let covers = geos_a
            .covers(&geos_b)
            .map_err(|e| format!("GEOS covers failed at row {i}: {e}"))?;

        builder.append_value(covers);
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
    fn test_st_covers_udf_creation() {
        let udf = create_st_covers_udf();
        assert_eq!(udf.name(), "st_covers");
    }

    #[test]
    fn test_covers_point_inside_polygon() {
        // Polygon covers point inside it
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let point_wkb = wkt_to_wkb("POINT(5 5)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![polygon_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![point_wkb.as_slice()]));

        let result = compute_covers(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(result.value(0), "Polygon should cover point inside it");
    }

    #[test]
    fn test_covers_point_on_boundary() {
        // Polygon covers point on its boundary
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let point_wkb = wkt_to_wkb("POINT(10 5)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![polygon_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![point_wkb.as_slice()]));

        let result = compute_covers(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(
            result.value(0),
            "Polygon should cover point on its boundary"
        );
    }

    #[test]
    fn test_covers_point_outside_polygon() {
        // Polygon does not cover point outside
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let point_wkb = wkt_to_wkb("POINT(20 20)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![polygon_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![point_wkb.as_slice()]));

        let result = compute_covers(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(!result.value(0), "Polygon should not cover point outside");
    }

    #[test]
    fn test_covers_line_on_boundary() {
        // Polygon covers line on its boundary
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let line_wkb = wkt_to_wkb("LINESTRING(0 0, 10 0)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![polygon_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![line_wkb.as_slice()]));

        let result = compute_covers(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(result.value(0), "Polygon should cover line on its boundary");
    }

    #[test]
    fn test_covers_polygon_inside() {
        // Large polygon covers smaller polygon inside
        let large_wkb = wkt_to_wkb("POLYGON((0 0, 20 0, 20 20, 0 20, 0 0))");
        let small_wkb = wkt_to_wkb("POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![large_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![small_wkb.as_slice()]));

        let result = compute_covers(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(
            result.value(0),
            "Large polygon should cover smaller polygon inside"
        );
    }

    #[test]
    fn test_covers_null_handling() {
        let poly_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(poly_wkb.as_slice()),
            None,
            Some(poly_wkb.as_slice()),
        ]));

        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(poly_wkb.as_slice()),
            Some(poly_wkb.as_slice()),
            None,
        ]));

        let result = compute_covers(
            &arr1,
            &arr2,
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
    fn test_covers_same_polygon() {
        // A polygon covers itself
        let poly_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![poly_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![poly_wkb.as_slice()]));

        let result = compute_covers(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(result.value(0), "Polygon should cover itself");
    }
}
