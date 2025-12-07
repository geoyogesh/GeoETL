//! `ST_GeometryType` implementation using `GEOS` via `GeoArrow` arrays
//!
//! This function returns the type of a geometry as a string.
//! It accepts `GeoArrow` geometry types:
//! - `geoarrow.point` (`FixedSizeList<Float64, 2>`) - Returns "Point"
//! - `geoarrow.wkb` (Binary)
//! - `geoarrow.geometry` (Union) - mixed geometry types from `GeoJSON`
//!
//! Returns the OGC geometry type name (Point, `LineString`, Polygon, etc.).

use super::geoarrow_types::{GEOARROW_POINT, get_geoarrow_type, is_geoarrow_geometry};
use super::geos_helpers::array_to_geos;
use datafusion::arrow::array::{Array, ArrayRef, StringBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, TypeSignature, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::{Geom, GeometryTypes};
use std::sync::Arc;

/// Create the `ST_GeometryType` User Defined Function
///
/// `ST_GeometryType` returns the type of a geometry as a string.
/// Returns standard OGC type names: Point, `LineString`, Polygon,
/// `MultiPoint`, `MultiLineString`, `MultiPolygon`, `GeometryCollection`.
///
/// # SQL Usage
///
/// ```sql
/// -- Get type of a geometry
/// SELECT ST_GeometryType(ST_GeomFromText('POINT(0 0)'));
/// -- Returns: 'Point'
///
/// -- Filter by geometry type
/// SELECT * FROM features WHERE ST_GeometryType(geometry) = 'Polygon';
///
/// -- Group by geometry type
/// SELECT ST_GeometryType(geometry), COUNT(*) FROM features GROUP BY 1;
/// ```
///
/// # Arguments
///
/// - `geometry`: A `GeoArrow` geometry (Point, WKB, or mixed geometry)
///
/// # Returns
///
/// `Utf8`: The geometry type name
#[must_use]
pub fn create_st_geometry_type_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StGeometryTypeUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StGeometryTypeUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StGeometryTypeUDF {
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

impl datafusion::logical_expr::ScalarUDFImpl for StGeometryTypeUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_geometrytype"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Utf8)
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
                "ST_GeometryType requires GeoArrow geometry input. Use ST_Point, ST_GeomFromText, or ST_GeomFromWKB.".to_string(),
            ));
        }

        let results = compute_geometry_type(&geom_array, geo_type, field)
            .map_err(|e| DataFusionError::Execution(format!("ST_GeometryType failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(results)))
    }
}

/// Convert GEOS geometry type enum to string
fn geometry_type_to_string(geom_type: GeometryTypes) -> &'static str {
    match geom_type {
        GeometryTypes::Point => "Point",
        GeometryTypes::LineString => "LineString",
        GeometryTypes::LinearRing => "LinearRing",
        GeometryTypes::Polygon => "Polygon",
        GeometryTypes::MultiPoint => "MultiPoint",
        GeometryTypes::MultiLineString => "MultiLineString",
        GeometryTypes::MultiPolygon => "MultiPolygon",
        GeometryTypes::GeometryCollection => "GeometryCollection",
        _ => "Unknown",
    }
}

/// Compute geometry types for a geometry array using `GEOS`
fn compute_geometry_type(
    arr: &ArrayRef,
    geo_type: Option<&str>,
    field: Option<&arrow_schema::Field>,
) -> Result<datafusion::arrow::array::StringArray, String> {
    let len = arr.len();
    let mut builder = StringBuilder::with_capacity(len, len * 16);

    // Optimization for native point arrays
    if geo_type == Some(GEOARROW_POINT) || matches!(arr.data_type(), DataType::FixedSizeList(_, 2))
    {
        for i in 0..len {
            if arr.is_null(i) {
                builder.append_null();
            } else {
                builder.append_value("Point");
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
        let type_str = geometry_type_to_string(geos_geom.geometry_type());

        builder.append_value(type_str);
    }

    Ok(builder.finish())
}

#[cfg(test)]
mod tests {
    use super::super::geoarrow_types::{GEOARROW_POINT, GEOARROW_WKB};
    use super::*;
    use datafusion::arrow::array::{BinaryArray, FixedSizeListArray, Float64Array};
    use geos::Geom;
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
    fn test_st_geometry_type_udf_creation() {
        let udf = create_st_geometry_type_udf();
        assert_eq!(udf.name(), "st_geometrytype");
    }

    #[test]
    fn test_geometry_type_native_points() {
        let points = create_point_array(&[(0.0, 0.0), (1.0, 1.0)]);

        let result = compute_geometry_type(&points, Some(GEOARROW_POINT), None).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result.value(0), "Point");
        assert_eq!(result.value(1), "Point");
    }

    #[test]
    fn test_geometry_type_wkb_point() {
        let wkb = wkt_to_wkb("POINT(0 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_geometry_type(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), "Point");
    }

    #[test]
    fn test_geometry_type_linestring() {
        let wkb = wkt_to_wkb("LINESTRING(0 0, 1 1, 2 0)");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_geometry_type(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), "LineString");
    }

    #[test]
    fn test_geometry_type_polygon() {
        let wkb = wkt_to_wkb("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_geometry_type(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), "Polygon");
    }

    #[test]
    fn test_geometry_type_multi_types() {
        let wkb_multipoint = wkt_to_wkb("MULTIPOINT((0 0), (1 1))");
        let wkb_multiline = wkt_to_wkb("MULTILINESTRING((0 0, 1 1), (2 2, 3 3))");
        let wkb_multipoly =
            wkt_to_wkb("MULTIPOLYGON(((0 0, 1 0, 1 1, 0 1, 0 0)), ((2 0, 3 0, 3 1, 2 1, 2 0)))");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            wkb_multipoint.as_slice(),
            wkb_multiline.as_slice(),
            wkb_multipoly.as_slice(),
        ]));

        let result = compute_geometry_type(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result.value(0), "MultiPoint");
        assert_eq!(result.value(1), "MultiLineString");
        assert_eq!(result.value(2), "MultiPolygon");
    }

    #[test]
    fn test_geometry_type_collection() {
        let wkb = wkt_to_wkb("GEOMETRYCOLLECTION(POINT(0 0), LINESTRING(1 1, 2 2))");
        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![wkb.as_slice()]));

        let result = compute_geometry_type(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.value(0), "GeometryCollection");
    }

    #[test]
    fn test_geometry_type_null_handling() {
        let wkb1 = wkt_to_wkb("POINT(0 0)");
        let wkb2 = wkt_to_wkb("LINESTRING(0 0, 1 1)");

        let arr: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb1.as_slice()),
            None,
            Some(wkb2.as_slice()),
        ]));

        let result = compute_geometry_type(&arr, Some(GEOARROW_WKB), None).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
        assert_eq!(result.value(0), "Point");
        assert_eq!(result.value(2), "LineString");
    }
}
