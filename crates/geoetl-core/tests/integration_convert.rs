//! End-to-end integration tests for the convert operation
//!
//! These tests verify the complete conversion workflow from file I/O
//! through the driver system to the final output.

use geoetl_core::drivers::{Driver, SupportStatus, find_driver};
use geoetl_core::operations::convert;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Helper to create a sample CSV file with spatial data
fn create_spatial_csv(path: &std::path::Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "id,name,lat,lon,category,value")?;
    writeln!(file, "1,Location A,40.7128,-74.0060,retail,100")?;
    writeln!(file, "2,Location B,34.0522,-118.2437,warehouse,250")?;
    writeln!(file, "3,Location C,41.8781,-87.6298,office,175")?;
    writeln!(file, "4,Location D,29.7604,-95.3698,retail,320")?;
    writeln!(file, "5,Location E,33.4484,-112.0740,office,280")?;
    Ok(())
}

/// Helper to create a sample `GeoJSON` file
fn create_sample_geojson(path: &std::path::Path) -> std::io::Result<()> {
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
        "coordinates": [-122.4194, 37.7749]
      }},
      "properties": {{
        "city": "San Francisco",
        "state": "CA",
        "population": 883305,
        "established": 1776
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [-87.6298, 41.8781]
      }},
      "properties": {{
        "city": "Chicago",
        "state": "IL",
        "population": 2746388,
        "established": 1837
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [-74.0060, 40.7128]
      }},
      "properties": {{
        "city": "New York",
        "state": "NY",
        "population": 8336817,
        "established": 1624
      }}
    }}
  ]
}}"#
    )?;
    Ok(())
}

#[tokio::test]
async fn test_e2e_csv_to_csv_conversion() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input_data.csv");
    let output_path = temp_dir.path().join("output_data.csv");

    // Create input data
    create_spatial_csv(&input_path).unwrap();

    // Get drivers
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Perform conversion
    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &csv_driver,
        &csv_driver,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output_path.exists(), "Output file was not created");

    // Verify output content
    let output = std::fs::read_to_string(&output_path).unwrap();
    assert!(output.contains("id,name,lat,lon,category,value"));
    assert!(output.contains("Location A"));
    assert!(output.contains("Location B"));
    assert!(output.contains("Location C"));
    assert!(output.contains("40.7128"));
    assert!(output.contains("-74.006"));

    // Verify row count
    let line_count = output.lines().count();
    assert_eq!(line_count, 6); // Header + 5 data rows
}

#[tokio::test]
async fn test_e2e_geojson_to_geojson_conversion() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("cities.geojson");
    let output_path = temp_dir.path().join("cities_output.geojson");

    // Create input data
    create_sample_geojson(&input_path).unwrap();

    // Get drivers
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");

    // Perform conversion
    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &geojson_driver,
        &geojson_driver,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output_path.exists(), "Output file was not created");

    // Verify output is valid GeoJSON
    let output = std::fs::read_to_string(&output_path).unwrap();
    assert!(output.contains("FeatureCollection"));
    assert!(output.contains("San Francisco"));
    assert!(output.contains("Chicago"));
    assert!(output.contains("New York"));

    // Verify it has the expected structure
    assert!(output.contains("\"type\""));
    assert!(output.contains("\"features\""));
}

#[tokio::test]
async fn test_e2e_large_csv_conversion() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("large_data.csv");
    let output_path = temp_dir.path().join("large_output.csv");

    // Create a larger CSV file with 1000 rows
    let mut file = File::create(&input_path).unwrap();
    writeln!(file, "id,value,category").unwrap();
    for i in 1..=1000 {
        writeln!(file, "{},{},category_{}", i, i * 10, i % 5).unwrap();
    }

    // Get drivers
    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    // Perform conversion
    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &csv_driver,
        &csv_driver,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output_path.exists(), "Output file was not created");

    // Verify output
    let output = std::fs::read_to_string(&output_path).unwrap();
    let line_count = output.lines().count();
    assert_eq!(line_count, 1001); // Header + 1000 data rows

    // Verify some sample data
    assert!(output.contains("500,5000,category_0"));
    assert!(output.contains("1000,10000,category_0"));
}

#[tokio::test]
async fn test_e2e_driver_validation() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test.csv");
    let output_path = temp_dir.path().join("output.shp");

    // Create input
    create_spatial_csv(&input_path).unwrap();

    // Create unsupported driver
    let input_driver = Driver::new(
        "CSV",
        "CSV",
        SupportStatus::Supported,
        SupportStatus::Supported,
        SupportStatus::Supported,
    );
    let output_driver = Driver::new(
        "ESRI Shapefile",
        "ESRI Shapefile",
        SupportStatus::NotSupported,
        SupportStatus::NotSupported,
        SupportStatus::NotSupported,
    );

    // Attempt conversion
    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &input_driver,
        &output_driver,
    )
    .await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("does not support writing")
    );
}

#[tokio::test]
async fn test_e2e_csv_with_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("special.csv");
    let output_path = temp_dir.path().join("special_output.csv");

    // Create CSV with special characters and proper quoting
    let mut file = File::create(&input_path).unwrap();
    writeln!(file, "id,name,description").unwrap();
    writeln!(file, "1,O'Brien,Simple name").unwrap();
    writeln!(file, "2,Smith,Another name").unwrap();
    writeln!(file, "3,Müller,Unicode name").unwrap();

    let csv_driver = find_driver("CSV").expect("CSV driver should exist");

    let result = convert(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        &csv_driver,
        &csv_driver,
    )
    .await;

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output_path.exists(), "Output file was not created");

    let output = std::fs::read_to_string(&output_path).unwrap();
    assert!(output.contains("O'Brien"));
    assert!(output.contains("Smith"));
    assert!(output.contains("Müller"));
}

#[tokio::test]
async fn test_e2e_multiple_conversions_same_session() {
    let temp_dir = TempDir::new().unwrap();

    // First conversion
    let input1 = temp_dir.path().join("input1.csv");
    let output1 = temp_dir.path().join("output1.csv");
    create_spatial_csv(&input1).unwrap();

    // Second conversion
    let input2 = temp_dir.path().join("input2.geojson");
    let output2 = temp_dir.path().join("output2.geojson");
    create_sample_geojson(&input2).unwrap();

    let csv_driver = find_driver("CSV").expect("CSV driver should exist");
    let geojson_driver = find_driver("GeoJSON").expect("GeoJSON driver should exist");

    // First conversion
    let result1 = convert(
        input1.to_str().unwrap(),
        output1.to_str().unwrap(),
        &csv_driver,
        &csv_driver,
    )
    .await;

    // Second conversion
    let result2 = convert(
        input2.to_str().unwrap(),
        output2.to_str().unwrap(),
        &geojson_driver,
        &geojson_driver,
    )
    .await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(output1.exists());
    assert!(output2.exists());
}
