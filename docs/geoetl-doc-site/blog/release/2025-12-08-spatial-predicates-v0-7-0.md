---
slug: spatial-predicates-v0-7-0
title: "GeoETL v0.7.0: Complete Spatial SQL Toolkit with 29 New Functions"
authors: [geoyogesh]
tags: [release, spatial, geos, geoarrow, sql, v0.7.0]
date: 2025-12-08
---

**TL;DR**: GeoETL v0.7.0 adds 29 new spatial SQL functions including predicates (`ST_Intersects`, `ST_Contains`), geometry generators (`ST_Buffer`, `ST_ConvexHull`), validators (`ST_IsValid`), and accessors (`ST_X`, `ST_Y`).

<!--truncate-->

## What's New

v0.6.0 introduced area and length calculations. v0.7.0 delivers a **complete spatial SQL toolkit** with 29 new functions across 6 categories, bringing the total to 36 spatial UDFs.

## Spatial Predicates: Query by Location

Test spatial relationships between geometries with 10 new binary predicates:

| Function | Description |
|----------|-------------|
| `ST_Intersects` | True if geometries share any space |
| `ST_Contains` | True if first geometry contains second |
| `ST_Within` | True if first geometry is within second |
| `ST_Overlaps` | True if geometries overlap but neither contains the other |
| `ST_Touches` | True if geometries touch at boundary only |
| `ST_Crosses` | True if geometries cross each other |
| `ST_Disjoint` | True if geometries share no space |
| `ST_Equals` | True if geometries are spatially equal |
| `ST_Covers` | True if first geometry covers second |
| `ST_CoveredBy` | True if first geometry is covered by second |

**Example: Find Buildings in Flood Zones**

```bash
geoetl-cli convert \
  -i buildings.geojson \
  -o at_risk.geojson \
  --input-driver GeoJSON \
  --output-driver GeoJSON \
  --sql "SELECT b.* FROM buildings b, flood_zones f
         WHERE ST_Intersects(b.geometry, f.geometry)"
```

**Example: Points Within Polygon**

```bash
geoetl-cli convert \
  -i cities.geojson \
  -o cities_in_state.csv \
  --input-driver GeoJSON \
  --output-driver CSV \
  --sql "SELECT name, population
         FROM cities
         WHERE ST_Within(geometry, ST_GeomFromText('POLYGON(...)'))"
```

## Geometry Generators: Create New Shapes

Transform geometries with 6 new generator functions:

| Function | Description |
|----------|-------------|
| `ST_Buffer(geom, distance)` | Create buffer around geometry |
| `ST_Envelope` | Bounding box of geometry |
| `ST_ConvexHull` | Convex hull of geometry |
| `ST_Boundary` | Boundary of geometry |
| `ST_PointOnSurface` | Point guaranteed to be on surface |
| `ST_Simplify(geom, tolerance)` | Douglas-Peucker simplification |

**Example: Create Buffer Zones**

```bash
geoetl-cli convert \
  -i hospitals.geojson \
  -o hospital_zones.geojson \
  --input-driver GeoJSON \
  --output-driver GeoJSON \
  --sql "SELECT name, ST_Buffer(geometry, 1000) as geometry
         FROM hospitals"
```

**Example: Simplify Complex Polygons**

```bash
geoetl-cli convert \
  -i detailed_boundaries.geojson \
  -o simplified.geojson \
  --input-driver GeoJSON \
  --output-driver GeoJSON \
  --sql "SELECT name, ST_Simplify(geometry, 100) as geometry
         FROM detailed_boundaries"
```

## Set Operations: Combine Geometries

Perform geometric set operations with 4 functions:

| Function | Description |
|----------|-------------|
| `ST_Intersection` | Shared area between geometries |
| `ST_Union` | Combined area of geometries |
| `ST_Difference` | First geometry minus second |
| `ST_SymDifference` | Area in either but not both |

**Example: Find Overlapping Areas**

