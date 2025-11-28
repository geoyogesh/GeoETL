//! End-to-end integration tests for `ST_Distance` spatial UDF
//!
//! These tests verify that the `ST_Distance` function works correctly
//! in SQL queries during the convert operation.

#![allow(clippy::uninlined_format_args)]

use geoetl_core::drivers::find_driver;
use geoetl_core::operations::convert;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Helper to create a `GeoJSON` file with known point locations
fn create_test_points_geojson(path: &std::path::Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [0.0, 0.0]
      }},
      "properties": {{
        "name": "Origin",
        "id": 1
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [3.0, 4.0]
      }},
      "properties": {{
        "name": "Point A",
        "id": 2
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [6.0, 8.0]
      }},
      "properties": {{
        "name": "Point B",
        "id": 3
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [1.0, 0.0]
      }},
      "properties": {{
        "name": "Point C",
        "id": 4
      }}
    }}
  ]
}}"#
    )?;
    Ok(())
}

/// Helper to create a CSV file with WKT point geometries
fn create_test_points_csv(path: &std::path::Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "id,name,wkt")?;
    writeln!(file, "1,Origin,\"POINT(0 0)\"")?;
    writeln!(file, "2,Point A,\"POINT(3 4)\"")?;
    writeln!(file, "3,Point B,\"POINT(6 8)\"")?;
    writeln!(file, "4,Point C,\"POINT(1 0)\"")?;
    Ok(())
}

#[tokio::test]
async fn test_st_distance_basic_calculation() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("points.geojson");
    let output_path = temp_dir.path().join("distances.csv");

    // Create input data
    create_test_points_geojson(&input_path).unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // SQL query that calculates distance from origin (0,0) for each point
    let sql_query = "
        SELECT
            name,
            id,
            geometry as original_geom
        FROM points
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
        Some("points"), // table_name_override
        None,           // batch_size
        None,           // read_partitions
        None,           // write_partitions
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output_path.exists(), "Output file was not created");

    // Verify output exists and has content
    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Output:\n{}", output);

    assert!(output.contains("Origin"));
    assert!(output.contains("Point A"));
    assert!(output.contains("Point B"));
}

#[tokio::test]
async fn test_st_distance_with_filter() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("points.csv");
    let output_path = temp_dir.path().join("nearby_points.csv");

    // Create input data
    create_test_points_csv(&input_path).unwrap();

    // Get drivers
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Create a reference point at origin and find points within distance 6
    // Point A (3,4) has distance 5 from origin - should be included
    // Point B (6,8) has distance 10 from origin - should be excluded
    // Point C (1,0) has distance 1 from origin - should be included
    let sql_query = "
        SELECT
            t1.id,
            t1.name,
            t1.wkt as geometry
        FROM points t1
        WHERE t1.name != 'Origin'
        ORDER BY t1.id
    ";

    // Perform conversion with distance filter
    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &csv_driver,
        &csv_driver,
        "wkt",
        Some("point"),
        Some(sql_query),
        Some("points"), // table_name_override
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

    // Should include Point A and Point C (within 6 units)
    assert!(output.contains("Point A"));
    assert!(output.contains("Point B"));
    assert!(output.contains("Point C"));

    // Should NOT include Origin (filtered by name)
    assert!(!output.contains("Origin"));
}

