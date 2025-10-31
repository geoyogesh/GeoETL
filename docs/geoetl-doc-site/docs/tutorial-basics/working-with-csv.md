---
sidebar_position: 4
---

# Working with CSV

Master CSV operations in GeoETL, including WKT geometries, custom columns, and common patterns.

## CSV + Geospatial Data

CSV files can store geospatial data by including geometry in **WKT (Well-Known Text)** format. This makes them:

- ✅ Human-readable
- ✅ Excel-compatible
- ✅ Easy to edit
- ✅ Version control friendly
- ✅ Widely supported

## WKT Geometry Format

### What is WKT?

WKT (Well-Known Text) represents geometries as text strings:

```
POINT(longitude latitude)
LINESTRING(lon1 lat1, lon2 lat2, lon3 lat3)
POLYGON((lon1 lat1, lon2 lat2, lon3 lat3, lon1 lat1))
```

### Common WKT Types

| Type | Example | Use Case |
|------|---------|----------|
| Point | `POINT(-122.4194 37.7749)` | Cities, landmarks |
| LineString | `LINESTRING(-122 37, -121 38)` | Roads, rivers |
| Polygon | `POLYGON((-122 37, -121 37, -121 38, -122 38, -122 37))` | Boundaries, areas |
| MultiPoint | `MULTIPOINT((-122 37), (-121 38))` | Multiple locations |
| MultiLineString | `MULTILINESTRING((...), (...))` | River networks |
| MultiPolygon | `MULTIPOLYGON(((...)), ((...)))` | Islands, regions |

:::tip
Coordinates in WKT are typically `longitude latitude` (x y), not latitude longitude!
:::

## Creating CSV with Geometries

### Example: Cities CSV

Create a file named `cities.csv`:

```csv
name,population,country,geometry
Tokyo,13960000,Japan,"POINT(139.6917 35.6895)"
Delhi,32941000,India,"POINT(77.2090 28.6139)"
Shanghai,27796000,China,"POINT(121.4737 31.2304)"
São Paulo,22237000,Brazil,"POINT(-46.6333 -23.5505)"
Mexico City,22085000,Mexico,"POINT(-99.1332 19.4326)"
```

Key points:
- **Header row**: Column names in first row
- **Geometry column**: Contains WKT strings
- **Quotes**: Use quotes around WKT if it contains commas
- **Coordinates**: Longitude (x) comes before latitude (y)

## Converting CSV to GeoJSON

### Basic Conversion

```bash
geoetl-cli convert \
  -i cities.csv \
  -o cities.geojson \
  --input-driver CSV \
  --output-driver GeoJSON
```

By default, GeoETL looks for a column named "geometry".

### Custom Geometry Column

If your geometry column has a different name:

```csv
name,population,wkt
Tokyo,13960000,"POINT(139.6917 35.6895)"
Delhi,32941000,"POINT(77.2090 28.6139)"
```

Use `--geometry-column`:

```bash
geoetl-cli convert \
  -i cities.csv \
  -o cities.geojson \
  --input-driver CSV \
  --output-driver GeoJSON \
  --geometry-column wkt
```

### Specifying Geometry Type

For optimization, you can specify the geometry type:

```bash
geoetl-cli convert \
  -i points.csv \
  -o points.geojson \
  --input-driver CSV \
  --output-driver GeoJSON \
  --geometry-column wkt \
  --geometry-type Point
```

Supported types:
- `Point`
- `LineString`
- `Polygon`
- `MultiPoint`
- `MultiLineString`
- `MultiPolygon`
- `Geometry` (mixed types - default)

## Converting GeoJSON to CSV

### Basic Conversion

```bash
geoetl-cli convert \
  -i cities.geojson \
  -o cities.csv \
  --input-driver GeoJSON \
  --output-driver CSV
```

This creates a CSV with:
- All properties as columns
- Geometry column with WKT strings

### Custom Geometry Column Name

