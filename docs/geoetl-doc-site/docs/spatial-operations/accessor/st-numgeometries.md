---
sidebar_position: 4
title: ST_NumGeometries
description: Count of geometries in collection
---

# ST_NumGeometries

Returns the number of geometries in a geometry collection or Multi* geometry.

## Syntax

```sql
ST_NumGeometries(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Int64 | Number of component geometries |

## Examples

### Count in MultiPolygon

```sql
SELECT ST_NumGeometries(ST_GeomFromText(
    'MULTIPOLYGON(((0 0, 1 0, 1 1, 0 1, 0 0)), ((2 2, 3 2, 3 3, 2 3, 2 2)))'
));
-- Returns: 2
```

### Count in GeometryCollection

```sql
SELECT ST_NumGeometries(ST_GeomFromText(
    'GEOMETRYCOLLECTION(POINT(0 0), LINESTRING(1 1, 2 2), POLYGON((3 3, 4 3, 4 4, 3 4, 3 3)))'
));
-- Returns: 3
```

### Single Geometry

```sql
SELECT ST_NumGeometries(ST_GeomFromText('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))'));
-- Returns: 1
```

### Find Multi-Part Features

```sql
SELECT id, ST_NumGeometries(geom) as part_count
FROM parcels
WHERE ST_NumGeometries(geom) > 1;
```

## Notes

- Single geometries (Point, LineString, Polygon) return 1
- Multi* geometries return count of components
- GeometryCollections return count of contained geometries
- Useful for identifying multi-part features

## See Also

- [ST_NumPoints](./st-numpoints) - Count vertices
- [ST_GeometryType](./st-geometrytype) - Get geometry type
