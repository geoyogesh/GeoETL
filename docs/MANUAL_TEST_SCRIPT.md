# Manual Test Script for GeoETL CLI

This document provides manual test scripts for verifying the `info` and `convert` commands using existing test data.

## Quick Command Reference

### Info Commands
```bash
# CSV file info (--geometry-column is required for CSV)
cargo run -p geoetl-cli -- info crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv -f CSV --geometry-column geometry

# GeoJSON file info (--geometry-column is optional)
cargo run -p geoetl-cli -- info crates/geoetl-cli/tests/e2e_data/geojson/natural-earth_cities.geojson -f GeoJSON

# Verbose mode with CSV
cargo run -p geoetl-cli -- --verbose info crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv -f CSV --geometry-column geometry
```

### Convert Commands
```bash
# CSV to GeoJSON
cargo run -p geoetl-cli -- convert --input crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv --output /tmp/csv_to_geojson.geojson --input-driver CSV --output-driver GeoJSON --geometry-column geometry

# GeoJSON to CSV
cargo run -p geoetl-cli -- convert --input crates/geoetl-cli/tests/e2e_data/geojson/natural-earth_cities.geojson --output /tmp/geojson_to_csv.csv --input-driver GeoJSON --output-driver CSV --geometry-column geometry

# CSV to CSV (with verbose)
cargo run -p geoetl-cli -- --verbose convert --input crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv --output /tmp/csv_to_csv.csv --input-driver CSV --output-driver CSV --geometry-column geometry

# GeoJSON to GeoJSON
cargo run -p geoetl-cli -- convert --input crates/geoetl-cli/tests/e2e_data/geojson/natural-earth_cities.geojson --output /tmp/geojson_to_geojson.geojson --input-driver GeoJSON --output-driver GeoJSON
```

### Utility Commands
```bash
# List available drivers
cargo run -p geoetl-cli -- drivers

# Show help
cargo run -p geoetl-cli -- --help

# Show convert help
cargo run -p geoetl-cli -- convert --help

# Show info help
cargo run -p geoetl-cli -- info --help
```

---

## Test Data

The test suite uses existing data in `crates/geoetl-cli/tests/e2e_data/`:
- **CSV**: `csv/natural-earth_cities_native_AS_WKT.csv` (cities with WKT geometry)
- **GeoJSON**: `geojson/natural-earth_cities.geojson` (cities as GeoJSON features)

---

## Test 1: Info Command - CSV

**Purpose**: Verify that the info command correctly displays schema information for CSV files.

**Command**:
```bash
cargo run -p geoetl-cli -- info crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv -f CSV --geometry-column geometry
```

**Expected Output**:
- Dataset path should show the absolute path to the CSV file
- Driver should show "CSV (Comma Separated Value (.csv))"
- Geometry Columns table should show:
  - Column: `geometry`
  - Extension: `geoarrow.geometry`
  - CRS: N/A
- Fields table should show:
  - `name` (String, Nullable: Yes)

**Pass Criteria**:
- ✅ Command exits with code 0
- ✅ Dataset path is displayed
- ✅ Driver information is correct
- ✅ Geometry column is listed
- ✅ Name field is listed with correct type

---

## Test 2: Info Command - GeoJSON

**Purpose**: Verify that the info command correctly displays schema information for GeoJSON files.

**Command**:
```bash
cargo run -p geoetl-cli -- info crates/geoetl-cli/tests/e2e_data/geojson/natural-earth_cities.geojson -f GeoJSON
```

**Expected Output**:
- Dataset path should show the absolute path to the GeoJSON file
- Driver should show "GeoJSON (GeoJSON)"
- Geometry Columns table should show:
  - Column: `geometry`
  - Extension: `geoarrow.geometry`
  - CRS: N/A
- Fields table should show:
  - `name` (String, Nullable: Yes)

**Pass Criteria**:
- ✅ Command exits with code 0
- ✅ Dataset path is displayed
- ✅ Driver information is correct
- ✅ Geometry column is listed
- ✅ Name field is listed with correct type

---

