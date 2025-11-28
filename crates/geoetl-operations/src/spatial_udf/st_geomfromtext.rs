//! `ST_GeomFromText` implementation - converts WKT to `GeoArrow` WKB format

use super::geoarrow_types::{wkb_data_type, wkb_field};
use datafusion::arrow::array::{Array, ArrayRef, BinaryBuilder, StringArray};
use datafusion::arrow::datatypes::{DataType, FieldRef};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ReturnFieldArgs, ScalarFunctionArgs, ScalarUDF, Volatility};
use datafusion::physical_plan::ColumnarValue;
use geos::{Geom, Geometry as GeosGeometry};
use std::sync::Arc;

/// Create the `ST_GeomFromText` User Defined Function
///
/// Parses WKT (Well-Known Text) strings into `GeoArrow` WKB format.
///
/// # SQL Usage
///
/// ```sql
/// SELECT ST_GeomFromText('POINT(1 2)');
/// SELECT ST_Distance(ST_GeomFromText(wkt_column), ST_Point(0, 0)) FROM table;
/// ```
///
/// # Arguments
///
/// - `wkt`: WKT string representation of a geometry
///
/// # Returns
///
/// `GeoArrow` WKB: `Binary` with `geoarrow.wkb` extension metadata
#[must_use]
pub fn create_st_geomfromtext_udf() -> ScalarUDF {
    ScalarUDF::new_from_impl(StGeomFromTextUDF::new())
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct StGeomFromTextUDF {
    signature: datafusion::logical_expr::Signature,
}

impl StGeomFromTextUDF {
    fn new() -> Self {
        use datafusion::logical_expr::TypeSignature;
        Self {
            signature: datafusion::logical_expr::Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![DataType::Utf8]),
                    TypeSignature::Exact(vec![DataType::LargeUtf8]),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl datafusion::logical_expr::ScalarUDFImpl for StGeomFromTextUDF {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "st_geomfromtext"
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
        let wkt_array = match &args.args[0] {
            ColumnarValue::Array(array) => array.clone(),
            ColumnarValue::Scalar(scalar) => scalar.to_array()?,
        };

        let result = wkt_to_wkb(&wkt_array)
            .map_err(|e| DataFusionError::Execution(format!("ST_GeomFromText failed: {e}")))?;

        Ok(ColumnarValue::Array(result))
    }
}

/// Convert WKT strings to WKB binary format
fn wkt_to_wkb(arr: &ArrayRef) -> Result<ArrayRef, String> {
    let len = arr.len();
    let mut builder = BinaryBuilder::with_capacity(len, len * 64); // estimate 64 bytes per geometry

    let string_array = arr
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or("ST_GeomFromText requires string input")?;

    for i in 0..len {
        if string_array.is_null(i) {
            builder.append_null();
            continue;
        }

        let wkt_str = string_array.value(i);
        let geos_geom = GeosGeometry::new_from_wkt(wkt_str)
            .map_err(|e| format!("Invalid WKT at row {i}: {e}"))?;

        let wkb_bytes: Vec<u8> = geos_geom
            .to_wkb()
            .map_err(|e| format!("Failed to convert to WKB at row {i}: {e}"))?
            .into();

        builder.append_value(&wkb_bytes);
    }

    Ok(Arc::new(builder.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::StringArray;

    #[test]
    fn test_st_geomfromtext_udf_creation() {
        let udf = create_st_geomfromtext_udf();
        assert_eq!(udf.name(), "st_geomfromtext");
    }

    #[test]
    fn test_wkt_to_wkb_point() {
        let wkt_array: ArrayRef = Arc::new(StringArray::from(vec!["POINT(1 2)"]));
        let result = wkt_to_wkb(&wkt_array).unwrap();
        assert_eq!(result.len(), 1);
        assert!(!result.is_null(0));
    }

    #[test]
    fn test_wkt_to_wkb_null_handling() {
        let wkt_array: ArrayRef = Arc::new(StringArray::from(vec![
            Some("POINT(1 2)"),
            None,
            Some("POINT(3 4)"),
        ]));
        let result = wkt_to_wkb(&wkt_array).unwrap();
        assert_eq!(result.len(), 3);
        assert!(!result.is_null(0));
        assert!(result.is_null(1));
        assert!(!result.is_null(2));
    }

    #[test]
    fn test_wkt_to_wkb_invalid_wkt() {
        let wkt_array: ArrayRef = Arc::new(StringArray::from(vec!["NOT VALID WKT"]));
        let result = wkt_to_wkb(&wkt_array);
        assert!(result.is_err());
    }
}
