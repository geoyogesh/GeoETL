//! `ST_Boundary` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the boundary of a geometry.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns empty geometry
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the boundary as WKB (Binary) with `geoarrow.wkb` metadata.

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

/// Create the `ST_Boundary` User Defined Function
///
/// `ST_Boundary` returns the closure of the combinatorial boundary of a geometry.
///
/// For Points, returns an empty `GeometryCollection`.
/// For `LineStrings`, returns a `MultiPoint` containing the endpoints (empty if closed).
/// For Polygons, returns a `LineString` or `MultiLineString` (exterior and interior rings).
///
/// # SQL Usage
///
/// ```sql
/// -- Get boundary of geometries
/// SELECT ST_Boundary(geometry) FROM parcels;
///
/// -- Get polygon boundary (its ring)
/// SELECT ST_Boundary(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'));
///
/// -- Get linestring endpoints
/// SELECT ST_Boundary(ST_GeomFromText('LINESTRING(0 0, 10 10)'));
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
///
/// # Returns
///
/// `Binary` (WKB): The boundary as WKB with `geoarrow.wkb` metadata
#[must_use]
pub fn create_st_boundary_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StBoundaryUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StBoundaryUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StBoundaryUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StBoundaryUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_boundary"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_boundary", nullable)))
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
                "ST_Boundary requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_boundaries(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_Boundary failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute boundaries for a geometry array using `GEOS`
fn compute_boundaries(
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
        let boundary = geos_geom
            .boundary()
            .map_err(|e| format!("GEOS boundary failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = boundary
            .to_wkb()
            .map_err(|e| format!("Failed to convert boundary to WKB at row {i}: {e}"))?
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
    fn test_st_boundary_udf_creation() {
        let udf = create_st_boundary_udf();
        assert_eq!(udf.name(), "st_boundary");
    }

    #[test]
    fn test_boundary_polygon() {
        // Polygon boundary is a linestring (the ring)
        let wkb = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_boundaries(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        let boundary_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        // Boundary of polygon is a linestring with perimeter = 16
        let length = boundary_geom.length().unwrap();
        assert!(
            (length - 16.0).abs() < 1e-10,
            "Expected length=16, got {length}"
        );
    }

    #[test]
    fn test_boundary_linestring() {
        // Linestring boundary is a multipoint with the two endpoints
        let wkb = wkt_to_wkb("LINESTRING(0 0, 10 10)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_boundaries(&arr, Some(GEOARROW_WKB), None).unwrap();

        let boundary_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let geom_type = format!("{:?}", boundary_geom.geometry_type());
        assert!(
            geom_type.contains("Point") || geom_type.contains("Multi"),
            "Linestring boundary should be point(s), got {geom_type}"
        );
    }

    #[test]
    fn test_boundary_closed_linestring() {
        // Closed linestring has empty boundary
        let wkb = wkt_to_wkb("LINESTRING(0 0, 4 0, 4 4, 0 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_boundaries(&arr, Some(GEOARROW_WKB), None).unwrap();

        let boundary_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        // Closed ring boundary is empty
        assert!(
            boundary_geom.is_empty().unwrap(),
            "Closed linestring should have empty boundary"
        );
    }

    #[test]
    fn test_boundary_point() {
        // Point boundary is empty
        let wkb = wkt_to_wkb("POINT(5 5)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_boundaries(&arr, Some(GEOARROW_WKB), None).unwrap();

        let boundary_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        assert!(
            boundary_geom.is_empty().unwrap(),
            "Point should have empty boundary"
        );
    }

    #[test]
    fn test_boundary_null_handling() {
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb.as_slice()),
            None,
            Some(wkb.as_slice()),
        ]));

        let result = compute_boundaries(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }
}
