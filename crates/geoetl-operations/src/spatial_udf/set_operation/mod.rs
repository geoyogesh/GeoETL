//! Set-theoretic geometry operations
//!
//! These functions compute set-theoretic operations on geometries.

mod st_difference;
mod st_intersection;
mod st_sym_difference;
mod st_union;

pub use st_difference::create_st_difference_udf;
pub use st_intersection::create_st_intersection_udf;
pub use st_sym_difference::create_st_sym_difference_udf;
pub use st_union::create_st_union_udf;
