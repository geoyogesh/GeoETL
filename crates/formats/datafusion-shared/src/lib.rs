use std::error::Error as StdError;
use std::fmt;

use datafusion_common::DataFusionError;

/// A position within a source file, such as a CSV record.
///
/// All indices are 1-based where possible to align with human expectations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourcePosition {
    /// Line number in the source (1-based)
    pub line: Option<u64>,
    /// Column (field) number in the source (1-based)
    pub column: Option<u64>,
    /// Byte offset from the start of the source
    pub byte_offset: Option<u64>,
    /// Logical record number reported by the parser
    pub record: Option<u64>,
    /// Field index reported by the parser (1-based)
    pub field: Option<u64>,
}

impl SourcePosition {
    /// Returns true when the position does not contain any location metadata.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.line.is_none()
            && self.column.is_none()
            && self.byte_offset.is_none()
            && self.record.is_none()
            && self.field.is_none()
    }
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if let Some(line) = self.line {
            parts.push(format!("line {line}"));
        }
        if let Some(column) = self.column {
            parts.push(format!("column {column}"));
        }
        if let Some(record) = self.record {
            parts.push(format!("record {record}"));
        }
        if let Some(byte) = self.byte_offset {
            parts.push(format!("byte {byte}"));
        }
        if let Some(field) = self.field {
            parts.push(format!("field {field}"));
        }

        if parts.is_empty() {
            write!(f, "unknown position")
        } else {
            write!(f, "{}", parts.join(", "))
        }
    }
}

/// Errors that can occur when reading spatial data formats from tabular sources.
#[derive(Debug)]
pub enum SpatialFormatReadError {
    /// An underlying I/O failure occurred.
    Io {
        /// The originating error.
        source: std::io::Error,
        /// Optional context describing what was being read.
        context: Option<String>,
    },
    /// Parsing failed for the input source.
    Parse {
        /// Human readable description of the failure.
        message: String,
        /// Optional position describing where the failure occurred.
        position: Option<SourcePosition>,
        /// Optional context describing what was being read.
        context: Option<String>,
    },
    /// Schema inference failed for the source.
    SchemaInference {
        /// Human readable description of the failure.
        message: String,
        /// Optional context describing what was being read.
        context: Option<String>,
    },
    /// Other error type not classified above.
    Other {
        /// Human readable description of the failure.
        message: String,
    },
}

impl SpatialFormatReadError {
    fn fmt_context(context: Option<&str>) -> String {
        context
            .map(|c| format!(" while reading {c}"))
            .unwrap_or_default()
    }

    fn fmt_position(position: Option<&SourcePosition>) -> String {
        position.map(|pos| format!(" at {pos}")).unwrap_or_default()
    }

    /// Attach additional context to the error, returning the updated error.
    #[must_use]
    pub fn with_additional_context(mut self, context: impl Into<String>) -> Self {
        let context = context.into();
        match &mut self {
            SpatialFormatReadError::Io {
                context: existing, ..
            }
            | SpatialFormatReadError::Parse {
                context: existing, ..
            }
            | SpatialFormatReadError::SchemaInference {
                context: existing, ..
            } => match existing {
                Some(existing) if !existing.is_empty() => {
                    existing.push_str("; ");
                    existing.push_str(&context);
                },
                _ => *existing = Some(context),
            },
            SpatialFormatReadError::Other { message } => {
                message.push_str(" (");
                message.push_str(&context);
                message.push(')');
            },
        }
        self
    }
}

impl fmt::Display for SpatialFormatReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpatialFormatReadError::Io { source, context } => {
                write!(
                    f,
                    "I/O error{}: {source}",
                    Self::fmt_context(context.as_deref())
                )
            },
            SpatialFormatReadError::Parse {
                message,
                position,
                context,
            } => write!(
                f,
                "Parse error{}{}: {message}",
                Self::fmt_context(context.as_deref()),
                Self::fmt_position(position.as_ref())
            ),
            SpatialFormatReadError::SchemaInference { message, context } => write!(
                f,
                "Schema inference error{}: {message}",
                Self::fmt_context(context.as_deref())
            ),
            SpatialFormatReadError::Other { message } => f.write_str(message),
        }
    }
}

impl StdError for SpatialFormatReadError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            SpatialFormatReadError::Io { source, .. } => Some(source),
            SpatialFormatReadError::Parse { .. }
            | SpatialFormatReadError::SchemaInference { .. }
            | SpatialFormatReadError::Other { .. } => None,
        }
    }
}

impl From<SpatialFormatReadError> for DataFusionError {
    fn from(err: SpatialFormatReadError) -> Self {
        DataFusionError::External(Box::new(err))
    }
}

