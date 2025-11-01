//! Utility functions and extension traits for geospatial ETL operations.
//!
//! This module provides helper functions for data type formatting,
//! conversions, and other common operations.

use arrow_schema::DataType;

/// Extension trait for formatting Arrow [`DataType`] into human-readable strings.
///
/// This trait extends Arrow's [`DataType`] with a convenient `format()` method
/// that converts internal data type representations into user-friendly string
/// labels suitable for display in schemas and documentation.
///
/// # Examples
///
/// ```
/// use arrow_schema::DataType;
/// use geoetl_core::utils::ArrowDataTypeExt;
///
/// let data_type = DataType::Int32;
/// assert_eq!(data_type.format(), "Int32");
///
/// let data_type = DataType::Utf8;
/// assert_eq!(data_type.format(), "String");
/// ```
pub trait ArrowDataTypeExt {
    /// Format the data type into a human-readable string.
    ///
    /// # Returns
    ///
    /// A string representation suitable for display to users.
    fn format(&self) -> String;
}

impl ArrowDataTypeExt for DataType {
    fn format(&self) -> String {
        match self {
            DataType::Boolean => "Boolean".to_string(),
            DataType::Int8 => "Int8".to_string(),
            DataType::Int16 => "Int16".to_string(),
            DataType::Int32 => "Int32".to_string(),
            DataType::Int64 => "Int64".to_string(),
            DataType::UInt8 => "UInt8".to_string(),
            DataType::UInt16 => "UInt16".to_string(),
            DataType::UInt32 => "UInt32".to_string(),
            DataType::UInt64 => "UInt64".to_string(),
            DataType::Float16 => "Float16".to_string(),
            DataType::Float32 => "Float32".to_string(),
            DataType::Float64 => "Float64".to_string(),
            DataType::Utf8 => "String".to_string(),
            DataType::LargeUtf8 => "LargeString".to_string(),
            DataType::Binary => "Binary".to_string(),
            DataType::LargeBinary => "LargeBinary".to_string(),
            DataType::Date32 => "Date32".to_string(),
            DataType::Date64 => "Date64".to_string(),
            DataType::Timestamp(unit, tz) => {
                let tz_str = tz.as_ref().map_or("", |t| t.as_ref());
                format!("Timestamp({unit:?}, {tz_str})")
            },
            DataType::List(_) => "List".to_string(),
            DataType::LargeList(_) => "LargeList".to_string(),
            DataType::Struct(_) => "Struct".to_string(),
            DataType::Map(_, _) => "Map".to_string(),
            _ => format!("{self:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_boolean() {
        assert_eq!(DataType::Boolean.format(), "Boolean");
    }

    #[test]
    fn test_format_integers() {
        assert_eq!(DataType::Int8.format(), "Int8");
        assert_eq!(DataType::Int16.format(), "Int16");
        assert_eq!(DataType::Int32.format(), "Int32");
        assert_eq!(DataType::Int64.format(), "Int64");
        assert_eq!(DataType::UInt8.format(), "UInt8");
        assert_eq!(DataType::UInt16.format(), "UInt16");
        assert_eq!(DataType::UInt32.format(), "UInt32");
        assert_eq!(DataType::UInt64.format(), "UInt64");
    }

    #[test]
    fn test_format_floats() {
        assert_eq!(DataType::Float16.format(), "Float16");
        assert_eq!(DataType::Float32.format(), "Float32");
        assert_eq!(DataType::Float64.format(), "Float64");
    }

    #[test]
    fn test_format_strings() {
        assert_eq!(DataType::Utf8.format(), "String");
        assert_eq!(DataType::LargeUtf8.format(), "LargeString");
    }

    #[test]
    fn test_format_binary() {
        assert_eq!(DataType::Binary.format(), "Binary");
        assert_eq!(DataType::LargeBinary.format(), "LargeBinary");
    }

    #[test]
    fn test_format_dates() {
        assert_eq!(DataType::Date32.format(), "Date32");
        assert_eq!(DataType::Date64.format(), "Date64");
    }

    #[test]
    fn test_format_timestamp_without_timezone() {
        use arrow_schema::TimeUnit;
        let dt = DataType::Timestamp(TimeUnit::Millisecond, None);
        let formatted = dt.format();
        assert!(formatted.starts_with("Timestamp("));
        assert!(formatted.contains("Millisecond"));
    }

    #[test]
    fn test_format_timestamp_with_timezone() {
        use arrow_schema::TimeUnit;
        let dt = DataType::Timestamp(TimeUnit::Second, Some("UTC".into()));
        let formatted = dt.format();
        assert!(formatted.starts_with("Timestamp("));
        assert!(formatted.contains("Second"));
        assert!(formatted.contains("UTC"));
    }

    #[test]
    fn test_format_list_types() {
        use arrow_schema::Field;
        use std::sync::Arc;

        let field = Arc::new(Field::new("item", DataType::Int32, true));
        assert_eq!(DataType::List(field.clone()).format(), "List");
        assert_eq!(DataType::LargeList(field).format(), "LargeList");
    }

    #[test]
    fn test_format_struct() {
        use arrow_schema::Field;
        use std::sync::Arc;

        let fields = vec![
            Arc::new(Field::new("a", DataType::Int32, false)),
            Arc::new(Field::new("b", DataType::Utf8, true)),
        ];
        assert_eq!(DataType::Struct(fields.into()).format(), "Struct");
    }

    #[test]
    fn test_format_map() {
        use arrow_schema::Field;
        use std::sync::Arc;

        let field = Arc::new(Field::new(
            "entries",
            DataType::Struct(
                vec![
                    Arc::new(Field::new("key", DataType::Utf8, false)),
                    Arc::new(Field::new("value", DataType::Int32, true)),
                ]
                .into(),
            ),
            false,
        ));
        assert_eq!(DataType::Map(field, false).format(), "Map");
    }

    #[test]
    fn test_format_other_types() {
        // Test that other types fall through to debug formatting
        let dt = DataType::Null;
        let formatted = dt.format();
        assert!(formatted.contains("Null"));
    }
}
