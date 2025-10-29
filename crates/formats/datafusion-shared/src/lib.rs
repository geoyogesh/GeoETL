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
}
