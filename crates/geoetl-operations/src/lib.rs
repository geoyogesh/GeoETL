//! `GeoETL` Operations - Spatial operations and UDFs for `DataFusion`
//!
//! This crate provides spatial operations (like `ST_Distance`, `ST_Buffer`, etc.)
//! as User Defined Functions (UDFs) that can be used in `DataFusion` SQL queries.

mod spatial_udf;

pub use spatial_udf::{create_st_distance_udf, register_spatial_udfs};
