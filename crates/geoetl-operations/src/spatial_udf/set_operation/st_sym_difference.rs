//! `ST_SymDifference` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the symmetric difference of two geometries.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the symmetric difference as WKB (Binary) with `geoarrow.wkb` metadata.

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

/// Create the `ST_SymDifference` User Defined Function
///
/// `ST_SymDifference` returns a geometry that represents the parts of both
/// geometries that do not intersect. This is the set-theoretic symmetric
/// difference, equivalent to `(A - B) UNION (B - A)` or `(A UNION B) - (A INTERSECTION B)`.
///
/// Unlike `ST_Difference`, the order of arguments does not matter.
///
/// # SQL Usage
///
/// ```sql
/// -- Find areas unique to either geometry
/// SELECT ST_SymDifference(zone_a, zone_b) FROM zones;
///
/// -- Calculate non-overlapping area
/// SELECT ST_Area(ST_SymDifference(geometry_a, geometry_b)) FROM shapes;
///
/// -- XOR operation on geometries
/// SELECT ST_SymDifference(old_boundary, new_boundary) as changed_area FROM boundaries;
/// ```
///
/// # Arguments
///
/// - `geometry_a`: First `GeoArrow` geometry
/// - `geometry_b`: Second `GeoArrow` geometry
///
/// # Returns
///
/// `Binary` (WKB): The symmetric difference as WKB with `geoarrow.wkb` metadata.
/// Returns the union of both geometries if they are disjoint.
#[must_use]
pub fn create_st_sym_difference_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StSymDifferenceUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StSymDifferenceUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StSymDifferenceUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StSymDifferenceUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_symdifference"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_symdifference", nullable)))
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
                "ST_SymDifference requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        if geom1_array.len() != geom2_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_SymDifference: geometry arrays must have same length: {} vs {}",
                geom1_array.len(),
                geom2_array.len()
            )));
        }

        let sym_differences =
            compute_sym_differences(&geom1_array, &geom2_array, type1, type2, field1, field2)
                .map_err(|e| DataFusionError::Execution(format!("ST_SymDifference failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(sym_differences)))
    }
}

/// Compute symmetric differences for two geometry arrays using `GEOS`
fn compute_sym_differences(
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

        let sym_diff = geos1
            .sym_difference(&geos2)
            .map_err(|e| format!("GEOS sym_difference failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = sym_diff
            .to_wkb()
            .map_err(|e| format!("Failed to convert sym_difference to WKB at row {i}: {e}"))?
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
    fn test_st_sym_difference_udf_creation() {
        let udf = create_st_sym_difference_udf();
        assert_eq!(udf.name(), "st_symdifference");
    }

    #[test]
    fn test_sym_difference_overlapping_polygons() {
        // Two overlapping squares: sym difference is the non-overlapping parts
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))"); // Area 4
        let wkb2 = wkt_to_wkb("POLYGON((1 0, 3 0, 3 2, 1 2, 1 0))"); // Area 4, overlaps 2

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_sym_differences(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);

        let sym_diff = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = sym_diff.area().unwrap();

        // (4 - 2) + (4 - 2) = 4 (non-overlapping parts only)
        assert!(
            (area - 4.0).abs() < 0.001,
            "Expected symmetric difference area 4.0, got {area}"
        );
    }

    #[test]
    fn test_sym_difference_disjoint_polygons() {
        // Non-overlapping: sym difference is union of both
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))"); // Area 1
        let wkb2 = wkt_to_wkb("POLYGON((5 0, 6 0, 6 1, 5 1, 5 0))"); // Area 1

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_sym_differences(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let sym_diff = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = sym_diff.area().unwrap();

        // Disjoint: sym_diff = union = 1 + 1 = 2
        assert!(
            (area - 2.0).abs() < 0.001,
            "Expected area 2.0 (union), got {area}"
        );
    }

    #[test]
    fn test_sym_difference_identical_polygons() {
        // Same geometry: sym difference is empty
        let wkb = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_sym_differences(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let sym_diff = GeosGeometry::new_from_wkb(result.value(0)).unwrap();

        // Same geometry: sym_diff is empty
        assert!(
            sym_diff.is_empty().unwrap(),
            "Symmetric difference of identical geometries should be empty"
        );
    }

    #[test]
    fn test_sym_difference_order_invariant() {
        // Unlike difference, symmetric difference is order-invariant
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((1 0, 3 0, 3 2, 1 2, 1 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        // A sym_diff B
        let result_ab = compute_sym_differences(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        // B sym_diff A
        let result_ba = compute_sym_differences(
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

        // Symmetric difference is commutative
        let area_ab = diff_ab.area().unwrap();
        let area_ba = diff_ba.area().unwrap();

        assert!(
            (area_ab - area_ba).abs() < 0.001,
            "Symmetric difference should be order-invariant"
        );

        // They should be equal geometries
        assert!(
            diff_ab.equals(&diff_ba).unwrap(),
            "A sym_diff B should equal B sym_diff A"
        );
    }

    #[test]
    fn test_sym_difference_null_handling() {
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

        let result = compute_sym_differences(
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