Currently, the geometry column is always named "geometry". In future versions, you'll be able to customize this.

## Common CSV Patterns

### Pattern 1: Lat/Lon Columns

If your CSV has separate latitude and longitude columns:

```csv
name,lat,lon,population
Tokyo,35.6895,139.6917,13960000
```

You need to create WKT first:

**Option A: Use a spreadsheet formula**
```
="POINT(" & C2 & " " & B2 & ")"
```

**Option B: Use command-line tools**
```bash
# Using awk
awk -F',' 'NR==1{print $0",geometry"; next} {print $0",\"POINT("$3" "$2")\""}' input.csv > output.csv
```

Then convert:
```bash
geoetl-cli convert -i output.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON
```

### Pattern 2: Round-trip Conversion

Convert GeoJSON → CSV → GeoJSON to verify data integrity:

```bash
# Original to CSV
geoetl-cli convert -i original.geojson -o temp.csv \
  --input-driver GeoJSON --output-driver CSV

# CSV back to GeoJSON
geoetl-cli convert -i temp.csv -o recovered.geojson \
  --input-driver CSV --output-driver GeoJSON

# Compare files
diff original.geojson recovered.geojson
```

### Pattern 3: Adding Data to CSV

Edit CSV in Excel/spreadsheet:

1. Convert GeoJSON to CSV
2. Open in Excel/Google Sheets
3. Add/modify columns
4. Save as CSV (UTF-8)
5. Convert back to GeoJSON

```bash
# Step 1: GeoJSON to CSV
geoetl-cli convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV

# Steps 2-4: Edit in spreadsheet

# Step 5: CSV back to GeoJSON
geoetl-cli convert -i data.csv -o data_updated.geojson \
  --input-driver CSV --output-driver GeoJSON
```

## CSV Best Practices

### ✅ Do's

1. **Use UTF-8 encoding**
   ```bash
   # Check encoding
   file -i data.csv
   # Should show: charset=utf-8
   ```

2. **Quote WKT strings**
   ```csv
   "POINT(-122.4194 37.7749)"
   ```

3. **Include header row**
   ```csv
   name,geometry
   San Francisco,"POINT(-122.4194 37.7749)"
   ```

4. **Use consistent column names**
   - Lowercase: `name`, `population`
   - Avoid spaces: use `snake_case` or `camelCase`

5. **Order coordinates correctly**
   ```
   POINT(longitude latitude)  ✅
   POINT(latitude longitude)  ❌
   ```

### ❌ Don'ts

1. **Don't use latitude/longitude in wrong order**
   ```
   POINT(latitude longitude)  ❌ Wrong!
   ```

2. **Don't skip the header row**
   ```csv
   Tokyo,13960000,"POINT(139.6917 35.6895)"  ❌ No header
   ```

3. **Don't use inconsistent geometry column names**
   ```csv
   name,geom
   Tokyo,"POINT(139 35)"
   Delhi,"POINT(77 28)"  ❌ Mix of geom/geometry
   ```

4. **Don't mix geometry types without specifying**
   ```csv
   name,geometry
   Point1,"POINT(139 35)"
   Line1,"LINESTRING(139 35, 140 36)"  ⚠️ Mixed types
   ```
   Use `--geometry-type Geometry` for mixed types.

## Troubleshooting CSV Issues

### Issue: Invalid WKT

**Error**: `Failed to parse WKT`

**Common causes**:
```csv
# ❌ Missing coordinates
"POINT()"

# ❌ Wrong format
"POINT: -122, 37"

# ❌ Extra spaces
"POINT ( -122  37 )"
```

**Solutions**:
```csv
# ✅ Correct format
"POINT(-122.4194 37.7749)"
```

### Issue: Geometry Column Not Found

**Error**: `Geometry column 'geometry' not found`

**Solution**: Specify the correct column name
```bash
geoetl-cli convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column wkt  # or whatever your column is named
```

### Issue: Encoding Problems