## Test 3: Convert Command - CSV to GeoJSON

**Purpose**: Verify conversion from CSV (with WKT geometry) to GeoJSON.

**Command**:
```bash
cargo run -p geoetl-cli -- convert \
  --input crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv \
  --output /tmp/csv_to_geojson.geojson \
  --input-driver CSV \
  --output-driver GeoJSON \
  --geometry-column geometry
```

**Verification**:
```bash
# Check file exists and size
ls -lh /tmp/csv_to_geojson.geojson

# Validate it's valid JSON
jq . /tmp/csv_to_geojson.geojson > /dev/null && echo "✓ Valid JSON"

# Count features
echo "Feature count: $(jq '.features | length' /tmp/csv_to_geojson.geojson)"

# Check first feature
jq '.features[0]' /tmp/csv_to_geojson.geojson
```

**Pass Criteria**:
- ✅ Command exits with code 0
- ✅ Output file is created
- ✅ Output is valid JSON
- ✅ Contains FeatureCollection with features
- ✅ City names are preserved in properties
- ✅ Coordinates match original WKT

---

## Test 4: Convert Command - GeoJSON to CSV

**Purpose**: Verify conversion from GeoJSON to CSV with WKT geometry.

**Command**:
```bash
cargo run -p geoetl-cli -- convert \
  --input crates/geoetl-cli/tests/e2e_data/geojson/natural-earth_cities.geojson \
  --output /tmp/geojson_to_csv.csv \
  --input-driver GeoJSON \
  --output-driver CSV \
  --geometry-column geometry
```

**Verification**:
```bash
# Check file exists
ls -lh /tmp/geojson_to_csv.csv

# Count lines
wc -l /tmp/geojson_to_csv.csv

# View first few lines
head -10 /tmp/geojson_to_csv.csv

# Check for WKT geometry format
grep "POINT" /tmp/geojson_to_csv.csv | head -3
```

**Pass Criteria**:
- ✅ Command exits with code 0
- ✅ Output file is created
- ✅ Contains header row with columns
- ✅ Geometry is in WKT format (contains "POINT")
- ✅ City names are preserved
- ✅ Row count matches original feature count

---

## Test 5: Convert Command - CSV to CSV

**Purpose**: Verify CSV to CSV conversion preserves data.

**Command**:
```bash
cargo run -p geoetl-cli -- convert \
  --input crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv \
  --output /tmp/csv_to_csv.csv \
  --input-driver CSV \
  --output-driver CSV \
  --geometry-column geometry
```

**Verification**:
```bash
# Compare line counts
wc -l crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv /tmp/csv_to_csv.csv

# View sample data
head -5 /tmp/csv_to_csv.csv
tail -5 /tmp/csv_to_csv.csv
```

**Pass Criteria**:
- ✅ Command exits with code 0
- ✅ Output file is created
- ✅ Line count matches input (header + data rows)
- ✅ All city names are present
- ✅ Geometry coordinates are preserved

---

## Test 6: Convert Command - GeoJSON to GeoJSON

**Purpose**: Verify GeoJSON to GeoJSON conversion preserves feature data.

**Command**:
```bash
cargo run -p geoetl-cli -- convert \
  --input crates/geoetl-cli/tests/e2e_data/geojson/natural-earth_cities.geojson \
  --output /tmp/geojson_to_geojson.geojson \
  --input-driver GeoJSON \
  --output-driver GeoJSON
```

**Verification**:
```bash
# Validate JSON
jq . /tmp/geojson_to_geojson.geojson > /dev/null && echo "✓ Valid JSON"

# Compare feature counts
echo "Original: $(jq '.features | length' crates/geoetl-cli/tests/e2e_data/geojson/natural-earth_cities.geojson)"
echo "Output: $(jq '.features | length' /tmp/geojson_to_geojson.geojson)"

# Check structure
jq '.type' /tmp/geojson_to_geojson.geojson
```

**Pass Criteria**:
- ✅ Command exits with code 0
- ✅ Output file is created
- ✅ Output is valid GeoJSON FeatureCollection
- ✅ Feature count matches input
- ✅ All properties are preserved

