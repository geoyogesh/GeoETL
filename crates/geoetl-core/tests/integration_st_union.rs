//! End-to-end integration tests for `ST_Union` spatial UDF
//!
//! These tests verify that the `ST_Union` function works correctly
//! in SQL queries during the convert operation.

#![allow(clippy::uninlined_format_args)]

use geoetl_core::drivers::find_driver;
use geoetl_core::operations::convert;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Helper to create a `GeoJSON` file with polygons and WKT for second geometry
fn create_test_polygon_pairs_geojson(path: &std::path::Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Polygon",
        "coordinates": [[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]
      }},
      "properties": {{
        "name": "Overlapping Pair",
        "id": 1,
        "geom2_wkt": "POLYGON((1 0, 3 0, 3 2, 1 2, 1 0))"
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Polygon",
        "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]
      }},
      "properties": {{
        "name": "Disjoint Pair",
        "id": 2,
        "geom2_wkt": "POLYGON((5 5, 6 5, 6 6, 5 6, 5 5))"
      }}
    }}
  ]
}}"#
    )?;
    Ok(())
}

#[tokio::test]
async fn test_st_union_overlapping_area() {
    // Test that union of overlapping polygons has correct area
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("union_areas.csv");

    create_test_polygon_pairs_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Union and calculate area
    let sql_query = "
        SELECT
            name,
            id,
            ST_Area(geometry) as geom1_area,
            ST_Area(ST_GeomFromText(geom2_wkt)) as geom2_area,
            ST_Area(ST_Union(geometry, ST_GeomFromText(geom2_wkt))) as union_area
        FROM polygons
        WHERE id = 1
    ";

    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &csv_driver,
        "geometry",
        None,
        Some(sql_query),
        Some("polygons"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Union areas:\n{}", output);

    // First polygon (0,0)-(2,2) area = 4
    // Second polygon (1,0)-(3,2) area = 4
    // Overlap (1,0)-(2,2) area = 2
    // Union area = 4 + 4 - 2 = 6
    let lines: Vec<&str> = output.lines().collect();
    let parts: Vec<&str> = lines[1].split(',').collect();
    let union_area: f64 = parts[4].parse().expect("Union area should be a number");

    assert!(
        (union_area - 6.0).abs() < 0.001,
        "Union area should be 6.0, got {}",
        union_area
    );
}

#[tokio::test]
async fn test_st_union_disjoint_area() {
    // Test that union of disjoint polygons has sum of areas
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("disjoint_union_areas.csv");

    create_test_polygon_pairs_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Union disjoint polygons
    let sql_query = "
        SELECT
            name,
            ST_Area(geometry) as geom1_area,
            ST_Area(ST_GeomFromText(geom2_wkt)) as geom2_area,
            ST_Area(ST_Union(geometry, ST_GeomFromText(geom2_wkt))) as union_area
        FROM polygons
        WHERE id = 2
    ";

    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &csv_driver,
        "geometry",
        None,
        Some(sql_query),
        Some("polygons"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Disjoint union areas:\n{}", output);

    // First polygon (0,0)-(1,1) area = 1
    // Second polygon (5,5)-(6,6) area = 1
    // Union area = 1 + 1 = 2 (disjoint, creates MultiPolygon)
    let lines: Vec<&str> = output.lines().collect();
    let parts: Vec<&str> = lines[1].split(',').collect();
    let geom1_area: f64 = parts[1].parse().expect("Area should be a number");
    let geom2_area: f64 = parts[2].parse().expect("Area should be a number");
    let union_area: f64 = parts[3].parse().expect("Union area should be a number");

    assert!(
        (union_area - (geom1_area + geom2_area)).abs() < 0.001,
        "Disjoint union area should be sum of individual areas: {} + {} = {}, got {}",
        geom1_area,
        geom2_area,
        geom1_area + geom2_area,
        union_area
    );
}

#[tokio::test]
async fn test_st_union_with_buffer() {
    // Test union of two buffered points
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("point.geojson");
    let output_path = temp_dir.path().join("buffer_union.csv");

    let mut file = File::create(&input_path).unwrap();
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{ "type": "Point", "coordinates": [0.0, 0.0] }},
      "properties": {{ "name": "Center Point" }}
    }}
  ]
}}"#
    )
    .unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Create two concentric buffers and union them
    // Union of concentric circles = larger circle
    let sql_query = "
        SELECT
            name,
            ST_Area(ST_Buffer(geometry, 1.0)) as small_buffer_area,
            ST_Area(ST_Buffer(geometry, 2.0)) as large_buffer_area,
            ST_Area(ST_Union(ST_Buffer(geometry, 1.0), ST_Buffer(geometry, 2.0))) as union_area
        FROM point
    ";

    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &csv_driver,
        "geometry",
        None,
        Some(sql_query),
        Some("point"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Buffer union:\n{}", output);

    let lines: Vec<&str> = output.lines().collect();
    let parts: Vec<&str> = lines[1].split(',').collect();
    let large_buffer_area: f64 = parts[2].parse().expect("Area should be a number");
    let union_area: f64 = parts[3].parse().expect("Union area should be a number");

    // Union of concentric circles equals the larger circle
    assert!(
        (union_area - large_buffer_area).abs() < 0.1,
        "Union of concentric buffers should equal larger buffer: expected {}, got {}",
        large_buffer_area,
        union_area
    );
}

#[tokio::test]
async fn test_st_union_multiple_rows() {
    // Test union across multiple rows
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("multi_union.csv");

    create_test_polygon_pairs_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Calculate union for all rows
    let sql_query = "
        SELECT
            name,
            id,
            ST_Area(ST_Union(geometry, ST_GeomFromText(geom2_wkt))) as union_area
        FROM polygons
        ORDER BY id
    ";

    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &csv_driver,
        "geometry",
        None,
        Some(sql_query),
        Some("polygons"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Multi union:\n{}", output);

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 3, "Should have header + 2 data rows");

    // First row: overlapping, union = 6
    let parts1: Vec<&str> = lines[1].split(',').collect();
    let area1: f64 = parts1[2].parse().expect("Area should be a number");
    assert!(
        (area1 - 6.0).abs() < 0.001,
        "First union area should be 6.0, got {}",
        area1
    );

    // Second row: disjoint, union = 2
    let parts2: Vec<&str> = lines[2].split(',').collect();
    let area2: f64 = parts2[2].parse().expect("Area should be a number");
    assert!(
        (area2 - 2.0).abs() < 0.001,
        "Second union area should be 2.0, got {}",
        area2
    );
}