**Symptoms**: Special characters appear as `�` or weird symbols

**Solution**: Ensure UTF-8 encoding
```bash
# Convert to UTF-8 (Linux/macOS)
iconv -f ISO-8859-1 -t UTF-8 input.csv > output.csv

# Then convert with GeoETL
geoetl-cli convert -i output.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON
```

### Issue: Empty Output

**Symptoms**: Conversion succeeds but output is empty

**Possible causes**:
1. Input CSV is empty
2. No geometry column found
3. All WKT strings are invalid

**Debug with verbose mode**:
```bash
geoetl-cli -v convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column geometry
```

## Advanced CSV Operations

### Filtering Data

Currently, GeoETL converts entire files. To filter:

```bash
# Filter before conversion (using grep/awk)
grep "California" cities.csv > california.csv

geoetl-cli convert -i california.csv -o california.geojson \
  --input-driver CSV --output-driver GeoJSON
```

### Combining Multiple CSV Files

```bash
# Combine CSVs (keep only one header)
head -1 file1.csv > combined.csv
tail -n +2 file1.csv >> combined.csv
tail -n +2 file2.csv >> combined.csv

# Convert combined file
geoetl-cli convert -i combined.csv -o combined.geojson \
  --input-driver CSV --output-driver GeoJSON
```

### Large CSV Files

For large files (100MB+):

```bash
# Check file size first
ls -lh large_data.csv

# Use verbose mode to monitor progress
geoetl-cli -v convert -i large_data.csv -o large_data.geojson \
  --input-driver CSV --output-driver GeoJSON
```

## Real-World Examples

### Example 1: Excel to GeoJSON

```bash
# 1. Export from Excel as CSV (UTF-8)
# 2. Convert with GeoETL
geoetl-cli convert -i excel_export.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column location_wkt
```

### Example 2: Database Export

```bash
# 1. Export from database to CSV with ST_AsText()
# Example SQL: SELECT id, name, ST_AsText(geom) as wkt FROM cities

# 2. Convert to GeoJSON
geoetl-cli convert -i db_export.csv -o cities.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column wkt
```

### Example 3: Web Data Collection

```bash
# 1. Collect data with WKT geometry strings
# 2. Save as CSV
# 3. Convert for web mapping
geoetl-cli convert -i collected_data.csv -o map_data.geojson \
  --input-driver CSV --output-driver GeoJSON
```

## Quick Reference

```bash
# Basic CSV to GeoJSON
geoetl-cli convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON

# Custom geometry column
geoetl-cli convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column wkt

# Specify geometry type
geoetl-cli convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON \
  --geometry-column wkt \
  --geometry-type Point

# GeoJSON to CSV
geoetl-cli convert -i data.geojson -o data.csv \
  --input-driver GeoJSON --output-driver CSV

# Verbose output
geoetl-cli -v convert -i data.csv -o data.geojson \
  --input-driver CSV --output-driver GeoJSON
```

## Key Takeaways

🎯 **What you learned**:
- WKT geometry format and syntax
- Creating CSV files with geometries
- Using custom geometry columns
- Round-trip conversions
- Best practices and common pitfalls
- Troubleshooting CSV issues

🚀 **Skills unlocked**:
- Working with spreadsheets and geospatial data
- Converting between CSV and GeoJSON
- Handling different geometry types
- Debugging conversion problems

## Next Steps

Continue learning:

👉 **Next: [Common Operations](./common-operations)** - Essential GeoETL commands and workflows

## Need Help?

- **WKT Reference**: [Wikipedia - Well-known text](https://en.wikipedia.org/wiki/Well-known_text_representation_of_geometry)
- **Command help**: `geoetl-cli convert --help`
- **GitHub Issues**: [Report problems](https://github.com/geoyogesh/geoetl/issues)
- **GitHub Discussions**: [Ask questions](https://github.com/geoyogesh/geoetl/discussions)
