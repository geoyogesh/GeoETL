//! End-to-end integration tests for `ST_Centroid` spatial UDF
//!
//! These tests verify that the `ST_Centroid` function works correctly
//! in SQL queries during the convert operation.

#![allow(clippy::uninlined_format_args)]

use geoetl_core::drivers::find_driver;
use geoetl_core::operations::convert;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Helper to create a `GeoJSON` file with polygon geometries
fn create_test_polygons_geojson(path: &std::path::Path) -> std::io::Result<()> {
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
        "name": "Square",
        "id": 1
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Polygon",
        "coordinates": [[[0.0, 0.0], [4.0, 0.0], [4.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]
      }},
      "properties": {{
        "name": "Rectangle",
        "id": 2
      }}
    }}
  ]
}}"#
    )?;
    Ok(())
}

#[tokio::test]
async fn test_st_centroid_area_verification() {
    // Test that centroid area is 0 (point has no area)
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("centroid_areas.csv");

    create_test_polygons_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Calculate area of centroid (should be 0 for points)
    let sql_query = "
        SELECT
            name,
            id,
            ST_Area(ST_Centroid(geometry)) as centroid_area
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
    println!("Centroid areas:\n{}", output);

    // Centroid is a point, area should be 0
    let lines: Vec<&str> = output.lines().collect();
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        let area: f64 = parts[2].parse().expect("Area should be a number");
        assert!(
            area.abs() < 0.001,
            "Centroid area should be 0 (point), got {}",
            area
        );
    }
}

#[tokio::test]
async fn test_st_centroid_buffer_chain_area() {
    // Test chaining ST_Centroid with ST_Buffer and verify area
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("buffered_centroid_areas.csv");

    create_test_polygons_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Chain: get centroid, buffer it, calculate area
    let sql_query = "
        SELECT
            name,
            ST_Area(ST_Buffer(ST_Centroid(geometry), 1.0)) as buffered_area
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

    assert!(
        result.is_ok(),
        "Chained operation failed: {:?}",
        result.err()
    );

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Buffered centroid areas:\n{}", output);

    // Buffer of radius 1 around centroid point ≈ π
    let lines: Vec<&str> = output.lines().collect();
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        let area: f64 = parts[1].parse().expect("Area should be a number");
        assert!(
            (area - std::f64::consts::PI).abs() < 0.2,
            "Buffered centroid area should be approximately π, got {}",
            area
        );
    }
}

#[tokio::test]
async fn test_st_centroid_linestring() {
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("lines.geojson");
    let output_path = temp_dir.path().join("line_centroid_lengths.csv");

    // Create GeoJSON with linestring
    let mut file = File::create(&input_path).unwrap();
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{
        "type": "LineString",
        "coordinates": [[0.0, 0.0], [4.0, 0.0]]
      }},
      "properties": {{ "name": "Horizontal Line" }}
    }}
  ]
}}"#
    )
    .unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Centroid of a line is a point, which has length 0
    let sql_query = "
        SELECT
            name,
            ST_Length(ST_Centroid(geometry)) as centroid_length
        FROM lines
    ";

    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &csv_driver,
        "geometry",
        None,
        Some(sql_query),
        Some("lines"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Line centroid lengths:\n{}", output);

    // Centroid is a point, length should be 0
    let lines: Vec<&str> = output.lines().collect();
    let data_line = lines[1];
    let parts: Vec<&str> = data_line.split(',').collect();
    let length: f64 = parts[1].parse().expect("Length should be a number");
    assert!(
        length.abs() < 0.001,
        "Centroid length should be 0 (point), got {}",
        length
    );
}
