//! End-to-end integration tests for `ST_Intersection` spatial UDF
//!
//! These tests verify that the `ST_Intersection` function works correctly
//! in SQL queries during the convert operation.

#![allow(clippy::uninlined_format_args)]

use geoetl_core::drivers::find_driver;
use geoetl_core::operations::convert;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Helper to create a `GeoJSON` file with overlapping polygons
fn create_overlapping_polygons_geojson(path: &std::path::Path) -> std::io::Result<()> {
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
        "name": "Overlapping",
        "id": 1,
        "geom2_wkt": "POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))"
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Polygon",
        "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]
      }},
      "properties": {{
        "name": "Disjoint",
        "id": 2,
        "geom2_wkt": "POLYGON((5 5, 6 5, 6 6, 5 6, 5 5))"
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Polygon",
        "coordinates": [[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0], [0.0, 0.0]]]
      }},
      "properties": {{
        "name": "Containing",
        "id": 3,
        "geom2_wkt": "POLYGON((1 1, 2 1, 2 2, 1 2, 1 1))"
      }}
    }}
  ]
}}"#
    )?;
    Ok(())
}

#[tokio::test]
async fn test_st_intersection_overlapping_area() {
    // Test that intersection of overlapping polygons has correct area
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("intersection_areas.csv");

    create_overlapping_polygons_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Calculate area of intersection for overlapping pair
    let sql_query = "
        SELECT
            name,
            id,
            ST_Area(geometry) as geom1_area,
            ST_Area(ST_GeomFromText(geom2_wkt)) as geom2_area,
            ST_Area(ST_Intersection(geometry, ST_GeomFromText(geom2_wkt))) as intersection_area
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
    println!("Intersection areas:\n{}", output);

    // First polygon (0,0)-(2,2)
    // Second polygon (1,1)-(3,3)
    // Intersection (1,1)-(2,2) area = 1
    let lines: Vec<&str> = output.lines().collect();
    let parts: Vec<&str> = lines[1].split(',').collect();
    let intersection_area: f64 = parts[4]
        .parse()
        .expect("Intersection area should be a number");

    assert!(
        (intersection_area - 1.0).abs() < 0.001,
        "Intersection area should be 1.0, got {}",
        intersection_area
    );
}

#[tokio::test]
async fn test_st_intersection_disjoint_area() {
    // Test that intersection of disjoint polygons has zero area
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("disjoint_intersection.csv");

    create_overlapping_polygons_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Intersection of disjoint polygons = empty geometry with area 0
    let sql_query = "
        SELECT
            name,
            ST_Area(ST_Intersection(geometry, ST_GeomFromText(geom2_wkt))) as intersection_area
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
    println!("Disjoint intersection:\n{}", output);

    let lines: Vec<&str> = output.lines().collect();
    let parts: Vec<&str> = lines[1].split(',').collect();
    let intersection_area: f64 = parts[1]
        .parse()
        .expect("Intersection area should be a number");

    assert!(
        intersection_area.abs() < 0.001,
        "Disjoint intersection area should be 0, got {}",
        intersection_area
    );
}

#[tokio::test]
async fn test_st_intersection_containing_polygon() {
    // Test intersection where one polygon contains the other
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("contained_intersection.csv");

    create_overlapping_polygons_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Intersection of containing polygon with smaller one = smaller one
    let sql_query = "
        SELECT
            name,
            ST_Area(ST_GeomFromText(geom2_wkt)) as smaller_area,
            ST_Area(ST_Intersection(geometry, ST_GeomFromText(geom2_wkt))) as intersection_area
        FROM polygons
        WHERE id = 3
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
    println!("Contained intersection:\n{}", output);

    // Big polygon (0,0)-(4,4) contains small polygon (1,1)-(2,2)
    // Intersection = small polygon, area = 1
    let lines: Vec<&str> = output.lines().collect();
    let parts: Vec<&str> = lines[1].split(',').collect();
    let smaller_area: f64 = parts[1].parse().expect("Smaller area should be a number");
    let intersection_area: f64 = parts[2]
        .parse()
        .expect("Intersection area should be a number");

    assert!(
        (intersection_area - smaller_area).abs() < 0.001,
        "Intersection should equal smaller polygon: expected {}, got {}",
        smaller_area,
        intersection_area
    );
    assert!(
        (intersection_area - 1.0).abs() < 0.001,
        "Contained intersection area should be 1.0, got {}",
        intersection_area
    );
}

