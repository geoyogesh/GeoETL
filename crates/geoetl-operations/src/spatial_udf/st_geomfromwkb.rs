//! `ST_GeomFromWKB` implementation - validates and tags WKB as `GeoArrow` geometry

use super::geoarrow_types::{wkb_data_type, wkb_field};
use datafusion::arrow::array::{Array, ArrayRef, BinaryArray, BinaryBuilder};
use datafusion::arrow::datatypes::{DataType, FieldRef};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ReturnFieldArgs, ScalarFunctionArgs, ScalarUDF, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::Geometry as GeosGeometry;
use std::sync::Arc;

/// Create the `ST_GeomFromWKB` User Defined Function
///
/// Validates WKB (Well-Known Binary) and returns it as `GeoArrow` WKB format.
/// This function ensures the WKB is valid by parsing it with GEOS.
///
/// # SQL Usage
///
/// ```sql
/// SELECT ST_GeomFromWKB(wkb_column) FROM table;
/// SELECT ST_Distance(ST_GeomFromWKB(wkb1), ST_GeomFromWKB(wkb2)) FROM table;
/// ```
///
/// # Arguments
///
/// - `wkb`: WKB binary representation of a geometry
///
/// # Returns
///
/// `GeoArrow` WKB: `Binary` with `geoarrow.wkb` extension metadata
#[must_use]
pub fn create_st_geomfromwkb_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StGeomFromWkbUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StGeomFromWkbUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StGeomFromWkbUDF {
    fn new() -> Self {
        use datafusion::logical_expr::TypeSignature;
        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![DataType::Binary]),
                    TypeSignature::Exact(vec![DataType::LargeBinary]),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StGeomFromWkbUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_geomfromwkb"
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(wkb_data_type())
    }

    fn return_field_from_args(&self, args: ReturnFieldArgs) -> datafusion::error::Result<FieldRef> {
        let nullable = args.arg_fields[0].is_nullable();
        Ok(Arc::new(wkb_field(self.name(), nullable)))
    }

    fn invoke_with_args(
        &self,
        args: ScalarFunctionArgs,
    ) -> datafusion::error::Result<ColumnarValue> {
        let wkb_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        let result = validate_wkb(&wkb_array)
            .map_err(|e| DataFusionError::Execution(format!("ST_GeomFromWKB failed: {e}")))?;

        Ok(ColumnarValue::Array(result))
    }
}

/// Validate WKB binary data by parsing with GEOS
fn validate_wkb(arr: &ArrayRef) -> Result<ArrayRef, String> {
    let len = arr.len();
    let mut builder = BinaryBuilder::with_capacity(len, len * 64);

    let binary_array = arr
        .as_any()
        .downcast_ref::<BinaryArray>()
        .ok_or("ST_GeomFromWKB requires binary input")?;

    for i in 0..len {
        if binary_array.is_null(i) {
            builder.append_null();
            continue;
        }

        let wkb_bytes = binary_array.value(i);

        // Validate by parsing with GEOS
        let _ = GeosGeometry::new_from_wkb(wkb_bytes)
            .map_err(|e| format!("Invalid WKB at row {i}: {e}"))?;

        // Pass through the original WKB (already valid)
        builder.append_value(wkb_bytes);
    }

    Ok(Arc::new(builder.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::BinaryArray;
    use geos::{Geom, Geometry as GeosGeometry};

    #[test]
    fn test_st_geomfromwkb_udf_creation() {
        let udf = create_st_geomfromwkb_udf();
        assert_eq!(udf.name(), "st_geomfromwkb");
    }

    #[test]
    fn test_validate_wkb_point() {
        // Create valid WKB from WKT
        let geom = GeosGeometry::new_from_wkt("POINT(1 2)").unwrap();
        let wkb_data: Vec<u8> = geom.to_wkb().unwrap().into();

        let wkb_array: ArrayRef = Arc::new(BinaryArray::from(vec![wkb_data.as_slice()]));
        let result = validate_wkb(&wkb_array).unwrap();
        assert_eq!(result.len(), 1);
        assert!(!result.is_null(0));
    }

    #[test]
    fn test_validate_wkb_null_handling() {
        let geom = GeosGeometry::new_from_wkt("POINT(1 2)").unwrap();
        let wkb_data: Vec<u8> = geom.to_wkb().unwrap().into();

        let wkb_array: ArrayRef = Arc::new(BinaryArray::from(vec![
            Some(wkb_data.as_slice()),
            None,
            Some(wkb_data.as_slice()),
        ]));
        let result = validate_wkb(&wkb_array).unwrap();
        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }

    #[test]
    fn test_validate_wkb_invalid() {
        let wkb_array: ArrayRef = Arc::new(BinaryArray::from(vec![vec![0u8, 1, 2, 3].as_slice()]));
        let result = validate_wkb(&wkb_array);
        assert!(result.is_err());
    }
}
