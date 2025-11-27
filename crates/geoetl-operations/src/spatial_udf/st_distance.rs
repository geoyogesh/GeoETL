//! `ST_Distance` implementation using GEOS

use anyhow::{Result, anyhow};
use datafusion::arrow::array::{
    Array, ArrayRef, AsArray, BinaryArray, FixedSizeListArray, Float64Array, StringArray,
    StructArray, UnionArray,
};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ScalarFunctionArgs, ScalarUDF, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::{CoordSeq, Geom, Geometry as GeosGeometry};
use std::sync::Arc;

/// Create the `ST_Distance` User Defined Function
///
/// `ST_Distance` returns the minimum distance between two geometries.
///
/// # SQL Usage
///
/// ```sql
/// SELECT ST_Distance(geometry_column1, geometry_column2) FROM table;
/// SELECT * FROM table WHERE ST_Distance(geometry, POINT(0, 0)) < 1000;
/// ```
///
/// # Arguments
///
/// - `geometry1`: First geometry (as WKT string, WKB binary, or `GeoArrow` struct)
/// - `geometry2`: Second geometry (as WKT string, WKB binary, or `GeoArrow` struct)
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
        use datafusion::logical_expr::TypeSignature;
        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    // Accept Binary (WKB) geometries
                    TypeSignature::Exact(vec![DataType::Binary, DataType::Binary]),
                    // Accept String (WKT) geometries
                    TypeSignature::Exact(vec![DataType::Utf8, DataType::Utf8]),
                    // Accept LargeString (WKT) geometries
                    TypeSignature::Exact(vec![DataType::LargeUtf8, DataType::LargeUtf8]),
                    // Accept any type (for GeoArrow geometries)
                    TypeSignature::Any(2),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

#[allow(clippy::unnecessary_literal_bound)]
impl datafusion::logical_expr::ScalarUDFImpl for StDistanceUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

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
        // Extract the two geometry arrays from args
        let geom1_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(_) => {
                return Err(DataFusionError::Execution(
                    "ST_Distance does not support scalar geometry arguments yet".to_string(),
                ));
            },
        };

        let geom2_array = match &args.args[1] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(_) => {
                return Err(DataFusionError::Execution(
                    "ST_Distance does not support scalar geometry arguments yet".to_string(),
                ));
            },
        };

        // Compute distances
        let distances = compute_distances(&geom1_array, &geom2_array)
            .map_err(|e| DataFusionError::Execution(format!("ST_Distance failed: {e}")))?;

        Ok(ColumnarValue::Array(Arc::new(distances)))
    }
}

