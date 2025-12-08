---
sidebar_position: 1
title: ST_Intersects
description: Test if two geometries intersect
---

# ST_Intersects

Tests whether two geometries have any point in common (intersect).

## Syntax

```sql
ST_Intersects(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometries intersect, false otherwise |

## Examples

### Test Point in Polygon

```sql
SELECT ST_Intersects(
    ST_Point(2, 2),
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))')
);
-- Returns: true
```

### Spatial Join

```sql
SELECT a.id, b.name
FROM points a, regions b
WHERE ST_Intersects(a.geom, b.geom);
```

### Find Overlapping Features

```sql
SELECT a.id as id_a, b.id as id_b
FROM parcels a, parcels b
WHERE a.id < b.id
  AND ST_Intersects(a.geom, b.geom);
```

## Notes

- Returns true if geometries share any point (interior or boundary)
- Opposite of ST_Disjoint
- Most commonly used spatial predicate for spatial joins
- Uses GEOS for computation

## See Also

- [ST_Disjoint](./st-disjoint) - Test if geometries don't intersect
- [ST_Contains](./st-contains) - Test if geometry contains another
- [ST_Within](./st-within) - Test if geometry is within another
