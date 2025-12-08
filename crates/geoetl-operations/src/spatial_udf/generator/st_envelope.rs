//! `ST_Envelope` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the bounding box (envelope) of a geometry.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns a point (degenerate envelope)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the envelope as WKB (Binary) with `geoarrow.wkb` metadata.

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

/// Create the `ST_Envelope` User Defined Function
///
/// `ST_Envelope` returns the minimum bounding box for a geometry as a polygon.
/// For Points, returns a point (degenerate envelope).
/// For `LineStrings`, returns a linestring or polygon depending on orientation.
/// For Polygons, returns the bounding rectangle.
///
/// # SQL Usage
///
/// ```sql
/// -- Get bounding box of geometries
/// SELECT ST_Envelope(geometry) FROM buildings;
///
/// -- Get envelope from WKT
/// SELECT ST_Envelope(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 2 6, 0 4, 0 0))'));
///
/// -- Use envelope for spatial filtering
/// SELECT * FROM features WHERE ST_Intersects(ST_Envelope(geometry), filter_box);
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
///
/// # Returns
///
/// `Binary` (WKB): The envelope as WKB with `geoarrow.wkb` metadata
#[must_use]
pub fn create_st_envelope_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StEnvelopeUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StEnvelopeUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StEnvelopeUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StEnvelopeUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_envelope"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_envelope", nullable)))
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
                "ST_Envelope requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_envelopes(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_Envelope failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute envelopes for a geometry array using `GEOS`
fn compute_envelopes(
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
        let envelope = geos_geom
            .envelope()
            .map_err(|e| format!("GEOS envelope failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = envelope
            .to_wkb()
            .map_err(|e| format!("Failed to convert envelope to WKB at row {i}: {e}"))?
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
    fn test_st_envelope_udf_creation() {
        let udf = create_st_envelope_udf();
        assert_eq!(udf.name(), "st_envelope");
    }

    #[test]
    fn test_envelope_polygon() {
        // Irregular polygon: envelope should be (0,0) to (4,6)
        let wkb = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 2 6, 0 4, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_envelopes(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        let envelope_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let env_env = envelope_geom.envelope().unwrap();
        let area = env_env.area().unwrap();
        // Bounding box: 4 wide x 6 tall = 24 area
        assert!((area - 24.0).abs() < 1e-10, "Expected area=24, got {area}");
    }

    #[test]
    fn test_envelope_linestring() {
        let wkb = wkt_to_wkb("LINESTRING(1 2, 5 8)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_envelopes(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        let envelope_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        // Envelope of diagonal line is a rectangle
        let area = envelope_geom.area().unwrap();
        // 4 wide x 6 tall = 24
        assert!((area - 24.0).abs() < 1e-10, "Expected area=24, got {area}");
    }

    #[test]
    fn test_envelope_null_handling() {
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb.as_slice()),
            None,
            Some(wkb.as_slice()),
        ]));

        let result = compute_envelopes(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }
}