/// Result type alias that uses [`SpatialFormatReadError`].
pub type SpatialFormatResult<T> = Result<T, SpatialFormatReadError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_source_position() {
        let pos = SourcePosition {
            line: Some(10),
            column: Some(3),
            ..SourcePosition::default()
        };

        assert_eq!(pos.to_string(), "line 10, column 3");
    }

    #[test]
    fn display_parse_error_with_context() {
        let error = SpatialFormatReadError::Parse {
            message: "unexpected delimiter".to_string(),
            position: Some(SourcePosition {
                line: Some(5),
                column: Some(7),
                ..Default::default()
            }),
            context: Some("s3://example/data.csv".to_string()),
        };

        assert_eq!(
            error.to_string(),
            "Parse error while reading s3://example/data.csv at line 5, column 7: unexpected delimiter"
        );
    }

    #[test]
    fn test_source_position_empty() {
        let pos = SourcePosition::default();
        assert!(pos.is_empty());

        let pos = SourcePosition {
            line: Some(1),
            ..Default::default()
        };
        assert!(!pos.is_empty());
    }

    #[test]
    fn test_source_position_display_all_fields() {
        let pos = SourcePosition {
            line: Some(10),
            column: Some(3),
            byte_offset: Some(100),
            record: Some(5),
            field: Some(2),
        };

        let display = pos.to_string();
        assert!(display.contains("line 10"));
        assert!(display.contains("column 3"));
        assert!(display.contains("record 5"));
        assert!(display.contains("byte 100"));
        assert!(display.contains("field 2"));
    }

    #[test]
    fn test_source_position_display_empty() {
        let pos = SourcePosition::default();
        assert_eq!(pos.to_string(), "unknown position");
    }

    #[test]
    fn test_io_error_display() {
        let error = SpatialFormatReadError::Io {
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            context: Some("test.csv".to_string()),
        };

        let display = error.to_string();
        assert!(display.contains("I/O error"));
        assert!(display.contains("while reading test.csv"));
    }

    #[test]
    fn test_io_error_display_no_context() {
        let error = SpatialFormatReadError::Io {
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            context: None,
        };

        let display = error.to_string();
        assert!(display.contains("I/O error"));
        assert!(!display.contains("while reading"));
    }

    #[test]
    fn test_parse_error_display_no_position() {
        let error = SpatialFormatReadError::Parse {
            message: "invalid format".to_string(),
            position: None,
            context: Some("data.csv".to_string()),
        };

        let display = error.to_string();
        assert!(display.contains("Parse error"));
        assert!(display.contains("while reading data.csv"));
        assert!(display.contains("invalid format"));
    }

    #[test]
    fn test_parse_error_display_no_context() {
        let error = SpatialFormatReadError::Parse {
            message: "invalid format".to_string(),
            position: Some(SourcePosition {
                line: Some(5),
                ..Default::default()
            }),
            context: None,
        };

        let display = error.to_string();
        assert!(display.contains("Parse error"));
        assert!(display.contains("at line 5"));
        assert!(display.contains("invalid format"));
    }

    #[test]
    fn test_schema_inference_error_display() {
        let error = SpatialFormatReadError::SchemaInference {
            message: "inconsistent types".to_string(),
            context: Some("data.csv".to_string()),
        };

        let display = error.to_string();
        assert!(display.contains("Schema inference error"));
        assert!(display.contains("while reading data.csv"));
        assert!(display.contains("inconsistent types"));
    }

    #[test]
    fn test_schema_inference_error_display_no_context() {
        let error = SpatialFormatReadError::SchemaInference {
            message: "inconsistent types".to_string(),
            context: None,
        };

        let display = error.to_string();
        assert!(display.contains("Schema inference error"));
        assert!(display.contains("inconsistent types"));
    }

    #[test]
    fn test_other_error_display() {
        let error = SpatialFormatReadError::Other {
            message: "unknown error".to_string(),
        };

        assert_eq!(error.to_string(), "unknown error");
    }

    #[test]
    fn test_error_source() {
        let io_error = SpatialFormatReadError::Io {
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            context: None,
        };
        assert!(io_error.source().is_some());

        let parse_error = SpatialFormatReadError::Parse {
            message: "test".to_string(),
            position: None,
            context: None,
        };
        assert!(parse_error.source().is_none());

        let schema_error = SpatialFormatReadError::SchemaInference {
            message: "test".to_string(),
            context: None,
        };
        assert!(schema_error.source().is_none());

        let other_error = SpatialFormatReadError::Other {
            message: "test".to_string(),
        };
        assert!(other_error.source().is_none());
    }

    #[test]
    fn test_with_additional_context_io() {
        let error = SpatialFormatReadError::Io {
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            context: Some("original".to_string()),
        };

        let error = error.with_additional_context("additional");
        let display = error.to_string();
        assert!(display.contains("original; additional"));
    }

    #[test]
    fn test_with_additional_context_io_no_existing() {
        let error = SpatialFormatReadError::Io {
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            context: None,
        };

        let error = error.with_additional_context("new context");
        let display = error.to_string();
        assert!(display.contains("while reading new context"));
    }

    #[test]
    fn test_with_additional_context_parse() {
        let error = SpatialFormatReadError::Parse {
            message: "test".to_string(),
            position: None,
            context: Some("original".to_string()),
        };

        let error = error.with_additional_context("additional");
        let display = error.to_string();
        assert!(display.contains("original; additional"));
    }

    #[test]
    fn test_with_additional_context_schema() {
        let error = SpatialFormatReadError::SchemaInference {
            message: "test".to_string(),
            context: Some("original".to_string()),
        };

        let error = error.with_additional_context("additional");
        let display = error.to_string();
        assert!(display.contains("original; additional"));
    }

    #[test]
    fn test_with_additional_context_other() {
        let error = SpatialFormatReadError::Other {
            message: "test".to_string(),
        };

        let error = error.with_additional_context("additional");
        let display = error.to_string();
        assert!(display.contains("test"));
        assert!(display.contains("additional"));
    }

    #[test]
    fn test_datafusion_error_conversion() {
        let error = SpatialFormatReadError::Other {
            message: "test".to_string(),
        };

        let df_error: DataFusionError = error.into();
        assert!(matches!(df_error, DataFusionError::External(_)));
    }
}