#[tokio::test]
async fn test_st_intersection_with_buffer() {
    // Test intersection of buffered geometries
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("points.geojson");
    let output_path = temp_dir.path().join("buffer_intersection.csv");

    // Create two nearby points
    let mut file = File::create(&input_path).unwrap();
    writeln!(
        file,
        r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{ "type": "Point", "coordinates": [0.0, 0.0] }},
      "properties": {{
        "name": "Nearby Points",
        "point2_wkt": "POINT(1 0)"
      }}
    }}
  ]
}}"#
    )
    .unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Buffer both points by 1.0, then find intersection
    // Points are 1 unit apart, buffers of 1.0 will overlap
    let sql_query = "
        SELECT
            name,
            ST_Area(ST_Buffer(geometry, 1.0)) as buffer1_area,
            ST_Area(ST_Buffer(ST_GeomFromText(point2_wkt), 1.0)) as buffer2_area,
            ST_Area(ST_Intersection(
                ST_Buffer(geometry, 1.0),
                ST_Buffer(ST_GeomFromText(point2_wkt), 1.0)
            )) as intersection_area
        FROM points
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
    println!("Buffer intersection:\n{}", output);

    let lines: Vec<&str> = output.lines().collect();
    let parts: Vec<&str> = lines[1].split(',').collect();
    let intersection_area: f64 = parts[3]
        .parse()
        .expect("Intersection area should be a number");

    // Intersection of two overlapping circles should have positive area
    // The exact area depends on geometry, but it should be > 0 and < π (single buffer area)
    assert!(
        intersection_area > 0.0,
        "Overlapping buffer intersection should have positive area, got {}",
        intersection_area
    );
    assert!(
        intersection_area < std::f64::consts::PI,
        "Intersection area should be less than single buffer area π, got {}",
        intersection_area
    );
}

#[tokio::test]
async fn test_st_intersection_multiple_rows() {
    // Test intersection across multiple rows
    geoetl_core::init::initialize();

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("polygons.geojson");
    let output_path = temp_dir.path().join("multi_intersection.csv");

    create_overlapping_polygons_geojson(&input_path).unwrap();

    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Calculate intersection for all rows
    let sql_query = "
        SELECT
            name,
            id,
            ST_Area(ST_Intersection(geometry, ST_GeomFromText(geom2_wkt))) as intersection_area
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
    println!("Multi intersection:\n{}", output);

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 4, "Should have header + 3 data rows");

    // First row: overlapping, intersection = 1
    let parts1: Vec<&str> = lines[1].split(',').collect();
    let area1: f64 = parts1[2].parse().expect("Area should be a number");
    assert!(
        (area1 - 1.0).abs() < 0.001,
        "First intersection area should be 1.0, got {}",
        area1
    );

    // Second row: disjoint, intersection = 0
    let parts2: Vec<&str> = lines[2].split(',').collect();
    let area2: f64 = parts2[2].parse().expect("Area should be a number");
    assert!(
        area2.abs() < 0.001,
        "Second intersection area should be 0, got {}",
        area2
    );

    // Third row: containing, intersection = 1 (smaller polygon)
    let parts3: Vec<&str> = lines[3].split(',').collect();
    let area3: f64 = parts3[2].parse().expect("Area should be a number");
    assert!(
        (area3 - 1.0).abs() < 0.001,
        "Third intersection area should be 1.0, got {}",
        area3
    );
}
