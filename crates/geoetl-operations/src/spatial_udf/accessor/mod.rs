//! Geometry accessor functions
//!
//! These functions extract information from geometries.

mod st_dimension;
mod st_geometry_type;
mod st_num_geometries;
mod st_num_points;
mod st_x;
mod st_y;

pub use st_dimension::create_st_dimension_udf;
pub use st_geometry_type::create_st_geometry_type_udf;
pub use st_num_geometries::create_st_num_geometries_udf;
pub use st_num_points::create_st_num_points_udf;
pub use st_x::create_st_x_udf;
pub use st_y::create_st_y_udf;
