//! Driver registry for geospatial data format support and capabilities.
//!
//! This module provides a static registry of geospatial data format drivers, including
//! their current support status (supported, planned, or not supported) for various operations
//! (info, read, write). The registry is modeled after GDAL's driver system but designed for
//! modern Rust-based ETL workflows.

/// Support status for a specific driver operation.
///
/// Indicates whether a driver operation (info, read, or write) is currently supported,
/// planned for future implementation, or not supported at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportStatus {
    /// The feature is fully supported and implemented.
    Supported,
    /// The feature is not supported by the driver.
    NotSupported,
    /// The feature is planned for future implementation.
    Planned,
}

impl SupportStatus {
    /// Returns `true` if the operation is fully supported and implemented.
    #[must_use]
    pub fn is_supported(&self) -> bool {
        matches!(self, SupportStatus::Supported)
    }

    /// Returns `true` if the operation is supported or planned (i.e., not explicitly unsupported).
    #[must_use]
    pub fn is_available(&self) -> bool {
        !matches!(self, SupportStatus::NotSupported)
    }

    /// Returns the string representation of this support status.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            SupportStatus::Supported => "Supported",
            SupportStatus::NotSupported => "Not Supported",
            SupportStatus::Planned => "Planned",
        }
    }
}

/// Capabilities supported by a geospatial data format driver.
///
/// Each driver can support three types of operations: reading metadata (info),
/// reading data (read), and writing data (write). Each capability has an associated
/// [`SupportStatus`] indicating its current implementation status.
#[derive(Debug, Clone, Copy)]
pub struct DriverCapabilities {
    /// Support status for reading dataset metadata and information.
    pub info: SupportStatus,
    /// Support status for reading data from this format.
    pub read: SupportStatus,
    /// Support status for writing data to this format.
    pub write: SupportStatus,
}

impl DriverCapabilities {
    /// Returns `true` if at least one operation is supported or planned.
    #[must_use]
    pub fn has_any_support(&self) -> bool {
        self.info.is_available() || self.read.is_available() || self.write.is_available()
    }

    /// Returns `true` if at least one operation is fully supported and implemented.
    #[must_use]
    pub fn has_supported_operation(&self) -> bool {
        self.info.is_supported() || self.read.is_supported() || self.write.is_supported()
    }
}

/// Geospatial data format driver definition.
///
/// A driver represents support for a specific geospatial data format (e.g., `GeoJSON`, `Shapefile`).
/// Each driver has a short name (used in the CLI), a descriptive long name, and a set of
/// capabilities indicating what operations are supported.
#[derive(Debug, Clone)]
pub struct Driver {
    /// Short name used in the CLI and for driver identification (e.g., `"GeoJSON"`).
    pub short_name: &'static str,
    /// Long descriptive name for display purposes (e.g., `"GeoJSON"`).
    pub long_name: &'static str,
    /// Operations supported by this driver (info, read, write).
    pub capabilities: DriverCapabilities,
}

impl Driver {
    /// Creates a new driver definition with specified capabilities.
    #[must_use]
    pub const fn new(
        short_name: &'static str,
        long_name: &'static str,
        info: SupportStatus,
        read: SupportStatus,
        write: SupportStatus,
    ) -> Self {
        Self {
            short_name,
            long_name,
            capabilities: DriverCapabilities { info, read, write },
        }
    }
}
