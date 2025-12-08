---
sidebar_position: 4
title: ST_IsClosed
description: Test if geometry is closed
---

# ST_IsClosed

Tests whether a LineString or MultiLineString is closed (start point equals end point).

## Syntax

```sql
ST_IsClosed(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | LineString or MultiLineString to test |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometry is closed, false otherwise |

## Examples

### Closed LineString (Ring)

```sql
SELECT ST_IsClosed(ST_GeomFromText('LINESTRING(0 0, 4 0, 4 4, 0 4, 0 0)'));
-- Returns: true
```

### Open LineString

```sql
SELECT ST_IsClosed(ST_GeomFromText('LINESTRING(0 0, 4 0, 4 4)'));
-- Returns: false
```

### Find Unclosed Boundaries

```sql
SELECT id FROM boundaries
WHERE NOT ST_IsClosed(geom);
```

### Convert to Polygon

```sql
SELECT id, ST_GeomFromText('POLYGON((' || coords || '))')
FROM (
    SELECT id, coords FROM line_features
    WHERE ST_IsClosed(geom)
);
```

## Notes

- Only applies to LineStrings and MultiLineStrings
- For MultiLineString, all component lines must be closed
- Points and Polygons return NULL or error
- A closed LineString is not necessarily a valid ring (could self-intersect)

## See Also

- [ST_IsRing](./st-isring) - Test if LineString is a valid ring
- [ST_IsSimple](./st-issimple) - Test for self-intersection
- [ST_Boundary](../generator/st-boundary) - Get boundary of geometry
