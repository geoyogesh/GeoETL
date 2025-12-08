//! `ST_Crosses` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function tests if two geometries cross each other (have some but not
//! all interior points in common, and the dimension of the intersection is
//! less than that of at least one of the geometries).
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `true` if the geometries cross, `false` otherwise.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, BooleanBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Crosses` User Defined Function
///
/// `ST_Crosses` returns true if two geometries have some but not all interior
/// points in common, and the dimension of the intersection is less than that
/// of at least one of the geometries. This is typically used to test if a line
/// crosses another line or polygon.
///
/// # SQL Usage
///
/// ```sql
/// -- Check if a road crosses a river
/// SELECT ST_Crosses(road_geom, river_geom) FROM infrastructure;
///
/// -- Find all roads that cross a boundary
/// SELECT r.name FROM roads r, boundaries b
/// WHERE ST_Crosses(r.geom, b.geom);
///
/// -- Check if two lines cross
/// SELECT ST_Crosses(
///     ST_GeomFromText('LINESTRING(0 0, 10 10)'),
///     ST_GeomFromText('LINESTRING(0 10, 10 0)')
/// );
/// ```
///
/// # Arguments
///
/// - `geometry_a`: The first `GeoArrow` geometry
/// - `geometry_b`: The second `GeoArrow` geometry
///
/// # Returns
///
/// `Boolean`: `true` if geometries cross, `false` otherwise
#[must_use]
pub fn create_st_crosses_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StCrossesUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StCrossesUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StCrossesUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StCrossesUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_crosses"
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
                "ST_Crosses requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        // Validate arrays have same length
        if geom_a_array.len() != geom_b_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_Crosses: geometry arrays must have same length: {} vs {}",
                geom_a_array.len(),
                geom_b_array.len()
            )));
        }

        let results = compute_crosses(
            &geom_a_array,
            &geom_b_array,
            type_a,
            type_b,
            field_a,
            field_b,
        )
        .map_err(|e| DataFusionError::Execution(format!("ST_Crosses failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute crosses predicate for two geometry arrays using `GEOS`
fn compute_crosses(
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

        let crosses = geos_a
            .crosses(&geos_b)
            .map_err(|e| format!("GEOS crosses failed at row {i}: {e}"))?;

        builder.append_value(crosses);
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
    fn test_st_crosses_udf_creation() {
        let udf = create_st_crosses_udf();
        assert_eq!(udf.name(), "st_crosses");
    }

    #[test]
    fn test_crosses_two_lines() {
        // Two lines that cross each other
        let line1_wkb = wkt_to_wkb("LINESTRING(0 0, 10 10)");
        let line2_wkb = wkt_to_wkb("LINESTRING(0 10, 10 0)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![line1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![line2_wkb.as_slice()]));

        let result = compute_crosses(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(result.value(0), "Crossing lines should cross");
    }

    #[test]
    fn test_crosses_parallel_lines() {
        // Two parallel lines (don't cross)
        let line1_wkb = wkt_to_wkb("LINESTRING(0 0, 10 0)");
        let line2_wkb = wkt_to_wkb("LINESTRING(0 5, 10 5)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![line1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![line2_wkb.as_slice()]));

        let result = compute_crosses(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(!result.value(0), "Parallel lines should not cross");
    }

    #[test]
    fn test_crosses_line_polygon() {
        // Line crossing through a polygon
        let line_wkb = wkt_to_wkb("LINESTRING(-5 5, 15 5)");
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![line_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![polygon_wkb.as_slice()]));

        let result = compute_crosses(
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
            "Line crossing through polygon should cross"
        );
    }

    #[test]
    fn test_crosses_line_inside_polygon() {
        // Line completely inside a polygon (doesn't cross - it's within)
        let line_wkb = wkt_to_wkb("LINESTRING(2 5, 8 5)");
        let polygon_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![line_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![polygon_wkb.as_slice()]));

        let result = compute_crosses(
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
            "Line inside polygon should not cross (it's within)"
        );
    }

    #[test]
    fn test_crosses_polygons_do_not_cross() {
        // Two polygons cannot cross (they overlap or touch instead)
        let poly1_wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let poly2_wkb = wkt_to_wkb("POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![poly1_wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![poly2_wkb.as_slice()]));

        let result = compute_crosses(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(!result.value(0), "Polygons cannot cross (they overlap)");
    }

    #[test]
    fn test_crosses_null_handling() {
        let line_wkb = wkt_to_wkb("LINESTRING(0 0, 10 10)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(line_wkb.as_slice()),
            None,
            Some(line_wkb.as_slice()),
        ]));

        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(line_wkb.as_slice()),
            Some(line_wkb.as_slice()),
            None,
        ]));

        let result = compute_crosses(
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
}
