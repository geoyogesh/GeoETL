//! `ST_Length` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the length of a geometry.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns 0
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! For `LineStrings`, returns the total length.
//! For Polygons, returns the perimeter.
//! All length calculations use `GEOS` for consistency and correctness.

use super::geoarrow_types::{
    GEOARROW_GEOMETRY, GEOARROW_POINT, GEOARROW_WKB, get_geoarrow_type, is_geoarrow_geometry,
};
use datafusion::arrow::array::{
    Array, ArrayRef, BinaryArray, FixedSizeListArray, Float64Array, Float64Builder,
};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geoarrow_array::{GeoArrowArray, GeoArrowArrayAccessor};
use geos::{CoordSeq, Geom, Geometry as GeosGeometry};
use geozero::{CoordDimensions, ToWkb};
use std::sync::Arc;

/// Create the `ST_Length` User Defined Function
///
/// `ST_Length` returns the length of a geometry.
/// For Points, returns 0.
/// For `LineStrings`, returns the total length.
/// For Polygons, returns the perimeter.
/// Accepts `GeoArrow` geometry types and uses `GEOS` for all calculations.
///
/// # SQL Usage
///
/// ```sql
/// -- Length of a linestring
/// SELECT ST_Length(geometry) FROM roads;
///
/// -- Length from WKT
/// SELECT ST_Length(ST_GeomFromText('LINESTRING(0 0, 3 4)'));
///
/// -- Perimeter of a polygon
/// SELECT ST_Length(ST_GeomFromText('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))'));
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
///
/// # Returns
///
/// `Float64`: The length of the geometry (0 for Points)
#[must_use]
pub fn create_st_length_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StLengthUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StLengthUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StLengthUDF {
    fn new() -> Self {
        use super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    // Point (returns 0)
                    TypeSignature::Exact(vec![point_data_type()]),
                    // WKB (universal format)
                    TypeSignature::Exact(vec![DataType::Binary]),
                    // Any single geometry
                    TypeSignature::Any(1),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StLengthUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_length"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Float64)
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
                "ST_Length requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let lengths = compute_lengths(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_Length failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(lengths)))
    }
}

/// Compute lengths for a geometry array using `GEOS`
fn compute_lengths(
    arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<Float64Array, String> {
    let len = arr.len();
    let mut builder = Float64Builder::with_capacity(len);

    for i in 0..len {
        if arr.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos_geom = array_to_geos(arr, i, geo_type, field)?;
        let length = geos_geom
            .length()
            .map_err(|e| format!("GEOS length failed at row {i}: {e}"))?;

        builder.append_value(length);
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

    #[test]
    fn test_st_length_udf_creation() {
        let udf = create_st_length_udf();
        assert_eq!(udf.name(), "st_length");
    }

    #[test]
    fn test_length_point_returns_zero() {
        let points = create_point_array(&[(0.0, 0.0), (1.0, 1.0), (5.0, 5.0)]);

        let result = compute_lengths(&points, Some(GEOARROW_POINT), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!((result.value(0) - 0.0).abs() < 1e-10);
        assert!((result.value(1) - 0.0).abs() < 1e-10);
        assert!((result.value(2) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_length_linestring_horizontal() {
        // Horizontal line of length 5
        let wkb = wkt_to_wkb("LINESTRING(0 0, 5 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_lengths(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_length_linestring_diagonal() {
        // 3-4-5 triangle hypotenuse
        let wkb = wkt_to_wkb("LINESTRING(0 0, 3 4)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_lengths(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_length_linestring_multipart() {
        // Multiple segments: (0,0)->(3,0)->(3,4) = 3 + 4 = 7
        let wkb = wkt_to_wkb("LINESTRING(0 0, 3 0, 3 4)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_lengths(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_length_polygon_perimeter() {
        // Unit square perimeter = 4
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_lengths(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_length_polygon_rectangle_perimeter() {
        // 4x3 rectangle perimeter = 2*(4+3) = 14
        let wkb = wkt_to_wkb("POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_lengths(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 14.0).abs() < 1e-10);
    }

    #[test]
    fn test_length_multilinestring() {
        // Two lines: length 1 + length 2 = 3
        let wkb = wkt_to_wkb("MULTILINESTRING((0 0, 1 0), (0 0, 0 2))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_lengths(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_length_null_handling() {
        let wkb1 = wkt_to_wkb("LINESTRING(0 0, 3 4)"); // length 5
        let wkb2 = wkt_to_wkb("LINESTRING(0 0, 10 0)"); // length 10

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb1.as_slice()),
            None,
            Some(wkb2.as_slice()),
        ]));

        let result = compute_lengths(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
        assert!((result.value(0) - 5.0).abs() < 1e-10);
        assert!((result.value(2) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_length_multiple_geometries() {
        let wkb1 = wkt_to_wkb("LINESTRING(0 0, 1 0)"); // length 1
        let wkb2 = wkt_to_wkb("LINESTRING(0 0, 3 4)"); // length 5
        let wkb3 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))"); // perimeter 8

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            wkb1.as_slice(),
            wkb2.as_slice(),
            wkb3.as_slice(),
        ]));

        let result = compute_lengths(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!((result.value(0) - 1.0).abs() < 1e-10);
        assert!((result.value(1) - 5.0).abs() < 1e-10);
        assert!((result.value(2) - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_length_unsupported_geometry_type() {
        let points = create_point_array(&[(0.0, 0.0)]);

        let result = array_to_geos(&points, 0, Some("unsupported.type"), None);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Unsupported geometry type"));
    }

    #[test]
    fn test_length_invalid_wkb() {
        let invalid_wkb: ArrayRef = Arc::new(BinaryArray::from(vec![&[0u8, 1, 2, 3][..]]));

        let result = array_to_geos(&invalid_wkb, 0, Some(GEOARROW_WKB), None);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Invalid WKB"));
    }

    #[test]
    fn test_length_unknown_geometry_format() {
        use datafusion::arrow::array::Int32Array;

        let arr: ArrayRef = Arc::new(Int32Array::from(vec![1, 2, 3]));

        let result = array_to_geos(&arr, 0, None, None);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Unknown geometry format"));
    }
}
