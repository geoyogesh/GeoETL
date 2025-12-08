---
sidebar_position: 2
title: ST_Contains
description: Test if geometry A contains geometry B
---

# ST_Contains

Tests whether geometry A completely contains geometry B.

## Syntax

```sql
ST_Contains(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | Container geometry |
| geometry_b | Geometry | Contained geometry |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if A contains B, false otherwise |

## Examples

### Test Point in Polygon

```sql
SELECT ST_Contains(
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'),
    ST_Point(2, 2)
);
-- Returns: true
```

### Find Points in Region

```sql
SELECT p.id, p.name
FROM points p, regions r
WHERE r.name = 'District A'
  AND ST_Contains(r.geom, p.geom);
```

### Filter Features Within Boundary

```sql
SELECT * FROM buildings
WHERE ST_Contains(
    (SELECT geom FROM boundaries WHERE name = 'City Limits'),
    geom
);
```

## Notes

- A contains B means B is completely inside A (including on boundary)
- ST_Contains(A, B) is equivalent to ST_Within(B, A)
- Returns false if B extends outside A or if A equals B with matching boundaries
- Point on polygon boundary returns false (use ST_Covers for inclusive boundary)

## See Also

- [ST_Within](./st-within) - Inverse relationship
- [ST_Covers](./st-covers) - Contains including boundary
- [ST_Intersects](./st-intersects) - Any overlap
