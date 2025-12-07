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
//! - [`create_st_overlaps_udf`]: Test if two geometries overlap
//! - [`create_st_touches_udf`]: Test if geometries touch (share boundary but not interior)
//! - [`create_st_crosses_udf`]: Test if geometries cross
//! - [`create_st_disjoint_udf`]: Test if geometries are disjoint (no intersection)
//! - [`create_st_equals_udf`]: Test if geometries are spatially equal
//! - [`create_st_covers_udf`]: Test if geometry A covers geometry B
//! - [`create_st_covered_by_udf`]: Test if geometry A is covered by geometry B
//!
//! ## Unary Validators
//! These functions test geometric properties:
//! - [`create_st_is_valid_udf`]: Test if geometry is valid according to OGC rules
//! - [`create_st_is_empty_udf`]: Test if geometry is empty (has no points)
//! - [`create_st_is_simple_udf`]: Test if geometry is simple (no self-intersections)
//! - [`create_st_is_closed_udf`]: Test if geometry is closed (start equals end)
//! - [`create_st_is_ring_udf`]: Test if geometry is a ring (closed and simple)
//!
//! ## Geometry Generators
//! These functions create new geometries from existing ones:
//! - [`create_st_envelope_udf`]: Compute bounding box (envelope) of geometry
//! - [`create_st_convex_hull_udf`]: Compute convex hull of geometry
//! - [`create_st_boundary_udf`]: Compute boundary of geometry
//! - [`create_st_point_on_surface_udf`]: Get a point guaranteed on the surface
//! - [`create_st_simplify_udf`]: Simplify geometry using Douglas-Peucker
//! - [`create_st_simplify_preserve_topology_udf`]: Simplify preserving topology
//!
//! ## Set Operations
//! These functions compute set-theoretic operations on geometries:
//! - [`create_st_difference_udf`]: Compute difference of two geometries (A - B)
//! - [`create_st_sym_difference_udf`]: Compute symmetric difference (XOR)
//!
//! ## Accessors
//! These functions extract information from geometries:
//! - [`create_st_x_udf`]: Get X coordinate of a Point
//! - [`create_st_y_udf`]: Get Y coordinate of a Point
//! - [`create_st_num_points_udf`]: Count of points in geometry
//! - [`create_st_num_geometries_udf`]: Count of geometries in collection
//! - [`create_st_geometry_type_udf`]: Get geometry type as string
//! - [`create_st_dimension_udf`]: Get topological dimension
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
mod st_boundary;
mod st_buffer;
mod st_centroid;
mod st_contains;
mod st_convex_hull;
mod st_covered_by;
mod st_covers;
mod st_crosses;
mod st_difference;
mod st_dimension;
mod st_disjoint;
mod st_distance;
mod st_envelope;
mod st_equals;
mod st_geometry_type;
mod st_geomfromtext;
mod st_geomfromwkb;
mod st_intersection;
mod st_intersects;
mod st_is_closed;
mod st_is_empty;
mod st_is_ring;
mod st_is_simple;
mod st_is_valid;
mod st_length;
mod st_num_geometries;
mod st_num_points;
mod st_overlaps;
mod st_point;
mod st_point_on_surface;
mod st_simplify;
mod st_simplify_preserve_topology;
mod st_sym_difference;
mod st_touches;
mod st_union;
mod st_within;
mod st_x;
mod st_y;

