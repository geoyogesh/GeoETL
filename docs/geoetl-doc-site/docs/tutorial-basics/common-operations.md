---
sidebar_position: 5
---

# Common Operations

Master the essential GeoETL commands and operations you'll use every day.

## The Three Core Commands

GeoETL has three main commands:

1. **`drivers`** - List available format drivers
2. **`convert`** - Convert between formats
3. **`info`** - Display dataset information (coming soon)

## Command: `drivers`

List all supported format drivers and their capabilities.

### Basic Usage

```bash
geoetl-cli drivers
```

### What You See

A table showing:
- **Short Name**: Driver identifier
- **Long Name**: Full description
- **Info/Read/Write**: Capability status

### Filtering Output

Search for specific drivers:

```bash
# Find all JSON-related drivers
geoetl-cli drivers | grep -i json

# Find database drivers
geoetl-cli drivers | grep -i "sql\|database\|mongo"

# Find only supported drivers
geoetl-cli drivers | grep "Supported"

# Count total drivers
geoetl-cli drivers | wc -l
```

### Use Cases

**Before conversion**: Check if a format is supported
```bash
geoetl-cli drivers | grep -i "shapefile"
```

**Learn about formats**: See what's available
```bash
geoetl-cli drivers | less  # Scroll through list
```

## Command: `convert`

Convert geospatial data between formats.

### Basic Syntax

```bash
geoetl-cli convert \
  --input <file> \
  --output <file> \
  --input-driver <DRIVER> \
  --output-driver <DRIVER>
```

### Short Flags

```bash
# These are equivalent
geoetl-cli convert --input in.geojson --output out.csv \
  --input-driver GeoJSON --output-driver CSV

geoetl-cli convert -i in.geojson -o out.csv \
  --input-driver GeoJSON --output-driver CSV
```

### Common Conversions

#### GeoJSON to CSV
```bash
geoetl-cli convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV
```

#### CSV to GeoJSON
```bash
geoetl-cli convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column geometry
```

#### CSV to CSV (round-trip)
```bash
geoetl-cli convert -i input.csv -o output.csv \
  --input-driver CSV --output-driver CSV \
  --geometry-column wkt
```

### Advanced Options

#### Custom Geometry Column
```bash
geoetl-cli convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column location_wkt
```

#### Specify Geometry Type
```bash
geoetl-cli convert -i points.csv -o points.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column wkt \
  --geometry-type Point
```

## Global Options

These work with any command:

### Verbose Output (`-v`, `--verbose`)

See detailed logging information:

```bash
geoetl-cli -v convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV
```

**Shows**:
- Input/output file paths
- Drivers being used
- Number of records processed
- Timing information

**Example output**:
```
INFO Converting data.geojson to data.csv
INFO Input: data.geojson (Driver: GeoJSON)
INFO Output: data.csv (Driver: CSV)
INFO Read 1 record batch(es)
INFO Total rows: 243
INFO Conversion complete
```

### Debug Output (`-d`, `--debug`)

Even more detailed logging for troubleshooting:

```bash
geoetl-cli -d convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV
```

**Shows**:
- All verbose output
- Internal function calls
- DataFusion query plans
- Memory usage details

**Use when**:
- Investigating errors
- Reporting bugs
- Performance debugging

### Help (`--help`, `-h`)

Get help for any command:

```bash
# General help
geoetl-cli --help

# Command-specific help
geoetl-cli convert --help
geoetl-cli drivers --help

# Show version
geoetl-cli --version
```

## Common Workflows

### Workflow 1: Check Format Support

```bash
# 1. Check if format is supported
geoetl-cli drivers | grep -i "parquet"

# 2. If not supported yet, choose alternative
geoetl-cli drivers | grep "Supported"

# 3. Proceed with supported format
geoetl-cli convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV
```

### Workflow 2: Inspect Then Convert

```bash
# 1. Check file exists
ls -lh data.geojson

# 2. Validate it's readable (coming soon: geoetl-cli info)
head data.geojson

# 3. Convert
geoetl-cli -v convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV

# 4. Verify output
head data.csv
wc -l data.csv
```

### Workflow 3: Round-Trip Test

```bash
# 1. Convert to intermediate format
geoetl-cli convert -i original.geojson -o temp.csv \
  --input-driver GeoJSON --output-driver CSV

# 2. Convert back
geoetl-cli convert -i temp.csv -o recovered.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column geometry

# 3. Compare
diff original.geojson recovered.geojson
```

### Workflow 4: Process Multiple Files

```bash
# Using a for loop (bash)
for file in *.geojson; do
  output="${file%.geojson}.csv"
  geoetl-cli convert -i "$file" -o "$output" \
    --input-driver GeoJSON --output-driver CSV
done
```

## File Path Handling

### Relative Paths

```bash
# Current directory
geoetl-cli convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV

# Subdirectory
geoetl-cli convert -i input/data.geojson -o output/data.csv \
  --input-driver GeoJSON --output-driver CSV

# Parent directory
geoetl-cli convert -i ../data.geojson -o ./data.csv \
  --input-driver GeoJSON --output-driver CSV
```

### Absolute Paths

```bash
# Linux/macOS
geoetl-cli convert -i /home/user/data.geojson -o /tmp/data.csv \
  --input-driver GeoJSON --output-driver CSV

# Windows
geoetl-cli convert -i C:\Users\user\data.geojson -o C:\Temp\data.csv \
  --input-driver GeoJSON --output-driver CSV
```

### Paths with Spaces

