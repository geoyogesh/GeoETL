---
slug: spatial-area-length-v0-6-0
title: "GeoETL v0.6.0: ST_Area and ST_Length Spatial Functions"
authors: [geoyogesh]
tags: [release, spatial, geos, geoarrow, sql, v0.6.0]
date: 2025-12-07
---

**TL;DR**: GeoETL v0.6.0 adds `ST_Area` and `ST_Length` spatial UDFs for calculating polygon areas and line lengths/perimeters in SQL queries.

<!--truncate-->

## What's New

v0.5.0 introduced spatial distance calculations with `ST_Distance`. v0.6.0 expands the spatial SQL toolkit with **area and length measurements**.

**New Functions**:

| Function | Description |
|----------|-------------|
| `ST_Area` | Calculate area of polygon geometries |
| `ST_Length` | Calculate length of lines or perimeter of polygons |

Both functions are powered by GEOS and work with all geometry input types: WKB binary, GeoArrow native types, and mixed geometry arrays from GeoJSON.

## ST_Area: Calculate Polygon Areas

`ST_Area` calculates the area of polygon geometries. It returns 0 for non-areal geometries like Points and LineStrings.

**Example: Calculate Parcel Areas**

```bash
geoetl-cli convert \
  -i parcels.geojson \
  -o areas.csv \
  --input-driver GeoJSON \
  --output-driver CSV \
  --sql "SELECT parcel_id, owner, ST_Area(geometry) as area_sqm
         FROM parcels
         ORDER BY area_sqm DESC"
```

**Example: Filter by Minimum Area**

```bash
geoetl-cli convert \
  -i buildings.geojson \
  -o large_buildings.geojson \
  --input-driver GeoJSON \
  --output-driver GeoJSON \
  --sql "SELECT * FROM buildings
         WHERE ST_Area(geometry) > 1000"
```

## ST_Length: Calculate Lengths and Perimeters

`ST_Length` calculates the length of LineString geometries or the perimeter of Polygons. Points return 0.

**Example: Calculate Road Lengths**

```bash
geoetl-cli convert \
  -i roads.geojson \
  -o road_lengths.csv \
  --input-driver GeoJSON \
  --output-driver CSV \
  --sql "SELECT road_name, road_type, ST_Length(geometry) as length_m
         FROM roads
         ORDER BY length_m DESC"
```

**Example: Calculate Building Perimeters**

```bash
geoetl-cli convert \
  -i buildings.geojson \
  -o perimeters.csv \
  --input-driver GeoJSON \
  --output-driver CSV \
  --sql "SELECT building_id, ST_Length(geometry) as perimeter
         FROM buildings"
```

**Example: Filter Long Routes**

```bash
geoetl-cli convert \
  -i hiking_trails.geojson \
  -o long_trails.geojson \
  --input-driver GeoJSON \
  --output-driver GeoJSON \
  --sql "SELECT name, difficulty, ST_Length(geometry) as length
         FROM hiking_trails
         WHERE ST_Length(geometry) > 10000"
```

## Combining with ST_Distance

You can combine the new functions with existing spatial UDFs for powerful queries:

```bash
geoetl-cli convert \
  -i parks.geojson \
  -o park_analysis.csv \
  --input-driver GeoJSON \
  --output-driver CSV \
  --sql "SELECT
           name,
           ST_Area(geometry) as area,
           ST_Length(geometry) as perimeter,
           ST_Distance(geometry, ST_Point(-122.4, 37.8)) as dist_from_sf
         FROM parks
         ORDER BY area DESC"
```

## Complete Spatial SQL Toolkit

GeoETL now provides 7 spatial SQL functions:

| Function | Description |
|----------|-------------|
| `ST_Point` / `ST_MakePoint` | Create point from X, Y coordinates |
| `ST_GeomFromText` | Parse WKT to geometry |
| `ST_GeomFromWKB` | Tag WKB binary as geometry |
| `ST_Distance` | Minimum distance between geometries |
| `ST_Area` | Area of polygon geometries |
| `ST_Length` | Length/perimeter of geometries |

## What's Next

**v0.7.0**: `ST_Buffer`, `ST_Contains`, `ST_Intersects` for spatial predicates and buffering.

See the [Roadmap](https://geoetl.com/docs/community/roadmap).

## Links

- [Installation](https://geoetl.com/docs/getting-started/installation)
- [Changelog](https://geoetl.com/docs/community/changelog#060---2025-12-07)
- [Download v0.6.0](https://github.com/geoyogesh/geoetl/releases/tag/v0.6.0)
