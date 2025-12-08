---
sidebar_position: 3
title: ST_NumPoints
description: Count of points in geometry
---

# ST_NumPoints

Returns the number of points (vertices) in a geometry.

## Syntax

```sql
ST_NumPoints(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Int64 | Number of points in the geometry |

## Examples

### Points in LineString

```sql
SELECT ST_NumPoints(ST_GeomFromText('LINESTRING(0 0, 1 1, 2 0)'));
-- Returns: 3
```

### Points in Polygon

```sql
SELECT ST_NumPoints(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'));
-- Returns: 5 (includes closing point)
```

### Measure Complexity

```sql
SELECT id, ST_NumPoints(geom) as vertex_count
FROM roads
ORDER BY vertex_count DESC;
```

### Compare Original and Simplified

```sql
SELECT
    ST_NumPoints(geom) as original,
    ST_NumPoints(ST_Simplify(geom, 0.01)) as simplified
FROM coastlines;
```

## Notes

- Point geometry returns 1
- Polygon rings count the closing point separately
- Useful for measuring geometry complexity
- Higher point counts may indicate need for simplification

## See Also

- [ST_NumGeometries](./st-numgeometries) - Count geometries in collection
- [ST_Simplify](../generator/st-simplify) - Reduce point count
- [ST_Length](../measurement/st-length) - Measure length
