---
sidebar_position: 0
title: Supported Operations
description: Overview of all spatial SQL functions available in GeoETL
---

# Supported Spatial Operations

GeoETL provides 39 PostGIS-compatible spatial SQL functions powered by the GEOS library. These functions enable geometry manipulation, spatial analysis, and data transformation within your SQL queries.

## Quick Reference

| Category | Functions | Description |
|----------|-----------|-------------|
| [Construction](#construction-functions) | 4 | Create geometries from various input formats |
| [Measurement](#measurement-functions) | 3 | Measure distance, area, and length |
| [Predicate](#predicate-functions) | 10 | Test spatial relationships |
| [Validator](#validator-functions) | 5 | Validate geometry properties |
| [Generator](#generator-functions) | 8 | Generate new geometries |
| [Set Operations](#set-operation-functions) | 4 | Combine geometries using set theory |
| [Accessor](#accessor-functions) | 6 | Extract geometry properties |

## All Functions

### Construction Functions

| Function | Description |
|----------|-------------|
| [ST_GeomFromText](./construction/st-geomfromtext) | Parse WKT string to geometry |
| [ST_GeomFromWKB](./construction/st-geomfromwkb) | Validate WKB binary data |
| [ST_Point](./construction/st-point) | Create point from X, Y coordinates |
| [ST_MakePoint](./construction/st-makepoint) | Alias for ST_Point |

### Measurement Functions

| Function | Description |
|----------|-------------|
| [ST_Distance](./measurement/st-distance) | Calculate minimum distance between geometries |
| [ST_Area](./measurement/st-area) | Calculate area of a geometry |
| [ST_Length](./measurement/st-length) | Calculate length/perimeter of a geometry |

### Predicate Functions

| Function | Description |
|----------|-------------|
| [ST_Intersects](./predicate/st-intersects) | Test if two geometries intersect |
| [ST_Contains](./predicate/st-contains) | Test if geometry A contains geometry B |
| [ST_Within](./predicate/st-within) | Test if geometry A is within geometry B |
| [ST_Overlaps](./predicate/st-overlaps) | Test if two geometries overlap |
| [ST_Touches](./predicate/st-touches) | Test if geometries touch at boundaries |
| [ST_Crosses](./predicate/st-crosses) | Test if geometries cross each other |
| [ST_Disjoint](./predicate/st-disjoint) | Test if geometries are disjoint |
| [ST_Equals](./predicate/st-equals) | Test if geometries are spatially equal |
| [ST_Covers](./predicate/st-covers) | Test if geometry A covers geometry B |
| [ST_CoveredBy](./predicate/st-coveredby) | Test if geometry A is covered by geometry B |

### Validator Functions

| Function | Description |
|----------|-------------|
| [ST_IsValid](./validator/st-isvalid) | Test if geometry is valid (OGC rules) |
| [ST_IsEmpty](./validator/st-isempty) | Test if geometry is empty |
| [ST_IsSimple](./validator/st-issimple) | Test if geometry is simple |
| [ST_IsClosed](./validator/st-isclosed) | Test if geometry is closed |
| [ST_IsRing](./validator/st-isring) | Test if geometry is a ring |

### Generator Functions

| Function | Description |
|----------|-------------|
| [ST_Buffer](./generator/st-buffer) | Create buffer polygon around geometry |
| [ST_Centroid](./generator/st-centroid) | Calculate centroid point |
| [ST_Envelope](./generator/st-envelope) | Compute bounding box |
| [ST_ConvexHull](./generator/st-convexhull) | Compute convex hull |
| [ST_Boundary](./generator/st-boundary) | Compute geometry boundary |
| [ST_PointOnSurface](./generator/st-pointonsurface) | Get point guaranteed on surface |
| [ST_Simplify](./generator/st-simplify) | Douglas-Peucker simplification |
| [ST_SimplifyPreserveTopology](./generator/st-simplifypreservetopology) | Topology-preserving simplification |

### Set Operation Functions

| Function | Description |
|----------|-------------|
| [ST_Union](./set-operation/st-union) | Combine two geometries |
| [ST_Intersection](./set-operation/st-intersection) | Intersection of two geometries |
| [ST_Difference](./set-operation/st-difference) | Difference of geometries (A - B) |
| [ST_SymDifference](./set-operation/st-symdifference) | Symmetric difference (XOR) |

### Accessor Functions

| Function | Description |
|----------|-------------|
| [ST_X](./accessor/st-x) | Get X coordinate of a Point |
| [ST_Y](./accessor/st-y) | Get Y coordinate of a Point |
| [ST_NumPoints](./accessor/st-numpoints) | Count of points in geometry |
| [ST_NumGeometries](./accessor/st-numgeometries) | Count of geometries in collection |
| [ST_GeometryType](./accessor/st-geometrytype) | Get geometry type as string |
| [ST_Dimension](./accessor/st-dimension) | Get topological dimension |

## Usage Examples

### Basic Geometry Creation

```sql
SELECT ST_Point(longitude, latitude) as geom FROM locations;
SELECT ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))') as polygon;
```

### Spatial Analysis

```sql
-- Find all buildings within 100 meters of a point
SELECT * FROM buildings
WHERE ST_Distance(geom, ST_Point(-122.4, 37.8)) < 100;

-- Calculate total area by category
SELECT category, SUM(ST_Area(geom)) as total_area
FROM parcels
GROUP BY category;
```

### Geometry Transformation

```sql
-- Create 50-meter buffer zones
SELECT id, ST_Buffer(geom, 50) as buffer FROM features;

-- Simplify complex geometries
SELECT id, ST_Simplify(geom, 0.001) as simplified FROM coastlines;
```

### Spatial Filtering

```sql
-- Find intersecting features
SELECT a.id, b.id
FROM layer_a a, layer_b b
WHERE ST_Intersects(a.geom, b.geom);

-- Filter by containment
SELECT * FROM points
WHERE ST_Within(geom, (SELECT geom FROM boundaries WHERE name = 'Region A'));
```

## Notes

- All functions use the GEOS library for computational geometry
- Input geometries can be in GeoArrow Point, WKB, or mixed geometry formats
- Output geometries are returned as WKB with `geoarrow.wkb` metadata
- NULL inputs return NULL outputs (SQL standard behavior)
- Functions are case-insensitive in SQL queries