```bash
geoetl-cli convert \
  -i parcels.geojson \
  -o overlaps.geojson \
  --input-driver GeoJSON \
  --output-driver GeoJSON \
  --sql "SELECT ST_Intersection(a.geometry, b.geometry) as geometry
         FROM parcels a, zones b
         WHERE ST_Intersects(a.geometry, b.geometry)"
```

## Geometry Validators: Check Quality

Validate geometry quality with 5 new functions:

| Function | Description |
|----------|-------------|
| `ST_IsValid` | True if geometry is valid |
| `ST_IsEmpty` | True if geometry is empty |
| `ST_IsSimple` | True if geometry has no self-intersection |
| `ST_IsClosed` | True if linestring is closed |
| `ST_IsRing` | True if linestring is closed and simple |

**Example: Find Invalid Geometries**

```bash
geoetl-cli convert \
  -i parcels.geojson \
  -o invalid_parcels.csv \
  --input-driver GeoJSON \
  --output-driver CSV \
  --sql "SELECT parcel_id, ST_IsValid(geometry) as is_valid
         FROM parcels
         WHERE NOT ST_IsValid(geometry)"
```

## Geometry Accessors: Extract Properties

Extract geometry properties with 6 new accessor functions:

| Function | Description |
|----------|-------------|
| `ST_X` | X coordinate of point |
| `ST_Y` | Y coordinate of point |
| `ST_NumPoints` | Count of points in geometry |
| `ST_NumGeometries` | Count of geometries in collection |
| `ST_GeometryType` | Type name (Point, Polygon, etc.) |
| `ST_Dimension` | Topological dimension (0, 1, or 2) |

**Example: Extract Coordinates**

```bash
geoetl-cli convert \
  -i cities.geojson \
  -o coordinates.csv \
  --input-driver GeoJSON \
  --output-driver CSV \
  --sql "SELECT name, ST_X(geometry) as lon, ST_Y(geometry) as lat
         FROM cities"
```

**Example: Filter by Complexity**

```bash
geoetl-cli convert \
  -i roads.geojson \
  -o complex_roads.geojson \
  --input-driver GeoJSON \
  --output-driver GeoJSON \
  --sql "SELECT * FROM roads
         WHERE ST_NumPoints(geometry) > 100"
```

## Complete Spatial SQL Reference

GeoETL v0.7.0 provides 36 spatial SQL functions:

| Category | Functions |
|----------|-----------|
| **Construction** | `ST_Point`, `ST_MakePoint`, `ST_GeomFromText`, `ST_GeomFromWKB` |
| **Measurements** | `ST_Distance`, `ST_Area`, `ST_Length` |
| **Predicates** | `ST_Intersects`, `ST_Contains`, `ST_Within`, `ST_Overlaps`, `ST_Touches`, `ST_Crosses`, `ST_Disjoint`, `ST_Equals`, `ST_Covers`, `ST_CoveredBy` |
| **Generators** | `ST_Buffer`, `ST_Centroid`, `ST_Envelope`, `ST_ConvexHull`, `ST_Boundary`, `ST_PointOnSurface`, `ST_Simplify`, `ST_SimplifyPreserveTopology` |
| **Set Operations** | `ST_Intersection`, `ST_Union`, `ST_Difference`, `ST_SymDifference` |
| **Validators** | `ST_IsValid`, `ST_IsEmpty`, `ST_IsSimple`, `ST_IsClosed`, `ST_IsRing` |
| **Accessors** | `ST_X`, `ST_Y`, `ST_NumPoints`, `ST_NumGeometries`, `ST_GeometryType`, `ST_Dimension` |

## Links

- [Installation](https://geoetl.com/docs/getting-started/installation)
- [Changelog](https://geoetl.com/docs/community/changelog#070---2025-12-07)
- [Download v0.7.0](https://github.com/geoyogesh/geoetl/releases/tag/v0.7.0)
