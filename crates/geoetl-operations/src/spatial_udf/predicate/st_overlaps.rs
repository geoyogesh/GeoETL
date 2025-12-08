//! `ST_Overlaps` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function tests if two geometries overlap (share space but neither
//! is completely contained by the other).
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `true` if the geometries overlap (have some but not all points
//! in common, have the same dimension, and the intersection of their
//! interiors has the same dimension), `false` otherwise.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, BooleanBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Overlaps` User Defined Function
///
/// `ST_Overlaps` returns true if two geometries overlap. Two geometries overlap if
/// they have some but not all points in common, they have the same dimension, and
/// the intersection of their interiors has the same dimension as the geometries.
///
/// Note: Two points or two lines cannot overlap (they would be equal or disjoint).
/// Overlap applies to geometries of the same dimension where there is partial
/// but not complete intersection.
///
/// # SQL Usage
///
/// ```sql
/// -- Check if two polygons overlap
/// SELECT ST_Overlaps(region_a, region_b) FROM regions;
///
/// -- Find overlapping zones
/// SELECT a.name, b.name FROM zones a, zones b
/// WHERE a.id < b.id AND ST_Overlaps(a.boundary, b.boundary);
///
/// -- Filter to non-overlapping geometries
/// SELECT * FROM parcels
/// WHERE NOT ST_Overlaps(boundary, ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'));
/// ```
///
/// # Arguments
///
/// - `geometry_a`: The first `GeoArrow` geometry
/// - `geometry_b`: The second `GeoArrow` geometry
///
/// # Returns
///
/// `Boolean`: `true` if geometries overlap, `false` otherwise
#[must_use]
pub fn create_st_overlaps_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StOverlapsUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StOverlapsUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StOverlapsUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StOverlapsUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_overlaps"
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
                "ST_Overlaps requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        // Validate arrays have same length
        if geom_a_array.len() != geom_b_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_Overlaps: geometry arrays must have same length: {} vs {}",
                geom_a_array.len(),
                geom_b_array.len()
            )));
        }

        let results = compute_overlaps(
            &geom_a_array,
            &geom_b_array,
            type_a,
            type_b,
            field_a,
            field_b,
        )
        .map_err(|e| DataFusionError::Execution(format!("ST_Overlaps failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute overlaps predicate for two geometry arrays using `GEOS`
fn compute_overlaps(
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

        let overlaps = geos_a
            .overlaps(&geos_b)
            .map_err(|e| format!("GEOS overlaps failed at row {i}: {e}"))?;

        builder.append_value(overlaps);
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
    fn test_st_overlaps_udf_creation() {
        let udf = create_st_overlaps_udf();
        assert_eq!(udf.name(), "st_overlaps");
    }

    #[test]
    fn test_overlaps_polygons_partial() {
        // Two polygons that partially overlap
        let poly1_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let poly2_wkb = wkt_to_wkb("POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![poly1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![poly2_wkb.as_slice()]));

        let result = compute_overlaps(
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
            "Partially overlapping polygons should overlap"
        );
    }

    #[test]
    fn test_overlaps_polygons_disjoint() {
        // Two disjoint polygons
        let poly1_wkb = wkt_to_wkb("POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))");
        let poly2_wkb = wkt_to_wkb("POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![poly1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![poly2_wkb.as_slice()]));

        let result = compute_overlaps(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(!result.value(0), "Disjoint polygons should not overlap");
    }

    #[test]
    fn test_overlaps_polygon_contains_other() {
        // One polygon completely contains the other - should not overlap
        let large_wkb = wkt_to_wkb("POLYGON((0 0, 20 0, 20 20, 0 20, 0 0))");
        let small_wkb = wkt_to_wkb("POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![large_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![small_wkb.as_slice()]));

        let result = compute_overlaps(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(
            !result.value(0),
            "Containment is not overlap (one completely contains the other)"
        );
    }

    #[test]
    fn test_overlaps_identical_polygons() {
        // Identical polygons - should not overlap (they are equal)
        let poly_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![poly_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![poly_wkb.as_slice()]));

        let result = compute_overlaps(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(
            !result.value(0),
            "Identical polygons should not overlap (they are equal)"
        );
    }

    #[test]
    fn test_overlaps_points_do_not_overlap() {
        // Two different points - points cannot overlap
        let point1_wkb = wkt_to_wkb("POINT(0 0)");
        let point2_wkb = wkt_to_wkb("POINT(5 5)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![point1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![point2_wkb.as_slice()]));

        let result = compute_overlaps(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(!result.value(0), "Points cannot overlap");
    }

    #[test]
    fn test_overlaps_null_handling() {
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

        let result = compute_overlaps(
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
    fn test_overlaps_lines_partial() {
        // Two lines that partially overlap
        let line1_wkb = wkt_to_wkb("LINESTRING(0 0, 10 0)");
        let line2_wkb = wkt_to_wkb("LINESTRING(5 0, 15 0)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![line1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![line2_wkb.as_slice()]));

        let result = compute_overlaps(
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
            "Partially overlapping lines should overlap"
        );
    }
}
