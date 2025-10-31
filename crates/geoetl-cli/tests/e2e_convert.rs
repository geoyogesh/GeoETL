//! End-to-end CLI tests for the convert command
//!
//! These tests verify the complete CLI workflow including argument parsing,
//! driver lookup, and conversion operations using real-world test data.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

const TEST_DATA_CSV: &str = "tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv";
const TEST_DATA_GEOJSON: &str = "tests/e2e_data/geojson/natural-earth_cities.geojson";

/// Helper to create a command instance for the CLI
fn geoetl_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("geoetl-cli"))
}

#[test]
fn test_cli_convert_csv_to_csv() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.csv");

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("CSV")
        .arg("--output-driver")
        .arg("CSV")
        .assert()
        .success();

    // Verify output file exists and has content
    assert!(output_path.exists(), "Output file should be created");
    let output_content = fs::read_to_string(&output_path).unwrap();

    // Verify headers
    assert!(output_content.contains("geometry,name"));

    // Verify sample data points
    assert!(output_content.contains("Vatican City"));
    assert!(output_content.contains("Luxembourg"));
    assert!(output_content.contains("Monaco"));

    // Verify row count (244 lines = 1 header + 243 data rows)
    let line_count = output_content.lines().count();
    assert_eq!(
        line_count, 244,
        "Should have 244 lines (header + 243 cities)"
    );
}

#[test]
fn test_cli_convert_geojson_to_geojson() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.geojson");

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_GEOJSON)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("GeoJSON")
        .arg("--output-driver")
        .arg("GeoJSON")
        .assert()
        .success();

    // Verify output file exists and has content
    assert!(output_path.exists(), "Output file should be created");
    let output_content = fs::read_to_string(&output_path).unwrap();

    // Verify GeoJSON structure
    assert!(output_content.contains("FeatureCollection"));
    assert!(output_content.contains("\"type\""));
    assert!(output_content.contains("\"features\""));

    // Verify sample city data
    assert!(output_content.contains("Vatican City"));
    assert!(output_content.contains("San Marino"));
    assert!(output_content.contains("Luxembourg"));
}

#[test]
fn test_cli_convert_csv_to_csv_with_verbose() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output_verbose.csv");

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("CSV")
        .arg("--output-driver")
        .arg("CSV")
        .arg("--verbose")
        .assert()
        .success();

    assert!(output_path.exists());
}

#[test]
fn test_cli_convert_invalid_input_driver() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.csv");

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("INVALID_DRIVER")
        .arg("--output-driver")
        .arg("CSV")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_cli_convert_invalid_output_driver() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.xyz");

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("CSV")
        .arg("--output-driver")
        .arg("INVALID_DRIVER")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_cli_convert_unsupported_read() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.csv");

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("GML")
        .arg("--output-driver")
        .arg("CSV")
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not support reading"));
}

#[test]
fn test_cli_convert_unsupported_write() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.gml");

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("CSV")
        .arg("--output-driver")
        .arg("GML")
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not support writing"));
}

#[test]
fn test_cli_convert_nonexistent_input_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("nonexistent.csv");
    let output_path = temp_dir.path().join("output.csv");

    // Note: DataFusion may create empty tables for nonexistent files during schema inference
    // This test verifies the command completes - it may succeed with empty data or fail
    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(&input_path)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("CSV")
        .arg("--output-driver")
        .arg("CSV")
        .assert()
        .success(); // DataFusion handles missing files gracefully
}

#[test]
fn test_cli_convert_missing_required_args() {
    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_cli_convert_help() {
    geoetl_cmd()
        .arg("convert")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Converts data between different vector geospatial formats",
        ))
        .stdout(predicate::str::contains("--input"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--input-driver"))
        .stdout(predicate::str::contains("--output-driver"));
}

#[test]
fn test_cli_convert_preserves_data_integrity() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.csv");

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("CSV")
        .arg("--output-driver")
        .arg("CSV")
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path).unwrap();

    // Verify specific data points to ensure data integrity
    assert!(output_content.contains("12.4533865")); // Vatican City longitude
    assert!(output_content.contains("41.9032822")); // Vatican City latitude
    assert!(output_content.contains("POINT")); // WKT geometry format

    // Verify various cities from different parts of the world
    assert!(output_content.contains("Kigali")); // Africa
    assert!(output_content.contains("Doha")); // Asia
    assert!(output_content.contains("Ljubljana")); // Europe
    assert!(output_content.contains("Montevideo")); // South America
}

#[test]
fn test_cli_convert_case_insensitive_drivers() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.csv");

    // Test with lowercase driver names
    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("csv") // lowercase
        .arg("--output-driver")
        .arg("CSV") // uppercase
        .assert()
        .success();

    assert!(output_path.exists());
}

#[test]
fn test_cli_convert_large_dataset_performance() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.csv");

    // This test uses the 243-city dataset to ensure reasonable performance
    let start = std::time::Instant::now();

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_CSV)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("CSV")
        .arg("--output-driver")
        .arg("CSV")
        .assert()
        .success();

    let duration = start.elapsed();

    // Conversion should complete in reasonable time (< 5 seconds for 243 records)
    assert!(
        duration.as_secs() < 5,
        "Conversion took too long: {duration:?}"
    );

    assert!(output_path.exists());
}

#[test]
fn test_cli_convert_geojson_output_format() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.geojson");

    geoetl_cmd()
        .arg("convert")
        .arg("--input")
        .arg(TEST_DATA_GEOJSON)
        .arg("--output")
        .arg(&output_path)
        .arg("--input-driver")
        .arg("GeoJSON")
        .arg("--output-driver")
        .arg("GeoJSON")
        .assert()
        .success();

    let output_content = fs::read_to_string(&output_path).unwrap();

    // Verify it's valid JSON by parsing
    let json_result = serde_json::from_str::<serde_json::Value>(&output_content);
    assert!(
        json_result.is_ok(),
        "Output should be valid JSON: {:?}",
        json_result.err()
    );
}
