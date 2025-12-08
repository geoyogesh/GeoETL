---
sidebar_position: 3
title: ST_IsSimple
description: Test if geometry is simple
---

# ST_IsSimple

Tests whether a geometry is simple - it has no self-intersections or self-tangencies.

## Syntax

```sql
ST_IsSimple(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry to test |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometry is simple, false otherwise |

## Examples

### Simple LineString

```sql
SELECT ST_IsSimple(ST_GeomFromText('LINESTRING(0 0, 1 1, 2 0)'));
-- Returns: true
```

### Self-Intersecting LineString

```sql
SELECT ST_IsSimple(ST_GeomFromText('LINESTRING(0 0, 2 2, 2 0, 0 2)'));
-- Returns: false (crosses itself)
```

### Point is Always Simple

```sql
SELECT ST_IsSimple(ST_Point(0, 0));
-- Returns: true
```

### Find Non-Simple Features

```sql
SELECT id FROM roads
WHERE NOT ST_IsSimple(geom);
```

## Notes

- Points are always simple
- LineStrings are simple if they don't cross themselves (except at endpoints for rings)
- Polygons are simple if they don't have self-intersecting rings
- Simple is a prerequisite for being valid

## See Also

- [ST_IsValid](./st-isvalid) - Test geometry validity
- [ST_IsRing](./st-isring) - Test if LineString is a ring
