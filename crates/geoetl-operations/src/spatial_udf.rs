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
//! ## Spatial Measurements
//! These functions calculate properties of geometries:
//! - [`create_st_distance_udf`]: Calculate minimum distance between geometries
//! - [`create_st_area_udf`]: Calculate area of a geometry
//! - [`create_st_length_udf`]: Calculate length/perimeter of a geometry
//!
//! ## Spatial Operations
//! These functions transform or combine geometries:
//! - [`create_st_buffer_udf`]: Create buffer polygon around geometry
//! - [`create_st_centroid_udf`]: Calculate centroid point of geometry
//! - [`create_st_union_udf`]: Combine two geometries
//! - [`create_st_intersection_udf`]: Intersection of two geometries
//!
//! ## Spatial Predicates
//! These functions test spatial relationships between geometries:
//! - [`create_st_intersects_udf`]: Test if two geometries intersect
//! - [`create_st_contains_udf`]: Test if geometry A contains geometry B
//! - [`create_st_within_udf`]: Test if geometry A is within geometry B
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
mod geos_helpers;
mod st_area;
mod st_buffer;
mod st_centroid;
mod st_contains;
mod st_distance;
mod st_geomfromtext;
mod st_geomfromwkb;
mod st_intersection;
mod st_intersects;
mod st_length;
mod st_point;
mod st_union;
mod st_within;

pub use st_area::create_st_area_udf;
pub use st_buffer::create_st_buffer_udf;
pub use st_centroid::create_st_centroid_udf;
pub use st_contains::create_st_contains_udf;
pub use st_distance::create_st_distance_udf;
pub use st_geomfromtext::create_st_geomfromtext_udf;
pub use st_geomfromwkb::create_st_geomfromwkb_udf;
pub use st_intersection::create_st_intersection_udf;
pub use st_intersects::create_st_intersects_udf;
pub use st_length::create_st_length_udf;
pub use st_point::{create_st_makepoint_udf, create_st_point_udf};
pub use st_union::create_st_union_udf;
pub use st_within::create_st_within_udf;

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
/// ## Spatial Measurements
/// - `ST_Distance(geom1, geom2)` - Minimum distance between geometries
/// - `ST_Area(geom)` - Area of a geometry
/// - `ST_Length(geom)` - Length/perimeter of a geometry
///
/// ## Spatial Operations
/// - `ST_Buffer(geom, distance)` - Buffer polygon around geometry
/// - `ST_Centroid(geom)` - Centroid point of geometry
/// - `ST_Union(geom1, geom2)` - Combine two geometries
/// - `ST_Intersection(geom1, geom2)` - Intersection of two geometries
///
/// ## Spatial Predicates
/// - `ST_Intersects(geom1, geom2)` - Test if geometries intersect
/// - `ST_Contains(geom_a, geom_b)` - Test if geometry A contains geometry B
/// - `ST_Within(geom_a, geom_b)` - Test if geometry A is within geometry B
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

    // Register spatial measurements
    ctx.register_udf(create_st_distance_udf());
    ctx.register_udf(create_st_area_udf());
    ctx.register_udf(create_st_length_udf());

    // Register spatial operations
    ctx.register_udf(create_st_buffer_udf());
    ctx.register_udf(create_st_centroid_udf());
    ctx.register_udf(create_st_union_udf());
    ctx.register_udf(create_st_intersection_udf());

    // Register spatial predicates
    ctx.register_udf(create_st_intersects_udf());
    ctx.register_udf(create_st_contains_udf());
    ctx.register_udf(create_st_within_udf());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_spatial_udfs() {
        let ctx = SessionContext::new();
        let result = register_spatial_udfs(&ctx);
        assert!(result.is_ok());

        // Verify all UDFs are registered
        let state = ctx.state();
        let udfs = state.scalar_functions();

        // Construction functions
        assert!(udfs.contains_key("st_geomfromtext"));
        assert!(udfs.contains_key("st_geomfromwkb"));
        assert!(udfs.contains_key("st_point"));
        assert!(udfs.contains_key("st_makepoint"));

        // Spatial measurements
        assert!(udfs.contains_key("st_distance"));
        assert!(udfs.contains_key("st_area"));
        assert!(udfs.contains_key("st_length"));

        // Spatial operations
        assert!(udfs.contains_key("st_buffer"));
        assert!(udfs.contains_key("st_centroid"));
        assert!(udfs.contains_key("st_union"));
        assert!(udfs.contains_key("st_intersection"));

        // Spatial predicates
        assert!(udfs.contains_key("st_intersects"));
        assert!(udfs.contains_key("st_contains"));
        assert!(udfs.contains_key("st_within"));
    }
}
