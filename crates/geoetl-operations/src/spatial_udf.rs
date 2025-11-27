//! Spatial User Defined Functions (UDFs) for `DataFusion`
//!
//! This module provides spatial operations using GEOS for use in SQL queries.

#![allow(clippy::must_use_candidate)]

use anyhow::Result;
use datafusion::prelude::SessionContext;

mod st_distance;

pub use st_distance::create_st_distance_udf;

/// Register all spatial UDFs with the `DataFusion` `SessionContext`
///
/// This function registers all available spatial functions so they can be used
/// in SQL queries executed through `DataFusion`.
///
/// # Errors
///
/// Currently this function does not return errors, but the `Result` return type
/// allows for future error handling when registering UDFs.
///
/// # Example
///
/// ```no_run
/// use datafusion::prelude::SessionContext;
/// use geoetl_operations::register_spatial_udfs;
///
/// let ctx = SessionContext::new();
/// register_spatial_udfs(&ctx).unwrap();
///
/// // Now you can use spatial functions in SQL queries
/// // ctx.sql("SELECT ST_Distance(geom1, geom2) FROM table").await;
/// ```
pub fn register_spatial_udfs(ctx: &SessionContext) -> Result<()> {
    // Register ST_Distance
    let st_distance_udf = create_st_distance_udf();
    ctx.register_udf(st_distance_udf);

    Ok(())
}
