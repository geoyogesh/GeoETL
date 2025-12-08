//! `ST_Touches` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function tests if two geometries touch (have at least one point in
//! common, but their interiors do not intersect).
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `true` if the geometries touch, `false` otherwise.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, BooleanBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Touches` User Defined Function
///
/// `ST_Touches` returns true if two geometries have at least one point in common,
/// but their interiors do not intersect. Two geometries touch if they have at
/// least one boundary point in common, but no interior points.
///
/// # SQL Usage
///
/// ```sql
/// -- Check if two polygons touch
/// SELECT ST_Touches(boundary_a, boundary_b) FROM zones;
///
/// -- Find adjacent parcels
/// SELECT a.id, b.id FROM parcels a, parcels b
/// WHERE a.id < b.id AND ST_Touches(a.boundary, b.boundary);
///
/// -- Check if line touches polygon boundary
/// SELECT ST_Touches(road, city_limits) FROM roads, cities;
/// ```
///
/// # Arguments
///
/// - `geometry_a`: The first `GeoArrow` geometry
/// - `geometry_b`: The second `GeoArrow` geometry
///
/// # Returns
///
/// `Boolean`: `true` if geometries touch, `false` otherwise
#[must_use]
pub fn create_st_touches_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StTouchesUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StTouchesUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StTouchesUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StTouchesUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_touches"
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
                "ST_Touches requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        // Validate arrays have same length
        if geom_a_array.len() != geom_b_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_Touches: geometry arrays must have same length: {} vs {}",
                geom_a_array.len(),
                geom_b_array.len()
            )));
        }

        let results = compute_touches(
            &geom_a_array,
            &geom_b_array,
            type_a,
            type_b,
            field_a,
            field_b,
        )
        .map_err(|e| DataFusionError::Execution(format!("ST_Touches failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute touches predicate for two geometry arrays using `GEOS`
fn compute_touches(
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

        let touches = geos_a
            .touches(&geos_b)
            .map_err(|e| format!("GEOS touches failed at row {i}: {e}"))?;

        builder.append_value(touches);
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
    fn test_st_touches_udf_creation() {
        let udf = create_st_touches_udf();
        assert_eq!(udf.name(), "st_touches");
    }

    #[test]
    fn test_touches_adjacent_polygons() {
        // Two polygons that share an edge (touch)
        let poly1_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let poly2_wkb = wkt_to_wkb("POLYGON((10 0, 20 0, 20 10, 10 10, 10 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![poly1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![poly2_wkb.as_slice()]));

        let result = compute_touches(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(result.value(0), "Adjacent polygons should touch");
    }

    #[test]
    fn test_touches_disjoint_polygons() {
        // Two disjoint polygons
        let poly1_wkb = wkt_to_wkb("POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))");
        let poly2_wkb = wkt_to_wkb("POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![poly1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![poly2_wkb.as_slice()]));

        let result = compute_touches(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(!result.value(0), "Disjoint polygons should not touch");
    }

    #[test]
    fn test_touches_overlapping_polygons() {
        // Two overlapping polygons (interiors intersect, so don't touch)
        let poly1_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let poly2_wkb = wkt_to_wkb("POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![poly1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![poly2_wkb.as_slice()]));

        let result = compute_touches(
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
            "Overlapping polygons should not touch (interiors intersect)"
        );
    }

    #[test]
    fn test_touches_point_on_polygon_boundary() {
        // Point on polygon boundary
        let point_wkb = wkt_to_wkb("POINT(10 5)");
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![point_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![polygon_wkb.as_slice()]));

        let result = compute_touches(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(result.value(0), "Point on boundary should touch polygon");
    }

    #[test]
    fn test_touches_point_inside_polygon() {
        // Point inside polygon (does not touch - interior intersects)
        let point_wkb = wkt_to_wkb("POINT(5 5)");
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![point_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![polygon_wkb.as_slice()]));

        let result = compute_touches(
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
            "Point inside polygon should not touch (interior intersects)"
        );
    }

    #[test]
    fn test_touches_null_handling() {
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

        let result = compute_touches(
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
    fn test_touches_polygons_corner() {
        // Two polygons that touch at a single corner point
        let poly1_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let poly2_wkb = wkt_to_wkb("POLYGON((10 10, 20 10, 20 20, 10 20, 10 10))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![poly1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![poly2_wkb.as_slice()]));

        let result = compute_touches(
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
            "Polygons touching at a corner should touch"
        );
    }
}