---

## Test 7: Verbose Mode

**Purpose**: Verify that verbose flag provides additional logging.

**Command**:
```bash
cargo run -p geoetl-cli -- --verbose convert \
  --input crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv \
  --output /tmp/verbose_output.csv \
  --input-driver CSV \
  --output-driver CSV \
  --geometry-column geometry
```

**Expected Output**:
- INFO level logs showing conversion steps
- Information about reading input
- Information about writing output
- Row/batch counts

**Pass Criteria**:
- ✅ Command exits with code 0
- ✅ Additional logging is displayed (INFO level)
- ✅ Shows input/output driver information
- ✅ Shows batch and row counts
- ✅ Output file is created successfully

---

## Test 8: Error Handling - Invalid Driver

**Purpose**: Verify appropriate error messages for invalid driver names.

**Command**:
```bash
cargo run -p geoetl-cli -- info crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv -f InvalidDriver --geometry-column geometry
```

**Expected Output**:
- Error message indicating driver not found
- Suggestion to use `drivers` command to list available drivers

**Pass Criteria**:
- ✅ Command exits with non-zero code
- ✅ Error message is clear and helpful
- ✅ Suggests using `drivers` command

---

## Test 9: Error Handling - Missing File

**Purpose**: Verify appropriate error messages for missing input files.

**Command**:
```bash
cargo run -p geoetl-cli -- info nonexistent_file.csv -f CSV --geometry-column geometry
```

**Expected Output**:
- Error message indicating file not found
- Shows both provided path and resolved absolute path

**Pass Criteria**:
- ✅ Command exits with non-zero code
- ✅ Error message clearly indicates file not found
- ✅ Shows the path that was searched

---

## Test 10: Error Handling - Missing Geometry Column for CSV

**Purpose**: Verify that CSV files require the --geometry-column parameter.

**Command**:
```bash
cargo run -p geoetl-cli -- info crates/geoetl-cli/tests/e2e_data/csv/natural-earth_cities_native_AS_WKT.csv -f CSV
```

**Expected Output**:
- Error message indicating --geometry-column is required for CSV files
- Example command showing correct usage

**Pass Criteria**:
- ✅ Command exits with non-zero code
- ✅ Error message clearly states that --geometry-column is required for CSV
- ✅ Shows example command with --geometry-column parameter

---

## Test 11: List Drivers

**Purpose**: Verify the drivers command lists all available format drivers.

**Command**:
```bash
cargo run -p geoetl-cli -- drivers
```

**Expected Output**:
- Table showing available drivers
- Columns: Short Name, Long Name, Info, Read, Write
- At least CSV and GeoJSON drivers shown
- Support status for each operation (Supported/Not Supported)

**Pass Criteria**:
- ✅ Command exits with code 0
- ✅ Table is displayed with driver information
- ✅ CSV driver is listed with Read and Write support
- ✅ GeoJSON driver is listed with Read and Write support

---

## Cleanup

After testing, clean up output files:
```bash
rm -f /tmp/csv_to_geojson.geojson
rm -f /tmp/geojson_to_csv.csv
rm -f /tmp/csv_to_csv.csv
rm -f /tmp/geojson_to_geojson.geojson
rm -f /tmp/verbose_output.csv
rm -f /tmp/test_output.geojson
```

---

## Test Summary Checklist

After running all tests, verify:

- [ ] Info command works for CSV files (with --geometry-column)
- [ ] Info command works for GeoJSON files (without --geometry-column)
- [ ] CSV to GeoJSON conversion works
- [ ] GeoJSON to CSV conversion works
- [ ] CSV to CSV conversion preserves data
- [ ] GeoJSON to GeoJSON conversion preserves data
- [ ] Verbose mode provides useful logging
- [ ] Invalid driver error is helpful
- [ ] Missing file error is helpful
- [ ] Missing geometry column error for CSV is helpful
- [ ] Drivers command lists all formats
- [ ] All test files were created in /tmp
- [ ] Data integrity is maintained through conversions
