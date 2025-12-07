//! End-to-end integration tests for `ST_Length` spatial UDF
//!
//! These tests verify that the `ST_Length` function works correctly
//! in SQL queries during the convert operation.

#![allow(clippy::uninlined_format_args)]

use geoetl_core::drivers::find_driver;
use geoetl_core::operations::convert;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Helper to create a `GeoJSON` file with linestring geometries
fn create_test_linestrings_geojson(path: &std::path::Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{
        "type": "LineString",
        "coordinates": [[0.0, 0.0], [5.0, 0.0]]
      }},
      "properties": {{
        "name": "Horizontal Line",
        "id": 1
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "LineString",
        "coordinates": [[0.0, 0.0], [3.0, 4.0]]
      }},
      "properties": {{
        "name": "Diagonal Line",
        "id": 2
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "LineString",
        "coordinates": [[0.0, 0.0], [3.0, 0.0], [3.0, 4.0]]
      }},
      "properties": {{
        "name": "L-Shape",
        "id": 3
      }}
    }}
  ]
}}"#
    )?;
    Ok(())
}

#[tokio::test]
async fn test_st_length_basic_calculation() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("lines.geojson");
    let output_path = temp_dir.path().join("lengths.csv");

    // Create input data
    create_test_linestrings_geojson(&input_path).unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // SQL query that calculates length for each linestring
    let sql_query = "
        SELECT
            name,
            id,
            ST_Length(geometry) as length
        FROM lines
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
        Some("lines"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output_path.exists(), "Output file was not created");

    // Verify output and length calculations
    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Output:\n{}", output);

    // Parse and verify actual length values
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 4, "Should have header + 3 linestrings");

    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        assert_eq!(parts.len(), 3, "Each line should have name,id,length");

        let name = parts[0];
        let length: f64 = parts[2].parse().expect("Length should be a number");

        match name {
            "Horizontal Line" => {
                assert!(
                    (length - 5.0).abs() < 0.001,
                    "Horizontal Line length should be 5.0, got {}",
                    length
                );
            },
            "Diagonal Line" => {
                // 3-4-5 triangle hypotenuse
                assert!(
                    (length - 5.0).abs() < 0.001,
                    "Diagonal Line length should be 5.0, got {}",
                    length
                );
            },
            "L-Shape" => {
                // 3 + 4 = 7
                assert!(
                    (length - 7.0).abs() < 0.001,
                    "L-Shape length should be 7.0, got {}",
                    length
                );
            },
            _ => panic!("Unexpected line name: {}", name),
        }
    }
}

#[tokio::test]
async fn test_st_length_polygon_perimeter() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("perimeters.csv");

    // Create GeoJSON with polygon geometries
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
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Polygon",
        "coordinates": [[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0], [0.0, 0.0]]]
      }},
      "properties": {{ "name": "Rectangle" }}
    }}
  ]
}}"#
    )
    .unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Calculate perimeter (length) of polygons
    let sql_query = "
        SELECT
            name,
            ST_Length(geometry) as perimeter
        FROM polygons
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
        Some("polygons"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Perimeters:\n{}", output);

    // Parse and verify perimeter values
    let lines: Vec<&str> = output.lines().collect();
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        let name = parts[0];
        let perimeter: f64 = parts[1].parse().expect("Perimeter should be a number");

        match name {
            "Unit Square" => {
                assert!(
                    (perimeter - 4.0).abs() < 0.001,
                    "Unit Square perimeter should be 4.0, got {}",
                    perimeter
                );
            },
            "Rectangle" => {
                // 2*(4+3) = 14
                assert!(
                    (perimeter - 14.0).abs() < 0.001,
                    "Rectangle perimeter should be 14.0, got {}",
                    perimeter
                );
            },
            _ => panic!("Unexpected polygon name: {}", name),
        }
    }
}

#[tokio::test]
async fn test_st_length_with_filter() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("lines.geojson");
    let output_path = temp_dir.path().join("long_lines.csv");

    // Create GeoJSON input data
    create_test_linestrings_geojson(&input_path).unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Filter linestrings with length > 5.5
    // Horizontal Line (length 5) - excluded
    // Diagonal Line (length 5) - excluded
    // L-Shape (length 7) - included
    let sql_query = "
        SELECT
            id,
            name,
            ST_Length(geometry) as length
        FROM lines
        WHERE ST_Length(geometry) > 5.5
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
        Some("lines"),
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

    // Should include only L-Shape (length 7 > 5.5)
    assert!(output.contains("L-Shape"));

    // Should NOT include others
    assert!(!output.contains("Horizontal Line"));
    assert!(!output.contains("Diagonal Line"));
}

#[tokio::test]
async fn test_st_length_point_returns_zero() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("points.geojson");
    let output_path = temp_dir.path().join("point_lengths.csv");

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

    // Calculate length for points (should be 0)
    let sql_query = "
        SELECT
            name,
            ST_Length(geometry) as length
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
    println!("Point lengths:\n{}", output);

    // Verify all lengths are 0
    let lines: Vec<&str> = output.lines().collect();
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        let length: f64 = parts[1].parse().expect("Length should be a number");
        assert!(
            length.abs() < 0.001,
            "Point length should be 0, got {}",
            length
        );
    }
}

#[tokio::test]
async fn test_st_length_single_line() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("single.geojson");
    let output_path = temp_dir.path().join("computed_lengths.csv");

    // Create a GeoJSON with a single 3-4-5 triangle linestring
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
        "coordinates": [[0.0, 0.0], [3.0, 4.0]]
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

    // Calculate length
    let sql_query = "
        SELECT
            id,
            ST_Length(geometry) as length
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
    println!("Length output:\n{}", output);

    // Parse and verify the length (3-4-5 = 5)
    let lines: Vec<&str> = output.lines().collect();
    let data_line = lines[1];
    let parts: Vec<&str> = data_line.split(',').collect();
    let length: f64 = parts[1].parse().expect("Length should be a number");
    assert!(
        (length - 5.0).abs() < 0.001,
        "Length of 3-4-5 line should be 5.0, got {}",
        length
    );
}
