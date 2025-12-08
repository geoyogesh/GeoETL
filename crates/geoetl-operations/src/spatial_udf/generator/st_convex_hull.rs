//! `ST_ConvexHull` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the convex hull of a geometry.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns the same point
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the convex hull as WKB (Binary) with `geoarrow.wkb` metadata.

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

/// Create the `ST_ConvexHull` User Defined Function
///
/// `ST_ConvexHull` returns the smallest convex polygon that contains all the
/// points of a geometry. Think of it as stretching a rubber band around the geometry.
///
/// For Points, returns the same point.
/// For `LineStrings`, returns the convex hull polygon (or line if collinear).
/// For Polygons with holes or concave boundaries, returns the convex outer boundary.
///
/// # SQL Usage
///
/// ```sql
/// -- Get convex hull of geometries
/// SELECT ST_ConvexHull(geometry) FROM point_clouds;
///
/// -- Get convex hull from WKT
/// SELECT ST_ConvexHull(ST_GeomFromText('MULTIPOINT(0 0, 1 3, 3 1, 2 2)'));
///
/// -- Calculate area of convex hull
/// SELECT ST_Area(ST_ConvexHull(geometry)) FROM features;
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
///
/// # Returns
///
/// `Binary` (WKB): The convex hull as WKB with `geoarrow.wkb` metadata
#[must_use]
pub fn create_st_convex_hull_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StConvexHullUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StConvexHullUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StConvexHullUDF {
    fn new() -> Self {
        use super::super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![point_data_type()]),
                    TypeSignature::Exact(vec![DataType::Binary]),
                    TypeSignature::Any(1),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StConvexHullUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_convexhull"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_convexhull", nullable)))
    }

    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        let geom_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
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
                "ST_ConvexHull requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_convex_hulls(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_ConvexHull failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute convex hulls for a geometry array using `GEOS`
fn compute_convex_hulls(
    arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<BinaryArray, String> {
    let len = arr.len();
    let mut builder = BinaryBuilder::with_capacity(len, len * 64);

    for i in 0..len {
        if arr.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos_geom = array_to_geos(arr, i, geo_type, field)?;
        let convex_hull = geos_geom
            .convex_hull()
            .map_err(|e| format!("GEOS convex_hull failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = convex_hull
            .to_wkb()
            .map_err(|e| format!("Failed to convert convex hull to WKB at row {i}: {e}"))?
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
    fn test_st_convex_hull_udf_creation() {
        let udf = create_st_convex_hull_udf();
        assert_eq!(udf.name(), "st_convexhull");
    }

    #[test]
    fn test_convex_hull_multipoint() {
        // Triangle of points: convex hull is the triangle
        let wkb = wkt_to_wkb("MULTIPOINT(0 0, 4 0, 2 3)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_convex_hulls(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        let hull_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        // Triangle area = base * height / 2 = 4 * 3 / 2 = 6
        let area = hull_geom.area().unwrap();
        assert!((area - 6.0).abs() < 1e-10, "Expected area=6, got {area}");
    }

    #[test]
    fn test_convex_hull_concave_polygon() {
        // L-shaped polygon: convex hull fills in the corner
        let wkb = wkt_to_wkb("POLYGON((0 0, 2 0, 2 1, 1 1, 1 2, 0 2, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_convex_hulls(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        let hull_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        // Convex hull of L-shape is larger than the L
        let original = GeosGeometry::new_from_wkb(&wkb).unwrap();
        let original_area = original.area().unwrap();
        let hull_area = hull_geom.area().unwrap();
        assert!(
            hull_area > original_area,
            "Convex hull should be larger than concave polygon"
        );
    }

    #[test]
    fn test_convex_hull_convex_polygon() {
        // Already convex: hull equals original
        let wkb = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_convex_hulls(&arr, Some(GEOARROW_WKB), None).unwrap();

        let hull_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let original = GeosGeometry::new_from_wkb(&wkb).unwrap();

        let hull_area = hull_geom.area().unwrap();
        let original_area = original.area().unwrap();
        assert!(
            (hull_area - original_area).abs() < 1e-10,
            "Convex polygon hull should equal original"
        );
    }

    #[test]
    fn test_convex_hull_null_handling() {
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb.as_slice()),
            None,
            Some(wkb.as_slice()),
        ]));

        let result = compute_convex_hulls(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }
}