/// Compute distances between two arrays of geometries
/// Handles WKT (String), WKB (Binary) and `GeoArrow` (Struct) geometry formats
#[allow(clippy::too_many_lines)]
fn compute_distances(arr1: &ArrayRef, arr2: &ArrayRef) -> Result<Float64Array> {
    if arr1.len() != arr2.len() {
        return Err(anyhow!(
            "Geometry arrays must have the same length: {} vs {}",
            arr1.len(),
            arr2.len()
        ));
    }

    let len = arr1.len();
    let mut distances = Vec::with_capacity(len);

    // Try to handle as String (WKT) first
    if let (Some(wkt1_array), Some(wkt2_array)) = (
        arr1.as_any().downcast_ref::<StringArray>(),
        arr2.as_any().downcast_ref::<StringArray>(),
    ) {
        // Handle WKT format
        for i in 0..len {
            if wkt1_array.is_null(i) || wkt2_array.is_null(i) {
                return Err(anyhow!("NULL geometries are not supported yet"));
            }

            let wkt1 = wkt1_array.value(i);
            let wkt2 = wkt2_array.value(i);

            let geos_g1 = GeosGeometry::new_from_wkt(wkt1)
                .map_err(|e| anyhow!("Failed to parse first WKT geometry '{wkt1}': {e}"))?;

            let geos_g2 = GeosGeometry::new_from_wkt(wkt2)
                .map_err(|e| anyhow!("Failed to parse second WKT geometry '{wkt2}': {e}"))?;

            let distance = geos_g1
                .distance(&geos_g2)
                .map_err(|e| anyhow!("GEOS distance calculation failed: {e}"))?;

            distances.push(distance);
        }
    } else if let (Some(wkb1_array), Some(wkb2_array)) = (
        // Try to handle as Binary (WKB)
        arr1.as_any().downcast_ref::<BinaryArray>(),
        arr2.as_any().downcast_ref::<BinaryArray>(),
    ) {
        // Handle WKB format
        for i in 0..len {
            if wkb1_array.is_null(i) || wkb2_array.is_null(i) {
                return Err(anyhow!("NULL geometries are not supported yet"));
            }

            let wkb1 = wkb1_array.value(i);
            let wkb2 = wkb2_array.value(i);

            let geos_g1 = GeosGeometry::new_from_wkb(wkb1)
                .map_err(|e| anyhow!("Failed to parse first WKB geometry: {e}"))?;

            let geos_g2 = GeosGeometry::new_from_wkb(wkb2)
                .map_err(|e| anyhow!("Failed to parse second WKB geometry: {e}"))?;

            let distance = geos_g1
                .distance(&geos_g2)
                .map_err(|e| anyhow!("GEOS distance calculation failed: {e}"))?;

            distances.push(distance);
        }
    } else if let (Some(struct1_array), Some(struct2_array)) = (
        // Try to handle as Struct (GeoArrow Point format: {x: f64, y: f64})
        arr1.as_any().downcast_ref::<StructArray>(),
        arr2.as_any().downcast_ref::<StructArray>(),
    ) {
        // Handle GeoArrow Point struct format directly
        // GeoArrow points are stored as struct{x: f64, y: f64}
        let x1_idx = struct1_array.column_by_name("x");
        let y1_idx = struct1_array.column_by_name("y");
        let x2_idx = struct2_array.column_by_name("x");
        let y2_idx = struct2_array.column_by_name("y");

        if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (x1_idx, y1_idx, x2_idx, y2_idx) {
            // Try to downcast to Float64 arrays
            let x1_arr = x1.as_primitive_opt::<datafusion::arrow::datatypes::Float64Type>();
            let y1_arr = y1.as_primitive_opt::<datafusion::arrow::datatypes::Float64Type>();
            let x2_arr = x2.as_primitive_opt::<datafusion::arrow::datatypes::Float64Type>();
            let y2_arr = y2.as_primitive_opt::<datafusion::arrow::datatypes::Float64Type>();

            if let (Some(x1_arr), Some(y1_arr), Some(x2_arr), Some(y2_arr)) =
                (x1_arr, y1_arr, x2_arr, y2_arr)
            {
                for i in 0..len {
                    if x1_arr.is_null(i)
                        || y1_arr.is_null(i)
                        || x2_arr.is_null(i)
                        || y2_arr.is_null(i)
                    {
                        return Err(anyhow!("NULL coordinates are not supported yet"));
                    }

                    let x1_val = x1_arr.value(i);
                    let y1_val = y1_arr.value(i);
                    let x2_val = x2_arr.value(i);
                    let y2_val = y2_arr.value(i);

                    // Create GEOS points directly from coordinates
                    let coords1 = CoordSeq::new_from_vec(&[[x1_val, y1_val]])
                        .map_err(|e| anyhow!("Failed to create coordinate sequence: {e}"))?;
                    let geos_g1 = GeosGeometry::create_point(coords1)
                        .map_err(|e| anyhow!("Failed to create GEOS point: {e}"))?;

                    let coords2 = CoordSeq::new_from_vec(&[[x2_val, y2_val]])
                        .map_err(|e| anyhow!("Failed to create coordinate sequence: {e}"))?;
                    let geos_g2 = GeosGeometry::create_point(coords2)
                        .map_err(|e| anyhow!("Failed to create GEOS point: {e}"))?;

                    let distance = geos_g1
                        .distance(&geos_g2)
                        .map_err(|e| anyhow!("GEOS distance calculation failed: {e}"))?;

                    distances.push(distance);
                }
            } else {
                return Err(anyhow!(
                    "GeoArrow struct fields are not Float64 type. Got x: {:?}, y: {:?}",
                    x1.data_type(),
                    y1.data_type()
                ));
            }
        } else {
            return Err(anyhow!(
                "Struct array does not have 'x' and 'y' fields for GeoArrow Point format"
            ));
        }
    } else if let (Some(list1_array), Some(list2_array)) = (
        // Try to handle as FixedSizeList (GeoArrow interleaved Point format: [x, y] or [x, y, z])
        arr1.as_any().downcast_ref::<FixedSizeListArray>(),
        arr2.as_any().downcast_ref::<FixedSizeListArray>(),
    ) {
        // Handle GeoArrow Point FixedSizeList format directly
        // GeoArrow points can be stored as FixedSizeList<Float64, 2> containing [x, y]
        #[allow(clippy::cast_sign_loss)]
        let list_size = list1_array.value_length() as usize;
        if list_size < 2 {
            return Err(anyhow!(
                "FixedSizeList must have at least 2 elements for Point geometry"
            ));
        }

        // Get the inner values arrays
        let values1 = list1_array.values();
        let values2 = list2_array.values();

        let coords1 = values1.as_primitive_opt::<datafusion::arrow::datatypes::Float64Type>();
        let coords2 = values2.as_primitive_opt::<datafusion::arrow::datatypes::Float64Type>();

        if let (Some(coords1), Some(coords2)) = (coords1, coords2) {
            for i in 0..len {
                if list1_array.is_null(i) || list2_array.is_null(i) {
                    return Err(anyhow!("NULL geometries are not supported yet"));
                }

                // Get coordinates for point i
                let offset1 = i * list_size;
                let offset2 = i * list_size;

                let x1 = coords1.value(offset1);
                let y1 = coords1.value(offset1 + 1);
                let x2 = coords2.value(offset2);
                let y2 = coords2.value(offset2 + 1);

                // Create GEOS points directly from coordinates
                let coord_seq1 = CoordSeq::new_from_vec(&[[x1, y1]])
                    .map_err(|e| anyhow!("Failed to create coordinate sequence: {e}"))?;
                let geos_g1 = GeosGeometry::create_point(coord_seq1)
                    .map_err(|e| anyhow!("Failed to create GEOS point: {e}"))?;

                let coord_seq2 = CoordSeq::new_from_vec(&[[x2, y2]])
                    .map_err(|e| anyhow!("Failed to create coordinate sequence: {e}"))?;
                let geos_g2 = GeosGeometry::create_point(coord_seq2)
                    .map_err(|e| anyhow!("Failed to create GEOS point: {e}"))?;

                let distance = geos_g1
                    .distance(&geos_g2)
                    .map_err(|e| anyhow!("GEOS distance calculation failed: {e}"))?;

                distances.push(distance);
            }
        } else {
            return Err(anyhow!(
                "FixedSizeList values are not Float64 type. Got: {:?}",
                values1.data_type()
            ));
        }
    } else if let (Some(union1_array), Some(union2_array)) = (
        // Try to handle as Union (GeoArrow mixed geometry / GeometryArray format)
        arr1.as_any().downcast_ref::<UnionArray>(),
        arr2.as_any().downcast_ref::<UnionArray>(),
    ) {
        // Handle GeoArrow Union (mixed geometry) format
        // Extract geometry from union for each row and compute distance
        for i in 0..len {
            let geos_g1 = extract_geometry_from_union(union1_array, i)?;
            let geos_g2 = extract_geometry_from_union(union2_array, i)?;

            let distance = geos_g1
                .distance(&geos_g2)
                .map_err(|e| anyhow!("GEOS distance calculation failed: {e}"))?;

            distances.push(distance);
        }
    } else {
        // Fallback: Handle other GeoArrow formats using geoarrow_array library
        // This requires proper extension metadata on the arrays
        // Convert both arrays to WKB first, then use GEOS
        use geoarrow_array::GeoArrowArray;
        use geoarrow_array::array::from_arrow_array;
        use geoarrow_array::cast::to_wkb;

        // Get the fields from the schema for proper conversion
        let geo1_field = arrow_schema::Field::new("geom", arr1.data_type().clone(), true);
        let geo2_field = arrow_schema::Field::new("geom", arr2.data_type().clone(), true);

        let geo1_array = from_arrow_array(arr1.as_ref(), &geo1_field)
            .map_err(|e| anyhow!("Failed to convert first geometry to GeoArrow: {e}"))?;

        let geo2_array = from_arrow_array(arr2.as_ref(), &geo2_field)
            .map_err(|e| anyhow!("Failed to convert second geometry to GeoArrow: {e}"))?;

        // Convert to WKB (using i32 offset size, which is standard)
        let wkb1_array: geoarrow_array::array::WkbArray = to_wkb(&geo1_array)
            .map_err(|e| anyhow!("Failed to convert first GeoArrow to WKB: {e}"))?;

        let wkb2_array: geoarrow_array::array::WkbArray = to_wkb(&geo2_array)
            .map_err(|e| anyhow!("Failed to convert second GeoArrow to WKB: {e}"))?;

        // The WKB array is a BinaryArray, so we can access raw bytes directly

        let wkb1_binary = wkb1_array.to_array_ref();
        let wkb2_binary = wkb2_array.to_array_ref();

        let wkb1_data = wkb1_binary
            .as_any()
            .downcast_ref::<BinaryArray>()
            .ok_or_else(|| anyhow!("Failed to downcast WKB array to BinaryArray"))?;
        let wkb2_data = wkb2_binary
            .as_any()
            .downcast_ref::<BinaryArray>()
            .ok_or_else(|| anyhow!("Failed to downcast WKB array to BinaryArray"))?;

        // Now compute distances using WKB bytes
        for i in 0..len {
            let wkb1_bytes = wkb1_data.value(i);
            let wkb2_bytes = wkb2_data.value(i);

            let geos_g1 = GeosGeometry::new_from_wkb(wkb1_bytes)
                .map_err(|e| anyhow!("Failed to create GEOS geometry from first GeoArrow: {e}"))?;

            let geos_g2 = GeosGeometry::new_from_wkb(wkb2_bytes)
                .map_err(|e| anyhow!("Failed to create GEOS geometry from second GeoArrow: {e}"))?;

            let distance = geos_g1
                .distance(&geos_g2)
                .map_err(|e| anyhow!("GEOS distance calculation failed: {e}"))?;

            distances.push(distance);
        }
    }

    Ok(Float64Array::from(distances))
}

