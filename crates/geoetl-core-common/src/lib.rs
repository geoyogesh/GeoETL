//! Common types and traits shared across `GeoETL` crates.
//!
//! This crate provides the core abstractions that are shared between
//! `geoetl-core` and format implementation crates, preventing circular dependencies.

pub mod drivers;
pub mod factory;
pub mod io;

// Re-export commonly used types
pub use drivers::{Driver, DriverCapabilities, SupportStatus};
pub use factory::{DriverRegistry, FormatFactory, FormatOptions, driver_registry};
pub use io::{DataReader, DataWriter};
