//! `ST_Difference` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the difference of two geometries (A - B).
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the difference geometry as WKB (Binary) with `geoarrow.wkb` metadata.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry, wkb_field};
use super::super::geos_helpers::array_to_geos;
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

/// Create the `ST_Difference` User Defined Function
///
/// `ST_Difference` returns a geometry that represents the part of geometry A
/// that does not intersect with geometry B. This is the set-theoretic difference.
///
/// The order of arguments matters: `ST_Difference(A, B)` gives "A minus B".
///
/// # SQL Usage
///
/// ```sql
/// -- Remove overlap from parcel
/// SELECT ST_Difference(parcel, easement) FROM parcels, easements;
///
/// -- Subtract a buffer zone
/// SELECT ST_Difference(geometry, ST_Buffer(obstacle, 10)) FROM features;
///
/// -- Calculate area after subtraction
/// SELECT ST_Area(ST_Difference(total_area, excluded_zone)) FROM regions;
/// ```
///
/// # Arguments
///
/// - `geometry_a`: First `GeoArrow` geometry (the geometry to subtract from)
/// - `geometry_b`: Second `GeoArrow` geometry (the geometry to subtract)
///
/// # Returns
///
/// `Binary` (WKB): The difference geometry (A - B) as WKB with `geoarrow.wkb` metadata.
/// Returns geometry A unchanged if there is no intersection.
#[must_use]
pub fn create_st_difference_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StDifferenceUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StDifferenceUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StDifferenceUDF {
    fn new() -> Self {
        use super::super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![point_data_type(), point_data_type()]),
                    TypeSignature::Exact(vec![DataType::Binary, DataType::Binary]),
                    TypeSignature::Any(2),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StDifferenceUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_difference"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_difference", nullable)))
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

        let field1 = args.arg_fields.first().map(std::convert::AsRef::as_ref);
        let field2 = args.arg_fields.get(1).map(std::convert::AsRef::as_ref);
        let type1 = field1.and_then(get_geoarrow_type);
        let type2 = field2.and_then(get_geoarrow_type);

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
                "ST_Difference requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        if geom1_array.len() != geom2_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_Difference: geometry arrays must have same length: {} vs {}",
                geom1_array.len(),
                geom2_array.len()
            )));
        }

        let differences =
            compute_differences(&geom1_array, &geom2_array, type1, type2, field1, field2)
                .map_err(|e| DataFusionError::Execution(format!("ST_Difference failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(differences)))
    }
}

/// Compute differences for two geometry arrays using `GEOS`
fn compute_differences(
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

        let difference = geos1
            .difference(&geos2)
            .map_err(|e| format!("GEOS difference failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = difference
            .to_wkb()
            .map_err(|e| format!("Failed to convert difference to WKB at row {i}: {e}"))?
            .into();

        builder.append_value(&wkb_bytes);
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
    fn test_st_difference_udf_creation() {
        let udf = create_st_difference_udf();
        assert_eq!(udf.name(), "st_difference");
    }

    #[test]
    fn test_difference_overlapping_polygons() {
        // Two overlapping squares: difference removes the overlap
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))"); // Area 4
        let wkb2 = wkt_to_wkb("POLYGON((1 0, 3 0, 3 2, 1 2, 1 0))"); // Overlaps right half

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_differences(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);

        let difference = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = difference.area().unwrap();

        // Original 4, minus overlap 2, equals 2
        assert!(
            (area - 2.0).abs() < 0.001,
            "Expected difference area 2.0, got {area}"
        );
    }

    #[test]
    fn test_difference_disjoint_polygons() {
        // Non-overlapping: difference equals original
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))"); // Area 1
        let wkb2 = wkt_to_wkb("POLYGON((5 0, 6 0, 6 1, 5 1, 5 0))"); // Far away

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_differences(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let difference = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = difference.area().unwrap();

        // No overlap, so difference = original
        assert!(
            (area - 1.0).abs() < 0.001,
            "Expected area 1.0 (original), got {area}"
        );
    }

    #[test]
    fn test_difference_contained_polygon() {
        // Small polygon fully inside larger: creates a "donut"
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))"); // Large, area 16
        let wkb2 = wkt_to_wkb("POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))"); // Small inside, area 4

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_differences(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let difference = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = difference.area().unwrap();

        // 16 - 4 = 12 (donut shape)
        assert!(
            (area - 12.0).abs() < 0.001,
            "Expected area 12.0 (donut), got {area}"
        );
    }

    #[test]
    fn test_difference_order_matters() {
        // A - B is different from B - A
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))"); // Area 4
        let wkb2 = wkt_to_wkb("POLYGON((1 0, 3 0, 3 2, 1 2, 1 0))"); // Area 4, overlaps 2

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        // A - B
        let result_ab = compute_differences(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        // B - A
        let result_ba = compute_differences(
            &arr2,
            &arr1,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let diff_ab = GeosGeometry::new_from_wkb(result_ab.value(0)).unwrap();
        let diff_ba = GeosGeometry::new_from_wkb(result_ba.value(0)).unwrap();

        // Both have same area but different positions
        let area_ab = diff_ab.area().unwrap();
        let area_ba = diff_ba.area().unwrap();

        assert!((area_ab - 2.0).abs() < 0.001);
        assert!((area_ba - 2.0).abs() < 0.001);

        // But they're different geometries (don't equal)
        assert!(!diff_ab.equals(&diff_ba).unwrap());
    }

    #[test]
    fn test_difference_null_handling() {
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

        let result = compute_differences(
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
