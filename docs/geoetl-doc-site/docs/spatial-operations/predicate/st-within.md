---
sidebar_position: 3
title: ST_Within
description: Test if geometry A is within geometry B
---

# ST_Within

Tests whether geometry A is completely within geometry B.

## Syntax

```sql
ST_Within(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | Inner geometry |
| geometry_b | Geometry | Outer geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if A is within B, false otherwise |

## Examples

### Test Point Within Polygon

```sql
SELECT ST_Within(
    ST_Point(2, 2),
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))')
);
-- Returns: true
```

### Find Points Within Region

```sql
SELECT * FROM points
WHERE ST_Within(geom, (
    SELECT geom FROM regions WHERE name = 'Zone A'
));
```

### Classify Points by Region

```sql
SELECT p.id, r.name as region
FROM points p
LEFT JOIN regions r ON ST_Within(p.geom, r.geom);
```

## Notes

- A is within B means A is completely inside B
- ST_Within(A, B) is equivalent to ST_Contains(B, A)
- Point on polygon boundary returns false (use ST_CoveredBy for inclusive)
- A geometry is within itself only if they're identical

## See Also

- [ST_Contains](./st-contains) - Inverse relationship
- [ST_CoveredBy](./st-coveredby) - Within including boundary
- [ST_Intersects](./st-intersects) - Any overlap
