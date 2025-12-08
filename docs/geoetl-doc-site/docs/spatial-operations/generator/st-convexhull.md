---
sidebar_position: 4
title: ST_ConvexHull
description: Compute convex hull of a geometry
---

# ST_ConvexHull

Returns the smallest convex polygon that contains all points of a geometry.

## Syntax

```sql
ST_ConvexHull(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Convex hull as WKB with `geoarrow.wkb` metadata |

## Examples

### Convex Hull of MultiPoint

```sql
SELECT ST_ConvexHull(ST_GeomFromText('MULTIPOINT(0 0, 4 0, 2 3)'));
-- Returns: triangle polygon
```

### Convex Hull of Concave Polygon

```sql
SELECT ST_ConvexHull(ST_GeomFromText('POLYGON((0 0, 2 0, 2 1, 1 1, 1 2, 0 2, 0 0))'));
-- Returns: POLYGON((0 0, 2 0, 2 1, 1 2, 0 2, 0 0)) - fills in the concave corner
```

### Calculate Convex Hull Area Ratio

```sql
SELECT id,
       ST_Area(geom) as original_area,
       ST_Area(ST_ConvexHull(geom)) as hull_area,
       ST_Area(geom) / ST_Area(ST_ConvexHull(geom)) as convexity
FROM parcels;
```

### Combined Convex Hull

```sql
SELECT ST_ConvexHull(ST_Union(geom)) as combined_hull
FROM points;
```

## Notes

- Think of it as stretching a rubber band around the geometry
- For convex polygons, returns the same polygon
- For concave polygons, fills in the "indentations"
- Useful for computing coverage areas and simplifying complex shapes
- Always returns a valid geometry

## See Also

- [ST_Envelope](./st-envelope) - Axis-aligned bounding box
- [ST_Boundary](./st-boundary) - Geometry boundary
- [ST_Simplify](./st-simplify) - Reduce complexity
