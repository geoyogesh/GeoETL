use datafusion::prelude::*;
use datafusion_common::{DataFusionError, Result};
use datafusion_geojson::{GeoJsonFormatOptions, SessionContextGeoJsonExt};
use geo_traits::GeometryTrait;
use geo_traits::{CoordTrait, PointTrait};
use geoarrow_array::array::GeometryArray;
use geoarrow_array::{GeoArrowArray, GeoArrowArrayAccessor};
use std::convert::TryFrom;

/// Test reading a `GeoJSON` file with point geometries
#[tokio::test]
async fn test_read_cities_geojson() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_geojson_file("cities", "tests/e2e_data/natural-earth_cities.geojson")
        .await?;

    let df = ctx
        .sql(r"SELECT name, geometry FROM cities LIMIT 5")
        .await?;

    let batches = df.collect().await?;
    assert!(!batches.is_empty(), "Should have at least one batch");

    let batch = &batches[0];
    assert_eq!(batch.num_rows(), 5);

    // Verify we have both property and geometry columns
    let schema = batch.schema();
    assert!(schema.field_with_name("name").is_ok());
    assert!(schema.field_with_name("geometry").is_ok());

    Ok(())
}

/// Test querying `GeoJSON` data with filtering
#[tokio::test]
async fn test_query_cities_with_filter() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_geojson_file("cities", "tests/e2e_data/natural-earth_cities.geojson")
        .await?;

    let df = ctx
        .sql(r"SELECT name FROM cities WHERE name = 'Monaco' LIMIT 1")
        .await?;

    let batches = df.collect().await?;
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].num_rows(), 1);

    Ok(())
}

/// Test reading `GeoJSON` and extracting `GeoArrow` point geometries
#[tokio::test]
async fn test_cities_to_geoarrow_points() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_geojson_file("cities", "tests/e2e_data/natural-earth_cities.geojson")
        .await?;

    let df = ctx
        .sql(r"SELECT geometry, name FROM cities WHERE name = 'Vatican City' LIMIT 1")
        .await?;

    let batches = df.collect().await?;
    assert_eq!(batches.len(), 1);

    let batch = &batches[0];
    let schema = batch.schema();
    let field = schema
        .field_with_name("geometry")
        .expect("geometry field to exist");
    let column = batch.column(schema.index_of("geometry").unwrap()).clone();

    let geometry_array = GeometryArray::try_from((column.as_ref(), field))
        .map_err(|err| DataFusionError::Execution(format!("Failed to decode geometry: {err}")))?;

    assert_eq!(geometry_array.len(), 1);

    let first_geom = geometry_array
        .value(0)
        .map_err(|err| DataFusionError::Execution(err.to_string()))?;

    // Extract point from geometry using geo-traits
    let geo_traits::GeometryType::Point(point) = first_geom.as_type() else {
        panic!("Expected point geometry")
    };
    let coord = point.coord().expect("point should have coordinates");

    // Vatican City coordinates from the test data
    assert!((coord.x() - 12.453_386_5).abs() < 1e-6);
    assert!((coord.y() - 41.903_282_2).abs() < 1e-6);

    Ok(())
}

/// Test reading `GeoJSON` with custom geometry column name
#[tokio::test]
async fn test_custom_geometry_column() -> Result<()> {
    let ctx = SessionContext::new();

    let options = GeoJsonFormatOptions::default().with_geometry_column_name("geom");

    ctx.register_geojson_with_options(
        "cities",
        "tests/e2e_data/natural-earth_cities.geojson",
        options,
    )
    .await?;

    let df = ctx.sql(r"SELECT name, geom FROM cities LIMIT 1").await?;

    let batches = df.collect().await?;
    assert_eq!(batches.len(), 1);

    let batch = &batches[0];
    let schema = batch.schema();

    // Should have 'geom' instead of 'geometry'
    assert!(schema.field_with_name("geom").is_ok());
    assert!(schema.field_with_name("geometry").is_err());

    Ok(())
}

/// Test reading `GeoJSON` with schema inference limit
#[tokio::test]
async fn test_schema_inference_limit() -> Result<()> {
    let ctx = SessionContext::new();

    let options = GeoJsonFormatOptions::default().with_schema_infer_max_features(Some(10));

    ctx.register_geojson_with_options(
        "cities",
        "tests/e2e_data/natural-earth_cities.geojson",
        options,
    )
    .await?;

    let df = ctx.sql(r"SELECT * FROM cities").await?;

    let batches = df.collect().await?;
    assert!(!batches.is_empty());

    Ok(())
}

/// Test reading `GeoJSON` with batch size option
#[tokio::test]
async fn test_custom_batch_size() -> Result<()> {
    let ctx = SessionContext::new();

    let options = GeoJsonFormatOptions::default().with_batch_size(10);

    ctx.register_geojson_with_options(
        "cities",
        "tests/e2e_data/natural-earth_cities.geojson",
        options,
    )
    .await?;

    let df = ctx.sql(r"SELECT name FROM cities").await?;

    let batches = df.collect().await?;
    assert!(!batches.is_empty());

    // With batch_size=10, we should have multiple batches if there are >10 rows
    let total_rows: usize = batches
        .iter()
        .map(datafusion::arrow::array::RecordBatch::num_rows)
        .sum();
    assert!(total_rows > 0);

    Ok(())
}

