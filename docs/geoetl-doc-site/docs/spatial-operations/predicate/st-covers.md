---
sidebar_position: 9
title: ST_Covers
description: Test if geometry A covers geometry B
---

# ST_Covers

Tests whether geometry A covers geometry B - no point of B is outside A.

## Syntax

```sql
ST_Covers(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | Covering geometry |
| geometry_b | Geometry | Covered geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if A covers B, false otherwise |

## Examples

### Point on Polygon Boundary

```sql
SELECT ST_Covers(
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'),
    ST_Point(0, 2)
);
-- Returns: true (point on boundary is covered)
```

### Compare with ST_Contains

```sql
SELECT
    ST_Contains(poly, point) as contains,
    ST_Covers(poly, point) as covers
FROM (SELECT
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))') as poly,
    ST_Point(0, 2) as point
);
-- contains: false, covers: true
```

### Find All Points in Region (Including Boundary)

```sql
SELECT p.id
FROM points p, regions r
WHERE r.name = 'Zone A'
  AND ST_Covers(r.geom, p.geom);
```

## Notes

- ST_Covers is more inclusive than ST_Contains
- ST_Contains returns false for points on boundary; ST_Covers returns true
- Use ST_Covers when boundary points should be included
- ST_Covers(A, B) is equivalent to ST_CoveredBy(B, A)

## See Also

- [ST_Contains](./st-contains) - Stricter containment
- [ST_CoveredBy](./st-coveredby) - Inverse relationship
- [ST_Within](./st-within) - Interior-only within
