---
sidebar_position: 2
title: ST_Intersection
description: Intersection of two geometries
---

# ST_Intersection

Returns the intersection (shared area) of two geometries.

## Syntax

```sql
ST_Intersection(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Intersection geometry as WKB with `geoarrow.wkb` metadata |

## Examples

### Intersection of Overlapping Polygons

```sql
SELECT ST_Intersection(
    ST_GeomFromText('POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))'),
    ST_GeomFromText('POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))')
);
-- Returns: POLYGON((1 1, 2 1, 2 2, 1 2, 1 1))
```

### Clip Features to Boundary

```sql
SELECT id, ST_Intersection(feature.geom, boundary.geom) as clipped
FROM features feature, boundaries boundary
WHERE boundary.name = 'Study Area'
  AND ST_Intersects(feature.geom, boundary.geom);
```

### Calculate Overlap Area

```sql
SELECT
    a.id as parcel_a,
    b.id as parcel_b,
    ST_Area(ST_Intersection(a.geom, b.geom)) as overlap_area
FROM parcels a, parcels b
WHERE a.id < b.id
  AND ST_Intersects(a.geom, b.geom);
```

### Line-Polygon Intersection

```sql
SELECT ST_Intersection(
    ST_GeomFromText('LINESTRING(0 0, 10 10)'),
    ST_GeomFromText('POLYGON((2 2, 8 2, 8 8, 2 8, 2 2))')
);
-- Returns: LINESTRING(2 2, 8 8) - portion of line inside polygon
```

## Notes

- Returns empty geometry if no intersection exists
- Result type depends on intersection geometry (point, line, or polygon)
- Always check ST_Intersects first for better performance
- Useful for clipping features to study areas

## See Also

- [ST_Intersects](../predicate/st-intersects) - Test for intersection
- [ST_Union](./st-union) - Combine geometries
- [ST_Difference](./st-difference) - Subtract geometries
