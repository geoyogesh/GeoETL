//! `ST_Dimension` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function returns the inherent dimension of a geometry.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns 0
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Dimension values:
//! - 0: Point
//! - 1: Line
//! - 2: Surface (Polygon)
//! - -1: Empty geometry

use super::geoarrow_types::{GEOARROW_POINT, get_geoarrow_type, is_geoarrow_geometry};
use super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, Int64Array, Int64Builder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geom;
use std::sync::Arc;

/// Create the `ST_Dimension` User Defined Function
///
/// `ST_Dimension` returns the inherent (topological) dimension of a geometry.
/// - 0 for Points and `MultiPoints`
/// - 1 for `LineStrings` and `MultiLineStrings`
/// - 2 for Polygons and `MultiPolygons`
/// - -1 for empty geometries
///
/// For `GeometryCollections`, returns the maximum dimension of constituent geometries.
///
/// # SQL Usage
///
/// ```sql
/// -- Get dimension of a point (returns 0)
/// SELECT ST_Dimension(ST_GeomFromText('POINT(0 0)'));
///
/// -- Get dimension of a linestring (returns 1)
/// SELECT ST_Dimension(ST_GeomFromText('LINESTRING(0 0, 1 1)'));
///
/// -- Get dimension of a polygon (returns 2)
/// SELECT ST_Dimension(ST_GeomFromText('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))'));
///
/// -- Filter by dimension
/// SELECT * FROM features WHERE ST_Dimension(geometry) = 2;
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
///
/// # Returns
///
/// `Int64`: The topological dimension (0, 1, 2, or -1 for empty)
#[must_use]
pub fn create_st_dimension_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StDimensionUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StDimensionUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StDimensionUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StDimensionUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_dimension"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Int64)
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
                "ST_Dimension requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_dimension(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_Dimension failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Compute dimensions for a geometry array using `GEOS`
fn compute_dimension(
    arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<Int64Array, String> {
    let len = arr.len();
    let mut builder = Int64Builder::with_capacity(len);

    // Optimization for native point arrays - dimension is always 0
    if geo_type == Some(GEOARROW_POINT) || matches!(arr.data_type(), DataType::FixedSizeList(_, 2))
    {
        for i in 0..len {
            if arr.is_null(i) {
                builder.append_null();
            } else {
                builder.append_value(0);
            }
        }
        return Ok(builder.finish());
    }

    // General case using GEOS
    for i in 0..len {
        if arr.is_null(i) {
            builder.append_null();
            continue;
        }

        let geos_geom = array_to_geos(arr, i, geo_type, field)?;
        let dimension = geos_geom
            .get_num_dimensions()
            .map_err(|e| format!("GEOS get_num_dimensions failed at row {i}: {e}"))?;

        #[allow(clippy::cast_possible_wrap)]
        builder.append_value(dimension as i64);
    }

    Ok(builder.finish())
}

#[cfg(test)]
mod tests {
    use super::super::geoarrow_types::{GEOARROW_POINT, GEOARROW_WKB};
    use super::*;
    use datafusion::arrow::array::{BinaryArray, FixedSizeListArray, Float64Array};
    use geos::Geometry as GeosGeometry;

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
    fn test_st_dimension_udf_creation() {
        let udf = create_st_dimension_udf();
        assert_eq!(udf.name(), "st_dimension");
    }

    #[test]
    fn test_dimension_native_points() {
        let points = create_point_array(&[(0.0, 0.0), (1.0, 1.0)]);

        let result = compute_dimension(&points, Some(GEOARROW_POINT), None).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result.value(0), 0);
        assert_eq!(result.value(1), 0);
    }

    #[test]
    fn test_dimension_point() {
        let wkb = wkt_to_wkb("POINT(0 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_dimension(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), 0);
    }

    #[test]
    fn test_dimension_linestring() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 1 1, 2 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_dimension(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), 1);
    }

    #[test]
    fn test_dimension_polygon() {
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_dimension(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), 2);
    }

    #[test]
    fn test_dimension_multipoint() {
        let wkb = wkt_to_wkb("MULTIPOINT((0 0), (1 1))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_dimension(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), 0);
    }

    #[test]
    fn test_dimension_multilinestring() {
        let wkb = wkt_to_wkb("MULTILINESTRING((0 0, 1 1), (2 2, 3 3))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_dimension(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), 1);
    }

    #[test]
    fn test_dimension_multipolygon() {
        let wkb =
            wkt_to_wkb("MULTIPOLYGON(((0 0, 1 0, 1 1, 0 1, 0 0)), ((2 0, 3 0, 3 1, 2 1, 2 0)))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_dimension(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), 2);
    }

    #[test]
    fn test_dimension_collection() {
        // Collection with Point (0), LineString (1), Polygon (2) -> max is 2
        let wkb = wkt_to_wkb(
            "GEOMETRYCOLLECTION(POINT(0 0), LINESTRING(1 1, 2 2), POLYGON((0 0, 1 0, 1 1, 0 1, 0 0)))",
        );
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_dimension(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), 2); // Max dimension of constituents
    }

    #[test]
    fn test_dimension_null_handling() {
        let wkb1 = wkt_to_wkb("POINT(0 0)");
        let wkb2 = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb1.as_slice()),
            None,
            Some(wkb2.as_slice()),
        ]));

        let result = compute_dimension(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
        assert_eq!(result.value(0), 0);
        assert_eq!(result.value(2), 2);
    }
}
