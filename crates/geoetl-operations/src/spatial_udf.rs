//! Spatial User Defined Functions (UDFs) for `DataFusion`
//!
//! This module provides spatial operations using GEOS for use in SQL queries.
//!
//! # Architecture
//!
//! Spatial UDFs are organized into two categories:
//!
//! ## Construction Functions
//! These functions create geometries from various input formats:
//! - [`create_st_geomfromtext_udf`]: Parse WKT strings to geometry
//! - [`create_st_geomfromwkb_udf`]: Validate WKB binary data
//! - [`create_st_point_udf`]: Create point from X, Y coordinates
//! - [`create_st_makepoint_udf`]: Alias for `ST_Point`
//!
//! ## Spatial Operations
//! These functions operate on geometries (in WKB format):
//! - [`create_st_distance_udf`]: Calculate minimum distance between geometries
//!
//! # Example Usage
//!
//! ```sql
//! -- Parse WKT and calculate distance
//! SELECT ST_Distance(ST_GeomFromText('POINT(0 0)'), ST_GeomFromText('POINT(3 4)'));
//!
//! -- Create points from coordinates and filter by distance
//! SELECT * FROM cities
//! WHERE ST_Distance(ST_Point(lon, lat), ST_Point(-122.4, 37.8)) < 0.1;
//! ```

#![allow(clippy::must_use_candidate)]

use anyhow::Result;
use datafusion::prelude::SessionContext;

mod geoarrow_types;
mod st_distance;
mod st_geomfromtext;
mod st_geomfromwkb;
mod st_point;

pub use st_distance::create_st_distance_udf;
pub use st_geomfromtext::create_st_geomfromtext_udf;
pub use st_geomfromwkb::create_st_geomfromwkb_udf;
pub use st_point::{create_st_makepoint_udf, create_st_point_udf};

/// Register all spatial UDFs with the `DataFusion` `SessionContext`
///
/// This function registers all available spatial functions so they can be used
/// in SQL queries executed through `DataFusion`.
///
/// # Registered Functions
///
/// ## Construction Functions
/// - `ST_GeomFromText(wkt)` - Parse WKT string to geometry
/// - `ST_GeomFromWKB(wkb)` - Validate and pass through WKB
/// - `ST_Point(x, y)` - Create point from coordinates
/// - `ST_MakePoint(x, y)` - Alias for `ST_Point`
///
/// ## Spatial Operations
/// - `ST_Distance(geom1, geom2)` - Minimum distance between geometries
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
/// // ctx.sql("SELECT ST_Distance(ST_Point(0, 0), ST_Point(3, 4))").await;
/// ```
pub fn register_spatial_udfs(ctx: &SessionContext) -> Result<()> {
    // Register construction functions
    ctx.register_udf(create_st_geomfromtext_udf());
    ctx.register_udf(create_st_geomfromwkb_udf());
    ctx.register_udf(create_st_point_udf());
    ctx.register_udf(create_st_makepoint_udf());

    // Register spatial operations
    ctx.register_udf(create_st_distance_udf());

    Ok(())
}
