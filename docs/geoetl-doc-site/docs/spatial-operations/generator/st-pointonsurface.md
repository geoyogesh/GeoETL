---
sidebar_position: 6
title: ST_PointOnSurface
description: Get a point guaranteed to be on the surface
---

# ST_PointOnSurface

Returns a point that is guaranteed to lie on the surface of a geometry.

## Syntax

```sql
ST_PointOnSurface(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Point on surface as WKB with `geoarrow.wkb` metadata |

## Examples

### Point on Polygon

```sql
SELECT ST_PointOnSurface(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'));
-- Returns a point inside the polygon (e.g., POINT(2 2))
```

### Point on Concave Polygon

```sql
SELECT ST_PointOnSurface(ST_GeomFromText('POLYGON((0 0, 3 0, 3 1, 1 1, 1 3, 0 3, 0 0))'));
-- Returns a point guaranteed inside the L-shape
```

### Label Placement

```sql
SELECT name, ST_PointOnSurface(geom) as label_point
FROM regions;
```

### Compare with Centroid

```sql
SELECT
    ST_Centroid(geom) as centroid,
    ST_PointOnSurface(geom) as point_on_surface,
    ST_Contains(geom, ST_Centroid(geom)) as centroid_inside,
    ST_Contains(geom, ST_PointOnSurface(geom)) as pos_inside
FROM concave_polygons;
```

## Notes

- Unlike ST_Centroid, always returns a point inside the geometry
- Essential for concave polygons where centroid may be outside
- Useful for label placement and representative point selection
- For Points, returns the same point
- For LineStrings, returns a point on the line

## See Also

- [ST_Centroid](./st-centroid) - Geometric center (may be outside)
- [ST_Contains](../predicate/st-contains) - Test containment
- [ST_Within](../predicate/st-within) - Test if inside
