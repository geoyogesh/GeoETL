---
sidebar_position: 6
title: ST_Crosses
description: Test if geometries cross each other
---

# ST_Crosses

Tests whether two geometries cross each other - their interiors intersect but neither contains the other.

## Syntax

```sql
ST_Crosses(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometries cross, false otherwise |

## Examples

### Line Crossing Polygon

```sql
SELECT ST_Crosses(
    ST_GeomFromText('LINESTRING(0 0, 4 4)'),
    ST_GeomFromText('POLYGON((1 0, 3 0, 3 2, 1 2, 1 0))')
);
-- Returns: true
```

### Intersecting Lines

```sql
SELECT ST_Crosses(
    ST_GeomFromText('LINESTRING(0 0, 4 4)'),
    ST_GeomFromText('LINESTRING(0 4, 4 0)')
);
-- Returns: true (lines cross at (2,2))
```

### Find Road-River Intersections

```sql
SELECT r.name as road, v.name as river
FROM roads r, rivers v
WHERE ST_Crosses(r.geom, v.geom);
```

## Notes

- Applies to line/line, line/polygon, and multipoint/line combinations
- For lines: they must intersect at an interior point (not just endpoints)
- For line/polygon: line must enter and exit the polygon
- Does not apply to polygon/polygon (use ST_Overlaps instead)

## See Also

- [ST_Intersects](./st-intersects) - Any shared points
- [ST_Overlaps](./st-overlaps) - Same dimension overlap
- [ST_Touches](./st-touches) - Boundary-only contact
