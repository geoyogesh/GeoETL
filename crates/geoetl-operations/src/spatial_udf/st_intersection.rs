//! `ST_Intersection` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the intersection of two geometries.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the intersection geometry as WKB (Binary) with `geoarrow.wkb` metadata.

use super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry, wkb_field};
use super::geos_helpers::array_to_geos;
use arrow_schema::FieldRef;
use datafusion::arrow::array::{Array, ArrayRef, BinaryArray, BinaryBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{
    ReturnFieldArgs, ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility,
};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Intersection` User Defined Function
///
/// `ST_Intersection` returns a geometry that represents the shared portion
/// of two input geometries.
///
/// # SQL Usage
///
/// ```sql
/// -- Intersection of two geometry columns
/// SELECT ST_Intersection(parcel, flood_zone) FROM parcels, flood_zones;
///
/// -- Find overlap with a fixed region
/// SELECT ST_Intersection(geometry, ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'))
/// FROM buildings;
///
/// -- Chain with area calculation
/// SELECT ST_Area(ST_Intersection(geom_a, geom_b)) as overlap_area FROM shapes;
/// ```
///
/// # Arguments
///
/// - `geometry1`: First `GeoArrow` geometry
/// - `geometry2`: Second `GeoArrow` geometry
///
/// # Returns
///
/// `Binary` (WKB): The intersection geometry as WKB with `geoarrow.wkb` metadata.
/// Returns an empty geometry if the geometries do not intersect.
#[must_use]
pub fn create_st_intersection_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StIntersectionUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StIntersectionUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StIntersectionUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StIntersectionUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_intersection"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_intersection", nullable)))
    }

    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        let geom1_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        let geom2_array = match &args.args[1] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
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
                "ST_Intersection requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        // Validate arrays have same length
        if geom1_array.len() != geom2_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_Intersection: geometry arrays must have same length: {} vs {}",
                geom1_array.len(),
                geom2_array.len()
            )));
        }

        let intersections =
            compute_intersections(&geom1_array, &geom2_array, type1, type2, field1, field2)
                .map_err(|e| DataFusionError::Execution(format!("ST_Intersection failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(intersections)))
    }
}

/// Compute intersections for two geometry arrays using `GEOS`
fn compute_intersections(
    arr1: &ArrayRef,
    arr2: &ArrayRef,
    type1: Option<&str>,
    type2: Option<&str>,
    field1: Option<&arrow_schema::Field>,
    field2: Option<&arrow_schema::Field>,
) -> Result<BinaryArray, String> {
    let len = arr1.len();
    let mut builder = BinaryBuilder::with_capacity(len, len * 256);

    for i in 0..len {
        if arr1.is_null(i) || arr2.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos1 = array_to_geos(arr1, i, type1, field1)?;
        let geos2 = array_to_geos(arr2, i, type2, field2)?;

        let intersection = geos1
            .intersection(&geos2)
            .map_err(|e| format!("GEOS intersection failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = intersection
            .to_wkb()
            .map_err(|e| format!("Failed to convert intersection to WKB at row {i}: {e}"))?
            .into();

        builder.append_value(&wkb_bytes);
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
    fn test_st_intersection_udf_creation() {
        let udf = create_st_intersection_udf();
        assert_eq!(udf.name(), "st_intersection");
    }

    #[test]
    fn test_intersection_overlapping_polygons() {
        // Two overlapping squares: intersection is 0.5x1 rectangle
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((0.5 0, 1.5 0, 1.5 1, 0.5 1, 0.5 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_intersections(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);

        let intersection = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = intersection.area().unwrap();

        // Overlap is 0.5x1 = 0.5
        assert!(
            (area - 0.5).abs() < 0.001,
            "Expected intersection area 0.5, got {area}"
        );
    }

    #[test]
    fn test_intersection_disjoint_polygons() {
        // Two non-overlapping squares: intersection is empty
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((2 0, 3 0, 3 1, 2 1, 2 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_intersections(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let intersection = GeosGeometry::new_from_wkb(result.value(0)).unwrap();

        // Disjoint intersection is empty
        assert!(intersection.is_empty().unwrap());
    }

    #[test]
    fn test_intersection_containing_polygon() {
        // Small square fully inside larger square
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))"); // Large
        let wkb2 = wkt_to_wkb("POLYGON((1 1, 2 1, 2 2, 1 2, 1 1))"); // Small

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_intersections(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let intersection = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = intersection.area().unwrap();

        // Intersection is the smaller polygon (area 1)
        assert!(
            (area - 1.0).abs() < 0.001,
            "Expected intersection area 1.0, got {area}"
        );
    }

    #[test]
    fn test_intersection_line_polygon() {
        // Line crossing through a polygon
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))");
        let wkb2 = wkt_to_wkb("LINESTRING(-1 2, 5 2)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_intersections(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let intersection = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let geom_type = intersection.geometry_type();

        // Line intersected with polygon gives a LineString
        assert_eq!(geom_type, geos::GeometryTypes::LineString);

        let length = intersection.length().unwrap();
        // Line from (0,2) to (4,2) has length 4
        assert!(
            (length - 4.0).abs() < 0.001,
            "Expected length 4.0, got {length}"
        );
    }

    #[test]
    fn test_intersection_null_handling() {
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

        let result = compute_intersections(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0)); // Both valid
        assert!(result.is_null(1)); // First null
        assert!(result.is_null(2)); // Second null
    }

    #[test]
    fn test_intersection_same_geometry() {
        // Intersection of geometry with itself = original geometry
        let wkb = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_intersections(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let intersection = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = intersection.area().unwrap();

        // Intersection with self = original (area 4)
        assert!((area - 4.0).abs() < 0.001, "Expected area 4.0, got {area}");
    }

    #[test]
    fn test_intersection_multiple_pairs() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((1 0, 3 0, 3 2, 1 2, 1 0))");
        let wkb3 = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))");
        let wkb4 = wkt_to_wkb("POLYGON((2 2, 6 2, 6 6, 2 6, 2 2))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice(), wkb3.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice(), wkb4.as_slice()]));

        let result = compute_intersections(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 2);

        // First intersection: 1x2 = 2
        let i1 = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        assert!((i1.area().unwrap() - 2.0).abs() < 0.001);

        // Second intersection: 2x2 = 4
        let i2 = GeosGeometry::new_from_wkb(result.value(1)).unwrap();
        assert!((i2.area().unwrap() - 4.0).abs() < 0.001);
    }
}