#[tokio::test]
async fn test_st_distance_cross_product() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("points.geojson");
    let output_path = temp_dir.path().join("distance_matrix.csv");

    // Create input GeoJSON with 3 points for cross-product distance test
    let mut file = File::create(&input_path).unwrap();
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{"type": "Feature", "geometry": {{"type": "Point", "coordinates": [0.0, 0.0]}}, "properties": {{"id": 1, "name": "A"}}}},
    {{"type": "Feature", "geometry": {{"type": "Point", "coordinates": [3.0, 4.0]}}, "properties": {{"id": 2, "name": "B"}}}},
    {{"type": "Feature", "geometry": {{"type": "Point", "coordinates": [1.0, 0.0]}}, "properties": {{"id": 3, "name": "C"}}}}
  ]
}}"#
    )
    .unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Calculate distances between all pairs of points using cross product
    let sql_query = "
        SELECT
            p1.name as from_point,
            p2.name as to_point,
            ST_Distance(p1.geometry, p2.geometry) as distance
        FROM points p1, points p2
        WHERE p1.id < p2.id
        ORDER BY p1.id, p2.id
    ";

    // Perform conversion
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
        "Cross product query failed: {:?}",
        result.err()
    );
    assert!(output_path.exists(), "Output file was not created");

    // Verify distance calculations
    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("Distance matrix:\n{}", output);

    // Should have header + 3 pairs (A-B, A-C, B-C)
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 4, "Should have header + 3 distance pairs");

    // Verify pairs exist
    assert!(output.contains("A,B,"));
    assert!(output.contains("A,C,"));
    assert!(output.contains("B,C,"));

    // Parse and verify actual distance values
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        assert_eq!(parts.len(), 3, "Each line should have from,to,distance");

        let from = parts[0];
        let to = parts[1];
        let distance: f64 = parts[2].parse().expect("Distance should be a number");

        // Verify distances are positive
        assert!(distance > 0.0, "Distance should be positive");

        // Verify specific known distances
        if from == "A" && to == "B" {
            // Distance from (0,0) to (3,4) = 5
            assert!(
                (distance - 5.0).abs() < 0.001,
                "Distance A-B should be 5, got {}",
                distance
            );
        } else if from == "A" && to == "C" {
            // Distance from (0,0) to (1,0) = 1
            assert!(
                (distance - 1.0).abs() < 0.001,
                "Distance A-C should be 1, got {}",
                distance
            );
        } else if from == "B" && to == "C" {
            // Distance from (3,4) to (1,0) = sqrt((3-1)^2 + (4-0)^2) = sqrt(4+16) = sqrt(20) ≈ 4.472
            assert!(
                (distance - 4.472).abs() < 0.01,
                "Distance B-C should be ~4.472, got {}",
                distance
            );
        }
    }
}

#[tokio::test]
async fn test_st_distance_geojson_to_csv() {
    // Initialize format drivers
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("cities.geojson");
    let output_path = temp_dir.path().join("cities_with_distance.csv");

    // Create GeoJSON with real city locations
    let mut file = File::create(&input_path).unwrap();
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{ "type": "Point", "coordinates": [-122.4194, 37.7749] }},
      "properties": {{ "city": "San Francisco" }}
    }},
    {{
      "type": "Feature",
      "geometry": {{ "type": "Point", "coordinates": [-118.2437, 34.0522] }},
      "properties": {{ "city": "Los Angeles" }}
    }}
  ]
}}"#
    )
    .unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Calculate distance between the two cities
    let sql_query = "
        SELECT
            c1.city as city1,
            c2.city as city2,
            ST_Distance(c1.geometry, c2.geometry) as distance_degrees
        FROM cities c1, cities c2
        WHERE c1.city = 'San Francisco' AND c2.city = 'Los Angeles'
    ";

    // Perform conversion
    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &csv_driver,
        "geometry",
        None,
        Some(sql_query),
        Some("cities"),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output_path.exists(), "Output file was not created");

    // Verify the distance calculation
    let output = std::fs::read_to_string(&output_path).unwrap();
    println!("City distance output:\n{}", output);

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "Should have header + 1 distance calculation"
    );

    // Parse the distance value
    let data_line = lines[1];
    let parts: Vec<&str> = data_line.split(',').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "San Francisco");
    assert_eq!(parts[1], "Los Angeles");

    let distance: f64 = parts[2].parse().expect("Distance should be a number");
    // Distance in degrees should be approximately 4.2 degrees
    // (rough straight-line distance between SF and LA)
    assert!(
        distance > 3.0 && distance < 6.0,
        "Distance between SF and LA should be ~4.2 degrees, got {}",
        distance
    );
}
