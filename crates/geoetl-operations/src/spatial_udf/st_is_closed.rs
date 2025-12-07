//! `ST_IsClosed` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function tests if a `LineString` or `MultiLineString` is closed
//! (start point equals end point). Only applicable to linear geometries.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.wkb` (Binary) containing `LineString` or `MultiLineString`
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns `true` if the geometry is closed, `false` otherwise.
//! Returns an error for non-linear geometry types (Point, Polygon, etc.).

use super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, BooleanBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_IsClosed` User Defined Function
///
/// `ST_IsClosed` returns true if the `LineString` or `MultiLineString`'s start point
/// equals its end point (forms a ring). This function only works on linear geometries.
///
/// Note: Returns an error for Point, Polygon, and other non-linear geometry types.
///
/// # SQL Usage
///
/// ```sql
/// -- Check if linestrings are closed (form rings)
/// SELECT ST_IsClosed(geometry) FROM paths;
///
/// -- Find open linestrings
/// SELECT * FROM roads WHERE NOT ST_IsClosed(geometry);
///
/// -- Check if a linestring forms a ring
/// SELECT ST_IsClosed(ST_GeomFromText('LINESTRING(0 0, 10 0, 10 10, 0 10, 0 0)'));
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry to check
///
/// # Returns
///
/// `Boolean`: `true` if closed, `false` otherwise
#[must_use]
pub fn create_st_is_closed_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StIsClosedUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StIsClosedUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StIsClosedUDF {
    fn new() -> Self {
        use super::geoarrow_types::point_data_type;

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

impl datafusion::logical_expr::ScalarUDFImpl for StIsClosedUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_isclosed"
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
                "ST_IsClosed requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_is_closed(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_IsClosed failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute `is_closed` for a geometry array using `GEOS`
fn compute_is_closed(
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
        let is_closed = geos_geom
            .is_closed()
            .map_err(|e| format!("GEOS is_closed failed at row {i}: {e}"))?;

        builder.append_value(is_closed);
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
    fn test_st_is_closed_udf_creation() {
        let udf = create_st_is_closed_udf();
        assert_eq!(udf.name(), "st_isclosed");
    }

    #[test]
    fn test_is_closed_closed_linestring() {
        // A ring (closed linestring)
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 0, 10 10, 0 10, 0 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_closed(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(result.value(0), "Closed linestring should be closed");
    }

    #[test]
    fn test_is_closed_open_linestring() {
        // An open linestring
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 0, 10 10)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_closed(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(!result.value(0), "Open linestring should not be closed");
    }

    #[test]
    fn test_is_closed_multilinestring_all_closed() {
        let wkb =
            wkt_to_wkb("MULTILINESTRING((0 0, 10 0, 10 10, 0 0), (20 20, 30 20, 30 30, 20 20))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_closed(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(
            result.value(0),
            "MultiLineString with all closed parts should be closed"
        );
    }

    #[test]
    fn test_is_closed_multilinestring_one_open() {
        let wkb = wkt_to_wkb("MULTILINESTRING((0 0, 10 0, 10 10, 0 0), (20 20, 30 20, 30 30))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_is_closed(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert!(
            !result.value(0),
            "MultiLineString with one open part should not be closed"
        );
    }

    #[test]
    fn test_is_closed_null_handling() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 0, 10 10, 0 10, 0 0)");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb.as_slice()),
            None,
            Some(wkb.as_slice()),
        ]));

        let result = compute_is_closed(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }
}
