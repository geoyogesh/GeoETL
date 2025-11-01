//! Core library for the `GeoETL` project, providing fundamental functionalities for geospatial data processing.
//!
//! This crate includes:
//! - **Driver Registry**: A static registry of supported geospatial data formats and their capabilities.
//! - **Data Structures**: Core data structures for representing geospatial features and geometries (planned).
//! - **ETL Operations**: Core Extract, Transform, Load operations (planned).
//!
//! The [`drivers`] module exposes the static driver registry consumed by the CLI and other parts of the system.
//!
//! # Examples
//!
//! ```
//! use geoetl_core::drivers::{get_available_drivers, find_driver};
//!
//! // Get all drivers with at least one supported operation
//! let drivers = get_available_drivers();
//! println!("Found {} supported drivers", drivers.len());
//!
//! // Find a specific driver by name
//! if let Some(driver) = find_driver("GeoJSON") {
//!     println!("Driver: {}", driver.short_name);
//! }
//! ```

pub mod drivers;
pub mod init;
pub mod operations;
pub mod types;
pub mod utils;
