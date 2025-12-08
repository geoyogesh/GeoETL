---
sidebar_position: 5
title: ST_IsRing
description: Test if geometry is a ring
---

# ST_IsRing

Tests whether a LineString is a ring - closed and simple (no self-intersections).

## Syntax

```sql
ST_IsRing(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | LineString to test |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometry is a ring, false otherwise |

## Examples

### Valid Ring

```sql
SELECT ST_IsRing(ST_GeomFromText('LINESTRING(0 0, 4 0, 4 4, 0 4, 0 0)'));
-- Returns: true
```

### Not Closed (Not a Ring)

```sql
SELECT ST_IsRing(ST_GeomFromText('LINESTRING(0 0, 4 0, 4 4)'));
-- Returns: false (not closed)
```

### Self-Intersecting (Not a Ring)

```sql
SELECT ST_IsRing(ST_GeomFromText('LINESTRING(0 0, 2 2, 2 0, 0 2, 0 0)'));
-- Returns: false (self-intersects)
```

### Find Valid Polygon Candidates

```sql
SELECT id FROM line_features
WHERE ST_IsRing(geom);
```

## Notes

- A ring is a LineString that is both closed AND simple
- ST_IsRing = ST_IsClosed AND ST_IsSimple
- Rings can be used to construct valid polygons
- Only applies to LineStrings

## See Also

- [ST_IsClosed](./st-isclosed) - Test if LineString is closed
- [ST_IsSimple](./st-issimple) - Test if geometry is simple
- [ST_Boundary](../generator/st-boundary) - Get boundary of geometry