/// Extract a GEOS geometry from a `GeoArrow` Union array at a specific index
/// `GeoArrow` Union encodes different geometry types with `type_ids`:
/// - 1: Point (`FixedSizeList`[2] of `Float64`)
/// - 2: `LineString`, 3: Polygon, 4: `MultiPoint`, etc.
fn extract_geometry_from_union(union_array: &UnionArray, idx: usize) -> Result<GeosGeometry> {
    // Get the type_id for this row - indicates which geometry type
    let type_id = union_array.type_id(idx);

    // Get the offset into the child array for this row
    // For dense unions, we need to look at the offsets
    let offset = union_array.value_offset(idx);

    // Get the child array for this type
    let child = union_array.child(type_id);

    match type_id {
        1 => {
            // Point: FixedSizeList<Float64, 2> containing [x, y]
            let point_array = child
                .as_any()
                .downcast_ref::<FixedSizeListArray>()
                .ok_or_else(|| anyhow!("Expected FixedSizeListArray for Point geometry"))?;

            let values = point_array.values();
            let coords = values
                .as_primitive_opt::<datafusion::arrow::datatypes::Float64Type>()
                .ok_or_else(|| anyhow!("Expected Float64 coordinates for Point"))?;

            #[allow(clippy::cast_sign_loss)]
            let list_size = point_array.value_length() as usize;
            let val_offset = offset * list_size;

            let x = coords.value(val_offset);
            let y = coords.value(val_offset + 1);

            let coord_seq = CoordSeq::new_from_vec(&[[x, y]])
                .map_err(|e| anyhow!("Failed to create coordinate sequence: {e}"))?;
            GeosGeometry::create_point(coord_seq)
                .map_err(|e| anyhow!("Failed to create GEOS point: {e}"))
        },
        2..=7 | 11..=17 | 21..=27 | 31..=37 => {
            // Complex geometry types (LineString, Polygon, etc.)
            // For these, convert via WKB using geoarrow_array
            // This is a fallback - ideally we'd handle each type specifically
            Err(anyhow!(
                "Complex geometry type (type_id={type_id}) in Union not yet supported. Only Point geometries are currently supported in ST_Distance for mixed geometry arrays."
            ))
        },
        _ => Err(anyhow!("Unknown geometry type_id in Union: {type_id}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_st_distance_udf_creation() {
        let udf = create_st_distance_udf();
        assert_eq!(udf.name(), "st_distance");
    }
}
