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
