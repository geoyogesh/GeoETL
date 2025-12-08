---
sidebar_position: 10
title: ST_CoveredBy
description: Test if geometry A is covered by geometry B
---

# ST_CoveredBy

Tests whether geometry A is covered by geometry B - no point of A is outside B.

## Syntax

```sql
ST_CoveredBy(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | Inner geometry |
| geometry_b | Geometry | Outer geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if A is covered by B, false otherwise |

## Examples

### Point on Polygon Boundary

```sql
SELECT ST_CoveredBy(
    ST_Point(0, 2),
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))')
);
-- Returns: true (point on boundary is covered)
```

### Compare with ST_Within

```sql
SELECT
    ST_Within(point, poly) as within,
    ST_CoveredBy(point, poly) as covered_by
FROM (SELECT
    ST_Point(0, 2) as point,
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))') as poly
);
-- within: false, covered_by: true
```

### Find Points Covered by Region

```sql
SELECT * FROM points
WHERE ST_CoveredBy(geom, (
    SELECT geom FROM regions WHERE name = 'Area A'
));
```

## Notes

- ST_CoveredBy is more inclusive than ST_Within
- ST_Within returns false for points on boundary; ST_CoveredBy returns true
- Use ST_CoveredBy when boundary points should be included
- ST_CoveredBy(A, B) is equivalent to ST_Covers(B, A)

## See Also

- [ST_Within](./st-within) - Stricter within test
- [ST_Covers](./st-covers) - Inverse relationship
- [ST_Contains](./st-contains) - Containment test
