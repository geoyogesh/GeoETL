---
sidebar_position: 4
title: ST_Overlaps
description: Test if two geometries overlap
---

# ST_Overlaps

Tests whether two geometries overlap - sharing some but not all interior points.

## Syntax

```sql
ST_Overlaps(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometries overlap, false otherwise |

## Examples

### Test Overlapping Polygons

```sql
SELECT ST_Overlaps(
    ST_GeomFromText('POLYGON((0 0, 3 0, 3 3, 0 3, 0 0))'),
    ST_GeomFromText('POLYGON((2 2, 5 2, 5 5, 2 5, 2 2))')
);
-- Returns: true
```

### Find Overlapping Parcels

```sql
SELECT a.id as parcel_a, b.id as parcel_b
FROM parcels a, parcels b
WHERE a.id < b.id
  AND ST_Overlaps(a.geom, b.geom);
```

### Detect Data Quality Issues

```sql
SELECT COUNT(*) as overlap_count
FROM buildings a, buildings b
WHERE a.id < b.id
  AND ST_Overlaps(a.geom, b.geom);
```

## Notes

- Overlaps means geometries share some but not all points
- Both geometries must be of the same dimension (polygon/polygon or line/line)
- Returns false if one geometry contains the other entirely
- Returns false for point/polygon or line/polygon comparisons
- Use ST_Intersects for general intersection tests

## See Also

- [ST_Intersects](./st-intersects) - Any shared points
- [ST_Contains](./st-contains) - Complete containment
- [ST_Crosses](./st-crosses) - Different dimension crossing
