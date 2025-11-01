//! Initialization module for registering format drivers.
//!
//! This module handles the one-time registration of all supported format drivers
//! with the global driver registry. It should be called once at application startup.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initializes the `GeoETL` core library by registering all format drivers.
///
/// This function registers all built-in format drivers (`CSV`, `GeoJSON`, etc.)
/// with the global driver registry. It uses `Once` to ensure registration
/// happens only once, even if called multiple times.
///
/// # Examples
///
/// ```
/// use geoetl_core::init::initialize;
///
/// // Call this once at application startup
/// initialize();
/// ```
///
/// # Thread Safety
///
/// This function is thread-safe and can be called from multiple threads.
/// Only the first call will perform the actual initialization.
pub fn initialize() {
    INIT.call_once(|| {
        // Register all format drivers
        datafusion_csv::register_csv_format();
        datafusion_geojson::register_geojson_format();
    });
}
