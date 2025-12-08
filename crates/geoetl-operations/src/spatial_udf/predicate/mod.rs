//! Spatial predicate functions
//!
//! These functions test spatial relationships between geometries and return boolean results.

mod st_contains;
mod st_covered_by;
mod st_covers;
mod st_crosses;
mod st_disjoint;
mod st_equals;
mod st_intersects;
mod st_overlaps;
mod st_touches;
mod st_within;

pub use st_contains::create_st_contains_udf;
pub use st_covered_by::create_st_covered_by_udf;
pub use st_covers::create_st_covers_udf;
pub use st_crosses::create_st_crosses_udf;
pub use st_disjoint::create_st_disjoint_udf;
pub use st_equals::create_st_equals_udf;
pub use st_intersects::create_st_intersects_udf;
pub use st_overlaps::create_st_overlaps_udf;
pub use st_touches::create_st_touches_udf;
pub use st_within::create_st_within_udf;
