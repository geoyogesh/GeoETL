//! `ST_IsRing` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function tests if a geometry is a ring (closed and simple).
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `true` if the geometry is a ring, `false` otherwise.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, BooleanBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_IsRing` User Defined Function
///
/// `ST_IsRing` returns true if the geometry is both closed and simple,
/// meaning it forms a ring without self-intersections.
/// This is equivalent to `ST_IsClosed(g) AND ST_IsSimple(g)`.
///
/// # SQL Usage
///
/// ```sql
/// -- Check if linestrings are valid rings
/// SELECT ST_IsRing(geometry) FROM boundaries;
///
/// -- Find linestrings that don't form valid rings
/// SELECT * FROM paths WHERE NOT ST_IsRing(geometry);
///
/// -- Check if a closed linestring is a valid ring
/// SELECT ST_IsRing(ST_GeomFromText('LINESTRING(0 0, 10 0, 10 10, 0 10, 0 0)'));
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry to check
///
/// # Returns
///
/// `Boolean`: `true` if geometry is a ring (closed and simple), `false` otherwise
#[must_use]
pub fn create_st_is_ring_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StIsRingUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StIsRingUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StIsRingUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StIsRingUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_isring"
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
                "ST_IsRing requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_is_ring(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_IsRing failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute `is_ring` for a geometry array using `GEOS`
fn compute_is_ring(
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
        let is_ring = geos_geom
            .is_ring()
            .map_err(|e| format!("GEOS is_ring failed at row {i}: {e}"))?;

        builder.append_value(is_ring);
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
    fn test_st_is_ring_udf_creation() {
        let udf = create_st_is_ring_udf();
        assert_eq!(udf.name(), "st_isring");
    }

    #[test]
    fn test_is_ring_valid_ring() {
        // A valid ring: closed and simple
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 0, 10 10, 0 10, 0 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_ring(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(
            result.value(0),
            "Closed and simple linestring should be a ring"
        );
    }

    #[test]
    fn test_is_ring_open_linestring() {
        // Open linestring is not a ring
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 0, 10 10)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_ring(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(!result.value(0), "Open linestring should not be a ring");
    }

    #[test]
    fn test_is_ring_self_intersecting_closed() {
        // Closed but self-intersecting (figure-8) is not a ring
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 10, 10 0, 0 10, 0 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_ring(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(
            !result.value(0),
            "Self-intersecting closed linestring should not be a ring"
        );
    }

    #[test]
    fn test_is_ring_point() {
        // Point is not a linestring, so is_ring returns false
        let wkb = wkt_to_wkb("POINT(0 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_ring(&arr, Some(GEOARROW_WKB), None).unwrap();

        // Points return false for is_ring (it's specifically for linestrings)
        assert!(!result.value(0), "Point should not be a ring");
    }

    #[test]
    fn test_is_ring_polygon() {
        // Polygons return false for is_ring (it's specifically for linestrings)
        let wkb = wkt_to_wkb("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_ring(&arr, Some(GEOARROW_WKB), None).unwrap();

        // Polygons return false for is_ring
        assert!(
            !result.value(0),
            "Polygon should not be a ring (is_ring is for linestrings)"
        );
    }

    #[test]
    fn test_is_ring_null_handling() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 0, 10 10, 0 10, 0 0)");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb.as_slice()),
            None,
            Some(wkb.as_slice()),
        ]));

        let result = compute_is_ring(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }
}