pub use st_area::create_st_area_udf;
pub use st_boundary::create_st_boundary_udf;
pub use st_buffer::create_st_buffer_udf;
pub use st_centroid::create_st_centroid_udf;
pub use st_contains::create_st_contains_udf;
pub use st_convex_hull::create_st_convex_hull_udf;
pub use st_covered_by::create_st_covered_by_udf;
pub use st_covers::create_st_covers_udf;
pub use st_crosses::create_st_crosses_udf;
pub use st_difference::create_st_difference_udf;
pub use st_dimension::create_st_dimension_udf;
pub use st_disjoint::create_st_disjoint_udf;
pub use st_distance::create_st_distance_udf;
pub use st_envelope::create_st_envelope_udf;
pub use st_equals::create_st_equals_udf;
pub use st_geometry_type::create_st_geometry_type_udf;
pub use st_geomfromtext::create_st_geomfromtext_udf;
pub use st_geomfromwkb::create_st_geomfromwkb_udf;
pub use st_intersection::create_st_intersection_udf;
pub use st_intersects::create_st_intersects_udf;
pub use st_is_closed::create_st_is_closed_udf;
pub use st_is_empty::create_st_is_empty_udf;
pub use st_is_ring::create_st_is_ring_udf;
pub use st_is_simple::create_st_is_simple_udf;
pub use st_is_valid::create_st_is_valid_udf;
pub use st_length::create_st_length_udf;
pub use st_num_geometries::create_st_num_geometries_udf;
pub use st_num_points::create_st_num_points_udf;
pub use st_overlaps::create_st_overlaps_udf;
pub use st_point::{create_st_makepoint_udf, create_st_point_udf};
pub use st_point_on_surface::create_st_point_on_surface_udf;
pub use st_simplify::create_st_simplify_udf;
pub use st_simplify_preserve_topology::create_st_simplify_preserve_topology_udf;
pub use st_sym_difference::create_st_sym_difference_udf;
pub use st_touches::create_st_touches_udf;
pub use st_union::create_st_union_udf;
pub use st_within::create_st_within_udf;
pub use st_x::create_st_x_udf;
pub use st_y::create_st_y_udf;

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
/// - `ST_Overlaps(geom1, geom2)` - Test if geometries overlap
/// - `ST_Touches(geom1, geom2)` - Test if geometries touch
/// - `ST_Crosses(geom1, geom2)` - Test if geometries cross
/// - `ST_Disjoint(geom1, geom2)` - Test if geometries are disjoint
/// - `ST_Equals(geom1, geom2)` - Test if geometries are spatially equal
/// - `ST_Covers(geom_a, geom_b)` - Test if geometry A covers geometry B
/// - `ST_CoveredBy(geom_a, geom_b)` - Test if geometry A is covered by geometry B
///
/// ## Unary Validators
/// - `ST_IsValid(geom)` - Test if geometry is valid according to OGC rules
/// - `ST_IsEmpty(geom)` - Test if geometry is empty
/// - `ST_IsSimple(geom)` - Test if geometry is simple
/// - `ST_IsClosed(geom)` - Test if geometry is closed
/// - `ST_IsRing(geom)` - Test if geometry is a ring
///
/// ## Geometry Generators
/// - `ST_Envelope(geom)` - Bounding box of geometry
/// - `ST_ConvexHull(geom)` - Convex hull of geometry
/// - `ST_Boundary(geom)` - Boundary of geometry
/// - `ST_PointOnSurface(geom)` - Point guaranteed on surface
/// - `ST_Simplify(geom, tolerance)` - Douglas-Peucker simplification
/// - `ST_SimplifyPreserveTopology(geom, tolerance)` - Topology-preserving simplification
///
/// ## Set Operations
/// - `ST_Difference(geom1, geom2)` - Difference of geometries (A - B)
/// - `ST_SymDifference(geom1, geom2)` - Symmetric difference (XOR)
///
/// ## Accessors
/// - `ST_X(geom)` - Get X coordinate of a Point
/// - `ST_Y(geom)` - Get Y coordinate of a Point
/// - `ST_NumPoints(geom)` - Count of points in geometry
/// - `ST_NumGeometries(geom)` - Count of geometries in collection
/// - `ST_GeometryType(geom)` - Get geometry type as string
/// - `ST_Dimension(geom)` - Get topological dimension
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
    ctx.register_udf(create_st_overlaps_udf());
    ctx.register_udf(create_st_touches_udf());
    ctx.register_udf(create_st_crosses_udf());
    ctx.register_udf(create_st_disjoint_udf());
    ctx.register_udf(create_st_equals_udf());
    ctx.register_udf(create_st_covers_udf());
    ctx.register_udf(create_st_covered_by_udf());

    // Register unary validators
    ctx.register_udf(create_st_is_valid_udf());
    ctx.register_udf(create_st_is_empty_udf());
    ctx.register_udf(create_st_is_simple_udf());
    ctx.register_udf(create_st_is_closed_udf());
    ctx.register_udf(create_st_is_ring_udf());

    // Register geometry generators
    ctx.register_udf(create_st_envelope_udf());
    ctx.register_udf(create_st_convex_hull_udf());
    ctx.register_udf(create_st_boundary_udf());
    ctx.register_udf(create_st_point_on_surface_udf());
    ctx.register_udf(create_st_simplify_udf());
    ctx.register_udf(create_st_simplify_preserve_topology_udf());

    // Register set operations
    ctx.register_udf(create_st_difference_udf());
    ctx.register_udf(create_st_sym_difference_udf());

    // Register accessors
    ctx.register_udf(create_st_x_udf());
    ctx.register_udf(create_st_y_udf());
    ctx.register_udf(create_st_num_points_udf());
    ctx.register_udf(create_st_num_geometries_udf());
    ctx.register_udf(create_st_geometry_type_udf());
    ctx.register_udf(create_st_dimension_udf());

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
        assert!(udfs.contains_key("st_overlaps"));
        assert!(udfs.contains_key("st_touches"));
        assert!(udfs.contains_key("st_crosses"));
        assert!(udfs.contains_key("st_disjoint"));
        assert!(udfs.contains_key("st_equals"));
        assert!(udfs.contains_key("st_covers"));
        assert!(udfs.contains_key("st_coveredby"));

        // Unary validators
        assert!(udfs.contains_key("st_isvalid"));
        assert!(udfs.contains_key("st_isempty"));
        assert!(udfs.contains_key("st_issimple"));
        assert!(udfs.contains_key("st_isclosed"));
        assert!(udfs.contains_key("st_isring"));

        // Geometry generators
        assert!(udfs.contains_key("st_envelope"));
        assert!(udfs.contains_key("st_convexhull"));
        assert!(udfs.contains_key("st_boundary"));
        assert!(udfs.contains_key("st_pointonsurface"));
        assert!(udfs.contains_key("st_simplify"));
        assert!(udfs.contains_key("st_simplifypreservetopology"));

        // Set operations
        assert!(udfs.contains_key("st_difference"));
        assert!(udfs.contains_key("st_symdifference"));

        // Accessors
        assert!(udfs.contains_key("st_x"));
        assert!(udfs.contains_key("st_y"));
        assert!(udfs.contains_key("st_numpoints"));
        assert!(udfs.contains_key("st_numgeometries"));
        assert!(udfs.contains_key("st_geometrytype"));
        assert!(udfs.contains_key("st_dimension"));
    }
}
