//! Driver registry for geospatial data format support and capabilities.
//!
//! This module provides a static registry of geospatial data format drivers, including
//! their current support status (supported, planned, or not supported) for various operations
//! (info, read, write). The registry is modeled after GDAL's driver system but designed for
//! modern Rust-based ETL workflows.
//!
//! # Examples
//!
//! ```
//! use geoetl_core::init::initialize;
//! use geoetl_core::drivers::{find_driver, get_available_drivers};
//!
//! // Initialize the driver registry
//! initialize();
//!
//! // Find a specific driver
//! let geojson = find_driver("GeoJSON").expect("GeoJSON driver should exist");
//! assert!(geojson.capabilities.read.is_supported());
//!
//! // List all drivers with supported operations
//! let available = get_available_drivers();
//! for driver in available {
//!     println!("{}: {}", driver.short_name, driver.long_name);
//! }
//! ```

// Re-export types from geoetl-core-common to maintain public API
pub use geoetl_core_common::{Driver, DriverCapabilities, SupportStatus};

/// Returns all registered vector format drivers.
///
/// This function queries the driver registry and returns all drivers that have been
/// registered via format factories. Each driver includes its short name, long name,
/// and capabilities for info, read, and write operations.
///
/// # Examples
///
/// ```
/// use geoetl_core::drivers::get_drivers;
///
/// let all_drivers = get_drivers();
/// println!("Total drivers in registry: {}", all_drivers.len());
///
/// // Find drivers with specific characteristics
/// let read_capable = all_drivers.iter()
///     .filter(|d| d.capabilities.read.is_supported())
///     .count();
/// println!("Drivers with read support: {}", read_capable);
/// ```
#[must_use]
pub fn get_drivers() -> Vec<Driver> {
    geoetl_core_common::driver_registry().get_available_drivers()
}

/// Returns all drivers that have at least one fully supported operation.
///
/// This filters the driver registry to include only drivers where at least one
/// operation (info, read, or write) has [`SupportStatus::Supported`]. Drivers with
/// only planned or unsupported operations are excluded.
///
/// # Examples
///
/// ```
/// use geoetl_core::drivers::get_available_drivers;
///
/// let available = get_available_drivers();
/// for driver in available {
///     println!("{} is ready to use", driver.short_name);
/// }
/// ```
#[must_use]
pub fn get_available_drivers() -> Vec<Driver> {
    get_drivers()
        .into_iter()
        .filter(|d| d.capabilities.has_supported_operation())
        .collect()
}

/// Finds a driver by its short name (case-insensitive).
///
/// Returns `None` if no driver with the given name exists in the registry.
///
/// # Examples
///
/// ```
/// use geoetl_core::init::initialize;
/// use geoetl_core::drivers::find_driver;
///
/// // Initialize the driver registry
/// initialize();
///
/// // Case-insensitive lookup
/// let driver = find_driver("geojson").expect("GeoJSON should exist");
/// assert_eq!(driver.short_name, "GeoJSON");
///
/// // Non-existent driver
/// assert!(find_driver("InvalidDriver").is_none());
/// ```
#[must_use]
pub fn find_driver(name: &str) -> Option<Driver> {
    get_drivers()
        .into_iter()
        .find(|d| d.short_name.eq_ignore_ascii_case(name))
}

/// Lists all drivers that support specific capabilities.
///
/// Filters drivers based on whether they have full support ([`SupportStatus::Supported`])
/// for the requested operations. If a capability parameter is `false`, that operation
/// is not required; if `true`, the driver must support it.
///
/// # Arguments
///
/// * `read` - If `true`, only include drivers that support reading
/// * `write` - If `true`, only include drivers that support writing
/// * `info` - If `true`, only include drivers that support info operations
///
/// # Examples
///
/// ```
/// use geoetl_core::drivers::list_drivers_with_capability;
///
/// // Find drivers that support both read and write
/// let read_write_drivers = list_drivers_with_capability(true, true, false);
///
/// // Find drivers that support at least read (write optional)
/// let read_drivers = list_drivers_with_capability(true, false, false);
/// ```
#[must_use]
pub fn list_drivers_with_capability(read: bool, write: bool, info: bool) -> Vec<Driver> {
    get_drivers()
        .into_iter()
        .filter(|d| {
            let read_ok = !read || d.capabilities.read.is_supported();
            let write_ok = !write || d.capabilities.write.is_supported();
            let info_ok = !info || d.capabilities.info.is_supported();
            read_ok && write_ok && info_ok
        })
        .collect()
}

/// Returns all driver short names in alphabetically sorted order.
///
/// This is useful for displaying driver options to users or for validation.
///
/// # Examples
///
/// ```
/// use geoetl_core::init::initialize;
/// use geoetl_core::drivers::get_driver_names;
///
/// // Initialize the driver registry
/// initialize();
///
/// let names = get_driver_names();
/// assert!(names.contains(&"GeoJSON"));
/// assert!(names.contains(&"CSV"));
///
/// // Names are sorted
/// let mut sorted = names.clone();
/// sorted.sort_unstable();
/// assert_eq!(names, sorted);
/// ```
#[must_use]
pub fn get_driver_names() -> Vec<&'static str> {
    let mut names: Vec<_> = get_drivers().iter().map(|d| d.short_name).collect();
    names.sort_unstable();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_driver() {
        crate::init::initialize();
        let driver = find_driver("GeoJSON");
        assert!(driver.is_some());
        assert_eq!(driver.unwrap().short_name, "GeoJSON");
    }

    #[test]
    fn test_find_driver_case_insensitive() {
        crate::init::initialize();
        let driver = find_driver("geojson");
        assert!(driver.is_some());
        assert_eq!(driver.unwrap().short_name, "GeoJSON");
    }

    #[test]
    fn test_list_read_write_drivers() {
        crate::init::initialize();
        let drivers = list_drivers_with_capability(true, true, false);
        // GeoJSON and CSV are supported
        assert_eq!(drivers.len(), 2);
        assert!(drivers.iter().any(|d| d.short_name == "GeoJSON"));
        assert!(drivers.iter().any(|d| d.short_name == "CSV"));
    }

    #[test]
    fn test_available_drivers() {
        crate::init::initialize();
        let drivers = get_available_drivers();
        // Should have drivers with at least one Supported operation
        assert_eq!(drivers.len(), 2);
        assert!(drivers.iter().any(|d| d.short_name == "GeoJSON"));
        assert!(drivers.iter().any(|d| d.short_name == "CSV"));
    }

    #[test]
    fn test_support_status() {
        assert!(SupportStatus::Supported.is_supported());
        assert!(!SupportStatus::NotSupported.is_supported());
        assert!(!SupportStatus::Planned.is_supported());

        assert!(SupportStatus::Supported.is_available());
        assert!(!SupportStatus::NotSupported.is_available());
        assert!(SupportStatus::Planned.is_available());
    }
}
