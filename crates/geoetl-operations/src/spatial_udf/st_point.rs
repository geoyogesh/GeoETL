//! `ST_Point` and `ST_MakePoint` implementation - creates point geometry from coordinates
//!
//! Returns `GeoArrow` Point format (`FixedSizeList<Float64, 2>`) for efficient
//! in-memory representation without serialization overhead.

use super::geoarrow_types::{point_data_type, point_field};
use datafusion::arrow::array::{
    Array, ArrayRef, AsArray, FixedSizeListArray, Float64Array, Float64Builder,
};
use datafusion::arrow::buffer::NullBuffer;
use datafusion::arrow::datatypes::{DataType, FieldRef};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ReturnFieldArgs, ScalarFunctionArgs, ScalarUDF, Volatility};
use datafusion::physical_plan::ColumnarValue;
use std::sync::Arc;

/// Create the `ST_Point` User Defined Function
///
/// Creates a point geometry from X and Y coordinates.
/// Returns `GeoArrow` Point format for efficient memory representation.
///
/// # SQL Usage
///
/// ```sql
/// SELECT ST_Point(1.0, 2.0);
/// SELECT ST_Distance(geometry, ST_Point(0, 0)) FROM table;
/// SELECT ST_Point(lon_column, lat_column) FROM table;
/// ```
///
/// # Arguments
///
/// - `x`: X coordinate (longitude)
/// - `y`: Y coordinate (latitude)
///
/// # Returns
///
/// `GeoArrow` Point: `FixedSizeList<Float64, 2>` with extension metadata
#[must_use]
pub fn create_st_point_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StPointUDF::new("st_point"))
}

/// Create the `ST_MakePoint` User Defined Function (alias for `ST_Point`)
#[must_use]
pub fn create_st_makepoint_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StPointUDF::new("st_makepoint"))
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StPointUDF {
    name: &'static str,
    signature: datafusion::logical_expr::Signature,
}

impl StPointUDF {
    fn new(name: &'static str) -> Self {
        use datafusion::logical_expr::TypeSignature;
        Self {
            name,
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![DataType::Float64, DataType::Float64]),
                    TypeSignature::Exact(vec![DataType::Float32, DataType::Float32]),
                    TypeSignature::Exact(vec![DataType::Int64, DataType::Int64]),
                    TypeSignature::Exact(vec![DataType::Int32, DataType::Int32]),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StPointUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        self.name
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(point_data_type())
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        // Determine nullability: output is null if either input is nullable
        let nullable = args.arg_fields.iter().any(|f| f.is_nullable());
        Ok(Arc::new(point_field(self.name, nullable)))
    }

    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        let x_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        let y_array = match &args.args[1] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        let result = coords_to_point_array(&x_array, &y_array)
            .map_err(|e| DataFusionError::Execution(format!("{} failed: {e}", self.name)))?;

        Ok(ColumnarValue::Array(result))
    }
}

/// Convert X, Y coordinate arrays to `GeoArrow` Point array
///
/// Returns `FixedSizeList<Float64, 2>` containing interleaved [x, y] coordinates
fn coords_to_point_array(x_arr: &ArrayRef, y_arr: &ArrayRef) -> Result<ArrayRef, String> {
    if x_arr.len() != y_arr.len() {
        return Err(format!(
            "X and Y arrays must have same length: {} vs {}",
            x_arr.len(),
            y_arr.len()
        ));
    }

    let len = x_arr.len();

    // Cast to Float64 arrays
    let x_f64 = cast_to_f64(x_arr)?;
    let y_f64 = cast_to_f64(y_arr)?;

    // Build interleaved coordinate buffer [x0, y0, x1, y1, ...]
    let mut coord_builder = Float64Builder::with_capacity(len * 2);

    // Track nulls
    let mut null_buffer_builder = datafusion::arrow::array::BooleanBufferBuilder::new(len);

    for i in 0..len {
        let x_null = x_f64.is_null(i);
        let y_null = y_f64.is_null(i);

        if x_null || y_null {
            // Append placeholder values for null point
            coord_builder.append_value(0.0);
            coord_builder.append_value(0.0);
            null_buffer_builder.append(false); // false = null
        } else {
            coord_builder.append_value(x_f64.value(i));
            coord_builder.append_value(y_f64.value(i));
            null_buffer_builder.append(true); // true = valid
        }
    }

    let coords = coord_builder.finish();
    let nulls = NullBuffer::new(null_buffer_builder.finish());

    // Create FixedSizeList with the coordinate field
    let field = Arc::new(arrow_schema::Field::new("xy", DataType::Float64, false));
    let point_array = FixedSizeListArray::new(field, 2, Arc::new(coords), Some(nulls));

    Ok(Arc::new(point_array))
}

