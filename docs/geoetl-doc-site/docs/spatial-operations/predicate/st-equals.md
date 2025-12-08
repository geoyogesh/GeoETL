---
sidebar_position: 8
title: ST_Equals
description: Test if geometries are spatially equal
---

# ST_Equals

Tests whether two geometries are spatially equal (represent the same geometry).

## Syntax

```sql
ST_Equals(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometries are spatially equal, false otherwise |

## Examples

### Same Polygon, Different Order

```sql
SELECT ST_Equals(
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'),
    ST_GeomFromText('POLYGON((0 4, 0 0, 4 0, 4 4, 0 4))')
);
-- Returns: true (same polygon, different vertex order)
```

### Reversed LineString

```sql
SELECT ST_Equals(
    ST_GeomFromText('LINESTRING(0 0, 1 1, 2 0)'),
    ST_GeomFromText('LINESTRING(2 0, 1 1, 0 0)')
);
-- Returns: true (same line, reversed direction)
```

### Find Duplicate Geometries

```sql
SELECT a.id as id_a, b.id as id_b
FROM features a, features b
WHERE a.id < b.id
  AND ST_Equals(a.geom, b.geom);
```

## Notes

- Spatial equality ignores vertex order and direction
- Different from binary equality (bytes must match)
- Two geometries are equal if they cover the same space
- Useful for deduplication and data validation

## See Also

- [ST_Intersects](./st-intersects) - Any overlap
- [ST_Contains](./st-contains) - Containment test
