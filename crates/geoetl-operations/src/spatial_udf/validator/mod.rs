//! Geometry validation functions
//!
//! These functions test geometric properties and return boolean results.

mod st_is_closed;
mod st_is_empty;
mod st_is_ring;
mod st_is_simple;
mod st_is_valid;

pub use st_is_closed::create_st_is_closed_udf;
pub use st_is_empty::create_st_is_empty_udf;
pub use st_is_ring::create_st_is_ring_udf;
pub use st_is_simple::create_st_is_simple_udf;
pub use st_is_valid::create_st_is_valid_udf;
