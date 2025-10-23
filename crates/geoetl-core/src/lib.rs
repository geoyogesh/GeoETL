//! `geoetl-core` is the core library for the `GeoETL` project, providing fundamental functionalities
//! for geospatial data processing.
//!
//! This crate includes:
//! - **Driver Registry**: A static registry of supported geospatial data formats and their capabilities.
//! - **Data Structures**: Core data structures for representing geospatial features and geometries (planned).
//! - **ETL Operations**: Core Extract, Transform, Load operations (planned).
//!
//! The `drivers` module exposes the static driver registry consumed by the CLI and other parts of the system.

pub mod drivers;