```bash
# Use quotes
geoetl-cli convert -i "My Data/cities.geojson" -o "Output Data/cities.csv" \
  --input-driver GeoJSON --output-driver CSV

# Or escape spaces (bash)
geoetl-cli convert -i My\ Data/cities.geojson -o Output\ Data/cities.csv \
  --input-driver GeoJSON --output-driver CSV
```

## Output Control

### Overwriting Files

GeoETL will overwrite existing output files:

```bash
# This will replace data.csv if it exists
geoetl-cli convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV
```

To avoid accidental overwrites:

```bash
# Check if output exists
if [ -f data.csv ]; then
  echo "Output file exists! Use different name."
  exit 1
fi

# Then convert
geoetl-cli convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV
```

### Creating Output Directories

```bash
# Create directory if it doesn't exist
mkdir -p output

# Then convert
geoetl-cli convert -i data.geojson -o output/data.csv \
  --input-driver GeoJSON --output-driver CSV
```

## Performance Tips

### For Small Files (< 10MB)

Normal conversion is fine:

```bash
geoetl-cli convert -i small.geojson -o small.csv \
  --input-driver GeoJSON --output-driver CSV
```

### For Medium Files (10-100MB)

Use verbose mode to monitor progress:

```bash
geoetl-cli -v convert -i medium.geojson -o medium.csv \
  --input-driver GeoJSON --output-driver CSV
```

### For Large Files (> 100MB)

Monitor progress and timing:

```bash
time geoetl-cli -v convert -i large.geojson -o large.csv \
  --input-driver GeoJSON --output-driver CSV
```

## Error Handling

### Common Errors

#### 1. File Not Found
```
Error: No such file or directory
```

**Solution**:
```bash
# Check file exists
ls -la data.geojson

# Use correct path
geoetl-cli convert -i ./data.geojson -o ./data.csv \
  --input-driver GeoJSON --output-driver CSV
```

#### 2. Driver Not Found
```
Error: Input driver 'geojson' not found
```

**Solution**: Use correct case
```bash
# ❌ Wrong
--input-driver geojson

# ✅ Correct
--input-driver GeoJSON
```

#### 3. Driver Doesn't Support Operation
```
Error: Input driver 'GML' does not support reading
```

**Solution**: Check driver capabilities
```bash
geoetl-cli drivers | grep -i "GML"
# Use a supported driver instead
```

#### 4. Invalid Geometry
```
Error: Failed to parse WKT
```

**Solution**: Check WKT format
```bash
# Use debug mode to see details
geoetl-cli -d convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON
```

## Shell Integration

### Bash Scripts

```bash
#!/bin/bash
# convert_all.sh - Convert all GeoJSON files to CSV

for file in *.geojson; do
  output="${file%.geojson}.csv"
  echo "Converting $file to $output..."

  geoetl-cli convert -i "$file" -o "$output" \
    --input-driver GeoJSON --output-driver CSV

  if [ $? -eq 0 ]; then
    echo "✓ Success: $output"
  else
    echo "✗ Failed: $file"
  fi
done
```

### PowerShell Scripts

```powershell
# convert_all.ps1 - Convert all GeoJSON files to CSV

Get-ChildItem -Filter *.geojson | ForEach-Object {
  $output = $_.Name -replace '\.geojson$', '.csv'
  Write-Host "Converting $($_.Name) to $output..."

  geoetl-cli convert -i $_.Name -o $output `
    --input-driver GeoJSON --output-driver CSV

  if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Success: $output"
  } else {
    Write-Host "✗ Failed: $($_.Name)"
  }
}
```

### Make Commands

```makefile
# Makefile for GeoETL operations

.PHONY: all clean

all: output/cities.csv output/roads.csv

output/%.csv: input/%.geojson
  mkdir -p output
  geoetl-cli convert -i $< -o $@ \
    --input-driver GeoJSON --output-driver CSV

clean:
  rm -rf output
```

## Quick Reference

### Essential Commands

```bash
# List drivers
geoetl-cli drivers

# Convert files
geoetl-cli convert -i input.geojson -o output.csv \
  --input-driver GeoJSON --output-driver CSV

# Verbose output
geoetl-cli -v convert -i input.geojson -o output.csv \
  --input-driver GeoJSON --output-driver CSV

# Get help
geoetl-cli --help
geoetl-cli convert --help

# Show version
geoetl-cli --version
```

### File Operations

```bash
# Check file size
ls -lh data.geojson

# Preview file
head -n 20 data.csv

# Count records
wc -l data.csv

# Search in output
grep -i "california" data.csv
```

## Key Takeaways

🎯 **What you learned**:
- The three core GeoETL commands
- Common conversion patterns
- Global options (verbose, debug, help)
- File path handling
- Performance tips
- Error handling strategies
- Shell integration

🚀 **Skills unlocked**:
- Daily GeoETL operations
- Troubleshooting conversions
- Automating workflows
- Working efficiently with geospatial data

## Next Steps

You've completed the beginner tutorials! 🎉

You now know how to:
- Install and configure GeoETL
- Convert between different geospatial formats
- Work with CSV and GeoJSON data
- Use the core commands effectively
- Troubleshoot common issues

Ready to do more? Check out the [GeoETL GitHub repository](https://github.com/geoyogesh/geoetl) for advanced usage examples and contribution opportunities.

## Need Help?

- **Command help**: `geoetl-cli <command> --help`
- **GitHub Issues**: [Report problems](https://github.com/geoyogesh/geoetl/issues)
- **GitHub Discussions**: [Ask questions](https://github.com/geoyogesh/geoetl/discussions)
- **User Guide**: [Full documentation](https://github.com/geoyogesh/geoetl/blob/main/docs/USERGUIDE.md)
