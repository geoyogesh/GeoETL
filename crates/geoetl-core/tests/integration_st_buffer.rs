//! End-to-end integration tests for `ST_Buffer` spatial UDF
//!
//! These tests verify that the `ST_Buffer` function works correctly
//! in SQL queries during the convert operation.

#![allow(clippy::uninlined_format_args)]

use geoetl_core::drivers::find_driver;
use geoetl_core::operations::convert;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Helper to create a `GeoJSON` file with point geometries
fn create_test_points_geojson(path: &std::path::Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{ "type": "Point", "coordinates": [0.0, 0.0] }},
      "properties": {{
        "name": "Origin",
        "id": 1,
        "buffer_dist": 1.0
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{ "type": "Point", "coordinates": [10.0, 10.0] }},
      "properties": {{
        "name": "Other",
        "id": 2,
        "buffer_dist": 2.0
      }}
    }}
  ]
}}"#
    )?;
    Ok(())
}

#[tokio::test]
async fn test_st_buffer_point_area() {
    // Test buffer of point creates polygon with expected area
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("points.geojson");
    let output_path = temp_dir.path().join("buffer_areas.csv");

    create_test_points_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Buffer points and calculate area
    let sql_query = "
        SELECT
            name,
            id,
            ST_Area(ST_Buffer(geometry, 1.0)) as buffer_area
        FROM points
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
        Some("points"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Buffer areas:\n{}", output);

    // Area of circle with radius 1 ≈ π
    let lines: Vec<&str> = output.lines().collect();
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        let area: f64 = parts[2].parse().expect("Area should be a number");
        assert!(
            (area - std::f64::consts::PI).abs() < 0.2,
            "Buffer area should be approximately π, got {}",
            area
        );
    }
}

#[tokio::test]
async fn test_st_buffer_with_column_distance() {
    // Test using a column value as the buffer distance
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("points.geojson");
    let output_path = temp_dir.path().join("variable_buffer_areas.csv");

    create_test_points_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Use buffer_dist column as the distance
    let sql_query = "
        SELECT
            name,
            buffer_dist,
            ST_Area(ST_Buffer(geometry, buffer_dist)) as buffer_area
        FROM points
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
        Some("points"),
        None,
        None,
        None,
    )
    .await;

    assert!(
        result.is_ok(),
        "Variable buffer conversion failed: {:?}",
        result.err()
    );

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Variable buffer areas:\n{}", output);

    // First point: radius 1, area ≈ π
    // Second point: radius 2, area ≈ 4π
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 3, "Should have header + 2 data rows");

    let first = lines[1].split(',').collect::<Vec<&str>>();
    let area1: f64 = first[2].parse().expect("Area should be a number");
    assert!(
        (area1 - std::f64::consts::PI).abs() < 0.2,
        "First buffer area should be approximately π, got {}",
        area1
    );

    let second = lines[2].split(',').collect::<Vec<&str>>();
    let area2: f64 = second[2].parse().expect("Area should be a number");
    assert!(
        (area2 - 4.0 * std::f64::consts::PI).abs() < 0.5,
        "Second buffer area should be approximately 4π, got {}",
        area2
    );
}

#[tokio::test]
async fn test_st_buffer_polygon_expand() {
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygon.geojson");
    let output_path = temp_dir.path().join("expanded_areas.csv");

    // Create GeoJSON with a unit square polygon
    let mut file = File::create(&input_path).unwrap();
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Polygon",
        "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]
      }},
      "properties": {{ "name": "Unit Square" }}
    }}
  ]
}}"#
    )
    .unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Buffer the polygon and verify area increased
    let sql_query = "
        SELECT
            name,
            ST_Area(geometry) as original_area,
            ST_Area(ST_Buffer(geometry, 0.5)) as buffered_area
        FROM polygon
    ";

    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &csv_driver,
        "geometry",
        None,
        Some(sql_query),
        Some("polygon"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Expanded areas:\n{}", output);

    let lines: Vec<&str> = output.lines().collect();
    let parts: Vec<&str> = lines[1].split(',').collect();
    let original_area: f64 = parts[1].parse().expect("Original area should be a number");
    let buffered_area: f64 = parts[2].parse().expect("Buffered area should be a number");

    assert!(
        (original_area - 1.0).abs() < 0.001,
        "Original area should be 1.0, got {}",
        original_area
    );
    assert!(
        buffered_area > original_area,
        "Buffered area ({}) should be greater than original ({})",
        buffered_area,
        original_area
    );
    // Buffered unit square with 0.5 buffer adds rounded corners and edges
    // Approximate expected area: 1 + 4*0.5*1 (edges) + π*0.5^2 (corners) ≈ 1 + 2 + 0.785 ≈ 3.785
    assert!(
        (buffered_area - 3.785).abs() < 0.1,
        "Buffered area should be approximately 3.785, got {}",
        buffered_area
    );
}

#[tokio::test]
async fn test_st_buffer_zero_distance() {
    // Test that buffer with distance 0 returns same area
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygon.geojson");
    let output_path = temp_dir.path().join("zero_buffer.csv");

    let mut file = File::create(&input_path).unwrap();
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
      "properties": {{ "name": "Square" }}
    }}
  ]
}}"#
    )
    .unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    let sql_query = "
        SELECT
            name,
            ST_Area(geometry) as original_area,
            ST_Area(ST_Buffer(geometry, 0.0)) as buffered_area
        FROM polygon
    ";

    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &csv_driver,
        "geometry",
        None,
        Some(sql_query),
        Some("polygon"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Zero buffer:\n{}", output);

    let lines: Vec<&str> = output.lines().collect();
    let parts: Vec<&str> = lines[1].split(',').collect();
    let original_area: f64 = parts[1].parse().expect("Original area should be a number");
    let buffered_area: f64 = parts[2].parse().expect("Buffered area should be a number");

    assert!(
        (original_area - buffered_area).abs() < 0.001,
        "Zero buffer should preserve area: original={}, buffered={}",
        original_area,
        buffered_area
    );
}
