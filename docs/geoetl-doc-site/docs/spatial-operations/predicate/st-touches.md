---
sidebar_position: 5
title: ST_Touches
description: Test if geometries touch at boundaries
---

# ST_Touches

Tests whether two geometries touch at their boundaries but their interiors do not intersect.

## Syntax

```sql
ST_Touches(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometries touch, false otherwise |

## Examples

### Adjacent Polygons

```sql
SELECT ST_Touches(
    ST_GeomFromText('POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))'),
    ST_GeomFromText('POLYGON((2 0, 4 0, 4 2, 2 2, 2 0))')
);
-- Returns: true (share edge at x=2)
```

### Point on Polygon Boundary

```sql
SELECT ST_Touches(
    ST_Point(0, 1),
    ST_GeomFromText('POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))')
);
-- Returns: true (point is on boundary)
```

### Find Adjacent Regions

```sql
SELECT a.name as region_a, b.name as region_b
FROM regions a, regions b
WHERE a.id < b.id
  AND ST_Touches(a.geom, b.geom);
```

## Notes

- Geometries touch if they share boundary points but no interior points
- Useful for finding adjacent parcels or neighboring regions
- Two points can never touch (they either equal or disjoint)
- Returns false if interiors overlap

## See Also

- [ST_Intersects](./st-intersects) - Any shared points
- [ST_Disjoint](./st-disjoint) - No shared points
- [ST_Overlaps](./st-overlaps) - Partial interior overlap
