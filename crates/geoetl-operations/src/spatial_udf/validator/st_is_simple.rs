//! `ST_IsSimple` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function tests if a geometry is simple (has no self-intersections).
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `true` if the geometry is simple, `false` otherwise.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, BooleanBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_IsSimple` User Defined Function
///
/// `ST_IsSimple` returns true if the geometry has no anomalous geometric points,
/// such as self-intersections or self-tangency. Points and polygons are always simple.
/// Linestrings are simple if they do not self-intersect (except at endpoints for rings).
///
/// # SQL Usage
///
/// ```sql
/// -- Check if geometries are simple
/// SELECT ST_IsSimple(geometry) FROM roads;
///
/// -- Filter complex linestrings
/// SELECT * FROM paths WHERE NOT ST_IsSimple(geometry);
///
/// -- Check if a self-intersecting line is simple
/// SELECT ST_IsSimple(ST_GeomFromText('LINESTRING(0 0, 10 10, 10 0, 0 10)'));
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry to check
///
/// # Returns
///
/// `Boolean`: `true` if simple, `false` otherwise
#[must_use]
pub fn create_st_is_simple_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StIsSimpleUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StIsSimpleUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StIsSimpleUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StIsSimpleUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_issimple"
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
                "ST_IsSimple requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_is_simple(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_IsSimple failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute `is_simple` for a geometry array using `GEOS`
fn compute_is_simple(
    arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<datafusion::arrow::array::BooleanArray, String> {
    let len = arr.len();
    let mut builder = BooleanBuilder::with_capacity(len);

    for i in 0..len {
        if arr.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos_geom = array_to_geos(arr, i, geo_type, field)?;
        let is_simple = geos_geom
            .is_simple()
            .map_err(|e| format!("GEOS is_simple failed at row {i}: {e}"))?;

        builder.append_value(is_simple);
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
    fn test_st_is_simple_udf_creation() {
        let udf = create_st_is_simple_udf();
        assert_eq!(udf.name(), "st_issimple");
    }

    #[test]
    fn test_is_simple_point() {
        let wkb = wkt_to_wkb("POINT(0 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_simple(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(result.value(0), "Point should be simple");
    }

    #[test]
    fn test_is_simple_polygon() {
        let wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_simple(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(result.value(0), "Simple polygon should be simple");
    }

    #[test]
    fn test_is_simple_simple_linestring() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 10, 20 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_simple(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(result.value(0), "Simple linestring should be simple");
    }

    #[test]
    fn test_is_simple_self_intersecting_linestring() {
        // Figure-8 linestring (self-intersecting)
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 10, 10 0, 0 10)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_simple(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(
            !result.value(0),
            "Self-intersecting linestring should not be simple"
        );
    }

    #[test]
    fn test_is_simple_closed_linestring() {
        // A closed linestring (ring) is simple even though it touches at endpoints
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 0, 10 10, 0 10, 0 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_simple(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(result.value(0), "Closed linestring (ring) should be simple");
    }

    #[test]
    fn test_is_simple_null_handling() {
        let wkb = wkt_to_wkb("POINT(0 0)");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb.as_slice()),
            None,
            Some(wkb.as_slice()),
        ]));

        let result = compute_is_simple(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }
}
