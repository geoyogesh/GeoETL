---
sidebar_position: 7
title: ST_Disjoint
description: Test if geometries are disjoint
---

# ST_Disjoint

Tests whether two geometries have no points in common.

## Syntax

```sql
ST_Disjoint(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometries are disjoint, false otherwise |

## Examples

### Separate Polygons

```sql
SELECT ST_Disjoint(
    ST_GeomFromText('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))'),
    ST_GeomFromText('POLYGON((2 2, 3 2, 3 3, 2 3, 2 2))')
);
-- Returns: true
```

### Point Outside Polygon

```sql
SELECT ST_Disjoint(
    ST_Point(10, 10),
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))')
);
-- Returns: true
```

### Find Non-overlapping Features

```sql
SELECT a.id, b.id
FROM layer_a a, layer_b b
WHERE ST_Disjoint(a.geom, b.geom);
```

## Notes

- ST_Disjoint is the exact opposite of ST_Intersects
- NOT ST_Intersects(a, b) is equivalent to ST_Disjoint(a, b)
- Returns true if geometries don't share any points
- Useful for exclusion queries

## See Also

- [ST_Intersects](./st-intersects) - Opposite predicate
- [ST_Touches](./st-touches) - Boundary-only contact
