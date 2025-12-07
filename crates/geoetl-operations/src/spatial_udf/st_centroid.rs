//! `ST_Centroid` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the centroid (center point) of a geometry.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns the same point
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the centroid as WKB (Binary) with `geoarrow.wkb` metadata.

use super::geoarrow_types::{
    GEOARROW_GEOMETRY, GEOARROW_POINT, GEOARROW_WKB, get_geoarrow_type, is_geoarrow_geometry,
    wkb_field,
};
use arrow_schema::FieldRef;
use datafusion::arrow::array::{
    Array, ArrayRef, BinaryArray, BinaryBuilder, FixedSizeListArray, Float64Array,
};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{
    ReturnFieldArgs, ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility,
};
use datafusion::physical_plan::ColumnarValue;
use geoarrow_array::{GeoArrowArray, GeoArrowArrayAccessor};
use geos::{CoordSeq, Geom, Geometry as GeosGeometry};
use geozero::{CoordDimensions, ToWkb};
use std::sync::Arc;

/// Create the `ST_Centroid` User Defined Function
///
/// `ST_Centroid` returns the centroid (geometric center) of a geometry.
/// For Points, returns the same point.
/// For `LineStrings`, returns the midpoint.
/// For Polygons, returns the center of mass.
///
/// # SQL Usage
///
/// ```sql
/// -- Centroid of a polygon
/// SELECT ST_Centroid(geometry) FROM buildings;
///
/// -- Centroid from WKT
/// SELECT ST_Centroid(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'));
///
/// -- Chain with buffer
/// SELECT ST_Buffer(ST_Centroid(geometry), 10.0) FROM parcels;
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
///
/// # Returns
///
/// `Binary` (WKB): The centroid point as WKB with `geoarrow.wkb` metadata
#[must_use]
pub fn create_st_centroid_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StCentroidUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StCentroidUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StCentroidUDF {
    fn new() -> Self {
        use super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    // Point input
                    TypeSignature::Exact(vec![point_data_type()]),
                    // WKB input
                    TypeSignature::Exact(vec![DataType::Binary]),
                    // Any single geometry
                    TypeSignature::Any(1),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StCentroidUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_centroid"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_centroid", nullable)))
    }

    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        let geom_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        // Get geometry type and field from metadata if available
        let field = args.arg_fields.first().map(std::convert::AsRef::as_ref);
        let geo_type = field.and_then(get_geoarrow_type);

        // Validate input is GeoArrow type
        if let Some(f) = field
            && !is_geoarrow_geometry(f)
            && !matches!(
                geom_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            )
        {
            return Err(DataFusionError::Execution(
                "ST_Centroid requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let centroids = compute_centroids(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_Centroid failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(centroids)))
    }
}

/// Compute centroids for a geometry array using `GEOS`
fn compute_centroids(
    arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<BinaryArray, String> {
    let len = arr.len();
    let mut builder = BinaryBuilder::with_capacity(len, len * 32); // Estimate ~32 bytes per WKB point

    for i in 0..len {
        if arr.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos_geom = array_to_geos(arr, i, geo_type, field)?;
        let centroid = geos_geom
            .get_centroid()
            .map_err(|e| format!("GEOS centroid failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = centroid
            .to_wkb()
            .map_err(|e| format!("Failed to convert centroid to WKB at row {i}: {e}"))?
            .into();

        builder.append_value(&wkb_bytes);
    }

    Ok(builder.finish())
}

/// Convert a single geometry from an array to `GEOS`
fn array_to_geos(
    arr: &ArrayRef,
    idx: usize,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<GeosGeometry, String> {
    match geo_type {
        Some(GEOARROW_POINT) | None if matches!(arr.data_type(), DataType::FixedSizeList(_, 2)) => {
            // `GeoArrow` Point -> GEOS
            let points = arr
                .as_any()
                .downcast_ref::<FixedSizeListArray>()
                .ok_or("Expected FixedSizeList for Point")?;

            let coords = points
                .values()
                .as_any()
                .downcast_ref::<Float64Array>()
                .ok_or("Expected Float64 coordinates")?;

            let offset = idx * 2;
            let x = coords.value(offset);
            let y = coords.value(offset + 1);

            let coord_seq = CoordSeq::new_from_vec(&[[x, y]])
                .map_err(|e| format!("Failed to create CoordSeq: {e}"))?;

            GeosGeometry::create_point(coord_seq)
                .map_err(|e| format!("Failed to create GEOS point: {e}"))
        },
        Some(GEOARROW_WKB) | None if matches!(arr.data_type(), DataType::Binary) => {
            // `GeoArrow` WKB -> GEOS
            let wkb_array = arr
                .as_any()
                .downcast_ref::<BinaryArray>()
                .ok_or("Expected Binary for WKB")?;

            let wkb = wkb_array.value(idx);
            GeosGeometry::new_from_wkb(wkb).map_err(|e| format!("Invalid WKB at row {idx}: {e}"))
        },
        Some(GEOARROW_GEOMETRY) => {
            // `GeoArrow` mixed geometry (Union) -> GEOS via WKB
            geometry_array_to_geos(arr, idx, field)
        },
        Some(other) => Err(format!("Unsupported geometry type: {other}")),
        None => Err(format!(
            "Unknown geometry format: {:?}. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.",
            arr.data_type()
        )),
    }
}

/// Convert a `GeoArrow` geometry array element to `GEOS` via WKB
fn geometry_array_to_geos(
    arr: &ArrayRef,
    idx: usize,
    field: Option<&arrow_schema::Field>,
) -> Result<GeosGeometry, String> {
    use geoarrow_array::array::GeometryArray;

    // Need field metadata to construct GeometryArray
    let field = field.ok_or("Field metadata required for geoarrow.geometry type")?;

    let geom_arr = GeometryArray::try_from((arr.as_ref(), field))
        .map_err(|e| format!("Failed to convert to GeometryArray: {e}"))?;

    if geom_arr.is_null(idx) {
        return Err(format!("Null geometry at row {idx}"));
    }

    // Get geometry and convert to WKB
    let geom = geom_arr
        .value(idx)
        .map_err(|e| format!("Failed to get geometry at row {idx}: {e}"))?;

    let wkb_bytes = geom
        .to_wkb(CoordDimensions::xy())
        .map_err(|e| format!("Failed to convert geometry to WKB at row {idx}: {e}"))?;

    GeosGeometry::new_from_wkb(&wkb_bytes).map_err(|e| format!("Invalid WKB at row {idx}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::BinaryArray;

    /// Create a `GeoArrow` Point array from coordinate pairs
    fn create_point_array(coords: &[(f64, f64)]) -> ArrayRef {
        let len = coords.len();
        let mut values = Vec::with_capacity(len * 2);
        for (x, y) in coords {
            values.push(*x);
            values.push(*y);
        }

        let coords_array = Float64Array::from(values);
        let field = Arc::new(arrow_schema::Field::new("xy", DataType::Float64, false));
        let points = FixedSizeListArray::new(field, 2, Arc::new(coords_array), None);

        Arc::new(points)
    }

    fn wkt_to_wkb(wkt: &str) -> Vec<u8> {
        let geom = GeosGeometry::new_from_wkt(wkt).unwrap();
        geom.to_wkb().unwrap().into()
    }

    fn wkb_to_wkt(wkb: &[u8]) -> String {
        let geom = GeosGeometry::new_from_wkb(wkb).unwrap();
        geom.to_wkt().unwrap()
    }

    #[test]
    fn test_st_centroid_udf_creation() {
        let udf = create_st_centroid_udf();
        assert_eq!(udf.name(), "st_centroid");
    }

    #[test]
    fn test_centroid_point_returns_same() {
        let points = create_point_array(&[(5.0, 10.0)]);

        let result = compute_centroids(&points, Some(GEOARROW_POINT), None).unwrap();

        assert_eq!(result.len(), 1);
        let wkt = wkb_to_wkt(result.value(0));
        assert!(
            wkt.contains('5') && wkt.contains("10"),
            "Expected POINT(5 10), got {wkt}"
        );
    }

    #[test]
    fn test_centroid_polygon() {
        // Unit square: centroid should be (0.5, 0.5)
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_centroids(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        let centroid_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let coord_seq = centroid_geom.get_coord_seq().unwrap();
        let x = coord_seq.get_x(0).unwrap();
        let y = coord_seq.get_y(0).unwrap();

        assert!((x - 0.5).abs() < 1e-10, "Expected x=0.5, got {x}");
        assert!((y - 0.5).abs() < 1e-10, "Expected y=0.5, got {y}");
    }

    #[test]
    fn test_centroid_rectangle() {
        // 4x2 rectangle at (0,0): centroid should be (2, 1)
        let wkb = wkt_to_wkb("POLYGON((0 0, 4 0, 4 2, 0 2, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_centroids(&arr, Some(GEOARROW_WKB), None).unwrap();

        let centroid_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let coord_seq = centroid_geom.get_coord_seq().unwrap();
        let x = coord_seq.get_x(0).unwrap();
        let y = coord_seq.get_y(0).unwrap();

        assert!((x - 2.0).abs() < 1e-10, "Expected x=2.0, got {x}");
        assert!((y - 1.0).abs() < 1e-10, "Expected y=1.0, got {y}");
    }

    #[test]
    fn test_centroid_linestring() {
        // Line from (0,0) to (4,0): centroid should be (2, 0)
        let wkb = wkt_to_wkb("LINESTRING(0 0, 4 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_centroids(&arr, Some(GEOARROW_WKB), None).unwrap();

        let centroid_geom = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let coord_seq = centroid_geom.get_coord_seq().unwrap();
        let x = coord_seq.get_x(0).unwrap();
        let y = coord_seq.get_y(0).unwrap();

        assert!((x - 2.0).abs() < 1e-10, "Expected x=2.0, got {x}");
        assert!((y - 0.0).abs() < 1e-10, "Expected y=0.0, got {y}");
    }

    #[test]
    fn test_centroid_null_handling() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb1.as_slice()),
            None,
            Some(wkb2.as_slice()),
        ]));

        let result = compute_centroids(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }

    #[test]
    fn test_centroid_multiple_geometries() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))"); // centroid (1, 1)
        let wkb2 = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))"); // centroid (2, 2)

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice(), wkb2.as_slice()]));

        let result = compute_centroids(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 2);

        // Check first centroid
        let c1 = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let cs1 = c1.get_coord_seq().unwrap();
        assert!((cs1.get_x(0).unwrap() - 1.0).abs() < 1e-10);
        assert!((cs1.get_y(0).unwrap() - 1.0).abs() < 1e-10);

        // Check second centroid
        let c2 = GeosGeometry::new_from_wkb(result.value(1)).unwrap();
        let cs2 = c2.get_coord_seq().unwrap();
        assert!((cs2.get_x(0).unwrap() - 2.0).abs() < 1e-10);
        assert!((cs2.get_y(0).unwrap() - 2.0).abs() < 1e-10);
    }
}
