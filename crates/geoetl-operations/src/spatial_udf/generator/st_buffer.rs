//! `ST_Buffer` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function creates a buffer polygon around a geometry at a specified distance.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the buffered geometry as WKB (Binary) with `geoarrow.wkb` metadata.

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
use geos::{BufferParams, Geom};
use std::sync::Arc;

/// Default number of quadrant segments for buffer operations
const DEFAULT_QUADRANT_SEGMENTS: i32 = 8;

/// Create the `ST_Buffer` User Defined Function
///
/// `ST_Buffer` returns a geometry that represents all points within
/// a specified distance from the input geometry.
///
/// # SQL Usage
///
/// ```sql
/// -- Buffer a point by 10 units (creates a circle)
/// SELECT ST_Buffer(geometry, 10.0) FROM points;
///
/// -- Buffer polygons to expand them
/// SELECT ST_Buffer(geometry, 5.0) FROM parcels;
///
/// -- Negative buffer (shrink polygons)
/// SELECT ST_Buffer(geometry, -2.0) FROM buildings;
///
/// -- Chain with centroid
/// SELECT ST_Buffer(ST_Centroid(geometry), 100.0) FROM regions;
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
/// - `distance`: Buffer distance (`Float64`). Positive expands, negative shrinks.
///
/// # Returns
///
/// `Binary` (WKB): The buffered geometry as WKB with `geoarrow.wkb` metadata
#[must_use]
pub fn create_st_buffer_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StBufferUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StBufferUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StBufferUDF {
    fn new() -> Self {
        use super::super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    // Point + distance
                    TypeSignature::Exact(vec![point_data_type(), DataType::Float64]),
                    // WKB + distance
                    TypeSignature::Exact(vec![DataType::Binary, DataType::Float64]),
                    // Any geometry + numeric
                    TypeSignature::Any(2),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StBufferUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_buffer"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(wkb_field("st_buffer", nullable)))
    }

    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        let geom_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        // Handle scalar distance by repeating it for each geometry row
        let dist_array = match &args.args[1] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => {
                // Repeat the scalar value for each row in the geometry array
                scalar.to_array_of_size(geom_array.len())?
            },
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
                "ST_Buffer requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let buffers = compute_buffers(&geom_array, &dist_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_Buffer failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(buffers)))
    }
}

