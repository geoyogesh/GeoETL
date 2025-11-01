//! Custom error types for `GeoETL` operations.
//!
//! This module provides structured error handling using `thiserror`, replacing
//! generic `anyhow::Error` with domain-specific error types that preserve context
//! and enable better error messages and recovery strategies.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for `GeoETL` operations.
///
/// This is the root error type that encompasses all domain-specific errors.
/// It uses `#[error(transparent)]` to delegate display formatting to the
/// underlying error variants.
#[derive(Debug, Error)]
pub enum GeoEtlError {
    /// Driver-related errors (not found, unsupported operations, etc.)
    #[error(transparent)]
    Driver(#[from] DriverError),

    /// I/O errors (file read/write, path issues, permissions)
    #[error(transparent)]
    Io(#[from] IoError),

    /// Format parsing and validation errors
    #[error(transparent)]
    Format(#[from] FormatError),

    /// `DataFusion` query execution errors
    #[error(transparent)]
    DataFusion(#[from] DataFusionError),

    /// Configuration errors
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// Generic errors from dependencies (for gradual migration)
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Driver-related errors.
///
/// These errors occur when interacting with format drivers, such as
/// when a driver is not found, doesn't support an operation, or is
/// misconfigured.
#[derive(Debug, Error)]
pub enum DriverError {
    /// Driver was not found in the registry
    #[error("Driver '{name}' not found. Available drivers: {available}")]
    NotFound {
        /// The requested driver name
        name: String,
        /// Comma-separated list of available drivers
        available: String,
    },

    /// Driver does not support the requested operation
    #[error("Driver '{driver}' does not support {operation}")]
    OperationNotSupported {
        /// The driver name
        driver: String,
        /// The operation that's not supported (e.g., "reading", "writing")
        operation: String,
    },

    /// Driver configuration is invalid
    #[error("Invalid driver configuration: {message}")]
    InvalidConfiguration {
        /// Description of the configuration problem
        message: String,
    },

    /// Driver is not registered in the factory registry
    #[error("Driver '{driver}' is not registered in the registry")]
    NotRegistered {
        /// The driver name
        driver: String,
    },
}

/// I/O related errors.
///
/// These errors occur during file or stream operations, including
/// reading, writing, and path validation.
#[derive(Debug, Error)]
pub enum IoError {
    /// Failed to read from a file
    #[error("Failed to read {format} file '{path}': {source}")]
    Read {
        /// The format being read (e.g., "CSV", "`GeoJSON`")
        format: String,
        /// The file path
        path: PathBuf,
        /// The underlying error
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to write to a file
    #[error("Failed to write {format} file '{path}': {source}")]
    Write {
        /// The format being written
        format: String,
        /// The file path
        path: PathBuf,
        /// The underlying error
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Path is invalid
    #[error("Invalid path '{path}': {reason}")]
    InvalidPath {
        /// The invalid path
        path: PathBuf,
        /// Why the path is invalid
        reason: String,
    },

    /// File was not found
    #[error("File not found: '{path}'")]
    FileNotFound {
        /// The missing file path
        path: PathBuf,
    },

    /// Permission was denied
    #[error("Permission denied for '{path}'")]
    PermissionDenied {
        /// The path with permission issues
        path: PathBuf,
    },
}

/// Format parsing and validation errors.
///
/// These errors occur when parsing or validating geospatial data formats.
#[derive(Debug, Error)]
pub enum FormatError {
    /// Failed to parse a format
    #[error("Failed to parse {format} at line {line}: {message}", line = line.map(|l| l.to_string()).unwrap_or_else(|| "unknown".to_string()))]
    Parse {
        /// The format being parsed
        format: String,
        /// The line number where parsing failed (if available)
        line: Option<usize>,
        /// Description of the parse error
        message: String,
    },

    /// Schema inference failed
    #[error("Schema inference failed for {format}: {reason}")]
    SchemaInference {
        /// The format
        format: String,
        /// Why schema inference failed
        reason: String,
    },

    /// Invalid geometry
    #[error("Invalid geometry in {format}: {message}{}", feature_id.as_ref().map(|id| format!(" (feature {id})")).unwrap_or_default())]
    InvalidGeometry {
        /// The format
        format: String,
        /// Description of the geometry problem
        message: String,
        /// Optional feature ID where the error occurred
        feature_id: Option<String>,
    },

    /// Unsupported geometry type
    #[error("Unsupported geometry type: {geometry_type}")]
    UnsupportedGeometryType {
        /// The unsupported geometry type
        geometry_type: String,
    },

    /// Type mismatch in a field
    #[error("Field '{field}' has incompatible type: expected {expected}, found {found}")]
    TypeMismatch {
        /// The field name
        field: String,
        /// Expected type
        expected: String,
        /// Actual type found
        found: String,
    },
}

/// DataFusion-specific errors.
///
/// These errors occur during query execution or data processing.
#[derive(Debug, Error)]
pub enum DataFusionError {
    /// Query execution failed
    #[error("Query execution failed: {0}")]
    Query(#[from] datafusion::error::DataFusionError),

    /// Failed to collect query results
    #[error("Failed to collect results: {0}")]
    Collection(String),

    /// Schema-related error
    #[error("Schema error: {0}")]
    Schema(String),
}

/// Configuration errors.
///
/// These errors occur when options or configuration are invalid.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Invalid option value
    #[error("Invalid {option} option: {message}")]
    InvalidOption {
        /// The option name
        option: String,
        /// Why it's invalid
        message: String,
    },

    /// Required option is missing
    #[error("Missing required option: {option}")]
    MissingRequired {
        /// The missing option name
        option: String,
    },

    /// Options conflict with each other
    #[error("Conflicting options: {options}")]
    ConflictingOptions {
        /// Description of the conflicting options
        options: String,
    },
}

/// Type alias for Results using `GeoEtlError`.
pub type Result<T> = std::result::Result<T, GeoEtlError>;

impl GeoEtlError {
    /// Get a user-friendly error message with suggestions.
    ///
    /// This formats the error in a way that's helpful for end users,
    /// including context and actionable information.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Driver(e) => e.user_message(),
            Self::Io(e) => e.user_message(),
            Self::Format(e) => e.user_message(),
            Self::DataFusion(e) => format!("Query error: {e}"),
            Self::Config(e) => format!("Configuration error: {e}"),
            Self::Other(e) => format!("Error: {e}"),
        }
    }

    /// Get recovery suggestions if available.
    ///
    /// Returns helpful suggestions on how to fix or work around the error.
    #[must_use]
    pub fn recovery_suggestion(&self) -> Option<String> {
        match self {
            Self::Driver(e) => e.recovery_suggestion(),
            Self::Io(e) => e.recovery_suggestion(),
            Self::Format(e) => e.recovery_suggestion(),
            _ => None,
        }
    }

    /// Check if this error is potentially recoverable.
    ///
    /// Recoverable errors might be fixed by retrying with different
    /// parameters or after the user takes some action.
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Config(_) | Self::Driver(DriverError::InvalidConfiguration { .. })
        )
    }
}

impl DriverError {
    fn user_message(&self) -> String {
        match self {
            Self::NotFound { name, available } => {
                format!(
                    "Driver '{name}' not found.\n\nAvailable drivers:\n{}",
                    available
                        .split(", ")
                        .map(|d| format!("  - {d}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            },
            Self::OperationNotSupported { driver, operation } => {
                format!("The '{driver}' driver does not support {operation} operation.")
            },
            Self::InvalidConfiguration { .. } | Self::NotRegistered { .. } => self.to_string(),
        }
    }

    fn recovery_suggestion(&self) -> Option<String> {
        match self {
            Self::NotFound { .. } => {
                Some("Run 'geoetl drivers' to see all available drivers.".to_string())
            },
            Self::OperationNotSupported { .. } => {
                Some("Try using a different driver that supports this operation.".to_string())
            },
            Self::NotRegistered { .. } => {
                Some("This driver may not be enabled. Check your configuration.".to_string())
            },
            Self::InvalidConfiguration { .. } => None,
        }
    }
}

impl IoError {
    fn user_message(&self) -> String {
        match self {
            Self::Read { format, path, .. } => {
                format!("Failed to read {} file: {}", format, path.display())
            },
            Self::Write { format, path, .. } => {
                format!("Failed to write {} file: {}", format, path.display())
            },
            Self::FileNotFound { path } => {
                format!("File not found: {}", path.display())
            },
            _ => self.to_string(),
        }
    }

    fn recovery_suggestion(&self) -> Option<String> {
        match self {
            Self::FileNotFound { .. } => {
                Some("Check that the file path is correct and the file exists.".to_string())
            },
            Self::PermissionDenied { .. } => {
                Some("Check file permissions and ensure you have access.".to_string())
            },
            Self::InvalidPath { .. } => {
                Some("Ensure the path is valid and properly formatted.".to_string())
            },
            _ => None,
        }
    }
}

impl FormatError {
    fn user_message(&self) -> String {
        match self {
            Self::Parse {
                format,
                line,
                message,
            } => {
                if let Some(line_num) = line {
                    format!("Parse error in {format} at line {line_num}: {message}")
                } else {
                    format!("Parse error in {format}: {message}")
                }
            },
            Self::InvalidGeometry {
                format,
                message,
                feature_id,
            } => {
                if let Some(id) = feature_id {
                    format!("Invalid geometry in {format} (feature {id}): {message}")
                } else {
                    format!("Invalid geometry in {format}: {message}")
                }
            },
            Self::SchemaInference { .. }
            | Self::UnsupportedGeometryType { .. }
            | Self::TypeMismatch { .. } => self.to_string(),
        }
    }

    fn recovery_suggestion(&self) -> Option<String> {
        match self {
            Self::Parse { .. } => Some("Check the file format and ensure it's valid.".to_string()),
            Self::InvalidGeometry { .. } => {
                Some("Validate geometries using a GIS tool before importing.".to_string())
            },
            Self::SchemaInference { .. } => Some("Try specifying the schema manually.".to_string()),
            _ => None,
        }
    }
}

/// Extension trait for adding I/O context to errors.
///
/// This trait provides convenient methods to wrap errors with file and format
/// context, creating more informative error messages.
pub trait IoErrorExt<T> {
    /// Add read context to an error.
    ///
    /// # Errors
    ///
    /// Returns an [`IoError::Read`] if the underlying operation fails.
    fn with_read_context(self, format: &str, path: impl Into<PathBuf>) -> Result<T>;

    /// Add write context to an error.
    ///
    /// # Errors
    ///
    /// Returns an [`IoError::Write`] if the underlying operation fails.
    fn with_write_context(self, format: &str, path: impl Into<PathBuf>) -> Result<T>;
}

impl<T, E> IoErrorExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_read_context(self, format: &str, path: impl Into<PathBuf>) -> Result<T> {
        self.map_err(|e| {
            GeoEtlError::Io(IoError::Read {
                format: format.to_string(),
                path: path.into(),
                source: Box::new(e),
            })
        })
    }

    fn with_write_context(self, format: &str, path: impl Into<PathBuf>) -> Result<T> {
        self.map_err(|e| {
            GeoEtlError::Io(IoError::Write {
                format: format.to_string(),
                path: path.into(),
                source: Box::new(e),
            })
        })
    }
}

/// Helper to create `DriverError::NotFound` with available drivers.
#[must_use]
pub fn driver_not_found(name: &str) -> DriverError {
    use crate::drivers::get_driver_names;

    let available = get_driver_names().join(", ");
    DriverError::NotFound {
        name: name.to_string(),
        available,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_driver_not_found_error() {
        let error = driver_not_found("unknown");
        assert!(error.to_string().contains("unknown"));
        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn test_driver_not_found_user_message() {
        let error = GeoEtlError::Driver(driver_not_found("csv2"));
        let message = error.user_message();
        assert!(message.contains("csv2"));
        assert!(message.contains("not found"));
    }

    #[test]
    fn test_driver_not_found_recovery() {
        let error = GeoEtlError::Driver(driver_not_found("unknown"));
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("geoetl drivers"));
    }

    #[test]
    fn test_driver_operation_not_supported() {
        let error = DriverError::OperationNotSupported {
            driver: "flatgeobuf".to_string(),
            operation: "writing".to_string(),
        };
        assert!(error.to_string().contains("flatgeobuf"));
        assert!(error.to_string().contains("writing"));
    }

    #[test]
    fn test_driver_operation_not_supported_user_message() {
        let error = GeoEtlError::Driver(DriverError::OperationNotSupported {
            driver: "test".to_string(),
            operation: "reading".to_string(),
        });
        let message = error.user_message();
        assert!(message.contains("test"));
        assert!(message.contains("reading"));
    }

    #[test]
    fn test_driver_operation_not_supported_recovery() {
        let error = GeoEtlError::Driver(DriverError::OperationNotSupported {
            driver: "test".to_string(),
            operation: "reading".to_string(),
        });
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("different driver"));
    }

    #[test]
    fn test_driver_invalid_configuration() {
        let error = DriverError::InvalidConfiguration {
            message: "missing required option".to_string(),
        };
        assert!(error.to_string().contains("Invalid driver configuration"));
    }

    #[test]
    fn test_driver_not_registered() {
        let error = DriverError::NotRegistered {
            driver: "custom".to_string(),
        };
        assert!(error.to_string().contains("custom"));
        assert!(error.to_string().contains("not registered"));
    }

    #[test]
    fn test_driver_not_registered_recovery() {
        let error = GeoEtlError::Driver(DriverError::NotRegistered {
            driver: "custom".to_string(),
        });
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("not be enabled"));
    }

    #[test]
    fn test_io_error_read() {
        let error = IoError::Read {
            format: "CSV".to_string(),
            path: PathBuf::from("/tmp/test.csv"),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            )),
        };
        assert!(error.to_string().contains("CSV"));
        assert!(error.to_string().contains("/tmp/test.csv"));
    }

    #[test]
    fn test_io_error_read_user_message() {
        let error = GeoEtlError::Io(IoError::Read {
            format: "CSV".to_string(),
            path: PathBuf::from("/tmp/test.csv"),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            )),
        });
        let message = error.user_message();
        assert!(message.contains("CSV"));
        assert!(message.contains("/tmp/test.csv"));
    }

    #[test]
    fn test_io_error_write() {
        let error = IoError::Write {
            format: "GeoJSON".to_string(),
            path: PathBuf::from("/tmp/output.geojson"),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            )),
        };
        assert!(error.to_string().contains("GeoJSON"));
        assert!(error.to_string().contains("/tmp/output.geojson"));
    }

    #[test]
    fn test_io_error_write_user_message() {
        let error = GeoEtlError::Io(IoError::Write {
            format: "GeoJSON".to_string(),
            path: PathBuf::from("/tmp/output.geojson"),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            )),
        });
        let message = error.user_message();
        assert!(message.contains("GeoJSON"));
        assert!(message.contains("/tmp/output.geojson"));
    }

    #[test]
    fn test_io_error_file_not_found() {
        let error = IoError::FileNotFound {
            path: PathBuf::from("/missing/file.csv"),
        };
        assert!(error.to_string().contains("/missing/file.csv"));
    }

    #[test]
    fn test_io_error_file_not_found_recovery() {
        let error = GeoEtlError::Io(IoError::FileNotFound {
            path: PathBuf::from("/missing/file.csv"),
        });
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("file path is correct"));
    }

    #[test]
    fn test_io_error_permission_denied() {
        let error = IoError::PermissionDenied {
            path: PathBuf::from("/restricted/file.csv"),
        };
        assert!(error.to_string().contains("/restricted/file.csv"));
    }

    #[test]
    fn test_io_error_permission_denied_recovery() {
        let error = GeoEtlError::Io(IoError::PermissionDenied {
            path: PathBuf::from("/restricted/file.csv"),
        });
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("permissions"));
    }

    #[test]
    fn test_io_error_invalid_path() {
        let error = IoError::InvalidPath {
            path: PathBuf::from("invalid:::path"),
            reason: "invalid characters".to_string(),
        };
        assert!(error.to_string().contains("invalid:::path"));
        assert!(error.to_string().contains("invalid characters"));
    }

    #[test]
    fn test_io_error_invalid_path_recovery() {
        let error = GeoEtlError::Io(IoError::InvalidPath {
            path: PathBuf::from("invalid:::path"),
            reason: "test".to_string(),
        });
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("valid"));
    }

    #[test]
    fn test_format_error_parse() {
        let error = FormatError::Parse {
            format: "GeoJSON".to_string(),
            line: Some(10),
            message: "unexpected token".to_string(),
        };
        assert!(error.to_string().contains("GeoJSON"));
        assert!(error.to_string().contains("10"));
        assert!(error.to_string().contains("unexpected token"));
    }

    #[test]
    fn test_format_error_parse_no_line() {
        let error = FormatError::Parse {
            format: "CSV".to_string(),
            line: None,
            message: "invalid format".to_string(),
        };
        assert!(error.to_string().contains("CSV"));
        assert!(error.to_string().contains("invalid format"));
    }

    #[test]
    fn test_format_error_parse_user_message() {
        let error = GeoEtlError::Format(FormatError::Parse {
            format: "CSV".to_string(),
            line: Some(5),
            message: "test".to_string(),
        });
        let message = error.user_message();
        assert!(message.contains("CSV"));
        assert!(message.contains('5'));
    }

    #[test]
    fn test_format_error_parse_recovery() {
        let error = GeoEtlError::Format(FormatError::Parse {
            format: "CSV".to_string(),
            line: Some(5),
            message: "test".to_string(),
        });
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("file format"));
    }

    #[test]
    fn test_format_error_schema_inference() {
        let error = FormatError::SchemaInference {
            format: "CSV".to_string(),
            reason: "inconsistent types".to_string(),
        };
        assert!(error.to_string().contains("CSV"));
        assert!(error.to_string().contains("inconsistent types"));
    }

    #[test]
    fn test_format_error_schema_inference_recovery() {
        let error = GeoEtlError::Format(FormatError::SchemaInference {
            format: "CSV".to_string(),
            reason: "test".to_string(),
        });
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("schema manually"));
    }

    #[test]
    fn test_format_error_invalid_geometry() {
        let error = FormatError::InvalidGeometry {
            format: "GeoJSON".to_string(),
            message: "invalid WKT".to_string(),
            feature_id: Some("feature-123".to_string()),
        };
        assert!(error.to_string().contains("GeoJSON"));
        assert!(error.to_string().contains("invalid WKT"));
        assert!(error.to_string().contains("feature-123"));
    }

    #[test]
    fn test_format_error_invalid_geometry_no_id() {
        let error = FormatError::InvalidGeometry {
            format: "GeoJSON".to_string(),
            message: "invalid WKT".to_string(),
            feature_id: None,
        };
        assert!(error.to_string().contains("GeoJSON"));
        assert!(error.to_string().contains("invalid WKT"));
    }

    #[test]
    fn test_format_error_invalid_geometry_user_message() {
        let error = GeoEtlError::Format(FormatError::InvalidGeometry {
            format: "WKT".to_string(),
            message: "test".to_string(),
            feature_id: Some("123".to_string()),
        });
        let message = error.user_message();
        assert!(message.contains("WKT"));
        assert!(message.contains("123"));
    }

    #[test]
    fn test_format_error_invalid_geometry_recovery() {
        let error = GeoEtlError::Format(FormatError::InvalidGeometry {
            format: "WKT".to_string(),
            message: "test".to_string(),
            feature_id: None,
        });
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("GIS tool"));
    }

    #[test]
    fn test_format_error_unsupported_geometry_type() {
        let error = FormatError::UnsupportedGeometryType {
            geometry_type: "Unknown".to_string(),
        };
        assert!(error.to_string().contains("Unknown"));
    }

    #[test]
    fn test_format_error_type_mismatch() {
        let error = FormatError::TypeMismatch {
            field: "age".to_string(),
            expected: "Integer".to_string(),
            found: "String".to_string(),
        };
        assert!(error.to_string().contains("age"));
        assert!(error.to_string().contains("Integer"));
        assert!(error.to_string().contains("String"));
    }

    #[test]
    fn test_config_error_invalid_option() {
        let error = ConfigError::InvalidOption {
            option: "delimiter".to_string(),
            message: "must be single byte".to_string(),
        };
        assert!(error.to_string().contains("delimiter"));
        assert!(error.to_string().contains("must be single byte"));
    }

    #[test]
    fn test_config_error_missing_required() {
        let error = ConfigError::MissingRequired {
            option: "input_file".to_string(),
        };
        assert!(error.to_string().contains("input_file"));
    }

    #[test]
    fn test_config_error_conflicting_options() {
        let error = ConfigError::ConflictingOptions {
            options: "format and delimiter".to_string(),
        };
        assert!(error.to_string().contains("format and delimiter"));
    }

    #[test]
    fn test_error_is_recoverable() {
        let config_error = GeoEtlError::Config(ConfigError::InvalidOption {
            option: "test".to_string(),
            message: "test".to_string(),
        });
        assert!(config_error.is_recoverable());

        let driver_error = GeoEtlError::Driver(DriverError::InvalidConfiguration {
            message: "test".to_string(),
        });
        assert!(driver_error.is_recoverable());

        let io_error = GeoEtlError::Io(IoError::FileNotFound {
            path: PathBuf::from("/test"),
        });
        assert!(!io_error.is_recoverable());
    }

    #[test]
    fn test_io_error_ext_read_context() {
        let result: std::io::Result<()> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not found",
        ));
        let error = result.with_read_context("CSV", "/test.csv");
        assert!(error.is_err());
        let err_msg = error.unwrap_err().to_string();
        assert!(err_msg.contains("CSV"));
        assert!(err_msg.contains("/test.csv"));
    }

    #[test]
    fn test_io_error_ext_write_context() {
        let result: std::io::Result<()> = Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "permission denied",
        ));
        let error = result.with_write_context("GeoJSON", "/output.geojson");
        assert!(error.is_err());
        let err_msg = error.unwrap_err().to_string();
        assert!(err_msg.contains("GeoJSON"));
        assert!(err_msg.contains("/output.geojson"));
    }

    #[test]
    fn test_datafusion_error() {
        let df_error = datafusion::error::DataFusionError::Plan("test error".to_string());
        let error = GeoEtlError::DataFusion(DataFusionError::Query(df_error));
        assert!(error.user_message().contains("Query error"));
    }

    #[test]
    fn test_other_error() {
        let other_error = anyhow::anyhow!("generic error");
        let error = GeoEtlError::Other(other_error);
        assert!(error.user_message().contains("generic error"));
        assert!(error.recovery_suggestion().is_none());
    }
}
