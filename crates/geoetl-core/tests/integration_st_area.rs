//! End-to-end integration tests for `ST_Area` spatial UDF
//!
//! These tests verify that the `ST_Area` function works correctly
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
        "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]
      }},
      "properties": {{
        "name": "Unit Square",
        "id": 1
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Polygon",
        "coordinates": [[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0], [0.0, 0.0]]]
      }},
      "properties": {{
        "name": "Rectangle",
        "id": 2
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Polygon",
        "coordinates": [[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]
      }},
      "properties": {{
        "name": "Double Square",
        "id": 3
      }}
    }}
  ]
}}"#
    )?;
    Ok(())
}

#[tokio::test]
async fn test_st_area_basic_calculation() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("areas.csv");

    // Create input data
    create_test_polygons_geojson(&input_path).unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // SQL query that calculates area for each polygon
    let sql_query = "
        SELECT
            name,
            id,
            ST_Area(geometry) as area
        FROM polygons
        ORDER BY id
    ";

    // Perform conversion with SQL
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
    assert!(output_path.exists(), "Output file was not created");

    // Verify output and area calculations
    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Output:\n{}", output);

    // Parse and verify actual area values
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 4, "Should have header + 3 polygons");

    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        assert_eq!(parts.len(), 3, "Each line should have name,id,area");

        let name = parts[0];
        let area: f64 = parts[2].parse().expect("Area should be a number");

        match name {
            "Unit Square" => {
                assert!(
                    (area - 1.0).abs() < 0.001,
                    "Unit Square area should be 1.0, got {}",
                    area
                );
            },
            "Rectangle" => {
                assert!(
                    (area - 12.0).abs() < 0.001,
                    "Rectangle area should be 12.0, got {}",
                    area
                );
            },
            "Double Square" => {
                assert!(
                    (area - 4.0).abs() < 0.001,
                    "Double Square area should be 4.0, got {}",
                    area
                );
            },
            _ => panic!("Unexpected polygon name: {}", name),
        }
    }
}

#[tokio::test]
async fn test_st_area_with_filter() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("large_areas.csv");

    // Create GeoJSON input data
    create_test_polygons_geojson(&input_path).unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Filter polygons with area > 3
    // Unit Square (area 1) - excluded
    // Rectangle (area 12) - included
    // Double Square (area 4) - included
    let sql_query = "
        SELECT
            id,
            name,
            ST_Area(geometry) as area
        FROM polygons
        WHERE ST_Area(geometry) > 3.0
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
        "Conversion with filter failed: {:?}",
        result.err()
    );
    assert!(output_path.exists(), "Output file was not created");

    // Verify filtered results
    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Filtered output:\n{}", output);

    // Should include Rectangle and Double Square (area > 3)
    assert!(output.contains("Rectangle"));
    assert!(output.contains("Double Square"));

    // Should NOT include Unit Square (area = 1)
    assert!(!output.contains("Unit Square"));
}

#[tokio::test]
async fn test_st_area_point_returns_zero() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("points.geojson");
    let output_path = temp_dir.path().join("point_areas.csv");

    // Create GeoJSON with point geometries
    let mut file = File::create(&input_path).unwrap();
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{ "type": "Point", "coordinates": [0.0, 0.0] }},
      "properties": {{ "name": "Origin" }}
    }},
    {{
      "type": "Feature",
      "geometry": {{ "type": "Point", "coordinates": [5.0, 5.0] }},
      "properties": {{ "name": "Other Point" }}
    }}
  ]
}}"#
    )
    .unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Calculate area for points (should be 0)
    let sql_query = "
        SELECT
            name,
            ST_Area(geometry) as area
        FROM points
        ORDER BY name
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
    println!("Point areas:\n{}", output);

    // Verify all areas are 0
    let lines: Vec<&str> = output.lines().collect();
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        let area: f64 = parts[1].parse().expect("Area should be a number");
        assert!(area.abs() < 0.001, "Point area should be 0, got {}", area);
    }
}

#[tokio::test]
async fn test_st_area_single_polygon() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("single.geojson");
    let output_path = temp_dir.path().join("computed_areas.csv");

    // Create a GeoJSON with a single 4x3 rectangle
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
        "coordinates": [[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0], [0.0, 0.0]]]
      }},
      "properties": {{ "id": 1 }}
    }}
  ]
}}"#
    )
    .unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Calculate area
    let sql_query = "
        SELECT
            id,
            ST_Area(geometry) as area
        FROM data
    ";

    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &csv_driver,
        "geometry",
        None,
        Some(sql_query),
        Some("data"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Area output:\n{}", output);

    // Parse and verify the area (4x3 = 12)
    let lines: Vec<&str> = output.lines().collect();
    let data_line = lines[1];
    let parts: Vec<&str> = data_line.split(',').collect();
    let area: f64 = parts[1].parse().expect("Area should be a number");
    assert!(
        (area - 12.0).abs() < 0.001,
        "Area of 4x3 rectangle should be 12.0, got {}",
        area
    );
}
