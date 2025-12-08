//! `ST_SimplifyPreserveTopology` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function simplifies a geometry while preserving topology.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns the same point
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the simplified geometry as WKB (Binary) with `geoarrow.wkb` metadata.
//!
//! Unlike `ST_Simplify`, this function guarantees that the output geometry
//! will be valid if the input was valid.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry, wkb_field};
use super::super::geos_helpers::array_to_geos;
use arrow_schema::FieldRef;
use datafusion::arrow::array::{Array, ArrayRef, BinaryArray, BinaryBuilder, Float64Array};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{
    ReturnFieldArgs, ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility,
};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_SimplifyPreserveTopology` User Defined Function
///
/// `ST_SimplifyPreserveTopology` returns a simplified version of a geometry
/// using the Douglas-Peucker algorithm while ensuring the result maintains
/// topological validity.
///
/// Unlike `ST_Simplify`, this function:
/// - Prevents polygon rings from collapsing to lines
/// - Avoids creating self-intersecting polygons
/// - Preserves the overall structure of the geometry
///
/// # SQL Usage
///
/// ```sql
/// -- Simplify geometries while keeping them valid
/// SELECT ST_SimplifyPreserveTopology(geometry, 0.1) FROM boundaries;
///
/// -- Safe simplification with column-based tolerance
/// SELECT ST_SimplifyPreserveTopology(geometry, tolerance) FROM features;
///
/// -- Compare with regular simplify
/// SELECT
///     ST_IsValid(ST_Simplify(geometry, 10.0)) as regular_valid,
///     ST_IsValid(ST_SimplifyPreserveTopology(geometry, 10.0)) as topo_valid
/// FROM complex_polygons;
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
/// - `tolerance`: Distance tolerance for simplification (Float64)
///
/// # Returns
///
/// `Binary` (WKB): The simplified geometry as WKB with `geoarrow.wkb` metadata
#[must_use]
pub fn create_st_simplify_preserve_topology_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StSimplifyPreserveTopologyUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StSimplifyPreserveTopologyUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StSimplifyPreserveTopologyUDF {
    fn new() -> Self {
        use super::super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![point_data_type(), DataType::Float64]),
                    TypeSignature::Exact(vec![DataType::Binary, DataType::Float64]),
                    TypeSignature::Any(2),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StSimplifyPreserveTopologyUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_simplifypreservetopology"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_simplifypreservetopology", nullable)))
    }

    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        if args.args.len() != 2 {
            return Err(DataFusionError::Execution(
                "ST_SimplifyPreserveTopology requires exactly 2 arguments: geometry and tolerance"
                    .to_string(),
            ));
        }

        let geom_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        let tolerance_array = match &args.args[1] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array_of_size(geom_array.len())?,
        };

        let field = args.arg_fields.first().map(std::convert::AsRef::as_ref);
        let geo_type = field.and_then(get_geoarrow_type);

        if let Some(f) = field
            && !is_geoarrow_geometry(f)
            && !matches!(
                geom_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            )
        {
            return Err(DataFusionError::Execution(
                "ST_SimplifyPreserveTopology requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results =
            compute_simplify_preserve_topology(&geom_array, &tolerance_array, geo_type, field)
                .map_err(|e| {
                    DataFusionError::Execution(format!("ST_SimplifyPreserveTopology failed: {e}"))
                })?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute topology-preserving simplified geometries for a geometry array using `GEOS`
fn compute_simplify_preserve_topology(
    arr: &ArrayRef,
    tolerance_arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<BinaryArray, String> {
    let len = arr.len();
    let mut builder = BinaryBuilder::with_capacity(len, len * 64);

    let tolerances = tolerance_arr
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or("Tolerance must be Float64")?;

    for i in 0..len {
        if arr.is_null(i) || tolerances.is_null(i) {
            builder.append_null();
            continue;
        }

        let tolerance = tolerances.value(i);
        let geos_geom = array_to_geos(arr, i, geo_type, field)?;
        let simplified = geos_geom
            .topology_preserve_simplify(tolerance)
            .map_err(|e| format!("GEOS topology_preserve_simplify failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = simplified
            .to_wkb()
            .map_err(|e| format!("Failed to convert simplified geometry to WKB at row {i}: {e}"))?
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
    fn test_st_simplify_preserve_topology_udf_creation() {
        let udf = create_st_simplify_preserve_topology_udf();
        assert_eq!(udf.name(), "st_simplifypreservetopology");
    }

    #[test]
    fn test_simplify_preserve_topology_polygon() {
        // Valid polygon
        let wkb =
            wkt_to_wkb("POLYGON((0 0, 1 0.1, 2 0, 3 0.1, 4 0, 4 4, 3 3.9, 2 4, 1 3.9, 0 4, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![0.5]));

        let result =
            compute_simplify_preserve_topology(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

        let simplified = GeosGeometry::new_from_wkb(result.value(0)).unwrap();

        // Result must still be valid
        assert!(
            simplified.is_valid(),
            "Topology-preserving simplify should produce valid geometry"
        );

        // Should still be a polygon
        let geom_type = format!("{:?}", simplified.geometry_type());
        assert!(geom_type.contains("Polygon"), "Should still be a polygon");
    }

    #[test]
    fn test_simplify_preserve_topology_linestring() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 1 0.1, 2 0, 3 0.1, 4 0, 5 0.1, 6 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![0.5]));

        let result =
            compute_simplify_preserve_topology(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        let simplified = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let original = GeosGeometry::new_from_wkb(&wkb).unwrap();

        // Simplified should have fewer points
        let orig_points = original.get_num_points().unwrap();
        let simp_points = simplified.get_num_points().unwrap();
        assert!(
            simp_points <= orig_points,
            "Simplified should have fewer or equal points"
        );
    }

    #[test]
    fn test_simplify_preserve_topology_point() {
        // Points are unchanged by simplification
        let wkb = wkt_to_wkb("POINT(5 10)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![1.0]));

        let result =
            compute_simplify_preserve_topology(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

        let simplified = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let coord_seq = simplified.get_coord_seq().unwrap();
        let x = coord_seq.get_x(0).unwrap();
        let y = coord_seq.get_y(0).unwrap();

        assert!((x - 5.0).abs() < 1e-10, "Point should be unchanged");
        assert!((y - 10.0).abs() < 1e-10, "Point should be unchanged");
    }

    #[test]
    fn test_simplify_preserve_topology_null_handling() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 1 1, 2 0)");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb.as_slice()),
            None,
            Some(wkb.as_slice()),
        ]));
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![0.1, 0.1, 0.1]));

        let result =
            compute_simplify_preserve_topology(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }

    #[test]
    fn test_simplify_preserve_topology_maintains_validity() {
        // Test with a complex polygon that might become invalid with regular simplify
        let wkb = wkt_to_wkb(
            "POLYGON((0 0, 10 0, 10 10, 5 5, 0 10, 0 0))", // Concave polygon
        );
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![2.0]));

        let result =
            compute_simplify_preserve_topology(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

        let simplified = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        assert!(
            simplified.is_valid(),
            "Result should always be valid with topology preservation"
        );
    }
}
