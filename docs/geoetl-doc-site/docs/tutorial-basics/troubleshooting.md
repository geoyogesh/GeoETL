---
sidebar_position: 6
---

# Error Handling & Troubleshooting

GeoETL provides comprehensive error messages with helpful recovery suggestions to guide you when things go wrong.

## Understanding GeoETL Errors

GeoETL uses a structured error system that provides:

- **Clear error messages** explaining what went wrong
- **Contextual information** like file paths and driver names
- **Recovery suggestions** with actionable next steps
- **Error categories** to help identify the type of problem

### Error Categories

GeoETL errors fall into these main categories:

| Category | Description | Example |
|----------|-------------|---------|
| **Driver Errors** | Issues with format drivers | Driver not found, operation not supported |
| **I/O Errors** | File system problems | File not found, permission denied |
| **Format Errors** | Data parsing issues | Invalid geometry, schema mismatch |
| **Configuration Errors** | Invalid options or settings | Missing required parameter |
| **Query Errors** | DataFusion execution issues | SQL query failures |

## Common Errors and Solutions

### Driver Not Found

**What you'll see:**

```bash
$ geoetl-cli convert -i data.csv -o output.xyz \
    --input-driver CSV --output-driver XYZ

Error: Driver 'XYZ' not found.

Available drivers:
  - CSV
  - GeoJSON

Suggestion: Run 'geoetl drivers' to see all available drivers.
```

**Why this happens:**
- The driver name is misspelled or incorrect
- The driver hasn't been implemented yet

**How to fix:**
1. Run `geoetl-cli drivers` to see all available drivers
2. Check the spelling of your driver name
3. Verify the driver supports the operation you need (read/write)

---

### Operation Not Supported

**What you'll see:**

```bash
$ geoetl-cli convert -i data.gml -o output.json \
    --input-driver GML --output-driver GeoJSON

Error: The 'GML' driver does not support reading operation.

Suggestion: Try using a different driver that supports this operation.
```

**Why this happens:**
- The driver exists but doesn't support reading or writing
- Some drivers are read-only or write-only

**How to fix:**
1. Run `geoetl-cli drivers` to check driver capabilities
2. Look for "Yes" in the Read/Write columns
3. Use a different driver that supports your operation

---

### File Not Found

**What you'll see:**

```bash
$ geoetl-cli info data/missing.geojson -f GeoJSON

Error: File not found: 'data/missing.geojson'

Suggestion: Check that the file path is correct and the file exists.
```

**Why this happens:**
- The file path is incorrect
- The file doesn't exist
- You're in the wrong directory

**How to fix:**
1. Verify the file exists: `ls data/missing.geojson`
2. Check your current directory: `pwd`
3. Use an absolute path if needed: `/full/path/to/file.geojson`
4. Check for typos in the filename

---

### Permission Denied

**What you'll see:**

```bash
Error: Permission denied for '/protected/output.csv'

Suggestion: Check file permissions and ensure you have access.
```

**Why this happens:**
- You don't have permission to read the input file
- You don't have permission to write to the output directory
- The file or directory is protected

**How to fix:**
1. Check file permissions: `ls -l /protected/output.csv`
2. Ensure you have write access to the directory
3. Try writing to a different location
4. Use `sudo` if appropriate (be careful!)

---

### Invalid Geometry

**What you'll see:**

```bash
Error: Invalid geometry in CSV at line 42: Invalid WKT format

Suggestion: Validate geometries using a GIS tool before importing.
```

**Why this happens:**
- The WKT geometry string is malformed
- The geometry violates spatial constraints
- Coordinate values are out of range

**How to fix:**
1. Check the geometry at the specified line number
2. Validate WKT syntax (e.g., `POINT(x y)` not `POINT(x,y)`)
3. Ensure coordinates are valid (latitude: -90 to 90, longitude: -180 to 180)
4. Use a GIS tool to validate your data before conversion

---

### Missing Required Option

**What you'll see:**

```bash
$ geoetl-cli info data.csv -f CSV

Error: Missing required option: geometry-column (required for CSV files)

Suggestion: Specify the geometry column name using --geometry-column
```

**Why this happens:**
- CSV files require you to specify which column contains geometries
- A required parameter wasn't provided

**How to fix:**
1. Add the missing parameter:
   ```bash
   geoetl-cli info data.csv -f CSV --geometry-column wkt
   ```
