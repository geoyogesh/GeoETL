//! `ST_Distance` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function computes the minimum distance between two geometries.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`)
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! All distance calculations use `GEOS` for consistency and correctness.

use super::super::geoarrow_types::{get_geoarrow_type, is_geoarrow_geometry};
use super::super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, Float64Array, Float64Builder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Distance` User Defined Function
///
/// `ST_Distance` returns the minimum distance between two geometries.
/// Accepts `GeoArrow` geometry types and uses `GEOS` for all calculations.
///
/// # SQL Usage
///
/// ```sql
/// -- Point-to-point distance
/// SELECT ST_Distance(ST_Point(0, 0), ST_Point(3, 4));
///
/// -- Distance between geometries from WKT
/// SELECT ST_Distance(ST_GeomFromText(wkt1), ST_GeomFromText(wkt2)) FROM table;
///
/// -- Distance from geometry column to fixed point
/// SELECT ST_Distance(geometry, ST_Point(0, 0)) FROM table;
/// ```
///
/// # Arguments
///
/// - `geometry1`: First geometry (`GeoArrow` Point or WKB)
/// - `geometry2`: Second geometry (`GeoArrow` Point or WKB)
///
/// # Returns
///
/// `Float64`: The minimum distance between the two geometries
#[must_use]
pub fn create_st_distance_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StDistanceUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StDistanceUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StDistanceUDF {
    fn new() -> Self {
        use super::super::geoarrow_types::point_data_type;

        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    // Point-to-Point
                    TypeSignature::Exact(vec![point_data_type(), point_data_type()]),
                    // WKB-to-WKB
                    TypeSignature::Exact(vec![DataType::Binary, DataType::Binary]),
                    // Mixed types - use Any to allow combinations
                    TypeSignature::Any(2),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StDistanceUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_distance"
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
        let geom1_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        let geom2_array = match &args.args[1] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        // Get geometry types and fields from metadata if available
        let field1 = args.arg_fields.first().map(std::convert::AsRef::as_ref);
        let field2 = args.arg_fields.get(1).map(std::convert::AsRef::as_ref);
        let type1 = field1.and_then(get_geoarrow_type);
        let type2 = field2.and_then(get_geoarrow_type);

        // Validate inputs are GeoArrow types
        if let (Some(f1), Some(f2)) = (field1, field2)
            && (!is_geoarrow_geometry(f1) || !is_geoarrow_geometry(f2))
            && (!matches!(
                geom1_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            ) || !matches!(
                geom2_array.data_type(),
                DataType::Binary | DataType::FixedSizeList(_, 2)
            ))
        {
            return Err(DataFusionError::Execution(
                "ST_Distance requires GeoArrow geometry inputs. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let distances = compute_distances(&geom1_array, &geom2_array, type1, type2, field1, field2)
            .map_err(|e| DataFusionError::Execution(format!("ST_Distance failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(distances)))
    }
}

/// Compute distances between two geometry arrays using `GEOS`
fn compute_distances(
    arr1: &ArrayRef,
    arr2: &ArrayRef,
    type1: Option<&str>,
    type2: Option<&str>,
    field1: Option<&arrow_schema::Field>,
    field2: Option<&arrow_schema::Field>,
) -> Result<Float64Array, String> {
    if arr1.len() != arr2.len() {
        return Err(format!(
            "Geometry arrays must have same length: {} vs {}",
            arr1.len(),
            arr2.len()
        ));
    }

    compute_geos_distances(arr1, arr2, type1, type2, field1, field2)
}

/// `GEOS`-based distance calculation
fn compute_geos_distances(
    arr1: &ArrayRef,
    arr2: &ArrayRef,
    type1: Option<&str>,
    type2: Option<&str>,
    field1: Option<&arrow_schema::Field>,
    field2: Option<&arrow_schema::Field>,
) -> Result<Float64Array, String> {
    let len = arr1.len();
    let mut builder = Float64Builder::with_capacity(len);

    for i in 0..len {
        if arr1.is_null(i) || arr2.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos1 = array_to_geos(arr1, i, type1, field1)?;
        let geos2 = array_to_geos(arr2, i, type2, field2)?;

        let distance = geos1
            .distance(&geos2)
            .map_err(|e| format!("GEOS distance failed at row {i}: {e}"))?;

        builder.append_value(distance);
    }

    Ok(builder.finish())
}

#[cfg(test)]
mod tests {
    use super::super::super::geoarrow_types::{GEOARROW_POINT, GEOARROW_WKB};
    use super::*;
    use datafusion::arrow::array::{BinaryArray, FixedSizeListArray, Float64Array};
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

    fn point_to_wkb(x: f64, y: f64) -> Vec<u8> {
        let wkt = format!("POINT({x} {y})");
        let geom = GeosGeometry::new_from_wkt(&wkt).unwrap();
        geom.to_wkb().unwrap().into()
    }

    #[test]
    fn test_st_distance_udf_creation() {
        let udf = create_st_distance_udf();
        assert_eq!(udf.name(), "st_distance");
    }

    #[test]
    fn test_point_to_point_distance() {
        let points1 = create_point_array(&[(0.0, 0.0), (0.0, 0.0), (1.0, 1.0)]);
        let points2 = create_point_array(&[(3.0, 4.0), (1.0, 0.0), (4.0, 5.0)]);

        let result = compute_geos_distances(
            &points1,
            &points2,
            Some(GEOARROW_POINT),
            Some(GEOARROW_POINT),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 3);
        // (0,0) to (3,4) = 5
        assert!((result.value(0) - 5.0).abs() < 1e-10);
        // (0,0) to (1,0) = 1
        assert!((result.value(1) - 1.0).abs() < 1e-10);
        // (1,1) to (4,5) = 5
        assert!((result.value(2) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_wkb_distance() {
        let wkb1 = point_to_wkb(0.0, 0.0);
        let wkb2 = point_to_wkb(3.0, 4.0);

        let arr1: ArrayRef = Arc::new(BinaryArray::from(vec![wkb1.as_slice()]));
        let arr2: ArrayRef = Arc::new(BinaryArray::from(vec![wkb2.as_slice()]));

        let result = compute_geos_distances(
            &arr1,
            &arr2,
            Some(GEOARROW_WKB),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_mixed_point_wkb_distance() {
        let points = create_point_array(&[(0.0, 0.0)]);
        let wkb = point_to_wkb(3.0, 4.0);
        let wkb_arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_geos_distances(
            &points,
            &wkb_arr,
            Some(GEOARROW_POINT),
            Some(GEOARROW_WKB),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        assert!((result.value(0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_null_handling() {
        // Create arrays with nulls
        let values1 = Float64Array::from(vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let values2 = Float64Array::from(vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0]);

        let field = Arc::new(arrow_schema::Field::new("xy", DataType::Float64, false));

        // Create null buffer: [valid, null, valid]
        let null_buffer1 = datafusion::arrow::buffer::NullBuffer::from(vec![true, false, true]);
        let null_buffer2 = datafusion::arrow::buffer::NullBuffer::from(vec![true, true, false]);

        let points1: ArrayRef = Arc::new(FixedSizeListArray::new(
            field.clone(),
            2,
            Arc::new(values1),
            Some(null_buffer1),
        ));
        let points2: ArrayRef = Arc::new(FixedSizeListArray::new(
            field,
            2,
            Arc::new(values2),
            Some(null_buffer2),
        ));

        let result = compute_geos_distances(
            &points1,
            &points2,
            Some(GEOARROW_POINT),
            Some(GEOARROW_POINT),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0)); // Both valid
        assert!(result.is_null(1)); // First null
        assert!(result.is_null(2)); // Second null
    }

    #[test]
    fn test_array_length_mismatch() {
        let points1 = create_point_array(&[(0.0, 0.0), (1.0, 1.0)]);
        let points2 = create_point_array(&[(3.0, 4.0)]);

        let result = compute_distances(
            &points1,
            &points2,
            Some(GEOARROW_POINT),
            Some(GEOARROW_POINT),
            None,
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same length"));
    }
}