/// Test reading `GeoJSON` file with `read_geojson_file` convenience method
#[tokio::test]
async fn test_read_geojson_file() -> Result<()> {
    let ctx = SessionContext::new();

    let df = ctx
        .read_geojson_file("tests/e2e_data/natural-earth_cities.geojson")
        .await?;

    let batches = df.collect().await?;
    assert!(!batches.is_empty());

    let batch = &batches[0];
    assert!(batch.num_rows() > 0);

    Ok(())
}

/// Test querying multiple `GeoJSON` files
#[tokio::test]
async fn test_multiple_geojson_files() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_geojson_file("cities", "tests/e2e_data/natural-earth_cities.geojson")
        .await?;

    ctx.register_geojson_file(
        "countries_bounds",
        "tests/e2e_data/natural-earth_countries-bounds.geojson",
    )
    .await?;

    // Query first file
    let df1 = ctx.sql(r"SELECT COUNT(*) as count FROM cities").await?;
    let batches1 = df1.collect().await?;
    assert_eq!(batches1.len(), 1);

    // Query second file
    let df2 = ctx
        .sql(r"SELECT COUNT(*) as count FROM countries_bounds")
        .await?;
    let batches2 = df2.collect().await?;
    assert_eq!(batches2.len(), 1);

    Ok(())
}

/// Test reading `GeoJSON` with properties containing various data types
#[tokio::test]
async fn test_mixed_property_types() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_geojson_file(
        "countries",
        "tests/e2e_data/natural-earth_countries.geojson",
    )
    .await?;

    let df = ctx.sql(r"SELECT * FROM countries LIMIT 1").await?;

    let batches = df.collect().await?;
    assert!(!batches.is_empty());

    let batch = &batches[0];
    assert!(batch.num_rows() > 0);

    // Verify schema includes geometry column
    let schema = batch.schema();
    assert!(schema.field_with_name("geometry").is_ok());

    Ok(())
}

/// Test aggregation queries on `GeoJSON` data
#[tokio::test]
async fn test_aggregation_queries() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_geojson_file("cities", "tests/e2e_data/natural-earth_cities.geojson")
        .await?;

    // Test COUNT
    let df = ctx.sql(r"SELECT COUNT(*) as total FROM cities").await?;
    let batches = df.collect().await?;
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].num_rows(), 1);

    Ok(())
}

/// Test ordering results from `GeoJSON`
#[tokio::test]
async fn test_order_by() -> Result<()> {
    let ctx = SessionContext::new();

    ctx.register_geojson_file("cities", "tests/e2e_data/natural-earth_cities.geojson")
        .await?;

    let df = ctx
        .sql(r"SELECT name FROM cities ORDER BY name ASC LIMIT 5")
        .await?;

    let batches = df.collect().await?;
    assert!(!batches.is_empty());

    let batch = &batches[0];
    assert!(batch.num_rows() > 0);

    Ok(())
}

/// Test reading `GeoJSON` file with geometry but no properties
#[tokio::test]
async fn test_geometry_only() -> Result<()> {
    let ctx = SessionContext::new();

    // Create a temporary GeoJSON file with just geometries
    let temp_dir = tempfile::TempDir::new().unwrap();
    let path = temp_dir.path().join("points.geojson");

    std::fs::write(
        &path,
        r#"{
  "type": "FeatureCollection",
  "features": [
    {"type": "Feature", "geometry": {"type": "Point", "coordinates": [0.0, 1.0]}, "properties": {}},
    {"type": "Feature", "geometry": {"type": "Point", "coordinates": [2.0, 3.0]}, "properties": {}}
  ]
}
"#,
    )
    .unwrap();

    ctx.register_geojson_file("points", path.to_str().unwrap())
        .await?;

    let df = ctx.sql(r"SELECT geometry FROM points").await?;

    let batches = df.collect().await?;
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].num_rows(), 2);

    Ok(())
}

/// Test reading `GeoJSON` with null geometries
#[tokio::test]
async fn test_null_geometries() -> Result<()> {
    let ctx = SessionContext::new();

    // Create a temporary GeoJSON file with null geometries
    let temp_dir = tempfile::TempDir::new().unwrap();
    let path = temp_dir.path().join("mixed.geojson");

    std::fs::write(
        &path,
        r#"{
  "type": "FeatureCollection",
  "features": [
    {"type": "Feature", "geometry": {"type": "Point", "coordinates": [1.0, 2.0]}, "properties": {"name": "A"}},
    {"type": "Feature", "geometry": null, "properties": {"name": "B"}}
  ]
}
"#,
    )
    .unwrap();

    ctx.register_geojson_file("mixed", path.to_str().unwrap())
        .await?;

    let df = ctx.sql(r"SELECT name FROM mixed").await?;

    let batches = df.collect().await?;
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].num_rows(), 2);

    Ok(())
}