2. Check `geoetl-cli <command> --help` for required parameters

---

### Parse Error

**What you'll see:**

```bash
Error: Parse error in GeoJSON at line 15: Expected property value

Suggestion: Check the file format and ensure it's valid.
```

**Why this happens:**
- The file is malformed or corrupted
- The file format doesn't match the specified driver
- Invalid JSON/CSV syntax

**How to fix:**
1. Open the file and check line 15
2. Validate JSON syntax: Use a JSON validator or `jq`
3. Check for missing commas, quotes, or brackets
4. Ensure the file matches the format you specified

---

### Schema Mismatch

**What you'll see:**

```bash
Error: Field 'population' has incompatible type: expected Integer, found String

Suggestion: Check data types in your input file
```

**Why this happens:**
- Data types don't match expectations
- Mixed types in the same column
- Schema inference failed

**How to fix:**
1. Check the data types in your input file
2. Ensure consistent types throughout a column
3. Fix data type mismatches in your source data

---

## Getting More Information

### Debug Mode

For detailed error information, enable debug logging:

```bash
RUST_LOG=debug geoetl-cli convert -i input.csv -o output.geojson \
    --input-driver CSV --output-driver GeoJSON \
    --geometry-column wkt
```

This will show:
- Full error chains with source errors
- Internal processing steps
- DataFusion query execution details

### Verbose Mode

For less detailed but still helpful information:

```bash
geoetl-cli --verbose convert -i input.csv -o output.geojson \
    --input-driver CSV --output-driver GeoJSON \
    --geometry-column wkt
```

This shows:
- INFO level logging
- File processing progress
- Operation summaries

### Command Help

Get help for any command:

```bash
# General help
geoetl-cli --help

# Command-specific help
geoetl-cli convert --help
geoetl-cli info --help
```

## Best Practices for Error Prevention

### 1. Validate Your Data First

Before converting large datasets:

```bash
# Check dataset info first
geoetl-cli info input.csv -f CSV --geometry-column wkt

# Verify geometry types match expectations
# Check for null values or invalid geometries
```

### 2. Use the Right Driver

```bash
# List all available drivers
geoetl-cli drivers

# Verify driver capabilities before use
# Check that both input and output drivers support your operation
```

### 3. Start Small

Test with a small sample first:

```bash
# Create a test file with just a few records
head -n 10 large-dataset.csv > test-sample.csv

# Test your conversion
geoetl-cli convert -i test-sample.csv -o test-output.geojson \
    --input-driver CSV --output-driver GeoJSON \
    --geometry-column wkt

# If successful, proceed with full dataset
```

### 4. Check File Paths

```bash
# Verify input file exists
ls -l input.csv

# Verify output directory exists and is writable
ls -ld output-directory/

# Use absolute paths to avoid confusion
geoetl-cli convert -i /full/path/to/input.csv \
    -o /full/path/to/output.geojson \
    --input-driver CSV --output-driver GeoJSON
```

### 5. Specify Geometry Information

For CSV files, always provide:

```bash
geoetl-cli convert -i input.csv -o output.geojson \
    --input-driver CSV \
    --output-driver GeoJSON \
    --geometry-column wkt \
    --geometry-type Point  # Specify the geometry type
```

## Still Having Issues?

If you're still experiencing problems:

1. **Check the Documentation**
   - Review the [Common Operations](./common-operations) guide
   - Read format-specific guides like [Working with CSV](./working-with-csv)

2. **Search GitHub Issues**
   - Check if someone else had the same problem
   - Visit: https://github.com/geoyogesh/geoetl/issues

3. **Ask for Help**
   - Open a GitHub Discussion
   - Provide the full error message
   - Include your command and a small sample of your data
   - Visit: https://github.com/geoyogesh/geoetl/discussions

4. **Report a Bug**
   - If you found a bug, open an issue
   - Include steps to reproduce
   - Attach sample data if possible
   - Visit: https://github.com/geoyogesh/geoetl/issues/new

## Summary

GeoETL's error system is designed to help you:
- **Understand** what went wrong with clear messages
- **Recover** quickly with actionable suggestions
- **Prevent** future errors with validation and best practices

Most errors can be resolved by:
- Checking your command syntax
- Verifying file paths and permissions
- Ensuring driver capabilities match your needs
- Validating your data before conversion

Happy data converting! 🚀
