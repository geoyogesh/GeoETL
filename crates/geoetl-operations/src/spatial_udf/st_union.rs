//! `ST_Union` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the union (combination) of two geometries.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the union geometry as WKB (Binary) with `geoarrow.wkb` metadata.

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

/// Create the `ST_Union` User Defined Function
///
/// `ST_Union` returns a geometry that represents the union (combination)
/// of two input geometries.
///
/// # SQL Usage
///
/// ```sql
/// -- Union two geometry columns
/// SELECT ST_Union(geom_a, geom_b) FROM shapes;
///
/// -- Union with a fixed geometry
/// SELECT ST_Union(geometry, ST_GeomFromText('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))'))
/// FROM parcels;
///
/// -- Self-union (useful for fixing invalid geometries)
/// SELECT ST_Union(geometry, geometry) FROM polygons;
/// ```
///
/// # Arguments
///
/// - `geometry1`: First `GeoArrow` geometry
/// - `geometry2`: Second `GeoArrow` geometry
///
/// # Returns
///
/// `Binary` (WKB): The union geometry as WKB with `geoarrow.wkb` metadata
#[must_use]
pub fn create_st_union_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StUnionUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StUnionUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StUnionUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StUnionUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_union"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_union", nullable)))
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
                "ST_Union requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        // Validate arrays have same length
        if geom1_array.len() != geom2_array.len() {
            return Err(DataFusionError::Execution(format!(
                "ST_Union: geometry arrays must have same length: {} vs {}",
                geom1_array.len(),
                geom2_array.len()
            )));
        }

        let unions = compute_unions(&geom1_array, &geom2_array, type1, type2, field1, field2)
            .map_err(|e| DataFusionError::Execution(format!("ST_Union failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(unions)))
    }
}

/// Compute unions for two geometry arrays using `GEOS`
fn compute_unions(
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

        let union_geom = geos1
            .union(&geos2)
            .map_err(|e| format!("GEOS union failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = union_geom
            .to_wkb()
            .map_err(|e| format!("Failed to convert union to WKB at row {i}: {e}"))?
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
    fn test_st_union_udf_creation() {
        let udf = create_st_union_udf();
        assert_eq!(udf.name(), "st_union");
    }

    #[test]
    fn test_union_overlapping_polygons() {
        // Two overlapping unit squares
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((0.5 0, 1.5 0, 1.5 1, 0.5 1, 0.5 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_unions(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);

        let union_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = union_geom.area().unwrap();

        // Two overlapping 1x1 squares with 0.5x1 overlap = 1 + 1 - 0.5 = 1.5
        assert!(
            (area - 1.5).abs() < 0.001,
            "Expected union area ~1.5, got {area}"
        );
    }

    #[test]
    fn test_union_disjoint_polygons() {
        // Two non-overlapping squares
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((2 0, 3 0, 3 1, 2 1, 2 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_unions(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let union_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let geom_type = union_geom.geometry_type();

        // Disjoint polygons create a MultiPolygon
        assert_eq!(geom_type, geos::GeometryTypes::MultiPolygon);

        let area = union_geom.area().unwrap();
        assert!(
            (area - 2.0).abs() < 0.001,
            "Expected total area 2.0, got {area}"
        );
    }

    #[test]
    fn test_union_points() {
        // Two points create a MultiPoint
        let wkb1 = wkt_to_wkb("POINT(0 0)");
        let wkb2 = wkt_to_wkb("POINT(1 1)");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_unions(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let union_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let geom_type = union_geom.geometry_type();

        assert_eq!(geom_type, geos::GeometryTypes::MultiPoint);
    }

    #[test]
    fn test_union_null_handling() {
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

        let result = compute_unions(
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
    fn test_union_same_geometry() {
        // Union of geometry with itself should return same geometry
        let wkb = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_unions(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        let union_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = union_geom.area().unwrap();

        // Union with self = original (area 4)
        assert!((area - 4.0).abs() < 0.001, "Expected area 4.0, got {area}");
    }

    #[test]
    fn test_union_multiple_pairs() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((0.5 0, 1.5 0, 1.5 1, 0.5 1, 0.5 0))");
        let wkb3 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");
        let wkb4 = wkt_to_wkb("POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))");

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice(), wkb3.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice(), wkb4.as_slice()]));

        let result = compute_unions(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 2);

        // First union: 1.5 area
        let u1 = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        assert!((u1.area().unwrap() - 1.5).abs() < 0.001);

        // Second union: 4 + 4 - 1 = 7 (two 2x2 squares with 1x1 overlap)
        let u2 = GeosGeometry::new_from_wkb(result.value(1)).unwrap();
        assert!((u2.area().unwrap() - 7.0).abs() < 0.001);
    }
}
