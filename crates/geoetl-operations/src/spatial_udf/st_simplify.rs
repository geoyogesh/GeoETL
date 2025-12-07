//! `ST_Simplify` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function simplifies a geometry using the Douglas-Peucker algorithm.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns the same point
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the simplified geometry as WKB (Binary) with `geoarrow.wkb` metadata.
//!
//! Note: This function may produce invalid geometries (e.g., self-intersecting
//! polygons). Use `ST_SimplifyPreserveTopology` if validity must be maintained.

use super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry, wkb_field};
use super::geos_helpers::array_to_geos;
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

/// Create the `ST_Simplify` User Defined Function
///
/// `ST_Simplify` returns a simplified version of a geometry using the
/// Douglas-Peucker algorithm. Points within the tolerance distance of
/// the simplified line will be removed.
///
/// Warning: This function does not preserve topology - it may create
/// invalid geometries (self-intersections, collapsed polygons). Use
/// `ST_SimplifyPreserveTopology` if you need valid output geometries.
///
/// # SQL Usage
///
/// ```sql
/// -- Simplify geometries with 0.1 degree tolerance
/// SELECT ST_Simplify(geometry, 0.1) FROM coastlines;
///
/// -- Simplify with column-based tolerance
/// SELECT ST_Simplify(geometry, tolerance) FROM features;
///
/// -- Compare original and simplified
/// SELECT ST_NPoints(geometry), ST_NPoints(ST_Simplify(geometry, 10.0)) FROM roads;
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
pub fn create_st_simplify_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StSimplifyUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StSimplifyUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StSimplifyUDF {
    fn new() -> Self {
        use super::geoarrow_types::point_data_type;

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

impl datafusion::logical_expr::ScalarUDFImpl for StSimplifyUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_simplify"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_simplify", nullable)))
    }

    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        if args.args.len() != 2 {
            return Err(DataFusionError::Execution(
                "ST_Simplify requires exactly 2 arguments: geometry and tolerance".to_string(),
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
                "ST_Simplify requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_simplify(&geom_array, &tolerance_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_Simplify failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute simplified geometries for a geometry array using `GEOS`
fn compute_simplify(
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
            .simplify(tolerance)
            .map_err(|e| format!("GEOS simplify failed at row {i}: {e}"))?;

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
    use super::super::geoarrow_types::GEOARROW_WKB;
    use super::*;
    use datafusion::arrow::array::BinaryArray;
    use geos::Geometry as GeosGeometry;

    fn wkt_to_wkb(wkt: &str) -> Vec<u8> {
        let geom = GeosGeometry::new_from_wkt(wkt).unwrap();
        geom.to_wkb().unwrap().into()
    }

    #[test]
    fn test_st_simplify_udf_creation() {
        let udf = create_st_simplify_udf();
        assert_eq!(udf.name(), "st_simplify");
    }

    #[test]
    fn test_simplify_linestring() {
        // Complex linestring that can be simplified
        let wkb = wkt_to_wkb("LINESTRING(0 0, 1 0.1, 2 0, 3 0.1, 4 0, 5 0.1, 6 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![0.5]));

        let result = compute_simplify(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

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
    fn test_simplify_polygon() {
        // Polygon with wiggly edges
        let wkb =
            wkt_to_wkb("POLYGON((0 0, 1 0.1, 2 0, 3 0.1, 4 0, 4 4, 3 3.9, 2 4, 1 3.9, 0 4, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![0.5]));

        let result = compute_simplify(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

        let simplified = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        // Result should still be a polygon (though possibly invalid)
        let geom_type = format!("{:?}", simplified.geometry_type());
        assert!(geom_type.contains("Polygon"), "Should still be a polygon");
    }

    #[test]
    fn test_simplify_point() {
        // Points are unchanged by simplification
        let wkb = wkt_to_wkb("POINT(5 10)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![1.0]));

        let result = compute_simplify(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

        let simplified = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let coord_seq = simplified.get_coord_seq().unwrap();
        let x = coord_seq.get_x(0).unwrap();
        let y = coord_seq.get_y(0).unwrap();

        assert!((x - 5.0).abs() < 1e-10, "Point should be unchanged");
        assert!((y - 10.0).abs() < 1e-10, "Point should be unchanged");
    }

    #[test]
    fn test_simplify_null_handling() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 1 1, 2 0)");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb.as_slice()),
            None,
            Some(wkb.as_slice()),
        ]));
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![0.1, 0.1, 0.1]));

        let result = compute_simplify(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }

    #[test]
    fn test_simplify_varying_tolerance() {
        let wkb1 = wkt_to_wkb("LINESTRING(0 0, 0.5 0.1, 1 0, 1.5 0.1, 2 0)");
        let wkb2 = wkt_to_wkb("LINESTRING(0 0, 0.5 0.1, 1 0, 1.5 0.1, 2 0)");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice(), wkb2.as_slice()]));
        // Small tolerance keeps detail, large tolerance removes it
        let tolerance: ArrayRef = Arc::new(Float64Array::from(vec![0.05, 0.5]));

        let result = compute_simplify(&arr, &tolerance, Some(GEOARROW_WKB), None).unwrap();

        let simp1 = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let simp2 = GeosGeometry::new_from_wkb(result.value(1)).unwrap();

        let points1 = simp1.get_num_points().unwrap();
        let points2 = simp2.get_num_points().unwrap();

        assert!(
            points1 >= points2,
            "Smaller tolerance should keep more points"
        );
    }
}
