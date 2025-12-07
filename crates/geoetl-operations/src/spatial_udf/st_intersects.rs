//! `ST_Intersects` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function tests if two geometries intersect (share any portion of space).
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `true` if the geometries intersect, `false` otherwise.

use super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, BooleanBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Intersects` User Defined Function
///
/// `ST_Intersects` returns true if two geometries share any portion of space.
/// This is the opposite of `ST_Disjoint`.
///
/// # SQL Usage
///
/// ```sql
/// -- Check if two polygons intersect
/// SELECT ST_Intersects(geom_a, geom_b) FROM shapes;
///
/// -- Filter buildings in a region
/// SELECT * FROM buildings
/// WHERE ST_Intersects(geometry, ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'));
///
/// -- Count intersecting pairs
/// SELECT COUNT(*) FROM parcels p, flood_zones f
/// WHERE ST_Intersects(p.geometry, f.geometry);
/// ```
///
/// # Arguments
///
/// - `geometry1`: First `GeoArrow` geometry
/// - `geometry2`: Second `GeoArrow` geometry
///
/// # Returns
///
/// `Boolean`: `true` if the geometries intersect, `false` otherwise
#[must_use]
pub fn create_st_intersects_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StIntersectsUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StIntersectsUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StIntersectsUDF {
    fn new() -> Self {
        use super::geoarrow_types::point_data_type;

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

impl datafusion::logical_expr::ScalarUDFImpl for StIntersectsUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_intersects"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Boolean)
    }

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

        let geom1_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array_of_size(len)?,
        };

        let geom2_array = match &args.args[1] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array_of_size(len)?,
        };

        // Get geometry types and fields from metadata if available
        let field1 = args.arg_fields.first().map(std::convert::AsRef::as_ref);
        let field2 = args.arg_fields.get(1).map(std::convert::AsRef::as_ref);
        let type1 = field1.and_then(get_geoarrow_type);
        let type2 = field2.and_then(get_geoarrow_type);

        // Validate inputs are GeoArrow types
        if let (Some(f1), Some(f2)) = (field1, field2)
            && (!is_geoarrow_geometry(f1) || !is_geoarrow_geometry(f2))
            && (!matches!(
                geom1_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            ) || !matches!(
                geom2_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            ))
        {
            return Err(DataFusionError::Execution(
                "ST_Intersects requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        // Validate arrays have same length
        if geom1_array.len() != geom2_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_Intersects: geometry arrays must have same length: {} vs {}",
                geom1_array.len(),
                geom2_array.len()
            )));
        }

        let results = compute_intersects(&geom1_array, &geom2_array, type1, type2, field1, field2)
            .map_err(|e| DataFusionError::Execution(format!("ST_Intersects failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute intersects predicate for two geometry arrays using `GEOS`
fn compute_intersects(
    arr1: &ArrayRef,
    arr2: &ArrayRef,
    type1: Option<&str>,
    type2: Option<&str>,
    field1: Option<&arrow_schema::Field>,
    field2: Option<&arrow_schema::Field>,
) -> Result<datafusion::arrow::array::BooleanArray, String> {
    let len = arr1.len();
    let mut builder = BooleanBuilder::with_capacity(len);

    for i in 0..len {
        if arr1.is_null(i) || arr2.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos1 = array_to_geos(arr1, i, type1, field1)?;
        let geos2 = array_to_geos(arr2, i, type2, field2)?;

        let intersects = geos1
            .intersects(&geos2)
            .map_err(|e| format!("GEOS intersects failed at row {i}: {e}"))?;

        builder.append_value(intersects);
    }

    Ok(builder.finish())
}

#[cfg(test)]
mod tests {
    use super::super::geoarrow_types::GEOARROW_WKB;
    use super::*;
    use datafusion::arrow::array::BinaryArray;
    use geos::Geometry as GeosGeometry;

    fn wkt_to_wkb(wkt: &str) -> Vec<u8> {
        let geom = GeosGeometry::new_from_wkt(wkt).unwrap();
        geom.to_wkb().unwrap().into()
    }

    #[test]
    fn test_st_intersects_udf_creation() {
        let udf = create_st_intersects_udf();
        assert_eq!(udf.name(), "st_intersects");
    }

    #[test]
    fn test_intersects_overlapping_polygons() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_intersects(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.value(0), "Overlapping polygons should intersect");
    }

    #[test]
    fn test_intersects_disjoint_polygons() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((5 5, 6 5, 6 6, 5 6, 5 5))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_intersects(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        assert!(!result.value(0), "Disjoint polygons should not intersect");
    }

    #[test]
    fn test_intersects_touching_polygons() {
        // Polygons share an edge
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((1 0, 2 0, 2 1, 1 1, 1 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_intersects(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(result.value(0), "Touching polygons should intersect");
    }

    #[test]
    fn test_intersects_point_in_polygon() {
        let wkb1 = wkt_to_wkb("POINT(0.5 0.5)");
        let wkb2 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_intersects(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert!(result.value(0), "Point inside polygon should intersect");
    }

    #[test]
    fn test_intersects_null_handling() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb1.as_slice()),
            None,
            Some(wkb1.as_slice()),
        ]));

        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb2.as_slice()),
            Some(wkb2.as_slice()),
            None,
        ]));

        let result = compute_intersects(
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
    fn test_intersects_multiple_pairs() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((0.5 0.5, 1.5 0.5, 1.5 1.5, 0.5 1.5, 0.5 0.5))");
        let wkb3 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb4 = wkt_to_wkb("POLYGON((5 5, 6 5, 6 6, 5 6, 5 5))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice(), wkb3.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice(), wkb4.as_slice()]));

        let result = compute_intersects(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.value(0), "First pair should intersect");
        assert!(!result.value(1), "Second pair should not intersect");
    }
}
