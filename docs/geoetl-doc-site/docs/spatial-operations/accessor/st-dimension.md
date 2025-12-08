---
sidebar_position: 6
title: ST_Dimension
description: Get topological dimension of geometry
---

# ST_Dimension

Returns the topological dimension of a geometry.

## Syntax

```sql
ST_Dimension(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Int32 | Dimension value (0, 1, or 2) |

## Examples

### Dimension of Point

```sql
SELECT ST_Dimension(ST_Point(0, 0));
-- Returns: 0
```

### Dimension of LineString

```sql
SELECT ST_Dimension(ST_GeomFromText('LINESTRING(0 0, 1 1)'));
-- Returns: 1
```

### Dimension of Polygon

```sql
SELECT ST_Dimension(ST_GeomFromText('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))'));
-- Returns: 2
```

### Group by Dimension

```sql
SELECT ST_Dimension(geom) as dim, COUNT(*) as count
FROM features
GROUP BY ST_Dimension(geom);
```

### Filter Areal Features

```sql
SELECT * FROM mixed_features
WHERE ST_Dimension(geom) = 2;
```

## Notes

- Dimension 0: Point geometries (no length, no area)
- Dimension 1: Line geometries (have length, no area)
- Dimension 2: Polygon geometries (have area)
- For Multi* types, returns the dimension of the component type
- For GeometryCollection, returns the maximum dimension of components

## See Also

- [ST_GeometryType](./st-geometrytype) - Get type name
- [ST_Area](../measurement/st-area) - Calculate area (for dimension 2)
- [ST_Length](../measurement/st-length) - Calculate length (for dimension 1)
