---
sidebar_position: 1
title: ST_Distance
description: Calculate minimum distance between geometries
---

# ST_Distance

Calculates the minimum Cartesian distance between two geometries.

## Syntax

```sql
ST_Distance(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Float64 | Minimum distance between the two geometries |

## Examples

### Distance Between Two Points

```sql
SELECT ST_Distance(
    ST_Point(0, 0),
    ST_Point(3, 4)
);
-- Returns: 5.0
```

### Distance from Point to Polygon

```sql
SELECT ST_Distance(
    ST_Point(5, 5),
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))')
);
-- Returns: ~1.414 (diagonal distance to nearest corner)
```

### Find Nearby Features

```sql
SELECT id, name, ST_Distance(geom, ST_Point(-122.4, 37.8)) as distance
FROM landmarks
ORDER BY distance
LIMIT 10;
```

### Filter by Distance

```sql
SELECT * FROM buildings
WHERE ST_Distance(geom, ST_Point(-122.4194, 37.7749)) < 1000;
```

## Notes

- Returns 0 when geometries touch or overlap
- Distance is in the same units as the input geometry coordinates
- For geographic coordinates (lat/lon), returns distance in degrees (not meters)
- For meter-based calculations with geographic data, consider projecting to an appropriate local CRS first

## See Also

- [ST_Intersects](../predicate/st-intersects) - Test if geometries intersect
- [ST_Buffer](../generator/st-buffer) - Create buffer at specified distance