/// Compute buffers for a geometry array using `GEOS`
fn compute_buffers(
    geom_arr: &ArrayRef,
    dist_arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<BinaryArray, String> {
    let len = geom_arr.len();
    let mut builder = BinaryBuilder::with_capacity(len, len * 256); // Estimate ~256 bytes per WKB polygon

    let distances = dist_arr
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or("Expected Float64 array for distance parameter")?;

    for i in 0..len {
        if geom_arr.is_null(i) || distances.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos_geom = array_to_geos(geom_arr, i, geo_type, field)?;
        let distance = distances.value(i);

        let buffer_params = BufferParams::builder()
            .quadrant_segments(DEFAULT_QUADRANT_SEGMENTS)
            .build()
            .map_err(|e| format!("Failed to create buffer params: {e}"))?;

        let buffered = geos_geom
            .buffer_with_params(distance, &buffer_params)
            .map_err(|e| format!("GEOS buffer failed at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = buffered
            .to_wkb()
            .map_err(|e| format!("Failed to convert buffer to WKB at row {i}: {e}"))?
            .into();

        builder.append_value(&wkb_bytes);
    }

    Ok(builder.finish())
}

#[cfg(test)]
mod tests {
    use super::super::super::geoarrow_types::{GEOARROW_POINT, GEOARROW_WKB};
    use super::*;
    use datafusion::arrow::array::{BinaryArray, FixedSizeListArray};
    use geos::Geometry as GeosGeometry;

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
    fn test_st_buffer_udf_creation() {
        let udf = create_st_buffer_udf();
        assert_eq!(udf.name(), "st_buffer");
    }

    #[test]
    fn test_buffer_point_creates_polygon() {
        let points = create_point_array(&[(0.0, 0.0)]);
        let distances: ArrayRef = Arc::new(Float64Array::from(vec![1.0]));

        let result = compute_buffers(&points, &distances, Some(GEOARROW_POINT), None).unwrap();

        assert_eq!(result.len(), 1);

        // Verify result is a polygon (buffer of point is circle-like polygon)
        let buffered = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let geom_type = buffered.geometry_type();
        assert_eq!(geom_type, geos::GeometryTypes::Polygon);

        // Verify area is approximately pi*r^2 = pi*1^2 = pi
        let area = buffered.area().unwrap();
        assert!(
            (area - std::f64::consts::PI).abs() < 0.1,
            "Expected area ~{}, got {area}",
            std::f64::consts::PI
        );
    }

    #[test]
    fn test_buffer_polygon_expands() {
        // Unit square
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let distances: ArrayRef = Arc::new(Float64Array::from(vec![1.0]));

        let result = compute_buffers(&arr, &distances, Some(GEOARROW_WKB), None).unwrap();

        let buffered = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = buffered.area().unwrap();

        // Original area = 1, buffered should be larger
        // Approximate: 1 + 4*1 (sides) + pi*1^2 (corners) ≈ 1 + 4 + 3.14 = 8.14
        assert!(area > 7.0, "Buffered area should be > 7, got {area}");
        assert!(area < 10.0, "Buffered area should be < 10, got {area}");
    }

    #[test]
    fn test_buffer_zero_distance() {
        let wkb = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let distances: ArrayRef = Arc::new(Float64Array::from(vec![0.0]));

        let result = compute_buffers(&arr, &distances, Some(GEOARROW_WKB), None).unwrap();

        let buffered = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = buffered.area().unwrap();

        // Zero buffer should return same geometry (area = 4)
        assert!(
            (area - 4.0).abs() < 0.001,
            "Zero buffer should preserve area, got {area}"
        );
    }

    #[test]
    fn test_buffer_negative_shrinks() {
        // 4x4 square
        let wkb = wkt_to_wkb("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));
        let distances: ArrayRef = Arc::new(Float64Array::from(vec![-1.0]));

        let result = compute_buffers(&arr, &distances, Some(GEOARROW_WKB), None).unwrap();

        let buffered = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let area = buffered.area().unwrap();

        // Original 16, after -1 buffer should be 2x2 = 4
        assert!(
            (area - 4.0).abs() < 0.1,
            "Negative buffer should shrink to ~4, got {area}"
        );
    }

    #[test]
    fn test_buffer_null_handling() {
        let wkb1 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let wkb2 = wkt_to_wkb("POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");

        let geom_arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb1.as_slice()),
            None,
            Some(wkb2.as_slice()),
        ]));

        let dist_arr: ArrayRef = Arc::new(Float64Array::from(vec![Some(1.0), Some(1.0), None]));

        let result = compute_buffers(&geom_arr, &dist_arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0)); // Both valid
        assert!(result.is_null(1)); // Geometry null
        assert!(result.is_null(2)); // Distance null
    }

    #[test]
    fn test_buffer_multiple_geometries() {
        let wkb1 = wkt_to_wkb("POINT(0 0)");
        let wkb2 = wkt_to_wkb("POINT(10 10)");

        let geom_arr: ArrayRef =
            Arc::new(BinaryArray::from(vec![wkb1.as_slice(), wkb2.as_slice()]));
        let dist_arr: ArrayRef = Arc::new(Float64Array::from(vec![1.0, 2.0]));

        let result = compute_buffers(&geom_arr, &dist_arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 2);

        // First buffer: radius 1, area ~pi
        let b1 = GeosGeometry::new_from_wkb(result.value(0)).unwrap();
        let a1 = b1.area().unwrap();
        assert!(
            (a1 - std::f64::consts::PI).abs() < 0.1,
            "Expected area ~pi, got {a1}"
        );

        // Second buffer: radius 2, area ~4*pi
        let b2 = GeosGeometry::new_from_wkb(result.value(1)).unwrap();
        let a2 = b2.area().unwrap();
        assert!(
            (a2 - 4.0 * std::f64::consts::PI).abs() < 0.5,
            "Expected area ~4*pi, got {a2}"
        );
    }
}