/// Cast numeric array to Float64
#[allow(clippy::cast_precision_loss)]
fn cast_to_f64(arr: &ArrayRef) -> Result<Float64Array, String> {
    match arr.data_type() {
        DataType::Float64 => {
            let f64_arr = arr
                .as_primitive_opt::<datafusion::arrow::datatypes::Float64Type>()
                .ok_or("Failed to cast to Float64")?;
            Ok(f64_arr.clone())
        },
        DataType::Float32 => {
            let f32_arr = arr
                .as_primitive_opt::<datafusion::arrow::datatypes::Float32Type>()
                .ok_or("Failed to cast Float32 array")?;
            Ok(f32_arr.iter().map(|v| v.map(f64::from)).collect())
        },
        DataType::Int64 => {
            let i64_arr = arr
                .as_primitive_opt::<datafusion::arrow::datatypes::Int64Type>()
                .ok_or("Failed to cast Int64 array")?;
            Ok(i64_arr.iter().map(|v| v.map(|x| x as f64)).collect())
        },
        DataType::Int32 => {
            let i32_arr = arr
                .as_primitive_opt::<datafusion::arrow::datatypes::Int32Type>()
                .ok_or("Failed to cast Int32 array")?;
            Ok(i32_arr.iter().map(|v| v.map(f64::from)).collect())
        },
        dt => Err(format!("Unsupported coordinate type: {dt:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::{Array, Float64Array};

    #[test]
    fn test_st_point_udf_creation() {
        let udf = create_st_point_udf();
        assert_eq!(udf.name(), "st_point");
    }

    #[test]
    fn test_st_makepoint_udf_creation() {
        let udf = create_st_makepoint_udf();
        assert_eq!(udf.name(), "st_makepoint");
    }

    #[test]
    fn test_coords_to_point_array() {
        let x_array: ArrayRef = Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0]));
        let y_array: ArrayRef = Arc::new(Float64Array::from(vec![4.0, 5.0, 6.0]));

        let result = coords_to_point_array(&x_array, &y_array).unwrap();
        assert_eq!(result.len(), 3);

        let point_array = result
            .as_any()
            .downcast_ref::<FixedSizeListArray>()
            .unwrap();

        // Check first point [1.0, 4.0]
        let values = point_array.values();
        let coords = values.as_any().downcast_ref::<Float64Array>().unwrap();
        assert!((coords.value(0) - 1.0).abs() < 1e-10); // x0
        assert!((coords.value(1) - 4.0).abs() < 1e-10); // y0
        assert!((coords.value(2) - 2.0).abs() < 1e-10); // x1
        assert!((coords.value(3) - 5.0).abs() < 1e-10); // y1
    }

    #[test]
    fn test_coords_to_point_array_null_handling() {
        let x_array: ArrayRef = Arc::new(Float64Array::from(vec![Some(1.0), None, Some(3.0)]));
        let y_array: ArrayRef = Arc::new(Float64Array::from(vec![Some(4.0), Some(5.0), None]));

        let result = coords_to_point_array(&x_array, &y_array).unwrap();
        assert_eq!(result.len(), 3);

        let point_array = result
            .as_any()
            .downcast_ref::<FixedSizeListArray>()
            .unwrap();

        assert!(!point_array.is_null(0)); // Both non-null
        assert!(point_array.is_null(1)); // X is null
        assert!(point_array.is_null(2)); // Y is null
    }

    #[test]
    fn test_coords_to_point_array_length_mismatch() {
        let x_array: ArrayRef = Arc::new(Float64Array::from(vec![1.0, 2.0]));
        let y_array: ArrayRef = Arc::new(Float64Array::from(vec![4.0]));

        let result = coords_to_point_array(&x_array, &y_array);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same length"));
    }

    #[test]
    fn test_coords_to_point_array_int32() {
        use datafusion::arrow::array::Int32Array;

        let x_array: ArrayRef = Arc::new(Int32Array::from(vec![1, 2, 3]));
        let y_array: ArrayRef = Arc::new(Int32Array::from(vec![4, 5, 6]));

        let result = coords_to_point_array(&x_array, &y_array).unwrap();
        assert_eq!(result.len(), 3);

        let point_array = result
            .as_any()
            .downcast_ref::<FixedSizeListArray>()
            .unwrap();

        let values = point_array.values();
        let coords = values.as_any().downcast_ref::<Float64Array>().unwrap();
        assert!((coords.value(0) - 1.0).abs() < 1e-10);
        assert!((coords.value(1) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_coords_to_point_array_int64() {
        use datafusion::arrow::array::Int64Array;

        let x_array: ArrayRef = Arc::new(Int64Array::from(vec![1i64, 2, 3]));
        let y_array: ArrayRef = Arc::new(Int64Array::from(vec![4i64, 5, 6]));

        let result = coords_to_point_array(&x_array, &y_array).unwrap();
        assert_eq!(result.len(), 3);

        let point_array = result
            .as_any()
            .downcast_ref::<FixedSizeListArray>()
            .unwrap();

        let values = point_array.values();
        let coords = values.as_any().downcast_ref::<Float64Array>().unwrap();
        assert!((coords.value(0) - 1.0).abs() < 1e-10);
        assert!((coords.value(1) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_coords_to_point_array_float32() {
        use datafusion::arrow::array::Float32Array;

        let x_array: ArrayRef = Arc::new(Float32Array::from(vec![1.0f32, 2.0, 3.0]));
        let y_array: ArrayRef = Arc::new(Float32Array::from(vec![4.0f32, 5.0, 6.0]));

        let result = coords_to_point_array(&x_array, &y_array).unwrap();
        assert_eq!(result.len(), 3);

        let point_array = result
            .as_any()
            .downcast_ref::<FixedSizeListArray>()
            .unwrap();

        let values = point_array.values();
        let coords = values.as_any().downcast_ref::<Float64Array>().unwrap();
        assert!((coords.value(0) - 1.0).abs() < 1e-10);
        assert!((coords.value(1) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_coords_to_point_array_unsupported_type() {
        use datafusion::arrow::array::StringArray;

        let x_array: ArrayRef = Arc::new(StringArray::from(vec!["1.0", "2.0"]));
        let y_array: ArrayRef = Arc::new(Float64Array::from(vec![4.0, 5.0]));

        let result = coords_to_point_array(&x_array, &y_array);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported coordinate type"));
    }
}
