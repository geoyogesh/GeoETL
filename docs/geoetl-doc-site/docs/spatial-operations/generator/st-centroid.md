---
sidebar_position: 2
title: ST_Centroid
description: Calculate centroid point of a geometry
---

# ST_Centroid

Returns the geometric center (centroid) of a geometry.

## Syntax

```sql
ST_Centroid(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Centroid point as WKB with `geoarrow.wkb` metadata |

## Examples

### Centroid of Polygon

```sql
SELECT ST_Centroid(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'));
-- Returns: POINT(2 2)
```

### Centroid of LineString

```sql
SELECT ST_Centroid(ST_GeomFromText('LINESTRING(0 0, 4 0)'));
-- Returns: POINT(2 0)
```

### Calculate Centroids for Labeling

```sql
SELECT id, name, ST_Centroid(geom) as label_point
FROM regions;
```

### Chain with Buffer

```sql
SELECT id, ST_Buffer(ST_Centroid(geom), 100) as center_buffer
FROM parcels;
```

## Notes

- For Points, returns the same point
- For LineStrings, returns the midpoint along the line
- For Polygons, returns the center of mass
- For concave polygons, centroid may be outside the polygon boundary
- Use ST_PointOnSurface for a point guaranteed to be inside

## See Also

- [ST_PointOnSurface](./st-pointonsurface) - Point guaranteed on surface
- [ST_Envelope](./st-envelope) - Bounding box
- [ST_ConvexHull](./st-convexhull) - Convex hull
